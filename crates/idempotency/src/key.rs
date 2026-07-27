//! 幂等 Key 生成
//!
//! 支持两种模式：
//! 1. 客户端传入 requestId（推荐，端到端可追溯）
//! 2. 服务端用雪花ID生成（客户端未传时兜底）

use common::SnowflakeIdGenerator;

/// 幂等 Key
///
/// 全局唯一，作为 Redis 分布式锁的 key 和幂等记录的主键。
/// 格式：`idem:{业务前缀}:{key值}`，如 `idem:payment:1234567890`
#[derive(Debug, Clone)]
pub struct IdempotencyKey {
    /// Redis 存储的完整 key（含前缀）
    redis_key: String,
    /// 原始 key 值（不含前缀）
    raw_key: String,
}

impl IdempotencyKey {
    /// 从客户端传入的 requestId 创建
    ///
    /// `prefix` 为业务前缀，如 "payment"、"order"
    pub fn from_request(prefix: &str, request_id: &str) -> Self {
        let raw = format!("{}_{}", prefix, request_id);
        Self {
            redis_key: format!("idem:{}", raw),
            raw_key: raw,
        }
    }

    /// 由雪花ID自动生成（客户端未传 requestId 时兜底）
    pub fn from_snowflake(
        prefix: &str,
        generator: &SnowflakeIdGenerator,
    ) -> common::AppResult<Self> {
        let id = generator.generate()?;
        let raw = format!("{}_{}", prefix, id);
        Ok(Self {
            redis_key: format!("idem:{}", raw),
            raw_key: raw,
        })
    }

    /// 获取 Redis 完整 key（含前缀）
    pub fn redis_key(&self) -> &str {
        &self.redis_key
    }

    /// 获取原始 key 值（不含前缀）
    pub fn raw(&self) -> &str {
        &self.raw_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_request() {
        let key = IdempotencyKey::from_request("payment", "req-abc-123");
        assert_eq!(key.redis_key(), "idem:payment_req-abc-123");
        assert_eq!(key.raw(), "payment_req-abc-123");
    }

    #[test]
    fn test_different_prefixes() {
        let pay_key = IdempotencyKey::from_request("payment", "req-001");
        let order_key = IdempotencyKey::from_request("order", "req-001");
        assert_ne!(pay_key.redis_key(), order_key.redis_key());
    }
}
