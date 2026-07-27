use sqlx::PgPool;
use common::{AppResult, DatabaseConfig, create_pool};

#[derive(Clone)]
pub struct DatabaseConnection {
    pub pool: PgPool,
}

impl DatabaseConnection {
    /// Create database connection and run migrations
    pub async fn new_with_migration(config: &DatabaseConfig) -> AppResult<Self> {
        let pool = db_migration::setup_database(config).await?;
        Ok(Self { pool })
    }

    /// Create database connection without running migrations
    pub async fn new(config: &DatabaseConfig) -> AppResult<Self> {
        let pool = create_pool(config).await?;

        tracing::info!("✅ Database connection established");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
