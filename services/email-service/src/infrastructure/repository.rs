//! 邮件仓库 PostgreSQL 实现

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use common::AppResult;
use crate::domain::{Email, EmailRepository, EmailStatus, EmailType};

/// PostgreSQL 邮件仓库
#[derive(Clone)]
pub struct EmailRepositoryImpl {
    pool: PgPool,
}

impl EmailRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl EmailRepository for EmailRepositoryImpl {
    async fn save(&self, email: Email) -> AppResult<Email> {
        let record = sqlx::query_as::<_, EmailRecord>(
            r#"
            INSERT INTO email_logs (id, to_email, username, subject, html_content, email_type, status, message_id, error_message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, to_email, username, subject, html_content, email_type, status, message_id, error_message, created_at, updated_at
            "#,
        )
        .bind(email.id as i64)
        .bind(email.to_email)
        .bind(email.username)
        .bind(email.subject)
        .bind(email.html_content)
        .bind(email_type_to_string(email.email_type))
        .bind(email_status_to_string(email.status))
        .bind(email.message_id)
        .bind(email.error_message)
        .bind(email.created_at)
        .bind(email.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Email>> {
        let record = sqlx::query_as::<_, EmailRecord>(
            r#"
            SELECT id, to_email, username, subject, html_content, email_type, status, message_id, error_message, created_at, updated_at
            FROM email_logs
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn update_status(&self, email: Email) -> AppResult<Email> {
        let record = sqlx::query_as::<_, EmailRecord>(
            r#"
            UPDATE email_logs
            SET status = $2, message_id = $3, error_message = $4
            WHERE id = $1
            RETURNING id, to_email, username, subject, html_content, email_type, status, message_id, error_message, created_at, updated_at
            "#,
        )
        .bind(email.id as i64)
        .bind(email_status_to_string(email.status))
        .bind(email.message_id)
        .bind(email.error_message)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }
}

#[derive(sqlx::FromRow)]
struct EmailRecord {
    id: i64,
    to_email: String,
    username: Option<String>,
    subject: String,
    html_content: String,
    email_type: String,
    status: String,
    message_id: Option<String>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EmailRecord {
    fn into_domain(self) -> Email {
        Email {
            id: self.id as u64,
            to_email: self.to_email,
            username: self.username,
            subject: self.subject,
            html_content: self.html_content,
            email_type: email_type_from_string(&self.email_type),
            status: email_status_from_string(&self.status),
            message_id: self.message_id,
            error_message: self.error_message,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn email_type_to_string(t: EmailType) -> &'static str {
    match t {
        EmailType::Verification => "Verification",
        EmailType::OrderNotification => "OrderNotification",
        EmailType::PasswordReset => "PasswordReset",
        EmailType::Custom => "Custom",
    }
}

fn email_type_from_string(s: &str) -> EmailType {
    match s {
        "Verification" => EmailType::Verification,
        "OrderNotification" => EmailType::OrderNotification,
        "PasswordReset" => EmailType::PasswordReset,
        _ => EmailType::Custom,
    }
}

fn email_status_to_string(s: EmailStatus) -> &'static str {
    match s {
        EmailStatus::Pending => "Pending",
        EmailStatus::Sent => "Sent",
        EmailStatus::Failed => "Failed",
    }
}

fn email_status_from_string(s: &str) -> EmailStatus {
    match s {
        "Pending" => EmailStatus::Pending,
        "Sent" => EmailStatus::Sent,
        "Failed" => EmailStatus::Failed,
        _ => EmailStatus::Pending,
    }
}
