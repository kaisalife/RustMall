//! 退款实体
//!
//! 退款单关联一笔原支付订单，记录退款金额、原因和状态。
//! 退款也支持幂等（通过 idempotency_key 防重复退款）。

use chrono::{DateTime, Utc};

use common::{AppError, AppResult};

use super::money::Money;
use super::payment::PaymentStatus;

/// 退款单
#[derive(Debug, Clone, PartialEq)]
pub struct Refund {
    /// 退款单 ID（雪花算法生成）
    pub id: u64,
    /// 幂等 key（防重复退款）
    pub idempotency_key: String,
    /// 原支付订单 ID
    pub payment_id: u64,
    /// 退款金额
    pub refund_amount: Money,
    /// 退款原因
    pub reason: Option<String>,
    /// 退款状态（复用 PaymentStatus: Pending/Processing/Success/Failed）
    pub status: PaymentStatus,
    /// 渠道退款交易号
    pub channel_txn_id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl Refund {
    /// 创建退款单（初始状态为 Pending）
    pub fn new(
        id: u64,
        idempotency_key: String,
        payment_id: u64,
        refund_amount: Money,
        reason: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            idempotency_key,
            payment_id,
            refund_amount,
            reason,
            status: PaymentStatus::Pending,
            channel_txn_id: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 退款成功（Pending/Processing -> Success）
    pub fn succeed(&mut self, channel_txn_id: String) -> AppResult<()> {
        match self.status {
            PaymentStatus::Pending | PaymentStatus::Processing => {
                self.status = PaymentStatus::Success;
                self.channel_txn_id = channel_txn_id;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot succeed refund from status: {}",
                self.status
            ))),
        }
    }

    /// 退款失败（Pending/Processing -> Failed）
    pub fn fail(&mut self) -> AppResult<()> {
        match self.status {
            PaymentStatus::Pending | PaymentStatus::Processing => {
                self.status = PaymentStatus::Failed;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot fail refund from status: {}",
                self.status
            ))),
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Currency;
    use rust_decimal_macros::dec;

    fn create_test_refund() -> Refund {
        Refund::new(
            1,
            "refund-key-001".to_string(),
            1001,
            Money::new(dec!(50.00), Currency::CNY),
            Some("商品退货".to_string()),
        )
    }

    #[test]
    fn test_new_refund_is_pending() {
        let r = create_test_refund();
        assert_eq!(r.status, PaymentStatus::Pending);
        assert_eq!(r.channel_txn_id, "");
        assert_eq!(r.reason.as_deref(), Some("商品退货"));
    }

    #[test]
    fn test_refund_succeed() {
        let mut r = create_test_refund();
        assert!(r.succeed("wx_refund_001".to_string()).is_ok());
        assert_eq!(r.status, PaymentStatus::Success);
        assert_eq!(r.channel_txn_id, "wx_refund_001");
    }

    #[test]
    fn test_refund_fail() {
        let mut r = create_test_refund();
        assert!(r.fail().is_ok());
        assert_eq!(r.status, PaymentStatus::Failed);
    }

    #[test]
    fn test_refund_succeed_from_success_fails() {
        let mut r = create_test_refund();
        r.succeed("txn".to_string()).unwrap();
        assert!(r.succeed("txn2".to_string()).is_err());
    }
}
