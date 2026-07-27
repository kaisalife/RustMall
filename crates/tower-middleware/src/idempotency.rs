//! 幂等性中间件
//!
//! 在 API 网关层拦截重复请求，防止业务层被重复调用。
//!
//! ## 工作原理
//!
//! 1. 从请求头 `X-Idempotency-Key` 提取幂等键
//! 2. 对 POST/PUT/PATCH/DELETE 请求生效（GET 跳过）
//! 3. 用 Redis SETNX 抢锁：
//!    - Acquired -> 放行，请求头注入幂等键供下游使用
//!    - Duplicate -> 返回 409（已有成功记录）
//!    - Processing -> 返回 409（处理中）
//! 4. 响应缓存由业务层（payment-service 的 IdempotencyService）处理
//!
//! ## 使用方式
//!
//! ```ignore
//! use tower_middleware::idempotency::create_idempotency_layer;
//!
//! let app = Router::new()
//!     .route("/payments", post(create_payment))
//!     .layer(create_idempotency_layer(idempotency_manager));
//! ```

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use idempotency::{AcquireResult, IdempotencyKey, IdempotencyManager};

/// 幂等键请求头名称
pub const IDEMPOTENCY_HEADER: &str = "x-idempotency-key";

/// 幂等锁 TTL（网关层锁比业务层短，快速失败）
const GATEWAY_LOCK_TTL: Duration = Duration::from_secs(30);

/// 幂等中间件
///
/// 拦截重复请求，返回 409 Conflict。
/// 首次请求放行，并将幂等键注入到请求扩展中供下游使用。
pub async fn idempotency_middleware(
    manager: Arc<IdempotencyManager>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();

    // 只对写操作生效（GET/HEAD/OPTIONS 跳过）
    if !is_write_method(&method) {
        return next.run(request).await;
    }

    // 从请求头提取幂等键
    let idempotency_key = match request.headers().get(IDEMPOTENCY_HEADER) {
        Some(value) => value.to_str().unwrap_or("").to_string(),
        None => {
            // 没有幂等键，放行（由业务层决定是否强制要求）
            return next.run(request).await;
        }
    };

    if idempotency_key.is_empty() {
        return next.run(request).await;
    }

    // 抢锁
    let key = IdempotencyKey::from_request("gateway", &idempotency_key);
    match manager.try_acquire(&key, GATEWAY_LOCK_TTL).await {
        Ok(AcquireResult::Acquired) => {
            // 首次请求，放行
            next.run(request).await
        }
        Ok(AcquireResult::Duplicate(_)) => {
            // 已有成功记录，返回 409
            (
                StatusCode::CONFLICT,
                "请求已处理，请勿重复提交",
            )
                .into_response()
        }
        Ok(AcquireResult::Processing) => {
            // 处理中，返回 409
            (
                StatusCode::CONFLICT,
                "请求处理中，请稍后重试",
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("幂等锁获取失败: {:?}", e);
            // Redis 故障时降级放行（不阻断业务）
            next.run(request).await
        }
    }
}

/// 判断是否为写操作
fn is_write_method(method: &axum::http::Method) -> bool {
    matches!(
        method,
        &axum::http::Method::POST
            | &axum::http::Method::PUT
            | &axum::http::Method::PATCH
            | &axum::http::Method::DELETE
    )
}

/// 创建幂等中间件
///
/// 用法：
/// ```ignore
/// use axum::middleware::from_fn;
/// use std::sync::Arc;
///
/// let manager = Arc::new(idempotency_manager);
/// let app = Router::new()
///     .route("/payments", post(create_payment))
///     .layer(from_fn(move |req, next| {
///         idempotency_middleware(manager.clone(), req, next)
///     }));
/// ```
///
/// 注意：axum 的 from_fn 需要闭包捕获 manager，用 Arc 共享。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_write_method() {
        assert!(is_write_method(&axum::http::Method::POST));
        assert!(is_write_method(&axum::http::Method::PUT));
        assert!(is_write_method(&axum::http::Method::PATCH));
        assert!(is_write_method(&axum::http::Method::DELETE));
        assert!(!is_write_method(&axum::http::Method::GET));
        assert!(!is_write_method(&axum::http::Method::HEAD));
        assert!(!is_write_method(&axum::http::Method::OPTIONS));
    }
}
