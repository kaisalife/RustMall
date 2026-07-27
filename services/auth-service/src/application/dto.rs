#[derive(Debug, Clone)]
pub struct UserDto {
    pub user_id: u64,
    pub email: String,
    pub nickname: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct LoginResponseDto {
    pub user_id: u64,
    pub email: String,
    pub token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct RegisterResponseDto {
    pub user_id: u64,
    pub email: String,
    pub nickname: String,
    pub created_at: String,
}
