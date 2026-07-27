mod domain;
mod infrastructure;
mod application;
mod interface;

use std::sync::Arc;
use std::env;

use common::{load_config, init_tracing, SnowflakeIdGenerator};
use infrastructure::{DatabaseConnection, UserRepositoryImpl};
use interface::AuthServiceImpl;
use proto::auth::auth_service_server::AuthServiceServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = load_config()?;

    // Initialize logging (with OpenTelemetry)
    init_tracing(
        "auth-service",
        config.tracing.otlp_endpoint.as_deref(),
        "auth_service=debug,tonic=info,sqlx=debug",
    );

    tracing::info!("========================================");
    tracing::info!("  Simple Trade - Auth Service");
    tracing::info!("========================================");

    let addr = format!("{}:{}", config.auth_service.host, config.auth_service.port).parse()?;

    tracing::info!("Starting Auth Service on {}", addr);

    // Check if we should run migrations (default: true)
    let run_migrations = env::var("SKIP_MIGRATIONS")
        .map(|v| v.to_lowercase() != "true")
        .unwrap_or(true);

    // Initialize database connection
    let db = if run_migrations {
        tracing::info!("Running database migrations...");
        DatabaseConnection::new_with_migration(&config.database).await?
    } else {
        tracing::info!("Skipping database migrations");
        DatabaseConnection::new(&config.database).await?
    };

    // Initialize ID generator
    let id_generator = Arc::new(SnowflakeIdGenerator::new(config.auth_service.worker_id)
        .expect("Failed to create ID generator"));

    // Initialize repositories
    let user_repository = Arc::new(UserRepositoryImpl::new(db.pool().clone()));

    // Initialize email service client
    let email_client = match infrastructure::EmailServiceClientWrapper::new(config.email_service.address()).await {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::warn!("Failed to connect to email service: {}", e);
            None
        }
    };

    // Initialize application service
    let auth_service = application::AuthApplicationService::new(
        user_repository,
        id_generator,
        Arc::new(config.jwt),
        email_client,
    );

    // Create gRPC service
    let auth_service_impl = AuthServiceImpl::new(auth_service);

    tracing::info!("");
    tracing::info!("========================================");
    tracing::info!("  Auth Service started successfully! 🚀");
    tracing::info!("  Listening on: {}", addr);
    tracing::info!("========================================");

    // Start serving
    Server::builder()
        .add_service(AuthServiceServer::new(auth_service_impl))
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
