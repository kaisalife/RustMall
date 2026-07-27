#[derive(Debug, Clone)]
pub struct RegisterCommand {
    pub email: String,
    pub password: String,
    pub nickname: String,
}

#[derive(Debug, Clone)]
pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct UpdatePasswordCommand {
    pub user_id: u64,
    pub old_password: String,
    pub new_password: String,
}
