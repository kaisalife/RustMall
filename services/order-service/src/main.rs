mod application;
mod domain;
mod infrastructure;
mod interface;

use common::{init_tracing, load_config, SnowflakeIdGenerator};
use infrastructure::{DatabaseConnection, OrderRepositoryImpl};
use interface::OrderServiceImpl;
use proto::order::order_service_server::OrderServiceServer;
use service_discovery::{NacosConfig, NacosRegistry, ServiceInstance, ServiceRegistry};
use std::sync::Arc;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;

    init_tracing(
        "order-service",
        config.tracing.otlp_endpoint.as_deref(),
        "order_service=debug,tonic=info",
    );

    let addr = format!(
        "{}:{}",
        config.order_service.host, config.order_service.port
    )
    .parse()?;

    tracing::info!("Order Service starting on {}", addr);

    let db = DatabaseConnection::new(&config.database).await?;
    let id_generator = Arc::new(
        SnowflakeIdGenerator::new(config.order_service.worker_id)
            .expect("Failed to create ID generator"),
    );
    let order_repository = Arc::new(OrderRepositoryImpl::new(
        db.pool().clone(),
        id_generator.clone(),
    ));

    // 初始化 Kafka 事件生产者
    let event_producer = match event_bus::EventBusProducer::new(
        &config.kafka.brokers,
        "order-service",
        &config.kafka.topic_prefix,
        id_generator.clone(),
    ) {
        Ok(producer) => {
            tracing::info!("Kafka event producer initialized");
            Some(producer)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to init Kafka producer, events will not be published: {}",
                e
            );
            None
        }
    };

    let mut order_service =
        application::OrderApplicationService::new(order_repository, id_generator);
    if let Some(producer) = event_producer {
        order_service = order_service.with_event_producer(producer);
    }
    let order_service_impl = OrderServiceImpl::new(order_service);

    tracing::info!("Order Service started successfully");

    // 在 gRPC server 启动前注册到 Nacos
    if config.nacos.enabled {
        let nacos_config = NacosConfig::from(config.nacos.clone());
        match NacosRegistry::new(&nacos_config).await {
            Ok(registry) => {
                let instance = ServiceInstance::new(
                    "order-service",
                    config.order_service.advertise_ip(),
                    config.order_service.port,
                );
                if let Err(e) = registry.register(instance).await {
                    tracing::warn!("Failed to register to Nacos: {}, service will start anyway", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Nacos: {}, service will start anyway", e);
            }
        }
    }

    Server::builder()
        .add_service(OrderServiceServer::new(order_service_impl))
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
