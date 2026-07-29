//! 支付应用服务（核心业务编排）
//!
//! 应用服务是 DDD 应用层的核心，负责协调领域对象、仓储、幂等控制、
//! 路由引擎和渠道适配器，编排完整的支付业务用例。
//! 不包含业务规则（规则在 domain 层），只负责流程编排和事务边界管理。

use std::sync::Arc;

use common::{AppError, AppResult, SnowflakeIdGenerator};

use crate::domain::{PaymentRepository, RefundRepository, TransactionRepository};
use crate::infrastructure::PaymentChannelAdapter;

use super::command::{CallbackCommand, CreatePaymentCommand, RefundCommand};
use super::dto::{CallbackResultDto, PaymentDto, RefundDto};
use super::idempotency::IdempotencyService;
use super::routing::PaymentRouter;

/// 支付应用服务
///
/// 编排支付创建、查询、退款、回调处理等核心业务用例。
/// 通过 Arc 共享各依赖，支持 Clone（供 interface 层 gRPC handler 使用）。
#[derive(Clone)]
pub struct PaymentApplicationService {
    /// 支付订单仓储
    payment_repo: Arc<dyn PaymentRepository>,
    /// 资金流水仓储
    txn_repo: Arc<dyn TransactionRepository>,
    /// 退款仓储
    refund_repo: Arc<dyn RefundRepository>,
    /// 幂等性控制服务
    idempotency: IdempotencyService,
    /// 支付路由器
    router: PaymentRouter,
    /// 渠道适配器（从 infrastructure 层注入，对接微信/支付宝/Stub 等）
    channel_adapter: Arc<dyn PaymentChannelAdapter>,
    /// 雪花 ID 生成器
    id_generator: Arc<SnowflakeIdGenerator>,
    /// 事件总线 Producer（可选，Kafka 不可用时为 None）
    event_producer: Option<event_bus::EventBusProducer>,
}

impl PaymentApplicationService {
    pub fn new(
        payment_repo: Arc<dyn PaymentRepository>,
        txn_repo: Arc<dyn TransactionRepository>,
        refund_repo: Arc<dyn RefundRepository>,
        channel_adapter: Arc<dyn PaymentChannelAdapter>,
        id_generator: Arc<SnowflakeIdGenerator>,
    ) -> Self {
        // 构建幂等控制服务（复用支付/退款仓储）
        let idempotency = IdempotencyService::new(payment_repo.clone(), refund_repo.clone());
        // 使用默认加权路由策略
        let router = PaymentRouter::with_default_strategy();

        Self {
            payment_repo,
            txn_repo,
            refund_repo,
            idempotency,
            router,
            channel_adapter,
            id_generator,
            event_producer: None,
        }
    }

    /// 注入事件总线 Producer（启用 Kafka 事件发布）
    pub fn with_event_producer(mut self, producer: event_bus::EventBusProducer) -> Self {
        self.event_producer = Some(producer);
        self
    }

    /// 创建支付订单
    ///
    /// 业务流程：
    /// 1. **幂等检查**：若同一 idempotency_key 已有成功支付，直接返回已有结果
    /// 2. **风控校验**（TODO）：黑名单、金额阈值、频次、设备指纹等
    /// 3. **路由选择**：通过 PaymentRouter 选择最优渠道（可覆盖用户偏好）
    /// 4. **创建支付订单**：生成雪花 ID，持久化 Payment（状态 Pending）
    /// 5. **调用渠道适配器**：通过 PaymentChannelAdapter 发起真实支付，获取 pay_url / channel_txn_id
    /// 6. **记录流水**：写入 Transaction（txn_type = PAY）
    /// 7. **发布事件**：通过 EventBusProducer 发布 PaymentSucceeded/PaymentFailed 事件
    /// 8. **返回** PaymentDto
    pub async fn create_payment(&self, cmd: CreatePaymentCommand) -> AppResult<PaymentDto> {
        let _ = (
            &self.idempotency,
            &self.router,
            &self.channel_adapter,
            &self.id_generator,
            &self.payment_repo,
            &self.txn_repo,
            &self.event_producer,
            &cmd,
        );
        // TODO: 实现创建支付完整流程
        // 事件发布示例（在支付成功后）:
        //   if let Some(ref producer) = self.event_producer {
        //       producer.publish(EventPayload::PaymentSucceeded {
        //           payment_id, order_id, user_id,
        //           amount_cents, currency, channel,
        //       }).await?;
        //   }
        todo!("实现创建支付完整流程")
    }

    /// 查询支付订单
    ///
    /// 业务流程：
    /// 1. 根据 payment_id 从仓储加载 Payment
    /// 2. 转换为 PaymentDto 返回
    pub async fn get_payment(&self, payment_id: u64) -> AppResult<PaymentDto> {
        let payment = self
            .payment_repo
            .find_by_id(payment_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Payment {} not found", payment_id)))?;

        Ok(PaymentDto::from(payment))
    }

    /// 发起退款
    ///
    /// 业务流程：
    /// 1. **幂等检查**：若同一 idempotency_key 已有成功退款，直接返回已有结果
    /// 2. **查原支付订单**：根据 payment_id 加载 Payment
    /// 3. **校验可退款**：原支付须为 Success，退款金额 ≤ 可退金额（原金额 - 已退金额）
    /// 4. **调用渠道退款**：通过 PaymentChannelAdapter 发起退款
    /// 5. **更新状态**：Payment 状态流转为 Refunding/PartialRefunded/Refunded
    /// 6. **记录流水**：写入 Transaction（txn_type = REFUND）
    /// 7. **返回** RefundDto
    pub async fn refund(&self, cmd: RefundCommand) -> AppResult<RefundDto> {
        let _ = (
            &self.idempotency,
            &self.channel_adapter,
            &self.id_generator,
            &self.payment_repo,
            &self.refund_repo,
            &self.txn_repo,
            &cmd,
        );
        // TODO: 实现
        todo!("实现退款完整流程")
    }

    /// 查询退款订单
    ///
    /// 业务流程：
    /// 1. 根据 refund_id 从仓储加载 Refund
    /// 2. 转换为 RefundDto 返回
    pub async fn get_refund(&self, refund_id: u64) -> AppResult<RefundDto> {
        let refund = self
            .refund_repo
            .find_by_id(refund_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Refund {} not found", refund_id)))?;

        Ok(RefundDto::from(refund))
    }

    /// 处理渠道异步回调
    ///
    /// 业务流程：
    /// 1. **渠道验签**：通过 PaymentChannelAdapter 校验回调签名，防篡改
    /// 2. **更新支付状态**：根据回调内容流转 Payment 状态机（Success/Failed）
    /// 3. **记录流水**：写入 Transaction 记录渠道交易号与金额变动
    /// 4. **返回** CallbackResultDto（含 success、message、new_status）
    pub async fn handle_callback(&self, cmd: CallbackCommand) -> AppResult<CallbackResultDto> {
        let _ = (
            &self.channel_adapter,
            &self.payment_repo,
            &self.txn_repo,
            &cmd,
        );
        // TODO: 实现
        todo!("实现回调处理完整流程")
    }
}
