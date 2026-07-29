//! 邮件发送实现

use crate::domain::Email;
use common::AppResult;
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use tracing::{error, info};

/// 邮件发送器
#[derive(Clone)]
pub struct EmailSender {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from_address: String,
}

impl EmailSender {
    /// 创建新的邮件发送器
    pub fn new(
        smtp_host: &str,
        smtp_port: u16,
        smtp_username: String,
        smtp_password: String,
        from_address: String,
    ) -> AppResult<Self> {
        let creds = Credentials::new(smtp_username, smtp_password);

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
            .map_err(|e| common::AppError::Internal(format!("SMTP 配置错误：{}", e)))?
            .port(smtp_port)
            .credentials(creds)
            .build();

        Ok(Self {
            transport: Some(transport),
            from_address,
        })
    }

    /// 创建开发环境的邮件发送器（仅打印到控制台，不实际发送）
    pub fn new_dev(from_address: String) -> Self {
        info!("📧 使用开发模式邮件发送器（邮件将打印到控制台）");
        Self {
            transport: None,
            from_address,
        }
    }

    /// 发送邮件
    pub async fn send(&self, email: &Email) -> AppResult<String> {
        info!("📧 发送邮件到：{} ({})", email.to_email, email.subject);

        // 如果是开发模式，直接返回模拟结果
        let transport = match &self.transport {
            Some(t) => t,
            None => {
                info!("📧 [DEV MODE] 邮件内容预览：");
                info!("  收件人：{}", email.to_email);
                info!("  主题：{}", email.subject);
                info!("  内容长度：{} 字符", email.html_content.len());

                let message_id = format!("dev-msg-{}-{}", email.id, uuid::Uuid::new_v4());
                info!("  模拟 Message-ID：{}", message_id);

                return Ok(message_id);
            }
        };

        // 构建邮件
        let message = Message::builder()
            .from(
                self.from_address
                    .parse()
                    .map_err(|e| common::AppError::Internal(format!("发件人地址错误：{}", e)))?,
            )
            .to(email
                .to_email
                .parse()
                .map_err(|e| common::AppError::Internal(format!("收件人地址错误：{}", e)))?)
            .subject(&email.subject)
            .header(lettre::message::header::ContentType::TEXT_HTML)
            .body(email.html_content.clone())
            .map_err(|e| common::AppError::Internal(format!("邮件构建错误：{}", e)))?;

        // 发送邮件
        let result = transport.send(message).await;

        match result {
            Ok(response) => {
                // message() 返回的是一个迭代器，我们需要拼接所有内容
                let message_id: String = response.message().collect::<Vec<_>>().join("");
                info!("✅ 邮件发送成功，Message-ID：{}", message_id);
                Ok(message_id)
            }
            Err(e) => {
                error!("❌ 邮件发送失败：{}", e);
                Err(common::AppError::Internal(format!("邮件发送失败：{}", e)))
            }
        }
    }
}
