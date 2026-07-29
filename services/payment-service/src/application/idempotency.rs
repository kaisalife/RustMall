//! 幂等性控制模块（核心安全保障）
//!
//! 支付系统最核心的安全保障模块，防止重复扣款、重复退款。
//!
//! # 双重保障机制
//!
//! 1. **Redis SETNX 分布式锁**（并发防护）
//!    - 请求到达时先抢锁，同一 key 同时只有一个请求进入处理
//!    - 锁 TTL 防死锁（建议 30s-60s）
//!    - 锁状态：PROCESSING（处理中）/ SUCCESS（成功，缓存响应）/ FAILED（失败，允许重试）
//!
//! 2. **数据库唯一索引**（兜底防重放）
//!    - 即使 Redis 锁失效，DB 唯一索引也会拒绝插入重复记录
//!    - 数据库查询作为二次校验，确保数据一致性
//!
//! # 幂等性控制的 3 种状态处理逻辑
//!
//! | 状态 | Redis | 数据库 | 处理方式 |
//! |------|-------|--------|---------|
//! | **SUCCESS** | 返回缓存响应 | 返回已有记录 | 直接复用结果 |
//! | **PROCESSING** | 拒绝请求 | 拒绝请求 | 返回"处理中" |
//! | **FAILED** | 删除记录，允许重试 | 返回 None | 允许重试 |
//! | **无记录** | 抢锁 | 返回 None | 首次请求，继续处理 |

use std::sync::Arc;
use std::time::Duration;

use common::{AppError, AppResult};

use crate::domain::{PaymentRepository, PaymentStatus, RefundRepository};

use super::dto::{PaymentDto, RefundDto};

/// 幂等锁 TTL（处理中超时时间）
const LOCK_TTL: Duration = Duration::from_secs(60);

/// 成功记录保留时间（供重复请求查询）
const SUCCESS_TTL: Duration = Duration::from_secs(86400); // 24h

/// 失败记录保留时间（防短时间疯狂重试）
const FAILED_TTL: Duration = Duration::from_secs(300); // 5min

/// 幂等性控制服务
///
/// 双重保障：Redis 分布式锁 + 数据库唯一索引。
/// Redis 作为第一层防线防并发穿透，数据库作为兜底防重放。
#[derive(Clone)]
pub struct IdempotencyService {
    /// 支付订单仓储（数据库兜底校验）
    payment_repo: Arc<dyn PaymentRepository>,
    /// 退款仓储（数据库兜底校验）
    refund_repo: Arc<dyn RefundRepository>,
    /// Redis 幂等管理器（并发防护，可选--Redis 不可用时降级为仅 DB）
    idempotency_manager: Option<idempotency::IdempotencyManager>,
}

impl IdempotencyService {
    pub fn new(
        payment_repo: Arc<dyn PaymentRepository>,
        refund_repo: Arc<dyn RefundRepository>,
    ) -> Self {
        Self {
            payment_repo,
            refund_repo,
            idempotency_manager: None,
        }
    }

    /// 注入 Redis 幂等管理器（启用分布式锁）
    pub fn with_redis(mut self, manager: idempotency::IdempotencyManager) -> Self {
        self.idempotency_manager = Some(manager);
        self
    }

    /// 检查支付幂等性
    ///
    /// 双重校验流程：
    /// 1. Redis SETNX 抢锁（防并发穿透）
    ///    - Duplicate -> 返回缓存结果
    ///    - Processing -> 拒绝请求
    ///    - Acquired -> 继续数据库校验
    /// 2. 数据库查询（兜底防重放）
    ///    - SUCCESS -> 返回已有结果
    ///    - PROCESSING -> 拒绝请求
    ///    - FAILED/无记录 -> 允许处理
    pub async fn check_payment_idempotency(&self, key: &str) -> AppResult<Option<PaymentDto>> {
        // 1. Redis 分布式锁校验（如果启用）
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("payment", key);
            match manager.try_acquire(&idem_key, LOCK_TTL).await? {
                idempotency::AcquireResult::Duplicate(record) => {
                    // Redis 有成功记录，返回缓存响应
                    if let Some(response_data) = record.response_data {
                        let dto: PaymentDto =
                            serde_json::from_str(&response_data).map_err(|e| {
                                AppError::internal(format!("反序列化缓存响应失败: {}", e))
                            })?;
                        return Ok(Some(dto));
                    }
                    // response_data 为空，降级到数据库查询
                }
                idempotency::AcquireResult::Processing => {
                    return Err(AppError::conflict("支付处理中，请勿重复提交"));
                }
                idempotency::AcquireResult::Acquired => {
                    // 抢锁成功，继续数据库校验
                }
            }
        }

