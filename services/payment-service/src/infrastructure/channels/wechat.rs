//! 微信支付渠道适配器（框架，待实现）。
//!
//! 微信支付 API 调用方式概述：
//! - 统一下单：调用 `https://api.mch.weixin.qq.com/pay/unifiedorder`，
//!   传入 appid/mch_id/nonce_str/sign/body/out_trade_no/total_fee/notify_url 等参数，
//!   返回 prepay_id，再拼接生成支付链接（如扫码支付为 `weixin://wxpay/bizpayurl?pr=...`）。
//! - 订单查询：调用 `pay/orderquery`，按 out_trade_no 或 transaction_id 查询。
//! - 退款：调用 `pay/refund`，需上传商户证书，返回退款单号。
//! - 异步回调：微信以 POST XML 推送到 notify_url，需用 API key 校验签名。
//!
//! 金额单位：微信支付金额以「分」为单位（整数），需将 `Decimal` 元转换为分。
//! 签名算法：MD5（旧版）或 HMAC-SHA256，按 key 字典序拼接后加盐。

use async_trait::async_trait;
use common::AppResult;
use crate::domain::{Money, PaymentChannel};
use super::{
    ChannelPayResult, ChannelQueryResult, ChannelRefundResult, PaymentChannelAdapter,
};

/// 微信支付适配器。
///
/// 持有微信商户后台分配的凭证，在 `PaymentChannelAdapter` 各方法中
/// 调用微信支付 OpenAPI。当前为框架实现，具体 API 调用待补全。
#[derive(Clone)]
pub struct WeChatPayAdapter {
    /// 应用 ID（公众号/小程序/APP）
    pub app_id: String,
    /// 商户号
    pub mch_id: String,
    /// API 密钥（用于签名）
    pub api_key: String,
    /// 异步回调地址
    pub notify_url: String,
}

impl WeChatPayAdapter {
    pub fn new(app_id: String, mch_id: String, api_key: String, notify_url: String) -> Self {
        Self { app_id, mch_id, api_key, notify_url }
    }
}

#[async_trait]
impl PaymentChannelAdapter for WeChatPayAdapter {
    async fn pay(
        &self,
        payment_id: u64,
        amount: &Money,
        description: &str,
    ) -> AppResult<ChannelPayResult> {
        // TODO: 调用微信支付 API
        // 1. 组装统一下单参数（appid, mch_id, nonce_str, body=description,
        //    out_trade_no=payment_id, total_fee=amount 转分, notify_url, ...）
        // 2. 按字典序拼接并 MD5/HMAC-SHA256 签名
        // 3. POST XML 到统一下单接口，解析 prepay_id
        // 4. 拼接支付链接返回
        let _ = (payment_id, amount, description);
        todo!("调用微信支付统一下单 API")
    }

    async fn query(&self, channel_txn_id: &str) -> AppResult<ChannelQueryResult> {
        // TODO: 调用微信支付 API
        // 调用 pay/orderquery，按 transaction_id 查询并解析 trade_state
        let _ = channel_txn_id;
        todo!("调用微信支付订单查询 API")
    }

    async fn refund(
        &self,
        channel_txn_id: &str,
        refund_amount: &Money,
        reason: &str,
    ) -> AppResult<ChannelRefundResult> {
        // TODO: 调用微信支付 API
        // 调用 pay/refund（需商户证书），传入 transaction_id、退款金额、退款原因
        let _ = (channel_txn_id, refund_amount, reason);
        todo!("调用微信支付退款 API")
    }

    async fn verify_callback(&self, raw_data: &str, signature: &str) -> AppResult<bool> {
        // TODO: 调用微信支付 API
        // 使用 api_key 对回调 XML 重新计算签名，与传入 signature 比对
        let _ = (raw_data, signature);
        todo!("校验微信支付回调签名")
    }

    fn channel_type(&self) -> PaymentChannel {
        PaymentChannel::WeChat
    }
}
