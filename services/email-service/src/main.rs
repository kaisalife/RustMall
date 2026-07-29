//! 邮件服务入口

mod application;
mod domain;
mod infrastructure;
mod interface;

use common::{init_tracing, load_config, SnowflakeIdGenerator};
use interface::EmailServiceImpl;
use proto::email::email_service_server::EmailServiceServer;
use service_discovery::{NacosConfig, NacosRegistry, ServiceInstance, ServiceRegistry};
use std::sync::Arc;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = load_config()?;

    // 初始化日志（含 OpenTelemetry 分布式追踪）
    init_tracing(
        "email-service",
        config.tracing.otlp_endpoint.as_deref(),
        "email_service=debug,tonic=info",
    );

    tracing::info!("========================================");
    tracing::info!("  Simple Trade - Email Service");
    tracing::info!("========================================");

    let addr = format!(
        "{}:{}",
        config.email_service.host, config.email_service.port
    )
    .parse()?;

    tracing::info!("启动 Email Service 监听：{}", addr);

    // 初始化数据库连接池
    let pool = db_migration::setup_database(&config.database).await?;

    // 初始化 ID 生成器
    let id_generator = Arc::new(SnowflakeIdGenerator::new(config.email_service.worker_id)?);

    // 初始化邮件仓库
    let email_repository = Arc::new(infrastructure::EmailRepositoryImpl::new(pool));

    // 初始化邮件发送器
    let email_sender = if cfg!(debug_assertions) {
        tracing::info!("开发模式：使用模拟邮件发送器");
        Arc::new(infrastructure::EmailSender::new_dev(
            config.email.from_address.clone(),
        ))
    } else {
        tracing::info!("生产模式：使用真实 SMTP 发送器");
        Arc::new(infrastructure::EmailSender::new(
            &config.email.smtp_host,
            config.email.smtp_port,
            config.email.smtp_username.clone(),
            config.email.smtp_password.clone(),
            config.email.from_address.clone(),
        )?)
    };

    // 初始化应用服务
    let email_service =
        application::EmailApplicationService::new(id_generator, email_repository, email_sender);

    // 创建 gRPC 服务
    let email_service_impl = EmailServiceImpl::new(email_service);

    tracing::info!("");
    tracing::info!("========================================");
    tracing::info!("  Email Service 启动成功 🚀");
    tracing::info!("  监听地址：{}", addr);
    tracing::info!("========================================");

    // 在 gRPC server 启动前注册到 Nacos
    if config.nacos.enabled {
        let nacos_config = NacosConfig::from(config.nacos.clone());
        match NacosRegistry::new(&nacos_config).await {
            Ok(registry) => {
                let instance = ServiceInstance::new(
                    "email-service",
                    config.email_service.advertise_ip(),
                    config.email_service.port,
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

    // 启动服务
    Server::builder()
        .add_service(EmailServiceServer::new(email_service_impl))
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
