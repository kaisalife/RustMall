//! 支付订单实体 + 状态机
//!
//! 支付订单是支付系统的核心领域对象，包含金额、渠道、状态等关键信息。
//! 状态机控制支付生命周期的合法流转，非法转换将返回错误。
//!
//! ## 状态流转图
//!
//! ```text
//!                  ┌──────────┐
//!            创建   │ PENDING  │
//!        ─────────► │ (待支付)  │
//!                  └────┬─────┘
//!                       │ start_processing
//!                       ▼
//!                  ┌──────────┐
//!                  │PROCESSING│
//!                  │ (处理中)  │
//!                  └──┬───┬───┘
//!           succeed   │   │ fail
//!                     ▼   ▼
//!           ┌────────┐   ┌────────┐
//!           │SUCCESS │   │ FAILED │
//!           │(支付成功)│   │(支付失败)│
//!           └───┬────┘   └────────┘
//!      start_refund │
//!               ▼   │
//!         ┌──────────┐
//!         │REFUNDING │
//!         │ (退款中)  │
//!         └──┬───┬───┘
//!  complete   │   │
//!  (partial)  │   │
//!             ▼   ▼
//!   ┌──────────────┐  ┌───────────┐
//!   │PARTIAL_      │  │ REFUNDED  │
//!   │REFUNDED      │  │ (已退款)   │
//!   │(部分退款)     │  └───────────┘
//!   └──────────────┘
//!
//!  任何非终态 ──close──► CLOSED (已关闭)
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use common::{AppError, AppResult};

use super::money::Money;

/// 支付渠道
///
/// 与 proto::payment::PaymentChannel 对应。
/// 新增渠道只需添加变体，不影响现有逻辑。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentChannel {
    /// 未知渠道
    Unknown,
    /// 微信支付
    WeChat,
    /// 支付宝
    Alipay,
    /// 银行卡
    BankCard,
    /// 测试桩（开发环境用，不调用真实第三方）
    Stub,
}

