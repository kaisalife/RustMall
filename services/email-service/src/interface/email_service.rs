//! 邮件 gRPC 服务实现

use tonic::{Request, Response, Status};
use crate::application::EmailApplicationService;
use crate::application::command::{
    SendVerificationEmailCommand,
    SendOrderNotificationCommand,
    SendPasswordResetEmailCommand,
    SendCustomEmailCommand,
};

use proto::email::{
    email_service_server::EmailService,
    SendVerificationEmailRequest,
    SendOrderNotificationRequest,
    SendPasswordResetEmailRequest,
    SendCustomEmailRequest,
    SendEmailResponse,
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

        match self.service.send_verification_email(
            SendVerificationEmailCommand {
                to_email: req.to_email,
                username: req.username,
                verification_code: req.verification_code,
            },
        ).await {
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

        match self.service.send_order_notification(
            SendOrderNotificationCommand {
                to_email: req.to_email,
                username: req.username,
                order_id: req.order_id,
                total_amount: req.total_amount,
                status: req.status,
            },
        ).await {
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

        match self.service.send_password_reset_email(
            SendPasswordResetEmailCommand {
                to_email: req.to_email,
                username: req.username,
                reset_token: req.reset_token,
            },
        ).await {
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

        match self.service.send_custom_email(
            SendCustomEmailCommand {
                to_email: req.to_email,
                username: req.username,
                subject: req.subject,
                html_content: req.html_content,
            },
        ).await {
            Ok(message_id) => Ok(Response::new(SendEmailResponse {
                success: true,
                message_id,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
