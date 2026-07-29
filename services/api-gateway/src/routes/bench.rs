//! 压测专用端点
//!
//! 提供轻量级端点用于基准测试，不依赖后端服务。
//! 生产环境应移除这些端点。

use axum::{response::IntoResponse, Json};
use serde::Serialize;

/// 压测响应
#[derive(Debug, Serialize)]
pub struct BenchResponse {
    pub message: String,
    pub timestamp: i64,
    pub request_id: u64,
}

/// GET /bench/ping - 最轻量的 ping 端点
///
/// 直接返回 JSON，无任何 IO 操作。
/// 用于测量 API Gateway 的纯框架开销（路由匹配 + 序列化 + 网络）。
pub async fn ping_handler() -> impl IntoResponse {
    Json(BenchResponse {
        message: "pong".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        request_id: 0,
    })
}

/// GET /bench/echo/:id - 带路径参数
///
/// 用于测量路径解析开销。
pub async fn echo_handler(axum::extract::Path(id): axum::extract::Path<u64>) -> impl IntoResponse {
    Json(BenchResponse {
        message: format!("echo: {}", id),
        timestamp: chrono::Utc::now().timestamp_millis(),
        request_id: id,
    })
}
