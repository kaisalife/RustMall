//! Email Service gRPC 客户端库

pub use proto::email::{
    email_service_client::EmailServiceClient,
    SendVerificationEmailRequest,
    SendOrderNotificationRequest,
    SendPasswordResetEmailRequest,
    SendCustomEmailRequest,
    SendEmailResponse,
};
