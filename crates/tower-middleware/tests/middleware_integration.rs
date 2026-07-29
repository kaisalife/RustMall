//! 中间件集成测试
//!
//! 测试认证中间件和限流中间件在完整 axum 路由中的行为。

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use tower_middleware::{
    auth::create_auth_middleware,
    rate_limit::{create_rate_limit_middleware, RateLimiter},
};

/// 构建带认证中间件的测试路由
fn make_auth_app(secret: &str) -> Router {
    Router::new()
        .route("/protected", get(|| async { "ok" }))
        .layer(middleware::from_fn(create_auth_middleware(
            secret.to_string(),
        )))
}

/// 构建带限流中间件的测试路由
fn make_rate_limit_app(max_requests: u32) -> Router {
    let limiter = Arc::new(RateLimiter::new(max_requests, Duration::from_secs(60), 100));
    Router::new()
        .route("/api", get(|| async { "ok" }))
        .layer(middleware::from_fn(create_rate_limit_middleware(
            limiter,
            std::sync::Arc::new(vec![]),
        )))
}

#[tokio::test]
async fn test_auth_middleware_rejects_no_token() {
    let app = make_auth_app("test_secret");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_accepts_valid_token() {
    let secret = "test_secret";
    let app = make_auth_app(secret);

    let claims = common::Claims::new(1, "test@example.com".to_string(), 1, "user".to_string());
    let token = common::generate_jwt(&claims, secret).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_middleware_rejects_invalid_token() {
    let app = make_auth_app("test_secret");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("Authorization", "Bearer invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_rate_limit_allows_within_limit() {
    let app = make_rate_limit_app(5);

    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_rate_limit_blocks_over_limit() {
    let app = make_rate_limit_app(3);

    // 前 3 个请求应该通过
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // 第 4 个请求应该被限流
    let response = app
        .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
