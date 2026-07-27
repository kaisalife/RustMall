//! 测试桩渠道适配器（开发环境用）。
//!
//! `StubChannelAdapter` 不依赖任何真实第三方支付系统，
//! 所有方法返回模拟成功结果，便于本地开发与自动化测试。
//!
//! 生产环境应替换为真实的渠道适配器（如 `WeChatPayAdapter` / `AlipayAdapter`），
//! 通过 `PaymentRouter` 的渠道路由注入。

use async_trait::async_trait;
use common::AppResult;
use crate::domain::{Money, PaymentChannel};
use super::{
    ChannelPayResult, ChannelQueryResult, ChannelRefundResult, PaymentChannelAdapter,
};

/// 支付渠道测试桩。
///
/// 所有操作均返回模拟成功结果，渠道交易号以 `stub-` 前缀生成，
/// 便于在日志/数据库中识别来自测试桩的数据。
#[derive(Clone, Default)]
pub struct StubChannelAdapter;

impl StubChannelAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PaymentChannelAdapter for StubChannelAdapter {
    async fn pay(
        &self,
        payment_id: u64,
        _amount: &Money,
        _description: &str,
    ) -> AppResult<ChannelPayResult> {
        // 模拟渠道返回：生成一个可识别的渠道交易号与支付链接
        Ok(ChannelPayResult {
            channel_txn_id: format!("stub-pay-{}", payment_id),
            pay_url: format!("https://stub.example.com/pay/{}", payment_id),
        })
    }

    async fn query(&self, channel_txn_id: &str) -> AppResult<ChannelQueryResult> {
        // 模拟查询：始终返回成功
        Ok(ChannelQueryResult {
            success: true,
            channel_txn_id: channel_txn_id.to_string(),
        })
    }

    async fn refund(
        &self,
        channel_txn_id: &str,
        _refund_amount: &Money,
        _reason: &str,
    ) -> AppResult<ChannelRefundResult> {
        // 模拟退款：始终返回成功，退款交易号基于原交易号派生
        Ok(ChannelRefundResult {
            refund_txn_id: format!("stub-refund-{}", channel_txn_id),
            success: true,
        })
    }

    async fn verify_callback(&self, _raw_data: &str, _signature: &str) -> AppResult<bool> {
        // 模拟验签：始终返回验证通过
        Ok(true)
    }

    fn channel_type(&self) -> PaymentChannel {
        PaymentChannel::Stub
    }
}
