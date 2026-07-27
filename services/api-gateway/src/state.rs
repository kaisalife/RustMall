#[derive(Clone)]
pub struct AppState {
    pub config: common::AppConfig,
    pub clients: crate::grpc_clients::GrpcClients,
    pub cache: Option<redis_cache::RedisCache>,
}

impl AppState {
    pub fn new(
        config: common::AppConfig,
        clients: crate::grpc_clients::GrpcClients,
        cache: Option<redis_cache::RedisCache>,
    ) -> Self {
        Self { config, clients, cache }
    }
}
