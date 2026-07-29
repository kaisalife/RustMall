//! gRPC 客户端管理
//!
//! 支持两种创建模式：
//! - Nacos 动态服务发现（负载均衡，生产环境）
//! - 静态地址直连（开发环境 fallback）
//!
//! 每个后端服务配备独立熔断器，通过 `call_*` 方法自动在调用前后检查/记录状态。

use std::future::Future;
use std::time::Duration;

use tonic::transport::{Channel, Endpoint};

use proto::auth::auth_service_client::AuthServiceClient;
use proto::inventory::inventory_service_client::InventoryServiceClient;
use proto::order::order_service_client::OrderServiceClient;
use proto::product::product_service_client::ProductServiceClient;

use service_discovery::{NacosRegistry, ServiceRegistry};

use crate::circuit_breaker::CircuitBreaker;

/// 静态地址配置（Nacos 未启用时的 fallback）
#[derive(Debug, Clone)]
pub struct StaticAddrs {
    /// 认证服务地址（host:port）
    pub auth: String,
    /// 商品服务地址（host:port）
    pub product: String,
    /// 订单服务地址（host:port）
    pub order: String,
    /// 库存服务地址（host:port）
    pub inventory: String,
}

/// gRPC 客户端集合
///
/// 每个后端服务对应一个 tonic Client 和一个 CircuitBreaker。
/// 通过 `call_auth`/`call_product`/`call_order`/`call_inventory` 方法
/// 在 gRPC 调用前后自动检查和记录熔断器状态。
#[derive(Clone)]
pub struct GrpcClients {
    /// 认证服务客户端
    pub auth: AuthServiceClient<Channel>,
    /// 商品服务客户端
    pub product: ProductServiceClient<Channel>,
    /// 订单服务客户端
    pub order: OrderServiceClient<Channel>,
    /// 库存服务客户端
    pub inventory: InventoryServiceClient<Channel>,
    /// 认证服务熔断器
    pub auth_cb: CircuitBreaker,
    /// 商品服务熔断器
    pub product_cb: CircuitBreaker,
    /// 订单服务熔断器
    pub order_cb: CircuitBreaker,
    /// 库存服务熔断器
    pub inventory_cb: CircuitBreaker,
}

/// 判断 gRPC 错误是否为服务级故障（应计入熔断器失败计数）
///
/// 业务逻辑错误（NotFound、InvalidArgument 等）不计入熔断，
/// 仅服务不可用、超时、内部错误等才递增失败计数。
fn is_service_error(status: &tonic::Status) -> bool {
    matches!(
        status.code(),
        tonic::Code::Unavailable | tonic::Code::DeadlineExceeded | tonic::Code::Internal
    )
}

// ---------------------------------------------------------------------------
// Channel 创建
// ---------------------------------------------------------------------------

