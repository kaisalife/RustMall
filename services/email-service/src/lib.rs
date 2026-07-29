//! Email Service gRPC 客户端库

pub use proto::email::v1::{
    email_service_client::EmailServiceClient, SendCustomEmailRequest, SendEmailResponse,
    SendOrderNotificationRequest, SendPasswordResetEmailRequest, SendVerificationEmailRequest,
};
