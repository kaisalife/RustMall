//! 幂等性管理器
//!
//! 基于 Redis SETNX 实现分布式锁 + 幂等状态管理。
//!
//! ## 工作流程
//!
//! ```text
//! 请求到达
//!   │
//!   ▼
//! try_acquire(key)
//!   │
//!   ├── Acquired ──────► 执行业务逻辑
//!   │                       ├── 成功 --> save_success(key, response)
//!   │                       └── 失败 --> save_failure(key)
//!   │
//!   ├── Duplicate(record) --> 直接返回缓存的成功结果
//!   │
//!   ├── Processing ──────► 返回"处理中，请稍后重试"
//!   │
//!   └── Failed ──────────► 允许重试（删除旧记录，重新抢锁）
//! ```

use std::time::Duration;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::debug;

use common::AppResult;

use super::key::IdempotencyKey;

/// 幂等状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyStatus {
    /// 处理中（已抢到锁，业务逻辑执行中）
    Processing,
    /// 成功（业务逻辑执行完成，缓存了响应）
    Success,
    /// 失败（业务逻辑执行失败，允许重试）
    Failed,
}

impl IdempotencyStatus {
    fn as_str(&self) -> &'static str {
        match self {
            IdempotencyStatus::Processing => "PROCESSING",
            IdempotencyStatus::Success => "SUCCESS",
            IdempotencyStatus::Failed => "FAILED",
        }
    }
}

/// 幂等记录（存储在 Redis 中）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// 幂等状态
    pub status: IdempotencyStatus,
    /// 缓存的响应数据（仅 Success 时有值）
    pub response_data: Option<String>,
    /// 创建时间戳（Unix 毫秒）
    pub created_at: i64,
}