/// 从 Nacos 发现服务实例并创建负载均衡 Channel
///
/// 重试发现服务（等待后端注册完成），使用 tonic 内置 round-robin 负载均衡。
async fn create_channel_from_nacos(
    registry: &NacosRegistry,
    service_name: &str,
    timeout: Duration,
) -> common::AppResult<Channel> {
    let mut last_err = None;
    for attempt in 0..15 {
        match registry.discover(service_name).await {
            Ok(instances) if !instances.is_empty() => {
                let endpoints: Vec<Endpoint> = instances
                    .iter()
                    .map(|inst| {
                        Endpoint::from_shared(format!("http://{}", inst.address())).map(|e| {
                            e.timeout(timeout)
                                .connect_timeout(Duration::from_secs(3))
                                .tcp_keepalive(Some(Duration::from_secs(30)))
                                .http2_keep_alive_interval(Duration::from_secs(30))
                                .keep_alive_timeout(Duration::from_secs(10))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| common::AppError::grpc(format!("Invalid endpoint: {}", e)))?;

                tracing::info!(
                    "Discovered {} instances for {}: {:?}",
                    instances.len(),
                    service_name,
                    instances.iter().map(|i| i.address()).collect::<Vec<_>>()
                );
                return Ok(Channel::balance_list(endpoints.into_iter()));
            }
            Ok(_) => {
                tracing::warn!(
                    "No instances found for {} (attempt {})",
                    service_name,
                    attempt + 1
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Discovery failed for {} (attempt {}): {}",
                    service_name,
                    attempt + 1,
                    e
                );
                last_err = Some(e);
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(common::AppError::grpc(format!(
        "Service discovery failed for {} after 15 retries: {:?}",
        service_name, last_err
    )))
}

/// 从静态地址创建 Channel（fallback，Nacos 未启用时使用）
async fn create_channel(addr: &str, timeout: Duration) -> common::AppResult<Channel> {
    let endpoint = Endpoint::from_shared(format!("http://{}", addr))
        .map_err(|e| common::AppError::grpc(format!("Invalid endpoint: {}", e)))?
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(3))
        .tcp_keepalive(Some(Duration::from_secs(30)))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .keep_alive_timeout(Duration::from_secs(10));

    // 重试连接，最多 10 次（等待后端服务就绪）
    let mut last_err = None;
    for i in 0..10 {
        match endpoint.connect().await {
            Ok(channel) => {
                tracing::info!("Connected to {} on attempt {}", addr, i + 1);
                return Ok(channel);
            }
            Err(e) => {
                tracing::warn!("Connection attempt {} to {} failed: {}", i + 1, addr, e);
                last_err = Some(e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Err(common::AppError::grpc(format!(
        "Connection failed after 10 retries: {:?}",
        last_err
    )))
}

impl GrpcClients {
    /// 从 Nacos 服务发现创建客户端（生产模式）
    ///
    /// 通过 `Channel::balance_list` 实现多实例 round-robin 负载均衡。
    pub async fn new_from_nacos(
        registry: &NacosRegistry,
        grpc_timeout: Duration,
    ) -> common::AppResult<Self> {
        let auth_channel =
            create_channel_from_nacos(registry, "auth-service", grpc_timeout).await?;
        let product_channel =
            create_channel_from_nacos(registry, "product-service", grpc_timeout).await?;
        let order_channel =
            create_channel_from_nacos(registry, "order-service", grpc_timeout).await?;
        let inventory_channel =
            create_channel_from_nacos(registry, "inventory-service", grpc_timeout).await?;

        Ok(Self {
            auth: AuthServiceClient::new(auth_channel),
            product: ProductServiceClient::new(product_channel),
            order: OrderServiceClient::new(order_channel),
            inventory: InventoryServiceClient::new(inventory_channel),
            auth_cb: CircuitBreaker::default_cb(),
            product_cb: CircuitBreaker::default_cb(),
            order_cb: CircuitBreaker::default_cb(),
            inventory_cb: CircuitBreaker::default_cb(),
        })
    }

    /// 从静态地址创建客户端（开发模式 fallback）
    pub async fn new_from_static(
        addrs: StaticAddrs,
        grpc_timeout: Duration,
    ) -> common::AppResult<Self> {
        let auth_channel = create_channel(&addrs.auth, grpc_timeout).await?;
        let product_channel = create_channel(&addrs.product, grpc_timeout).await?;
        let order_channel = create_channel(&addrs.order, grpc_timeout).await?;
        let inventory_channel = create_channel(&addrs.inventory, grpc_timeout).await?;

        Ok(Self {
            auth: AuthServiceClient::new(auth_channel),
            product: ProductServiceClient::new(product_channel),
            order: OrderServiceClient::new(order_channel),
            inventory: InventoryServiceClient::new(inventory_channel),
            auth_cb: CircuitBreaker::default_cb(),
            product_cb: CircuitBreaker::default_cb(),
            order_cb: CircuitBreaker::default_cb(),
            inventory_cb: CircuitBreaker::default_cb(),
        })
    }

    // -----------------------------------------------------------------------
    // 熔断器保护的 gRPC 调用
    // -----------------------------------------------------------------------

    /// 执行带熔断器保护的 auth 服务调用
    ///
    /// 调用前检查熔断器状态，调用后记录成功/失败。
    /// 仅服务级错误（Unavailable/DeadlineExceeded/Internal）计入熔断。
    pub async fn call_auth<F, Fut, T>(&self, f: F) -> Result<T, tonic::Status>
    where
        F: FnOnce(AuthServiceClient<Channel>) -> Fut,
        Fut: Future<Output = Result<T, tonic::Status>>,
    {
        if !self.auth_cb.can_proceed() {
            return Err(tonic::Status::unavailable(
                "Circuit breaker open for auth service",
            ));
        }
        let client = self.auth.clone();
        match f(client).await {
            Ok(v) => {
                self.auth_cb.record_success();
                Ok(v)
            }
            Err(e) => {
                if is_service_error(&e) {
                    self.auth_cb.record_failure();
                }
                Err(e)
            }
        }
    }

    /// 执行带熔断器保护的 product 服务调用
    pub async fn call_product<F, Fut, T>(&self, f: F) -> Result<T, tonic::Status>
    where
        F: FnOnce(ProductServiceClient<Channel>) -> Fut,
        Fut: Future<Output = Result<T, tonic::Status>>,
    {
        if !self.product_cb.can_proceed() {
            return Err(tonic::Status::unavailable(
                "Circuit breaker open for product service",
            ));
        }
        let client = self.product.clone();
        match f(client).await {
            Ok(v) => {
                self.product_cb.record_success();
                Ok(v)
            }
            Err(e) => {
                if is_service_error(&e) {
                    self.product_cb.record_failure();
                }
                Err(e)
            }
        }
    }

    /// 执行带熔断器保护的 order 服务调用
    pub async fn call_order<F, Fut, T>(&self, f: F) -> Result<T, tonic::Status>
    where
        F: FnOnce(OrderServiceClient<Channel>) -> Fut,
        Fut: Future<Output = Result<T, tonic::Status>>,
    {
        if !self.order_cb.can_proceed() {
            return Err(tonic::Status::unavailable(
                "Circuit breaker open for order service",
            ));
        }
        let client = self.order.clone();
        match f(client).await {
            Ok(v) => {
                self.order_cb.record_success();
                Ok(v)
            }
            Err(e) => {
                if is_service_error(&e) {
                    self.order_cb.record_failure();
                }
                Err(e)
            }
        }
    }

    /// 执行带熔断器保护的 inventory 服务调用
    pub async fn call_inventory<F, Fut, T>(&self, f: F) -> Result<T, tonic::Status>
    where
        F: FnOnce(InventoryServiceClient<Channel>) -> Fut,
        Fut: Future<Output = Result<T, tonic::Status>>,
    {
        if !self.inventory_cb.can_proceed() {
            return Err(tonic::Status::unavailable(
                "Circuit breaker open for inventory service",
            ));
        }
        let client = self.inventory.clone();
        match f(client).await {
            Ok(v) => {
                self.inventory_cb.record_success();
                Ok(v)
            }
            Err(e) => {
                if is_service_error(&e) {
                    self.inventory_cb.record_failure();
                }
                Err(e)
            }
        }
    }
}
