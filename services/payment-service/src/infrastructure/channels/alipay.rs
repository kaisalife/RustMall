//! 支付宝渠道适配器（框架，待实现）。
//!
//! 支付宝 API 调用方式概述：
//! - 统一收单下单：调用 `alipay.trade.pay` / `alipay.trade.precreate`（扫码），
//!   公共参数含 app_id/method/sign_type/sign/timestamp/biz_content 等，
//!   biz_content 为 JSON，包含 out_trade_no/total_amount/subject 等。
//! - 交易查询：调用 `alipay.trade.query`，按 out_trade_no 或 trade_no 查询。
//! - 退款：调用 `alipay.trade.refund`，传入 out_trade_no、退款金额、退款请求号。
//! - 异步回调：支付宝以 POST form 推送到 notify_url，需用支付宝公钥验签。
//!
//! 金额单位：支付宝金额以「元」为单位（字符串，两位小数），与 `Decimal` 直接对应。
//! 签名算法：RSA2（推荐）或 RSA，使用商户私钥签名、支付宝公钥验签。

use super::{ChannelPayResult, ChannelQueryResult, ChannelRefundResult, PaymentChannelAdapter};
use crate::domain::{Money, PaymentChannel};
use async_trait::async_trait;
use common::AppResult;

/// 支付宝适配器。
///
/// 持有支付宝开放平台分配的凭证，在 `PaymentChannelAdapter` 各方法中
/// 调用支付宝 OpenAPI。当前为框架实现，具体 API 调用待补全。
#[derive(Clone)]
pub struct AlipayAdapter {
    /// 应用 ID
    pub app_id: String,
    /// 应用私钥（用于请求签名）
    pub private_key: String,
    /// 支付宝公钥（用于回调验签）
    pub alipay_public_key: String,
    /// 异步回调地址
    pub notify_url: String,
}

impl AlipayAdapter {
    pub fn new(
        app_id: String,
        private_key: String,
        alipay_public_key: String,
        notify_url: String,
    ) -> Self {
        Self {
            app_id,
            private_key,
            alipay_public_key,
            notify_url,
        }
    }
}

#[async_trait]
impl PaymentChannelAdapter for AlipayAdapter {
    async fn pay(
        &self,
        payment_id: u64,
        amount: &Money,
        description: &str,
    ) -> AppResult<ChannelPayResult> {
        // TODO: 调用支付宝 API
        // 1. 组装公共参数（app_id, method=alipay.trade.precreate, ...）
        // 2. 构造 biz_content（out_trade_no=payment_id, total_amount=amount, subject=description）
        // 3. 用商户私钥 RSA2 签名后请求网关
        // 4. 解析 qr_code（扫码链接）返回
        let _ = (payment_id, amount, description);
        todo!("调用支付宝统一收单下单 API")
    }

    async fn query(&self, channel_txn_id: &str) -> AppResult<ChannelQueryResult> {
        // TODO: 调用支付宝 API
        // 调用 alipay.trade.query，按 trade_no 查询并解析 trade_status
        let _ = channel_txn_id;
        todo!("调用支付宝交易查询 API")
    }

    async fn refund(
        &self,
        channel_txn_id: &str,
        refund_amount: &Money,
        reason: &str,
    ) -> AppResult<ChannelRefundResult> {
        // TODO: 调用支付宝 API
        // 调用 alipay.trade.refund，传入 trade_no、退款金额、退款请求号、退款原因
        let _ = (channel_txn_id, refund_amount, reason);
        todo!("调用支付宝退款 API")
    }

    async fn verify_callback(&self, raw_data: &str, signature: &str) -> AppResult<bool> {
        // TODO: 调用支付宝 API
        // 使用支付宝公钥对回调 form 数据重新验签，与传入 signature 比对
        let _ = (raw_data, signature);
        todo!("校验支付宝回调签名")
    }

    fn channel_type(&self) -> PaymentChannel {
        PaymentChannel::Alipay
    }
}
