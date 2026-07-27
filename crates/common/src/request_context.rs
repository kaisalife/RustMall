//! 请求上下文（跨服务 request_id 传播）
//!
//! 使用 Axum 请求扩展在请求生命周期内存储 request_id。
//! logger 中间件生成并注入，审计中间件和 handler 可读取。

use axum::http::{Request, header::HeaderName, HeaderValue};

/// 请求扩展中存储 request_id 的 key
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// 请求扩展中的 request_id 包装类型
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// 从请求扩展中获取 request_id
pub fn get_request_id<T>(request: &Request<T>) -> Option<String> {
    request
        .extensions()
        .get::<RequestId>()
        .map(|rid| rid.0.clone())
        .or_else(|| {
            request
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// 将 request_id 注入到响应头
pub fn inject_response_id(response: &mut axum::response::Response, request_id: &str) {
    if let Ok(val) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, val);
    }
}
