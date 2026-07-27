use tonic::transport::{Channel, Endpoint};
use std::time::Duration;
use common::AppResult;

use proto::auth::auth_service_client::AuthServiceClient;
use proto::product::product_service_client::ProductServiceClient;
use proto::order::order_service_client::OrderServiceClient;
use proto::inventory::inventory_service_client::InventoryServiceClient;

#[derive(Clone)]
pub struct GrpcClients {
    pub auth: AuthServiceClient<Channel>,
    pub product: ProductServiceClient<Channel>,
    pub order: OrderServiceClient<Channel>,
    pub inventory: InventoryServiceClient<Channel>,
}

async fn create_channel(addr: &str, timeout: Duration) -> AppResult<Channel> {
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
        "Connection failed after 10 retries: {:?}", last_err
    )))
}

impl GrpcClients {
    pub async fn new(
        auth_addr: String,
        product_addr: String,
        order_addr: String,
        inventory_addr: String,
        grpc_timeout: Duration,
    ) -> AppResult<Self> {
        let auth_channel = create_channel(&auth_addr, grpc_timeout).await?;
        let product_channel = create_channel(&product_addr, grpc_timeout).await?;
        let order_channel = create_channel(&order_addr, grpc_timeout).await?;
        let inventory_channel = create_channel(&inventory_addr, grpc_timeout).await?;

        Ok(Self {
            auth: AuthServiceClient::new(auth_channel),
            product: ProductServiceClient::new(product_channel),
            order: OrderServiceClient::new(order_channel),
            inventory: InventoryServiceClient::new(inventory_channel),
        })
    }
}