impl IdempotencyRecord {
    /// 创建 Processing 记录
    fn processing() -> Self {
        Self {
            status: IdempotencyStatus::Processing,
            response_data: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 创建 Success 记录
    fn success(response_data: String) -> Self {
        Self {
            status: IdempotencyStatus::Success,
            response_data: Some(response_data),
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 创建 Failed 记录
    fn failed() -> Self {
        Self {
            status: IdempotencyStatus::Failed,
            response_data: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// try_acquire 的返回结果
#[derive(Debug)]
pub enum AcquireResult {
    /// 成功获取锁（可以执行业务逻辑）
    Acquired,
    /// 重复请求 - 已有成功记录，直接返回缓存结果
    Duplicate(IdempotencyRecord),
    /// 正在处理中（其他请求正在执行，拒绝并发）
    Processing,
}

/// 幂等性管理器
///
/// 封装 Redis 分布式锁 + 幂等状态管理。
/// 通过 `Arc<IdempotencyManager>` 共享，Clone 成本低（内部 Arc）。
#[derive(Clone)]
pub struct IdempotencyManager {
    /// Redis 连接管理器
    conn: redis::aio::ConnectionManager,
}

impl IdempotencyManager {
    /// 创建幂等管理器
    ///
    /// `redis_url` 格式：`redis://host:port`
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let conn = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    /// 从已有的 Redis 连接创建（复用连接池）
    pub fn from_connection_manager(conn: redis::aio::ConnectionManager) -> Self {
        Self { conn }
    }

    /// 尝试获取幂等锁
    ///
    /// 流程：
    /// 1. 查询已有记录
    ///    - SUCCESS -> 返回 Duplicate（直接返回缓存结果）
    ///    - PROCESSING -> 返回 Processing（拒绝并发）
    ///    - FAILED -> 删除旧记录，继续抢锁
    ///    - 无记录 -> 继续抢锁
    /// 2. SETNX 抢锁（写入 PROCESSING 记录）
    ///    - 成功 -> 返回 Acquired
    ///    - 失败（已被其他请求抢到）-> 返回 Processing
    ///
    /// `ttl` 为锁的过期时间（防止死锁），建议 30s-60s
    pub async fn try_acquire(
        &self,
        key: &IdempotencyKey,
        ttl: Duration,
    ) -> AppResult<AcquireResult> {
        let redis_key = key.redis_key();

        // 1. 查询已有记录
        if let Some(record) = self.get_record(redis_key).await? {
            match record.status {
                IdempotencyStatus::Success => {
                    debug!(key = redis_key, "Idempotency hit: SUCCESS, returning cached");
                    return Ok(AcquireResult::Duplicate(record));
                }
                IdempotencyStatus::Processing => {
                    debug!(key = redis_key, "Idempotency hit: PROCESSING, rejecting");
                    return Ok(AcquireResult::Processing);
                }
                IdempotencyStatus::Failed => {
                    // 失败的记录允许重试，删除旧记录
                    debug!(key = redis_key, "Idempotency hit: FAILED, allowing retry");
                    self.delete(redis_key).await?;
                }
            }
        }

        // 2. SETNX 抢锁（原子操作：写入 PROCESSING + 设置 TTL）
        let record = IdempotencyRecord::processing();
        let record_json = serde_json::to_string(&record)
            .map_err(|e| common::AppError::internal(format!("Serialize idempotency record failed: {}", e)))?;

        let mut conn = self.conn.clone();
        // SET key value NX EX ttl（仅当 key 不存在时设置，带过期时间）
        // 成功返回 "OK"，失败（key 已存在）返回 nil
        let result: Option<String> = redis::cmd("SET")
            .arg(redis_key)
            .arg(&record_json)
            .arg("NX")
            .arg("EX")
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| common::AppError::internal(format!("Redis SETNX failed: {}", e)))?;

        if result.is_some() {
            debug!(key = redis_key, "Idempotency lock acquired");
            Ok(AcquireResult::Acquired)
        } else {
            // SETNX 失败说明 key 已存在（并发请求被其他实例抢到）
            debug!(key = redis_key, "Idempotency lock contention, rejecting");
            Ok(AcquireResult::Processing)
        }
    }

    /// 保存成功结果
    ///
    /// 将状态更新为 SUCCESS，并缓存响应数据。
    /// `ttl` 为成功记录的保留时间（建议 24h，供重复请求查询）。
    pub async fn save_success(
        &self,
        key: &IdempotencyKey,
        response_data: &str,
        ttl: Duration,
    ) -> AppResult<()> {
        let record = IdempotencyRecord::success(response_data.to_string());
        self.save_record(key.redis_key(), &record, ttl).await
    }

    /// 标记失败
    ///
    /// 将状态更新为 FAILED，允许后续重试。
    /// `ttl` 为失败记录的保留时间（建议 5min，防短时间疯狂重试）。
    pub async fn save_failure(
        &self,
        key: &IdempotencyKey,
        ttl: Duration,
    ) -> AppResult<()> {
        let record = IdempotencyRecord::failed();
        self.save_record(key.redis_key(), &record, ttl).await
    }

    /// 释放锁（仅用于异常回滚）
    ///
    /// 正常流程不需要调用：成功时 save_success，失败时 save_failure。
    /// 仅在业务逻辑 panic 或异常退出时需要手动释放。
    pub async fn release(&self, key: &IdempotencyKey) -> AppResult<()> {
        self.delete(key.redis_key()).await
    }

    /// 查询幂等记录
    pub async fn get_record(&self, redis_key: &str) -> AppResult<Option<IdempotencyRecord>> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn
            .get(redis_key)
            .await
            .map_err(|e| common::AppError::internal(format!("Redis GET failed: {}", e)))?;

        match value {
            Some(json) => {
                let record: IdempotencyRecord = serde_json::from_str(&json)
                    .map_err(|e| common::AppError::internal(format!("Deserialize idempotency record failed: {}", e)))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// 保存记录到 Redis（覆盖写入）
    async fn save_record(
        &self,
        redis_key: &str,
        record: &IdempotencyRecord,
        ttl: Duration,
    ) -> AppResult<()> {
        let json = serde_json::to_string(record)
            .map_err(|e| common::AppError::internal(format!("Serialize failed: {}", e)))?;

        let mut conn = self.conn.clone();
        conn.set_ex::<_, _, ()>(redis_key, &json, ttl.as_secs())
            .await
            .map_err(|e| common::AppError::internal(format!("Redis SET failed: {}", e)))?;

        debug!(key = redis_key, status = record.status.as_str(), "Idempotency record saved");
        Ok(())
    }

    /// 删除 Redis key
    async fn delete(&self, redis_key: &str) -> AppResult<()> {
        let mut conn = self.conn.clone();
        conn.del::<_, ()>(redis_key)
            .await
            .map_err(|e| common::AppError::internal(format!("Redis DEL failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_status_as_str() {
        assert_eq!(IdempotencyStatus::Processing.as_str(), "PROCESSING");
        assert_eq!(IdempotencyStatus::Success.as_str(), "SUCCESS");
        assert_eq!(IdempotencyStatus::Failed.as_str(), "FAILED");
    }

    #[test]
    fn test_record_processing() {
        let record = IdempotencyRecord::processing();
        assert_eq!(record.status, IdempotencyStatus::Processing);
        assert!(record.response_data.is_none());
    }

    #[test]
    fn test_record_success() {
        let record = IdempotencyRecord::success("response_payload".to_string());
        assert_eq!(record.status, IdempotencyStatus::Success);
        assert_eq!(record.response_data.as_deref(), Some("response_payload"));
    }

    #[test]
    fn test_record_failed() {
        let record = IdempotencyRecord::failed();
        assert_eq!(record.status, IdempotencyStatus::Failed);
        assert!(record.response_data.is_none());
    }

    #[test]
    fn test_record_serialization() {
        let record = IdempotencyRecord::success("test_data".to_string());
        let json = serde_json::to_string(&record).unwrap();
        let decoded: IdempotencyRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.status, IdempotencyStatus::Success);
        assert_eq!(decoded.response_data.as_deref(), Some("test_data"));
    }
}
