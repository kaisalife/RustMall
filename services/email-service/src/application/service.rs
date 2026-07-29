//! 邮件应用服务

use crate::domain::{Email, EmailRepository};
use crate::infrastructure::EmailSender;
use common::{AppResult, SnowflakeIdGenerator};
use std::sync::Arc;

use super::command::{
    SendCustomEmailCommand, SendOrderNotificationCommand, SendPasswordResetEmailCommand,
    SendVerificationEmailCommand,
};

/// 邮件应用服务
#[derive(Clone)]
pub struct EmailApplicationService {
    id_generator: Arc<SnowflakeIdGenerator>,
    email_repository: Arc<dyn EmailRepository>,
    email_sender: Arc<EmailSender>,
}

impl EmailApplicationService {
    /// 创建新的邮件应用服务
    pub fn new(
        id_generator: Arc<SnowflakeIdGenerator>,
        email_repository: Arc<dyn EmailRepository>,
        email_sender: Arc<EmailSender>,
    ) -> Self {
        Self {
            id_generator,
            email_repository,
            email_sender,
        }
    }

    /// 发送验证邮件
    pub async fn send_verification_email(
        &self,
        command: SendVerificationEmailCommand,
    ) -> AppResult<String> {
        // 生成邮件 ID
        let email_id = self.id_generator.generate()?;

        // 创建邮件实体
        let email = Email::new_verification(
            email_id,
            command.to_email,
            command.username,
            command.verification_code,
        );

        // 保存邮件记录
        let mut email = self.email_repository.save(email).await?;

        // 发送邮件
        match self.email_sender.send(&email).await {
            Ok(message_id) => {
                email.mark_sent(message_id.clone());
                self.email_repository.update_status(email).await?;
                Ok(message_id)
            }
            Err(e) => {
                email.mark_failed(e.to_string());
                let _ = self.email_repository.update_status(email).await;
                Err(e)
            }
        }
    }

    /// 发送订单通知邮件
    pub async fn send_order_notification(
        &self,
        command: SendOrderNotificationCommand,
    ) -> AppResult<String> {
        let email_id = self.id_generator.generate()?;
        let email = Email::new_order_notification(
            email_id,
            command.to_email,
            command.username,
            command.order_id,
            command.total_amount,
            command.status,
        );
        let mut email = self.email_repository.save(email).await?;

        match self.email_sender.send(&email).await {
            Ok(message_id) => {
                email.mark_sent(message_id.clone());
                self.email_repository.update_status(email).await?;
                Ok(message_id)
            }
            Err(e) => {
                email.mark_failed(e.to_string());
                let _ = self.email_repository.update_status(email).await;
                Err(e)
            }
        }
    }

    /// 发送密码重置邮件
    pub async fn send_password_reset_email(
        &self,
        command: SendPasswordResetEmailCommand,
    ) -> AppResult<String> {
        let email_id = self.id_generator.generate()?;
        let email = Email::new_password_reset(
            email_id,
            command.to_email,
            command.username,
            command.reset_token,
        );
        let mut email = self.email_repository.save(email).await?;

        match self.email_sender.send(&email).await {
            Ok(message_id) => {
                email.mark_sent(message_id.clone());
                self.email_repository.update_status(email).await?;
                Ok(message_id)
            }
            Err(e) => {
                email.mark_failed(e.to_string());
                let _ = self.email_repository.update_status(email).await;
                Err(e)
            }
        }
    }

    /// 发送自定义邮件
    pub async fn send_custom_email(&self, command: SendCustomEmailCommand) -> AppResult<String> {
        let email_id = self.id_generator.generate()?;
        let email = Email::new_custom(
            email_id,
            command.to_email,
            command.username,
            command.subject,
            command.html_content,
        );
        let mut email = self.email_repository.save(email).await?;

        match self.email_sender.send(&email).await {
            Ok(message_id) => {
                email.mark_sent(message_id.clone());
                self.email_repository.update_status(email).await?;
                Ok(message_id)
            }
            Err(e) => {
                email.mark_failed(e.to_string());
                let _ = self.email_repository.update_status(email).await;
                Err(e)
            }
        }
    }

    /// 查询邮件状态
    pub async fn get_email_status(&self, id: u64) -> AppResult<Option<Email>> {
        self.email_repository.find_by_id(id).await
    }
}
