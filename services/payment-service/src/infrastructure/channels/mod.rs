//! 第三方支付渠道适配器模块。
//!
//! 定义统一的 [`PaymentChannelAdapter`] trait，所有第三方支付渠道
//! （微信支付、支付宝、银行卡等）都实现该 trait，对上层（application 层）
//! 屏蔽各渠道 API 差异。application 层通过 [`crate::domain`] 的
//! `PaymentChannel` 枚举路由到具体适配器实现。
//!
//! - `stub`：开发/测试环境桩，不依赖真实第三方
//! - `wechat`：微信支付适配器（框架，待实现）
//! - `alipay`：支付宝适配器（框架，待实现）

pub mod stub;
pub mod wechat;
pub mod alipay;

pub use stub::StubChannelAdapter;
pub use wechat::WeChatPayAdapter;
pub use alipay::AlipayAdapter;

use async_trait::async_trait;
use common::AppResult;
use crate::domain::{Money, PaymentChannel};

/// 支付渠道适配器 trait。
///
/// 每个第三方支付渠道实现此 trait，对外统一接口。
/// application 层的 `PaymentRouter` 根据支付渠道类型选择对应的适配器实例，
/// 从而将业务逻辑与具体渠道 SDK 解耦。
#[async_trait]
pub trait PaymentChannelAdapter: Send + Sync {
    /// 发起支付，返回渠道侧交易号与支付链接（如微信扫码链接）。
    async fn pay(
        &self,
        payment_id: u64,
        amount: &Money,
        description: &str,
    ) -> AppResult<ChannelPayResult>;

    /// 查询渠道侧支付状态（用于主动对账/补单）。
    async fn query(&self, channel_txn_id: &str) -> AppResult<ChannelQueryResult>;

    /// 发起退款。
    async fn refund(
        &self,
        channel_txn_id: &str,
        refund_amount: &Money,
        reason: &str,
    ) -> AppResult<ChannelRefundResult>;

    /// 验证回调签名，防止伪造回调。
    async fn verify_callback(&self, raw_data: &str, signature: &str) -> AppResult<bool>;

    /// 返回该适配器对应的渠道类型。
    fn channel_type(&self) -> PaymentChannel;
}

/// 渠道支付结果。
pub struct ChannelPayResult {
    /// 渠道侧交易号（用于后续查询/对账）
    pub channel_txn_id: String,
    /// 支付链接（如微信扫码链接、支付宝跳转链接）
    pub pay_url: String,
}

/// 渠道查询结果。
pub struct ChannelQueryResult {
    /// 是否支付成功
    pub success: bool,
    /// 渠道侧交易号
    pub channel_txn_id: String,
}

/// 渠道退款结果。
pub struct ChannelRefundResult {
    /// 渠道侧退款交易号
    pub refund_txn_id: String,
    /// 退款是否成功
    pub success: bool,
}
