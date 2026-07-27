use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::time::Duration;

/// 基于 Redis 的分布式滑动窗口限流器
#[derive(Clone)]
pub struct RedisRateLimiter {
    conn: ConnectionManager,
    max_requests: u32,
    window_size: Duration,
}

impl RedisRateLimiter {
    pub fn new(conn: ConnectionManager, max_requests: u32, window_size: Duration) -> Self {
        Self { conn, max_requests, window_size }
    }

    pub async fn new_with_url(url: &str, max_requests: u32, window_size: Duration) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self::new(conn, max_requests, window_size))
    }

    /// 检查是否允许请求（滑动窗口算法）
    pub async fn check(&self, key: &str) -> bool {
        let mut conn = self.conn.clone();
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - self.window_size.as_millis() as i64;
        let redis_key = format!("rate_limit:{}", key);

        // 使用 Redis 事务实现原子操作
        let (count,): (i64,) = redis::pipe()
            .atomic()
            .cmd("ZREMRANGEBYSCORE")
            .arg(&redis_key)
            .arg(0)
            .arg(window_start)
            .ignore()
            .cmd("ZADD")
            .arg(&redis_key)
            .arg(now)
            .arg(format!("{}:{}", now, rand::random::<u64>()))
            .ignore()
            .cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .unwrap_or((0,));

        // 设置 key 过期时间
        let _: () = conn.expire(&redis_key, self.window_size.as_secs() as i64 + 1).await.unwrap_or(());

        count <= self.max_requests as i64
    }
}
