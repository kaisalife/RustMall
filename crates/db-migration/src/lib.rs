//! Database migration utilities
//!
//! This crate provides embedded database migrations using sqlx.
//! Migrations are embedded at compile time from the `./migrations` directory.

use sqlx::PgPool;
use tracing::{error, info};

use common::{AppError, AppResult, DatabaseConfig};

/// Run all pending database migrations
pub async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    info!("Running database migrations...");

    let migrator = sqlx::migrate!("./migrations");

    match migrator.run(pool).await {
        Ok(_) => {
            info!("✅ Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Database migration failed: {}", e);
            Err(AppError::Internal(format!("Migration failed: {}", e)))
        }
    }
}

/// Create database connection pool and run migrations
pub async fn setup_database(config: &DatabaseConfig) -> AppResult<PgPool> {
    let pool = common::create_pool(config).await?;

    info!("✅ Database connection established");

    run_migrations(&pool).await?;

    Ok(pool)
}
