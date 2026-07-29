//! 邮件 gRPC 服务实现

use crate::application::command::{
    SendCustomEmailCommand, SendOrderNotificationCommand, SendPasswordResetEmailCommand,
    SendVerificationEmailCommand,
};
use crate::application::EmailApplicationService;
use crate::domain::{EmailStatus, EmailType};
use tonic::{Request, Response, Status};

use proto::email::{
    email_service_server::EmailService, GetEmailStatusRequest, GetEmailStatusResponse,
    SendCustomEmailRequest, SendEmailResponse, SendOrderNotificationRequest,
    SendPasswordResetEmailRequest, SendVerificationEmailRequest,
};

/// 邮件 gRPC 服务实现
#[derive(Clone)]
pub struct EmailServiceImpl {
    service: EmailApplicationService,
}

impl EmailServiceImpl {
    pub fn new(service: EmailApplicationService) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl EmailService for EmailServiceImpl {
    async fn send_verification_email(
        &self,
        request: Request<SendVerificationEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("📨 收到发送验证邮件请求：{}", req.to_email);

        match self
            .service
            .send_verification_email(SendVerificationEmailCommand {
                to_email: req.to_email,
                username: req.username,
                verification_code: req.verification_code,
            })
            .await
        {
            Ok(message_id) => Ok(Response::new(SendEmailResponse {
                success: true,
                message_id,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn send_order_notification(
        &self,
        request: Request<SendOrderNotificationRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("📨 收到发送订单通知邮件请求：订单 #{}", req.order_id);

        match self
            .service
            .send_order_notification(SendOrderNotificationCommand {
                to_email: req.to_email,
                username: req.username,
                order_id: req.order_id,
                total_amount: req.total_amount,
                status: req.status,
            })
            .await
        {
            Ok(message_id) => Ok(Response::new(SendEmailResponse {
                success: true,
                message_id,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn send_password_reset_email(
        &self,
        request: Request<SendPasswordResetEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("📨 收到发送密码重置邮件请求：{}", req.to_email);

        match self
            .service
            .send_password_reset_email(SendPasswordResetEmailCommand {
                to_email: req.to_email,
                username: req.username,
                reset_token: req.reset_token,
            })
            .await
        {
            Ok(message_id) => Ok(Response::new(SendEmailResponse {
                success: true,
                message_id,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn send_custom_email(
        &self,
        request: Request<SendCustomEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("📨 收到发送自定义邮件请求：{}", req.to_email);

        match self
            .service
            .send_custom_email(SendCustomEmailCommand {
                to_email: req.to_email,
                username: req.username,
                subject: req.subject,
                html_content: req.html_content,
            })
            .await
        {
            Ok(message_id) => Ok(Response::new(SendEmailResponse {
                success: true,
                message_id,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_email_status(
        &self,
        request: Request<GetEmailStatusRequest>,
    ) -> Result<Response<GetEmailStatusResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("📨 收到查询邮件状态请求：ID {}", req.id);

        match self.service.get_email_status(req.id).await {
            Ok(Some(email)) => Ok(Response::new(GetEmailStatusResponse {
                id: email.id,
                to_email: email.to_email,
                username: email.username,
                subject: email.subject,
                html_content: email.html_content,
                email_type: email_type_to_string(email.email_type).to_string(),
                status: email_status_to_string(email.status).to_string(),
                message_id: email.message_id,
                error_message: email.error_message,
                created_at: email.created_at.to_rfc3339(),
                updated_at: email.updated_at.to_rfc3339(),
            })),
            Ok(None) => Err(Status::not_found(format!("邮件 ID {} 不存在", req.id))),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}

/// 将邮件类型枚举转换为字符串
fn email_type_to_string(t: EmailType) -> &'static str {
    match t {
        EmailType::Verification => "Verification",
        EmailType::OrderNotification => "OrderNotification",
        EmailType::PasswordReset => "PasswordReset",
        EmailType::Custom => "Custom",
    }
}

/// 将邮件状态枚举转换为字符串
fn email_status_to_string(s: EmailStatus) -> &'static str {
    match s {
        EmailStatus::Pending => "Pending",
        EmailStatus::Sent => "Sent",
        EmailStatus::Failed => "Failed",
    }
}
