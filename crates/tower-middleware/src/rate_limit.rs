//! 限流中间件

use axum::{
    body::Body, http::Request, http::StatusCode, middleware::Next, response::IntoResponse,
    response::Response, Json,
};
use lru::LruCache;
use serde_json::json;
use std::num::NonZeroUsize;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

/// 限流错误
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Too many requests")]
    TooManyRequests,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        match self {
            RateLimitError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Too many requests",
                    "message": "Please try again later"
                })),
            )
                .into_response(),
        }
    }
}

/// 令牌桶限流器
#[derive(Clone)]
pub struct RateLimiter {
    /// 用户请求记录缓存
    buckets: Arc<Mutex<LruCache<String, Bucket>>>,
    /// 每个窗口允许的最大请求数
    max_requests: u32,
    /// 窗口大小（秒）
    window_size: Duration,
}

/// 令牌桶
struct Bucket {
    /// 剩余令牌数
    tokens: u32,
    /// 上次刷新时间
    last_refill: std::time::Instant,
}

impl RateLimiter {
    /// 创建新的限流器
    ///
    /// # Arguments
    ///
    /// * `max_requests` - 每个窗口允许的最大请求数
    /// * `window_size` - 窗口大小
    /// * `cache_size` - LRU 缓存大小
    pub fn new(max_requests: u32, window_size: Duration, cache_size: usize) -> Self {
        let cache_size =
            NonZeroUsize::new(cache_size).unwrap_or_else(|| NonZeroUsize::new(1000).unwrap());

        Self {
            buckets: Arc::new(Mutex::new(LruCache::new(cache_size))),
            max_requests,
            window_size,
        }
    }

    /// 检查是否允许请求
    ///
    /// # Arguments
    ///
    /// * `key` - 限流键，可以是 IP 地址或用户 ID
    pub async fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = std::time::Instant::now();

        let bucket = buckets.get_or_insert_mut(key.to_string(), || Bucket {
            tokens: self.max_requests,
            last_refill: now,
        });

        // 计算需要补充的令牌数
        let elapsed = now.duration_since(bucket.last_refill);
        let windows_passed = elapsed.as_secs() / self.window_size.as_secs();

        if windows_passed > 0 {
            bucket.tokens = self.max_requests;
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// 创建一个默认的限流器（每分钟 60 个请求）
pub fn create_default_rate_limiter() -> RateLimiter {
    RateLimiter::new(60, Duration::from_secs(60), 10000)
}

#[allow(clippy::type_complexity)]
/// 限流中间件工厂函数
/// `whitelist` 中的 IP 直接放行，不消耗令牌
pub fn create_rate_limit_middleware(
    limiter: Arc<RateLimiter>,
    whitelist: Arc<Vec<String>>,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, RateLimitError>> + Send + 'static>,
> + Clone
       + Send
       + Sync
       + 'static {
    move |request: Request<Body>, next: Next| {
        let limiter = limiter.clone();
        let whitelist = whitelist.clone();
        Box::pin(async move {
            // 获取客户端 IP
            let client_ip = get_client_ip(&request);

            // 白名单 IP 直接放行（用于压测等可信来源）
            if whitelist.iter().any(|ip| ip == &client_ip) {
                return Ok(next.run(request).await);
            }

            // 检查是否允许请求
            if !limiter.check(&client_ip).await {
                return Err(RateLimitError::TooManyRequests);
            }

            Ok(next.run(request).await)
        })
    }
}

/// 从请求中获取客户端 IP 地址
/// 优先使用 TCP 连接地址，只在受信代理场景下信任 X-Forwarded-For
fn get_client_ip<B>(request: &Request<B>) -> String {
    // 优先从 ConnectInfo 获取真实 TCP 连接地址
    if let Some(connect_info) = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return connect_info.0.ip().to_string();
    }

    // 如果没有 ConnectInfo（可能未启用），fallback 到 header
    // 注意：这仅适用于 api-gateway 直接面向客户端的场景
    // 如果通过 nginx 等代理，需要在代理层设置 X-Real-IP
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.trim().to_string();
        }
    }

    // 最后 fallback
    "unknown".to_string()
}

/// 创建一个严格的限流器（用于敏感接口，如登录）
/// 每分钟 10 个请求
pub fn create_strict_rate_limiter() -> RateLimiter {
    RateLimiter::new(10, Duration::from_secs(60), 10000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60), 100);

        for _ in 0..5 {
            assert!(limiter.check("user1").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60), 100);

        // Use up all 3 tokens
        assert!(limiter.check("user2").await);
        assert!(limiter.check("user2").await);
        assert!(limiter.check("user2").await);

        // 4th request should be blocked
        assert!(!limiter.check("user2").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refills_after_window() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1), 100);

        // Use up all tokens
        assert!(limiter.check("user3").await);
        assert!(limiter.check("user3").await);
        assert!(!limiter.check("user3").await);

        // Wait for window to pass
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should be refilled
        assert!(limiter.check("user3").await);
    }
}
