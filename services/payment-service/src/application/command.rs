//! 支付服务命令对象（Command）
//!
//! Command 模式将应用服务的输入参数封装为独立的命令对象，而非使用一长串位置参数。
//! 好处：
//! 1. 参数语义清晰，调用方可读性强；
//! 2. 便于扩展参数（新增字段不破坏已有调用方签名）；
//! 3. 命令对象可在 interface 层与 application 层之间传递，解耦两层；
//! 4. 便于对单个命令做参数校验与风控打标。

use crate::domain::{Money, PaymentChannel};

/// 创建支付命令
///
/// 封装创建支付订单所需的全部输入参数。
/// 由 interface 层从 gRPC 请求构造，传递给应用服务执行。
#[derive(Debug, Clone)]
pub struct CreatePaymentCommand {
    /// 幂等 key（全局唯一，防重复扣款）
    pub idempotency_key: String,
    /// 用户 ID
    pub user_id: u64,
    /// 关联业务订单 ID
    pub order_id: u64,
    /// 支付金额
    pub amount: Money,
    /// 支付渠道
    pub channel: PaymentChannel,
    /// 支付描述
    pub description: String,
    /// 客户端 IP（风控用）
    pub client_ip: String,
    /// 设备指纹（风控用）
    pub device_id: String,
}

/// 退款命令
///
/// 封装发起退款所需的全部输入参数。
#[derive(Debug, Clone)]
pub struct RefundCommand {
    /// 幂等 key
    pub idempotency_key: String,
    /// 原支付订单 ID
    pub payment_id: u64,
    /// 退款金额（部分退款时小于原金额）
    pub refund_amount: Money,
    /// 退款原因
    pub reason: String,
}

/// 渠道回调命令
///
/// 封装第三方支付渠道异步回调的原始数据，供应用服务验签并流转状态机。
#[derive(Debug, Clone)]
pub struct CallbackCommand {
    /// 回调来源渠道
    pub channel: PaymentChannel,
    /// 回调原始数据（JSON/XML）
    pub raw_data: String,
    /// 签名（验签用）
    pub signature: String,
    /// 回调时间戳
    pub timestamp: String,
}
