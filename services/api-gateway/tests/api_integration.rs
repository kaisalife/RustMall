//! API Gateway 集成测试
//!
//! 测试 API 网关的 HTTP 层行为：
//! - 健康检查端点
//! - 受保护路由的认证拦截
//! - 请求体格式校验

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware, routing, Router,
};
use std::sync::Arc;
use tower::ServiceExt;

use api_gateway::circuit_breaker::CircuitBreaker;
use api_gateway::grpc_clients::GrpcClients;
use api_gateway::routes::{
    auth_routes, health_check_handler, inventory_routes, order_routes, product_routes, user_routes,
};
use api_gateway::state::AppState;

use proto::auth::v1::auth_service_client::AuthServiceClient;
use proto::auth::v2::auth_service_client::AuthServiceClient as AuthServiceClientV2;
use proto::inventory::v1::inventory_service_client::InventoryServiceClient;
use proto::order::v1::order_service_client::OrderServiceClient;
use proto::product::v1::product_service_client::ProductServiceClient;

use tower_middleware::auth::create_auth_middleware;
use tower_middleware::rate_limit::{create_default_rate_limiter, create_rate_limit_middleware};
use tower_middleware::TraceContextInjector;

/// 创建懒连接的 gRPC Channel（不实际连接，仅在调用时才尝试连接）
fn make_lazy_channel() -> tonic::transport::Channel {
    tonic::transport::Endpoint::from_shared("http://127.0.0.1:1")
        .unwrap()
        .connect_lazy()
}

/// 构建测试用的 AppState（使用懒连接的 gRPC 客户端，不需要后端服务运行）
fn make_test_state() -> Arc<AppState> {
    let channel = make_lazy_channel();

    let clients = GrpcClients {
        auth: AuthServiceClient::with_interceptor(channel.clone(), TraceContextInjector),
        auth_v2: AuthServiceClientV2::with_interceptor(channel.clone(), TraceContextInjector),
        product: ProductServiceClient::with_interceptor(channel.clone(), TraceContextInjector),
        order: OrderServiceClient::with_interceptor(channel.clone(), TraceContextInjector),
        inventory: InventoryServiceClient::with_interceptor(channel, TraceContextInjector),
        auth_cb: CircuitBreaker::default_cb(),
        product_cb: CircuitBreaker::default_cb(),
        order_cb: CircuitBreaker::default_cb(),
        inventory_cb: CircuitBreaker::default_cb(),
    };

    let config = common::AppConfig {
        gateway: common::config::GatewayConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_origins: vec![],
            grpc_timeout_seconds: 30,
        },
        auth_service: common::ServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50051,
            worker_id: 1,
            advertise_host: None,
        },
        product_service: common::ServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50052,
            worker_id: 2,
            advertise_host: None,
        },
        order_service: common::ServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50053,
            worker_id: 3,
            advertise_host: None,
        },
        inventory_service: common::ServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50054,
            worker_id: 4,
            advertise_host: None,
        },
        email_service: common::ServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50055,
            worker_id: 5,
            advertise_host: None,
        },
        payment_service: common::PaymentServiceConfig {
            host: "127.0.0.1".to_string(),
            port: 50056,
            worker_id: 6,
            advertise_host: None,
        },
        database: common::DatabaseConfig {
            host: "127.0.0.1".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            database: "simple_trade".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 3,
            idle_timeout_minutes: 10,
            max_lifetime_minutes: 30,
        },
        redis: common::RedisConfig {
            url: "redis://localhost:6379".to_string(),
        },
        jwt: common::JwtConfig {
            secret: "test_secret".to_string(),
            expiration_hours: 1,
            refresh_expiration_hours: 168,
            bcrypt_cost: 10,
        },
        email: common::EmailConfig {
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            smtp_username: "test@gmail.com".to_string(),
            smtp_password: "password".to_string(),
            from_address: "no-reply@test.com".to_string(),
        },
        tracing: common::TracingConfig::default(),
        rate_limit: common::RateLimitConfig::default(),
        nacos: common::config::NacosConfigSection::default(),
        kafka: common::KafkaConfig {
            brokers: "localhost:9092".to_string(),
            topic_prefix: "simple_trade".to_string(),
            consumer_group: "test-group".to_string(),
        },
    };

    Arc::new(AppState::new(config, clients, None, None))
}

/// 构建测试用 Router，模拟 api-gateway 的路由结构
fn build_test_app(state: Arc<AppState>) -> Router {
    let jwt_secret = "test_secret".to_string();
    let default_limiter = Arc::new(create_default_rate_limiter());
    let strict_limiter = Arc::new(create_default_rate_limiter());

    // 公共路由（无需认证）
    let public_routes = Router::new()
        .nest("/api/v1/auth", auth_routes())
        .layer(middleware::from_fn(create_rate_limit_middleware(
            strict_limiter,
            std::sync::Arc::new(vec![]),
        )));

    // 受保护路由（需要认证）
    let protected_routes = Router::new()
        .nest("/api/v1/users", user_routes())
        .nest("/api/v1/products", product_routes())
        .nest("/api/v1/orders", order_routes())
        .nest("/api/v1/inventory", inventory_routes())
        .layer(middleware::from_fn(create_auth_middleware(jwt_secret)))
        .layer(middleware::from_fn(create_rate_limit_middleware(
            default_limiter,
            std::sync::Arc::new(vec![]),
        )));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .route("/health", routing::get(health_check_handler))
        .with_state(state)
}

#[tokio::test]
async fn test_health_endpoint_returns_response() {
    let state = make_test_state();
    let app = build_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 健康检查返回 200（所有后端可用）或 503（后端不可用）
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
        "Expected 200 or 503, got {}",
        status
    );
}

#[tokio::test]
async fn test_unauthorized_access_to_protected_route() {
    let state = make_test_state();
    let app = build_test_app(state);

    // 无 token 访问受保护路由，应返回 401
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/users/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_register_with_invalid_body_returns_400() {
    let state = make_test_state();
    let app = build_test_app(state);

    // POST /api/v1/auth/register 无 body，JSON 解析失败，应返回 400
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
