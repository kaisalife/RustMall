#[derive(Clone)]
pub struct AppState {
    pub config: common::AppConfig,
    pub clients: crate::grpc_clients::GrpcClients,
    pub cache: Option<redis_cache::RedisCache>,
    /// 数据库连接池（用于审计日志查询等需要直连 PostgreSQL 的场景）
    pub db_pool: Option<sqlx::PgPool>,
}

impl AppState {
    pub fn new(
        config: common::AppConfig,
        clients: crate::grpc_clients::GrpcClients,
        cache: Option<redis_cache::RedisCache>,
        db_pool: Option<sqlx::PgPool>,
    ) -> Self {
        Self {
            config,
            clients,
            cache,
            db_pool,
        }
    }
}