impl PaymentChannel {
    /// 转为字符串（存储到 DB VARCHAR 列）
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentChannel::Unknown => "UNKNOWN",
            PaymentChannel::WeChat => "WECHAT",
            PaymentChannel::Alipay => "ALIPAY",
            PaymentChannel::BankCard => "BANK_CARD",
            PaymentChannel::Stub => "STUB",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_uppercase().as_str() {
            "UNKNOWN" => Ok(PaymentChannel::Unknown),
            "WECHAT" => Ok(PaymentChannel::WeChat),
            "ALIPAY" => Ok(PaymentChannel::Alipay),
            "BANK_CARD" => Ok(PaymentChannel::BankCard),
            "STUB" => Ok(PaymentChannel::Stub),
            _ => Err(AppError::invalid_input(format!(
                "Unknown payment channel: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for PaymentChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 支付状态
///
/// 与 proto::payment::PaymentStatus 对应。
/// 状态机控制合法的状态流转，非法转换返回错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// 待支付
    Pending,
    /// 处理中（已提交到渠道，等待回调）
    Processing,
    /// 支付成功
    Success,
    /// 支付失败
    Failed,
    /// 退款中
    Refunding,
    /// 已退款（全额）
    Refunded,
    /// 部分退款
    PartialRefunded,
    /// 已关闭
    Closed,
}

impl PaymentStatus {
    /// 转为字符串（存储到 DB VARCHAR 列）
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentStatus::Pending => "PENDING",
            PaymentStatus::Processing => "PROCESSING",
            PaymentStatus::Success => "SUCCESS",
            PaymentStatus::Failed => "FAILED",
            PaymentStatus::Refunding => "REFUNDING",
            PaymentStatus::Refunded => "REFUNDED",
            PaymentStatus::PartialRefunded => "PARTIAL_REFUNDED",
            PaymentStatus::Closed => "CLOSED",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(PaymentStatus::Pending),
            "PROCESSING" => Ok(PaymentStatus::Processing),
            "SUCCESS" => Ok(PaymentStatus::Success),
            "FAILED" => Ok(PaymentStatus::Failed),
            "REFUNDING" => Ok(PaymentStatus::Refunding),
            "REFUNDED" => Ok(PaymentStatus::Refunded),
            "PARTIAL_REFUNDED" => Ok(PaymentStatus::PartialRefunded),
            "CLOSED" => Ok(PaymentStatus::Closed),
            _ => Err(AppError::invalid_input(format!(
                "Unknown payment status: {}",
                s
            ))),
        }
    }

    /// 是否为终态（不可再变更）
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PaymentStatus::Failed | PaymentStatus::Refunded | PaymentStatus::Closed
        )
    }
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 支付订单实体
///
/// 记录一次支付请求的完整信息，包括金额、渠道、状态等。
/// 通过状态机方法控制合法的状态流转。
#[derive(Debug, Clone, PartialEq)]
pub struct Payment {
    /// 支付订单 ID（雪花算法生成）
    pub id: u64,
    /// 幂等 key（全局唯一，防重复扣款）
    pub idempotency_key: String,
    /// 用户 ID
    pub user_id: u64,
    /// 关联业务订单 ID
    pub order_id: u64,
    /// 支付金额
    pub amount: Money,
    /// 手续费
    pub fee: Money,
    /// 支付渠道
    pub channel: PaymentChannel,
    /// 支付状态
    pub status: PaymentStatus,
    /// 渠道交易号（渠道返回的交易 ID）
    pub channel_txn_id: String,
    /// 支付链接（如微信扫码链接，H5 支付链接等）
    pub pay_url: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl Payment {
    /// 创建新的支付订单（初始状态为 Pending）
    pub fn new(
        id: u64,
        idempotency_key: String,
        user_id: u64,
        order_id: u64,
        amount: Money,
        channel: PaymentChannel,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            idempotency_key,
            user_id,
            order_id,
            amount,
            fee: Money::zero(amount.currency),
            channel,
            status: PaymentStatus::Pending,
            channel_txn_id: String::new(),
            pay_url: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 开始处理（Pending -> Processing）
    ///
    /// 已提交到支付渠道，等待渠道回调。
    pub fn start_processing(&mut self) -> AppResult<()> {
        match self.status {
            PaymentStatus::Pending => {
                self.status = PaymentStatus::Processing;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot start processing from status: {}",
                self.status
            ))),
        }
    }

