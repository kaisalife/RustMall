mod application;
mod domain;
mod infrastructure;
mod interface;

use std::sync::Arc;

use common::{init_tracing, load_config};
use infrastructure::{DatabaseConnection, InventoryRepositoryImpl};
use interface::InventoryServiceImpl;
use proto::inventory::inventory_service_server::InventoryServiceServer;
use service_discovery::{NacosConfig, NacosRegistry, ServiceInstance, ServiceRegistry};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = load_config()?;

    // 初始化日志（含 OpenTelemetry 分布式追踪）
    init_tracing(
        "inventory-service",
        config.tracing.otlp_endpoint.as_deref(),
        "inventory_service=debug,tonic=info",
    );

    let addr = format!(
        "{}:{}",
        config.inventory_service.host, config.inventory_service.port
    )
    .parse()?;

    tracing::info!("Inventory Service starting on {}", addr);

    // 初始化数据库连接
    let db = DatabaseConnection::new(&config.database).await?;

    // 初始化仓储
    let inventory_repository = Arc::new(InventoryRepositoryImpl::new(db.pool().clone()));

    // 初始化应用服务
    let inventory_service = application::InventoryApplicationService::new(inventory_repository);

    // 创建gRPC服务
    let inventory_service_impl = InventoryServiceImpl::new(inventory_service.clone());

    // 启动 Kafka 事件消费者（消费 OrderCreated 事件，异步扣减库存）
    let order_created_topic = format!("{}.order_created", config.kafka.topic_prefix);
    match event_bus::EventBusConsumer::new(&config.kafka.brokers, "inventory-service") {
        Ok(consumer) => {
            if let Err(e) = consumer.subscribe(&[&order_created_topic]) {
                tracing::warn!("Failed to subscribe to {}: {}", order_created_topic, e);
            } else {
                tracing::info!("Kafka consumer subscribed to {}", order_created_topic);
                let svc = inventory_service.clone();
                tokio::spawn(async move {
                    use tokio_stream::StreamExt;
                    let mut stream = consumer.stream();
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(msg) => {
                                if let Ok(envelope) = event_bus::consumer::parse_event(&msg) {
                                    if let event_bus::EventPayload::OrderCreated {
                                        order_id,
                                        items,
                                        ..
                                    } = envelope.payload
                                    {
                                        tracing::info!(
                                            order_id,
                                            items = items.len(),
                                            "Processing OrderCreated event"
                                        );
                                        for item in items {
                                            use application::command::DeductStockCommand;
                                            let cmd = DeductStockCommand {
                                                product_id: item.product_id,
                                                quantity: item.quantity,
                                            };
                                            if let Err(e) = svc.deduct_stock(cmd).await {
                                                tracing::error!(
                                                    order_id,
                                                    product_id = item.product_id,
                                                    "Failed to deduct stock: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Kafka consume error: {}", e);
                            }
                        }
                    }
                });
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to init Kafka consumer, stock deduction will not work: {}",
                e
            );
        }
    }

    // 启动服务
    tracing::info!("Inventory Service started successfully");

    // 在 gRPC server 启动前注册到 Nacos
    if config.nacos.enabled {
        let nacos_config = NacosConfig::from(config.nacos.clone());
        match NacosRegistry::new(&nacos_config).await {
            Ok(registry) => {
                let instance = ServiceInstance::new(
                    "inventory-service",
                    config.inventory_service.advertise_ip(),
                    config.inventory_service.port,
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
        .add_service(InventoryServiceServer::new(inventory_service_impl))
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
