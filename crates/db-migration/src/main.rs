//! Standalone database migration tool
//!
//! Usage:
//!   cargo run --bin migrate
//!   DATABASE_URL=postgres://... cargo run --bin migrate

use std::env;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use common::{load_config, AppResult};

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("========================================");
    info!("  Simple Trade - Database Migration Tool");
    info!("========================================");

    // Get database config from environment or config file
    let database_url = env::var("DATABASE_URL").ok();

    let pool = if let Some(url) = database_url {
        info!("Using DATABASE_URL from environment");
        let pool = sqlx::PgPool::connect(&url).await.map_err(|e| {
            error!("Failed to connect using DATABASE_URL: {}", e);
            common::AppError::Database(e)
        })?;
        db_migration::run_migrations(&pool).await?;
        pool
    } else {
        info!("Loading config from config/base.toml");
        let config = load_config()?;
        db_migration::setup_database(&config.database).await?
    };

    // Verify migrations were applied
    info!("");
    info!("Verifying migration status...");

    let applied: Vec<(i64, String)> =
        sqlx::query_as("SELECT version, description FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch migration status: {}", e);
                common::AppError::Database(e)
            })?;

    info!("");
    info!("✅ Applied Migrations:");
    for (version, description) in applied {
        info!("   V{} - {}", version, description);
    }

    info!("");
    info!("========================================");
    info!("  Migration completed successfully! 🎉");
    info!("========================================");

    Ok(())
}
