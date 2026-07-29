use common::{AppResult, DatabaseConfig};

#[derive(Clone)]
pub struct DatabaseConnection {
    pub pool: sqlx::PgPool,
}

impl DatabaseConnection {
    pub async fn new(config: &DatabaseConfig) -> AppResult<Self> {
        // 创建连接池并自动执行数据库迁移
        let pool = db_migration::setup_database(config).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
