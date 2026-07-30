//! 支付服务（payment-service）启动入口。
//!
//! 启动流程：
//! 1. 加载配置（figment，读取 config/base.toml + 环境变量）
//! 2. 初始化 tracing 日志（含 OpenTelemetry 分布式追踪）
//! 3. 创建 PostgreSQL 连接池
//! 4. 依赖注入：构建数据库封装、仓储、ID 生成器、渠道适配器、
//!    应用服务（内部自建幂等服务与渠道路由）、gRPC 服务实现
//! 5. 启动 Tonic gRPC server，监听配置指定的地址
//! 6. 等待优雅关闭信号（Ctrl+C / SIGTERM）

// payment-service 尚在开发中，部分领域模型与应用服务方法尚未接入调用链。
#![allow(dead_code)]

mod application;
mod domain;
mod infrastructure;
mod interface;

use std::sync::Arc;

use common::{init_tracing, load_config, AppError, SnowflakeIdGenerator};
use infrastructure::{
    PaymentChannelAdapter, PaymentDatabase, PgPaymentRepository, PgRefundRepository,
    PgTransactionRepository, StubChannelAdapter,
};
use interface::PaymentServiceImpl;
use proto::payment::v1::payment_service_server::PaymentServiceServer;
use service_discovery::{NacosConfig, NacosRegistry, ServiceInstance, ServiceRegistry};
use tonic::transport::Server;

use application::PaymentApplicationService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置（数据库、tracing 等）
    let config = load_config()?;

    // 2. 初始化日志（含 OpenTelemetry 分布式追踪）
    let filter = if cfg!(debug_assertions) {
        "payment_service=debug,tonic=info"
    } else {
        "payment_service=info,tonic=info"
    };
    init_tracing(
        "payment-service",
        config.tracing.otlp_endpoint.as_deref(),
        filter,
    );

    let addr = format!(
        "{}:{}",
        config.payment_service.host, config.payment_service.port
    )
    .parse()
    .map_err(|e| AppError::Config(format!("Invalid payment service address: {}", e)))?;

    tracing::info!("Payment Service starting on {}", addr);

    // 3. 创建数据库连接池（含自动迁移），并包装为 PaymentDatabase
    let pool = db_migration::setup_database(&config.database).await?;
    let db = PaymentDatabase::new(pool);

    // 4. 依赖注入
    // 4.1 仓储实现（共享同一连接池）
    let payment_repository = Arc::new(PgPaymentRepository::new(db.pool().clone()));
    let transaction_repository = Arc::new(PgTransactionRepository::new(db.pool().clone()));
    let refund_repository = Arc::new(PgRefundRepository::new(db.pool().clone()));

    // 4.2 雪花 ID 生成器（用于支付订单、流水、退款单主键）
    let id_generator = Arc::new(
        SnowflakeIdGenerator::new(config.payment_service.worker_id)
            .expect("Failed to create ID generator"),
    );

    // 4.3 渠道适配器：开发环境使用测试桩，生产环境替换为真实适配器
    //     （WeChatPayAdapter / AlipayAdapter），由应用服务内部的路由器按渠道路由。
    let channel_adapter: Arc<dyn PaymentChannelAdapter> = if cfg!(debug_assertions) {
        tracing::warn!("Using StubChannelAdapter in debug build - payments will be simulated");
        Arc::new(StubChannelAdapter::new())
    } else {
        // 生产环境使用 StubChannelAdapter 作为占位
        // 替换为真实适配器：WeChatPayAdapter / AlipayAdapter
        tracing::warn!("Using StubChannelAdapter in release build - replace with real adapter before production!");
        Arc::new(StubChannelAdapter::new())
    };

    // 4.4 应用服务：编排仓储、幂等、路由、渠道、ID 生成
    //     幂等服务（IdempotencyService）与渠道路由（PaymentRouter）在应用服务内部构造，
    //     构造签名为 (payment_repo, txn_repo, refund_repo, channel_adapter, id_generator)。
    let payment_service = Arc::new(PaymentApplicationService::new(
        payment_repository,
        transaction_repository,
        refund_repository,
        channel_adapter,
        id_generator,
    ));

    // 4.5 gRPC 服务实现
    let payment_service_impl = PaymentServiceImpl::new(payment_service);

    tracing::info!("Payment Service started successfully");

    // 在 gRPC server 启动前注册到 Nacos
    if config.nacos.enabled {
        let nacos_config = NacosConfig::from(config.nacos.clone());
        match NacosRegistry::new(&nacos_config).await {
            Ok(registry) => {
                let instance = ServiceInstance::new(
                    "payment-service",
                    config.payment_service.advertise_ip(),
                    config.payment_service.port,
                );
                if let Err(e) = registry.register(instance).await {
                    tracing::warn!(
                        "Failed to register to Nacos: {}, service will start anyway",
                        e
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to Nacos: {}, service will start anyway",
                    e
                );
            }
        }
    }

    // 5. 启动 gRPC server，注册优雅关闭
    Server::builder()
        .add_service(PaymentServiceServer::with_interceptor(
            payment_service_impl,
            tower_middleware::TraceContextExtractor,
        ))
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;

    Ok(())
}

/// 优雅关闭信号处理。
///
/// 监听 Ctrl+C（所有平台）与 SIGTERM（Unix），
/// 收到信号后通知 gRPC server 进入关闭流程，完成在途请求后退出。
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}
