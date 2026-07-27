use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub nickname: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePasswordRequest {
    pub user_id: u64,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub user_id: u64,
    pub email: String,
    pub nickname: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponseDto {
    pub user_id: u64,
    pub email: String,
    pub token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct UpdatePasswordResponseDto {
    pub success: bool,
}