    /// 支付成功（Processing -> Success）
    ///
    /// 收到渠道回调确认支付成功，记录渠道交易号和支付链接。
    pub fn succeed(&mut self, channel_txn_id: String, pay_url: String) -> AppResult<()> {
        match self.status {
            PaymentStatus::Processing | PaymentStatus::Pending => {
                self.status = PaymentStatus::Success;
                self.channel_txn_id = channel_txn_id;
                self.pay_url = pay_url;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot succeed from status: {}",
                self.status
            ))),
        }
    }

    /// 支付失败（Processing/Pending -> Failed）
    pub fn fail(&mut self) -> AppResult<()> {
        match self.status {
            PaymentStatus::Processing | PaymentStatus::Pending => {
                self.status = PaymentStatus::Failed;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot fail from status: {}",
                self.status
            ))),
        }
    }

    /// 开始退款（Success/PartialRefunded -> Refunding）
    ///
    /// 只有支付成功的订单可以发起退款。
    pub fn start_refund(&mut self) -> AppResult<()> {
        match self.status {
            PaymentStatus::Success | PaymentStatus::PartialRefunded => {
                self.status = PaymentStatus::Refunding;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot start refund from status: {}",
                self.status
            ))),
        }
    }

    /// 完成退款（Refunding -> Refunded/PartialRefunded）
    ///
    /// `partial` 为 true 表示部分退款，否则为全额退款。
    pub fn complete_refund(&mut self, partial: bool) -> AppResult<()> {
        match self.status {
            PaymentStatus::Refunding => {
                self.status = if partial {
                    PaymentStatus::PartialRefunded
                } else {
                    PaymentStatus::Refunded
                };
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Cannot complete refund from status: {}",
                self.status
            ))),
        }
    }

    /// 关闭订单（非终态 -> Closed）
    ///
    /// 关闭后不可再操作。用于超时未支付的订单。
    pub fn close(&mut self) -> AppResult<()> {
        if self.status.is_terminal() {
            return Err(AppError::invalid_input(format!(
                "Cannot close from terminal status: {}",
                self.status
            )));
        }
        self.status = PaymentStatus::Closed;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 是否可以退款
    ///
    /// 只有支付成功或部分退款的订单可以退款。
    pub fn can_refund(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Success | PaymentStatus::PartialRefunded
        )
    }

    /// 是否可以取消
    ///
    /// 待支付状态可以取消。
    pub fn can_cancel(&self) -> bool {
        matches!(self.status, PaymentStatus::Pending)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Currency;
    use rust_decimal_macros::dec;

    fn create_test_payment() -> Payment {
        Payment::new(
            1,
            "test-key-001".to_string(),
            1001,
            2001,
            Money::new(dec!(99.99), Currency::CNY),
            PaymentChannel::WeChat,
        )
    }

    #[test]
    fn test_new_payment_is_pending() {
        let p = create_test_payment();
        assert_eq!(p.status, PaymentStatus::Pending);
        assert_eq!(p.channel_txn_id, "");
        assert_eq!(p.pay_url, "");
    }

    #[test]
    fn test_start_processing() {
        let mut p = create_test_payment();
        assert!(p.start_processing().is_ok());
        assert_eq!(p.status, PaymentStatus::Processing);
    }

    #[test]
    fn test_start_processing_from_success_fails() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("txn123".to_string(), "https://pay.url".to_string())
            .unwrap();
        assert!(p.start_processing().is_err());
    }

    #[test]
    fn test_succeed() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("wx_txn_001".to_string(), "https://pay.weixin.qq.com".to_string())
            .unwrap();
        assert_eq!(p.status, PaymentStatus::Success);
        assert_eq!(p.channel_txn_id, "wx_txn_001");
    }

    #[test]
    fn test_fail_from_processing() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        assert!(p.fail().is_ok());
        assert_eq!(p.status, PaymentStatus::Failed);
    }

    #[test]
    fn test_fail_from_success_fails() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("txn".to_string(), "url".to_string()).unwrap();
        assert!(p.fail().is_err());
    }

    #[test]
    fn test_refund_flow() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("txn".to_string(), "url".to_string()).unwrap();

        // 全额退款
        assert!(p.start_refund().is_ok());
        assert_eq!(p.status, PaymentStatus::Refunding);
        assert!(p.complete_refund(false).is_ok());
        assert_eq!(p.status, PaymentStatus::Refunded);
    }

    #[test]
    fn test_partial_refund_flow() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("txn".to_string(), "url".to_string()).unwrap();

        // 部分退款
        assert!(p.start_refund().is_ok());
        assert!(p.complete_refund(true).is_ok());
        assert_eq!(p.status, PaymentStatus::PartialRefunded);
    }

    #[test]
    fn test_can_refund() {
        let mut p = create_test_payment();
        assert!(!p.can_refund()); // Pending 不可退款

        p.start_processing().unwrap();
        assert!(!p.can_refund()); // Processing 不可退款

        p.succeed("txn".to_string(), "url".to_string()).unwrap();
        assert!(p.can_refund()); // Success 可退款
    }

    #[test]
    fn test_close_from_pending() {
        let mut p = create_test_payment();
        assert!(p.close().is_ok());
        assert_eq!(p.status, PaymentStatus::Closed);
    }

    #[test]
    fn test_close_from_refunded_fails() {
        let mut p = create_test_payment();
        p.start_processing().unwrap();
        p.succeed("txn".to_string(), "url".to_string()).unwrap();
        p.start_refund().unwrap();
        p.complete_refund(false).unwrap();
        assert!(p.close().is_err()); // 终态不可关闭
    }

    #[test]
    fn test_is_terminal() {
        assert!(!PaymentStatus::Pending.is_terminal());
        assert!(!PaymentStatus::Success.is_terminal());
        assert!(PaymentStatus::Failed.is_terminal());
        assert!(PaymentStatus::Refunded.is_terminal());
        assert!(PaymentStatus::Closed.is_terminal());
    }
}
