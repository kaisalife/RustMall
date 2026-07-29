mod application;
mod domain;
mod infrastructure;
mod interface;

use std::sync::Arc;

use common::{init_tracing, load_config, SnowflakeIdGenerator};
use infrastructure::{CategoryRepositoryImpl, DatabaseConnection, ProductRepositoryImpl};
use interface::ProductServiceImpl;
use proto::product::product_service_server::ProductServiceServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = load_config()?;

    // 初始化日志（含 OpenTelemetry 分布式追踪）
    init_tracing(
        "product-service",
        config.tracing.otlp_endpoint.as_deref(),
        "product_service=debug,tonic=info",
    );

    let addr = format!(
        "{}:{}",
        config.product_service.host, config.product_service.port
    )
    .parse()?;

    tracing::info!("Product Service starting on {}", addr);

    // 初始化数据库连接
    let db = DatabaseConnection::new(&config.database).await?;

    // 初始化ID生成器
    let id_generator = Arc::new(
        SnowflakeIdGenerator::new(config.product_service.worker_id)
            .expect("Failed to create ID generator"),
    );

    // 初始化仓储
    let product_repository = Arc::new(ProductRepositoryImpl::new(db.pool().clone()));
    let category_repository = Arc::new(CategoryRepositoryImpl::new(db.pool().clone()));

    // 初始化应用服务
    let product_service = application::ProductApplicationService::new(
        product_repository,
        category_repository,
        id_generator,
    );

    // 创建gRPC服务
    let product_service_impl = ProductServiceImpl::new(product_service);

    // 启动服务
    tracing::info!("Product Service started successfully");

    Server::builder()
        .add_service(ProductServiceServer::new(product_service_impl))
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;

    Ok(())
}

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
