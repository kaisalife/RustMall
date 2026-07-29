//! 请求日志中间件

use axum::{body::Body, http::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{error, info, warn, Instrument, Level};

use common::request_context::{inject_response_id, RequestId};

/// 日志中间件
///
/// 记录每个请求的：
/// - 请求方法
/// - 请求路径
/// - 响应状态码
/// - 处理时间
/// - request_id（注入请求扩展 + 响应头，供审计中间件和 handler 使用）
pub async fn logger_middleware(mut request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let start = Instant::now();

    // 生成请求 ID（用于跨服务追踪）
    let request_id = uuid::Uuid::new_v4().to_string();

    // 注入到请求扩展（审计中间件和 handler 可读取）
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let span = tracing::span!(
        Level::INFO,
        "request",
        method = %method,
        path = %path,
        request_id = %request_id
    );

    async move {
        info!(method = %method, path = %path, "-> IN");

        let response = next.run(request).await;

        let duration = start.elapsed();
        let status = response.status();

        match status {
            StatusCode::OK
            | StatusCode::CREATED
            | StatusCode::ACCEPTED
            | StatusCode::NO_CONTENT => {
                info!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration = ?duration,
                    "<- OUT"
                );
            }
            StatusCode::BAD_REQUEST
            | StatusCode::NOT_FOUND
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN => {
                warn!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration = ?duration,
                    "<- OUT"
                );
            }
            _ => {
                error!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration = ?duration,
                    "<- OUT"
                );
            }
        }

        // 注入 request_id 到响应头（客户端可追踪）
        let mut response = response;
        inject_response_id(&mut response, &request_id);

        response
    }
    .instrument(span)
    .await
}

/// 详细日志中间件（包含请求头和响应头）
pub async fn verbose_logger_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let headers = request.headers().clone();
    let start = Instant::now();

    let span = tracing::span!(
        Level::DEBUG,
        "verbose_request",
        method = %method,
        path = %path
    );

    async move {
        tracing::debug!(
            method = %method,
            path = %path,
            headers = ?headers,
            "-> IN"
        );

        let response = next.run(request).await;

        let duration = start.elapsed();
        let status = response.status();
        let response_headers = response.headers();

        tracing::debug!(
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration = ?duration,
            headers = ?response_headers,
            "<- OUT"
        );

        response
    }
    .instrument(span)
    .await
}

/// CORS 配置
pub fn create_cors_layer(allowed_origins: Vec<String>) -> tower_http::cors::CorsLayer {
    let methods = [
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
        axum::http::Method::DELETE,
        axum::http::Method::OPTIONS,
    ];
    let headers = [
        axum::http::header::AUTHORIZATION,
        axum::http::header::CONTENT_TYPE,
        axum::http::header::ACCEPT,
    ];

    // 通配符 * 使用 AllowOrigin::any()
    if allowed_origins.iter().any(|o| o == "*") {
        return tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(methods)
            .allow_headers(headers)
            .max_age(std::time::Duration::from_secs(3600));
    }

    let origins: Vec<_> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    if origins.is_empty() {
        // 如果没有配置，默认只允许 localhost
        return tower_http::cors::CorsLayer::new()
            .allow_origin(
                "http://localhost:3000"
                    .parse::<axum::http::HeaderValue>()
                    .unwrap(),
            )
            .allow_methods(methods)
            .allow_headers(headers)
            .max_age(std::time::Duration::from_secs(3600));
    }

    tower_http::cors::CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(methods)
        .allow_headers(headers)
        .max_age(std::time::Duration::from_secs(3600))
}
