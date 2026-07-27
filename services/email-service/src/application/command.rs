#[derive(Debug, Clone)]
pub struct SendVerificationEmailCommand {
    pub to_email: String,
    pub username: String,
    pub verification_code: String,
}

#[derive(Debug, Clone)]
pub struct SendOrderNotificationCommand {
    pub to_email: String,
    pub username: String,
    pub order_id: u64,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct SendPasswordResetEmailCommand {
    pub to_email: String,
    pub username: String,
    pub reset_token: String,
}

#[derive(Debug, Clone)]
pub struct SendCustomEmailCommand {
    pub to_email: String,
    pub username: Option<String>,
    pub subject: String,
    pub html_content: String,
}
