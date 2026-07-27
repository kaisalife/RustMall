//! 支付服务 gRPC 接口实现。
//!
//! [`PaymentServiceImpl`] 实现 proto 生成的 `PaymentService` trait，
//! 负责将 gRPC 请求转换为 application 层命令（Command）调用，
//! 再将 application 层返回的 DTO 转换为 gRPC 响应。
//!
//! ## 职责边界
//! - 接口层只做协议适配与类型转换，不包含业务逻辑
//! - proto `Money` 以字符串传递 Decimal，需与 domain `Money(Decimal)` 互转
//! - proto 枚举（`PaymentChannel`/`PaymentStatus`）需与 domain 枚举互转
//! - 业务逻辑（幂等、渠道路由、状态机流转）由 application 层负责
//!
//! ## RPC 方法
//! - `create_payment`：创建支付订单（含幂等控制）
//! - `get_payment`：查询支付状态
//! - `refund`：发起退款
//! - `get_refund`：查询退款状态
//! - `handle_callback`：处理渠道异步回调（验签 + 状态机流转）

use std::sync::Arc;

use tonic::{Request, Response, Status};

// 命令类型在 TODO 实现后用于构造 application 层调用入参，暂以 allow 抑制未使用告警。
#[allow(unused_imports)]
use crate::application::{
    CallbackCommand, CreatePaymentCommand, PaymentApplicationService, RefundCommand,
};

use proto::payment::{
    payment_service_server::PaymentService, CallbackRequest, CallbackResponse,
    CreatePaymentRequest, GetPaymentRequest, GetRefundRequest, PaymentResponse,
    RefundRequest, RefundResponse,
};

/// gRPC 服务实现，持有 application 层服务的共享引用。
///
/// 通过 `Arc<PaymentApplicationService>` 共享应用服务，
/// 所有 RPC 方法委托给应用服务完成业务处理。
#[derive(Clone)]
pub struct PaymentServiceImpl {
    service: Arc<PaymentApplicationService>,
}

impl PaymentServiceImpl {
    pub fn new(service: Arc<PaymentApplicationService>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl PaymentService for PaymentServiceImpl {
    /// 创建支付订单（含幂等控制）。
    ///
    /// 转换流程：`CreatePaymentRequest` -> `CreatePaymentCommand` ->
    /// `service.create_payment()` -> `PaymentDto` -> `PaymentResponse`。
    async fn create_payment(
        &self,
        request: Request<CreatePaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 实现 proto <-> domain 转换并调用 service
        // 1. 将 proto Money(amount: String) 解析为 domain Money(Decimal)
        // 2. 将 proto PaymentChannel 枚举映射为 domain PaymentChannel
        // 3. 组装 CreatePaymentCommand { idempotency_key, user_id, order_id,
        //    amount, channel, description, client_ip, device_id }
        // 4. let dto = self.service.create_payment(command).await
        //        .map_err(app_error_to_status)?;
        // 5. 将 PaymentDto 转换为 PaymentResponse 返回
        todo!("实现 create_payment 的 proto<->domain 转换并调用 service")
    }

    /// 查询支付状态。
    ///
    /// 转换流程：`GetPaymentRequest` -> `service.get_payment(payment_id)` ->
    /// `PaymentDto` -> `PaymentResponse`。
    async fn get_payment(
        &self,
        request: Request<GetPaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 实现 proto <-> domain 转换并调用 service
        // let dto = self.service.get_payment(_req.payment_id).await
        //     .map_err(app_error_to_status)?;
        // 将 PaymentDto 转换为 PaymentResponse 返回
        todo!("实现 get_payment 的 proto<->domain 转换并调用 service")
    }

    /// 发起退款。
    ///
    /// 转换流程：`RefundRequest` -> `RefundCommand` ->
    /// `service.refund()` -> `RefundDto` -> `RefundResponse`。
    async fn refund(
        &self,
        request: Request<RefundRequest>,
    ) -> Result<Response<RefundResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 实现 proto <-> domain 转换并调用 service
        // 1. 将 proto Money 解析为 domain Money(Decimal)
        // 2. 组装 RefundCommand { idempotency_key, payment_id, refund_amount, reason }
        // 3. let dto = self.service.refund(command).await
        //        .map_err(app_error_to_status)?;
        // 4. 将 RefundDto 转换为 RefundResponse 返回
        todo!("实现 refund 的 proto<->domain 转换并调用 service")
    }

    /// 查询退款状态。
    ///
    /// 转换流程：`GetRefundRequest` -> `service.get_refund(refund_id)` ->
    /// `RefundDto` -> `RefundResponse`。
    async fn get_refund(
        &self,
        request: Request<GetRefundRequest>,
    ) -> Result<Response<RefundResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 实现 proto <-> domain 转换并调用 service
        // let dto = self.service.get_refund(_req.refund_id).await
        //     .map_err(app_error_to_status)?;
        // 将 RefundDto 转换为 RefundResponse 返回
        todo!("实现 get_refund 的 proto<->domain 转换并调用 service")
    }

    /// 处理渠道异步回调（验签 + 状态机流转）。
    ///
    /// 转换流程：`CallbackRequest` -> `CallbackCommand` ->
    /// `service.handle_callback()` -> `CallbackResultDto` -> `CallbackResponse`。
    async fn handle_callback(
        &self,
        request: Request<CallbackRequest>,
    ) -> Result<Response<CallbackResponse>, Status> {
        let _req = request.into_inner();
        // TODO: 实现 proto <-> domain 转换并调用 service
        // 1. 将 proto PaymentChannel 映射为 domain PaymentChannel
        // 2. 组装 CallbackCommand { channel, raw_data, signature, timestamp }
        // 3. let dto = self.service.handle_callback(command).await
        //        .map_err(app_error_to_status)?;
        // 4. 将 CallbackResultDto 转换为 CallbackResponse 返回
        todo!("实现 handle_callback 的 proto<->domain 转换并调用 service")
    }
}

/// 将应用层错误转换为 gRPC 状态码。
///
/// 与其他服务保持一致的错误映射策略：
/// - `NotFound` -> NOT_FOUND
/// - `InvalidInput` -> INVALID_ARGUMENT
/// - `Conflict` -> ALREADY_EXISTS
/// - `Unauthorized`/`Authentication` -> UNAUTHENTICATED
/// - `Forbidden` -> PERMISSION_DENIED
/// - 其他 -> INTERNAL
fn app_error_to_status(error: common::AppError) -> Status {
    match error {
        common::AppError::NotFound(msg) => Status::not_found(msg),
        common::AppError::InvalidInput(msg) => Status::invalid_argument(msg),
        common::AppError::Conflict(msg) => Status::already_exists(msg),
        common::AppError::Unauthorized(msg) => Status::unauthenticated(msg),
        common::AppError::Authentication(msg) => Status::unauthenticated(msg),
        common::AppError::Forbidden(msg) => Status::permission_denied(msg),
        common::AppError::Database(e) => Status::internal(e.to_string()),
        common::AppError::Internal(msg) => Status::internal(msg),
        _ => Status::internal(error.to_string()),
    }
}
