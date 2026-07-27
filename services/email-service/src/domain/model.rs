//! 邮件领域模型

use chrono::{DateTime, Utc};

/// 邮件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailType {
    /// 验证码邮件
    Verification,
    /// 订单通知邮件
    OrderNotification,
    /// 密码重置邮件
    PasswordReset,
    /// 自定义邮件
    Custom,
}

/// 邮件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailStatus {
    /// 待发送
    Pending,
    /// 发送成功
    Sent,
    /// 发送失败
    Failed,
}

/// 邮件实体
#[derive(Debug, Clone)]
pub struct Email {
    /// 邮件 ID
    pub id: u64,
    /// 收件人邮箱
    pub to_email: String,
    /// 收件人用户名
    pub username: Option<String>,
    /// 邮件主题
    pub subject: String,
    /// 邮件 HTML 内容
    pub html_content: String,
    /// 邮件类型
    pub email_type: EmailType,
    /// 邮件状态
    pub status: EmailStatus,
    /// 发送结果消息 ID
    pub message_id: Option<String>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl Email {
    /// 创建新的验证邮件
    pub fn new_verification(
        id: u64,
        to_email: String,
        username: String,
        verification_code: String,
    ) -> Self {
        let subject = "请验证您的邮箱".to_string();
        let html_content = format!(
            r#"
            <html>
                <body>
                    <h2>您好，{}！</h2>
                    <p>感谢您注册我们的服务。请使用以下验证码完成邮箱验证：</p>
                    <h3 style="color: #2563eb; font-size: 24px; font-weight: bold;">{}</h3>
                    <p>此验证码将在 15 分钟后过期。</p>
                    <p>如果这不是您的操作，请忽略此邮件。</p>
                    <hr>
                    <p>此致，<br>电商团队</p>
                </body>
            </html>
            "#,
            username, verification_code
        );

        Self {
            id,
            to_email,
            username: Some(username),
            subject,
            html_content,
            email_type: EmailType::Verification,
            status: EmailStatus::Pending,
            message_id: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 创建新的订单通知邮件
    pub fn new_order_notification(
        id: u64,
        to_email: String,
        username: String,
        order_id: u64,
        total_amount: f64,
        status: String,
    ) -> Self {
        let subject = format!("订单 #{} 状态更新", order_id);
        let html_content = format!(
            r#"
            <html>
                <body>
                    <h2>您好，{}！</h2>
                    <p>您的订单 #{} 状态已更新为：<strong>{}</strong></p>
                    <p>订单金额：<strong>¥{:.2}</strong></p>
                    <p>感谢您的购买！</p>
                    <hr>
                    <p>此致，<br>电商团队</p>
                </body>
            </html>
            "#,
            username, order_id, status, total_amount
        );

        Self {
            id,
            to_email,
            username: Some(username),
            subject,
            html_content,
            email_type: EmailType::OrderNotification,
            status: EmailStatus::Pending,
            message_id: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 创建新的密码重置邮件
    pub fn new_password_reset(
        id: u64,
        to_email: String,
        username: String,
        reset_token: String,
    ) -> Self {
        let subject = "请重置您的密码".to_string();
        let html_content = format!(
            r#"
            <html>
                <body>
                    <h2>您好，{}！</h2>
                    <p>我们收到了您的密码重置请求。请使用以下令牌完成密码重置：</p>
                    <p style="background: #f3f4f6; padding: 12px; border-radius: 6px; font-family: monospace;">{}</p>
                    <p>此令牌将在 1 小时后过期。</p>
                    <p>如果这不是您的操作，请忽略此邮件。</p>
                    <hr>
                    <p>此致，<br>电商团队</p>
                </body>
            </html>
            "#,
            username, reset_token
        );

        Self {
            id,
            to_email,
            username: Some(username),
            subject,
            html_content,
            email_type: EmailType::PasswordReset,
            status: EmailStatus::Pending,
            message_id: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 创建自定义邮件
    pub fn new_custom(
        id: u64,
        to_email: String,
        username: Option<String>,
        subject: String,
        html_content: String,
    ) -> Self {
        Self {
            id,
            to_email,
            username,
            subject,
            html_content,
            email_type: EmailType::Custom,
            status: EmailStatus::Pending,
            message_id: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 标记为已发送
    pub fn mark_sent(&mut self, message_id: String) {
        self.status = EmailStatus::Sent;
        self.message_id = Some(message_id);
        self.updated_at = Utc::now();
    }

    /// 标记为发送失败
    pub fn mark_failed(&mut self, error_message: String) {
        self.status = EmailStatus::Failed;
        self.error_message = Some(error_message);
        self.updated_at = Utc::now();
    }
}
