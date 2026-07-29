use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::{AppError, AppResult, DatabaseConfig};

pub async fn create_pool(config: &DatabaseConfig) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(config.idle_timeout_minutes * 60))
        .max_lifetime(Duration::from_secs(config.max_lifetime_minutes * 60))
        .connect(&config.connection_string())
        .await
        .map_err(AppError::Database)?;
    Ok(pool)
}