        // 2. 数据库兜底校验
        let payment = self.payment_repo.find_by_idempotency_key(key).await?;

        match payment {
            Some(p) => match p.status {
                PaymentStatus::Success => Ok(Some(PaymentDto::from(p))),
                PaymentStatus::Failed => Ok(None),
                _ => Err(AppError::conflict("支付处理中，请勿重复提交")),
            },
            None => Ok(None),
        }
    }

    /// 检查退款幂等性
    ///
    /// 与支付幂等性检查逻辑相同，区别在于使用退款仓储。
    pub async fn check_refund_idempotency(&self, key: &str) -> AppResult<Option<RefundDto>> {
        // 1. Redis 分布式锁校验
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("refund", key);
            match manager.try_acquire(&idem_key, LOCK_TTL).await? {
                idempotency::AcquireResult::Duplicate(record) => {
                    if let Some(response_data) = record.response_data {
                        let dto: RefundDto = serde_json::from_str(&response_data).map_err(|e| {
                            AppError::internal(format!("反序列化缓存响应失败: {}", e))
                        })?;
                        return Ok(Some(dto));
                    }
                }
                idempotency::AcquireResult::Processing => {
                    return Err(AppError::conflict("退款处理中，请勿重复提交"));
                }
                idempotency::AcquireResult::Acquired => {}
            }
        }

        // 2. 数据库兜底校验
        let refund = self.refund_repo.find_by_idempotency_key(key).await?;

        match refund {
            Some(r) => match r.status {
                PaymentStatus::Success => Ok(Some(RefundDto::from(r))),
                PaymentStatus::Failed => Ok(None),
                _ => Err(AppError::conflict("退款处理中，请勿重复提交")),
            },
            None => Ok(None),
        }
    }

    /// 标记支付成功（缓存响应到 Redis）
    pub async fn save_payment_success(&self, key: &str, dto: &PaymentDto) -> AppResult<()> {
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("payment", key);
            let response_data = serde_json::to_string(dto)
                .map_err(|e| AppError::internal(format!("序列化响应失败: {}", e)))?;
            manager
                .save_success(&idem_key, &response_data, SUCCESS_TTL)
                .await?;
        }
        Ok(())
    }

    /// 标记退款成功（缓存响应到 Redis）
    pub async fn save_refund_success(&self, key: &str, dto: &RefundDto) -> AppResult<()> {
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("refund", key);
            let response_data = serde_json::to_string(dto)
                .map_err(|e| AppError::internal(format!("序列化响应失败: {}", e)))?;
            manager
                .save_success(&idem_key, &response_data, SUCCESS_TTL)
                .await?;
        }
        Ok(())
    }

    /// 标记支付失败（允许重试）
    pub async fn save_payment_failure(&self, key: &str) -> AppResult<()> {
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("payment", key);
            manager.save_failure(&idem_key, FAILED_TTL).await?;
        }
        Ok(())
    }

    /// 标记退款失败（允许重试）
    pub async fn save_refund_failure(&self, key: &str) -> AppResult<()> {
        if let Some(ref manager) = self.idempotency_manager {
            let idem_key = idempotency::IdempotencyKey::from_request("refund", key);
            manager.save_failure(&idem_key, FAILED_TTL).await?;
        }
        Ok(())
    }
}
