//! Email service client

use common::AppResult;
use tonic::transport::Channel;

use proto::email::v1::email_service_client::EmailServiceClient;
use proto::email::v1::SendVerificationEmailRequest;

#[derive(Clone)]
pub struct EmailServiceClientWrapper {
    client: EmailServiceClient<Channel>,
}

impl EmailServiceClientWrapper {
    /// Create a new email service client
    pub async fn new(addr: String) -> AppResult<Self> {
        let client = EmailServiceClient::connect(addr).await.map_err(|e| {
            common::AppError::Internal(format!("Failed to connect to email service: {}", e))
        })?;
        Ok(Self { client })
    }

    /// Send verification email
    pub async fn send_verification_email(
        &mut self,
        to_email: String,
        username: String,
        verification_code: String,
    ) -> AppResult<()> {
        let request = SendVerificationEmailRequest {
            to_email,
            username,
            verification_code,
        };

        let _response = self
            .client
            .send_verification_email(request)
            .await
            .map_err(|e| {
                common::AppError::Internal(format!("Failed to send verification email: {}", e))
            })?;

        Ok(())
    }
}
