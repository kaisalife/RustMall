use common::{AppResult, DatabaseConfig, create_pool};

#[derive(Clone)]
pub struct DatabaseConnection {
    pub pool: sqlx::PgPool,
}

impl DatabaseConnection {
    pub async fn new(config: &DatabaseConfig) -> AppResult<Self> {
        let pool = create_pool(config).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
