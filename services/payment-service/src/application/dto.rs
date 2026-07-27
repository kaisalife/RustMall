//! 支付服务数据传输对象（DTO）
//!
//! DTO 是应用层返回给 interface 层的数据载体，只包含展示所需的字段，
//! 不暴露领域对象的内部行为。interface 层负责将 DTO 转换为 proto 响应消息。

use serde::{Deserialize, Serialize};

use crate::domain::{Money, Payment, PaymentChannel, PaymentStatus, Refund};

/// 支付订单 DTO
///
/// 对应 proto::payment::PaymentResponse，由 interface 层转换。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentDto {
    /// 支付订单 ID
    pub payment_id: u64,
    /// 幂等 key
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
    /// 渠道交易号
    pub channel_txn_id: String,
    /// 支付链接（如微信扫码链接）
    pub pay_url: String,
    /// 创建时间（RFC3339）
    pub created_at: String,
    /// 更新时间（RFC3339）
    pub updated_at: String,
}

impl From<Payment> for PaymentDto {
    fn from(p: Payment) -> Self {
        Self {
            payment_id: p.id,
            idempotency_key: p.idempotency_key,
            user_id: p.user_id,
            order_id: p.order_id,
            amount: p.amount,
            fee: p.fee,
            channel: p.channel,
            status: p.status,
            channel_txn_id: p.channel_txn_id,
            pay_url: p.pay_url,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

/// 退款 DTO
///
/// 对应 proto::payment::RefundResponse，由 interface 层转换。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundDto {
    /// 退款单 ID
    pub refund_id: u64,
    /// 原支付订单 ID
    pub payment_id: u64,
    /// 退款金额
    pub refund_amount: Money,
    /// 退款状态
    pub status: PaymentStatus,
    /// 渠道退款交易号
    pub channel_txn_id: String,
    /// 创建时间（RFC3339）
    pub created_at: String,
    /// 更新时间（RFC3339）
    pub updated_at: String,
}

impl From<Refund> for RefundDto {
    fn from(r: Refund) -> Self {
        Self {
            refund_id: r.id,
            payment_id: r.payment_id,
            refund_amount: r.refund_amount,
            status: r.status,
            channel_txn_id: r.channel_txn_id,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

/// 回调处理结果 DTO
///
/// 对应 proto::payment::CallbackResponse，由 interface 层转换。
#[derive(Debug, Clone)]
pub struct CallbackResultDto {
    /// 回调处理是否成功
    pub success: bool,
    /// 处理结果描述
    pub message: String,
    /// 更新后的支付状态
    pub new_status: PaymentStatus,
}
