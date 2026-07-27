//! 幂等性控制 crate
//!
//! 基于 Redis 分布式锁实现幂等操作，防止重复扣款、重复下单。
//!
//! ## 设计原理
//!
//! 1. **幂等 Key**：每个请求携带唯一的 idempotency_key（或由雪花ID生成）
//! 2. **Redis SETNX 分布式锁**：请求到达时先抢锁，防并发穿透
//! 3. **状态机管理**：PROCESSING -> SUCCESS / FAILED
//!    - SUCCESS：直接返回缓存结果（防重复）
//!    - PROCESSING：拒绝请求（防并发）
//!    - FAILED：允许重试
//! 4. **TTL 过期**：记录自动过期，避免 Redis 无限膨胀

pub mod manager;
pub mod key;

pub use manager::{IdempotencyManager, IdempotencyStatus, IdempotencyRecord, AcquireResult};
pub use key::IdempotencyKey;
