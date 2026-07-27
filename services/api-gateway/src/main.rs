use std::sync::Arc;

use axum::{
    middleware,
    Router,
};
use common::{load_config, init_tracing};
use metrics::{MetricsMiddleware, metrics_handler};
use tower_middleware::{
    auth::create_auth_middleware,
    rate_limit::{create_default_rate_limiter, create_rate_limit_middleware, create_strict_rate_limiter},
    logger::logger_middleware,
    create_cors_layer,
};

use api_gateway::routes::{
    auth_routes, user_routes, product_routes, order_routes, inventory_routes,
    health_check_handler, ping_handler, echo_handler,
};
use api_gateway::state::AppState;
use api_gateway::grpc_clients;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = load_config()?;

    // 初始化日志（含 OpenTelemetry 分布式追踪）
    init_tracing(
        "api-gateway",
        config.tracing.otlp_endpoint.as_deref(),
        "api_gateway=debug,axum=info,tower_http=debug",
    );
    let addr = format!("{}:{}", config.gateway.host, config.gateway.port);

    tracing::info!("API Gateway starting on {}", addr);

    // 初始化 gRPC 客户端
    let grpc_timeout = std::time::Duration::from_secs(config.gateway.grpc_timeout_seconds);
    let clients = grpc_clients::GrpcClients::new(
        config.auth_service.address(),
        config.product_service.address(),
        config.order_service.address(),
        config.inventory_service.address(),
        grpc_timeout,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to connect to backend services: {}", e);
        e
    })?;

    tracing::info!("Connected to all backend services");

    // 初始化 Redis 缓存（失败时降级为 None，不阻断启动）
    let cache = match redis_cache::RedisCache::new(&config.redis.url).await {
        Ok(cache) => {
            tracing::info!("Redis cache connected");
            Some(cache)
        }
        Err(e) => {
            tracing::warn!("Redis cache unavailable, running without cache: {}", e);
            None
        }
    };

    // 创建应用状态
    let app_state = Arc::new(AppState::new(config.clone(), clients, cache));

    // JWT 密钥
    let jwt_secret = config.jwt.secret.clone();

    // 限流器（受配置控制：rate_limit.enabled + rate_limit.whitelist_ips）
    let rate_limit_cfg = &config.rate_limit;
    let whitelist = Arc::new(rate_limit_cfg.whitelist_ips.clone());
    let default_limiter = Arc::new(create_default_rate_limiter());
    let strict_limiter = Arc::new(create_strict_rate_limiter());

    // 公共路由（无需认证）
    let public_routes = Router::new()
        .nest("/api/auth", auth_routes());
    // 对认证接口使用严格限流（防止暴力破解）
    let public_routes = if rate_limit_cfg.enabled {
        public_routes.layer(middleware::from_fn(create_rate_limit_middleware(strict_limiter, whitelist.clone())))
    } else {
        public_routes
    };

    // 受保护路由（需要认证）
    let protected_routes = Router::new()
        .nest("/api/users", user_routes())
        .nest("/api/products", product_routes())
        .nest("/api/orders", order_routes())
        .nest("/api/inventory", inventory_routes())
        // JWT 认证中间件
        .layer(middleware::from_fn(create_auth_middleware(jwt_secret)));
    // 默认限流
    let protected_routes = if rate_limit_cfg.enabled {
        protected_routes.layer(middleware::from_fn(create_rate_limit_middleware(default_limiter, whitelist.clone())))
    } else {
        protected_routes
    };

    // 合并路由
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // 健康检查（检查后端服务连通性）
        .route("/health", axum::routing::get(health_check_handler))
        // 压测端点（生产环境移除）
        .route("/bench/ping", axum::routing::get(ping_handler))
        .route("/bench/echo/:id", axum::routing::get(echo_handler))
        // Prometheus 指标端点
        .route("/metrics", axum::routing::get(metrics_handler))
        // 全局中间件
        .layer(MetricsMiddleware::new())
        .layer(middleware::from_fn(logger_middleware))
        .layer(create_cors_layer(config.gateway.cors_origins.clone()))
        .with_state(app_state);

    tracing::info!("API Gateway started successfully");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
        .with_graceful_shutdown(shutdown_signal())
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
