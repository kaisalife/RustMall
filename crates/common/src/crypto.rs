use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::Semaphore;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: u64,
    pub exp: usize,
    pub iat: usize,
    /// Token 类型（区分 access token 和 refresh token）
    pub token_type: String,
}

impl Claims {
    pub fn new(user_id: u64, email: String, expiration_hours: i64) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours);

        Self {
            sub: email,
            user_id,
            iat: now.timestamp() as usize,
            exp: exp.timestamp() as usize,
            token_type: "access".to_string(),
        }
    }
}

/// Refresh Token Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    /// 用户 ID
    pub user_id: u64,
    /// 邮箱
    pub sub: String,
    /// 过期时间
    pub exp: usize,
    /// 签发时间
    pub iat: usize,
    /// Token 类型（区分 access token 和 refresh token）
    pub token_type: String,
}

impl RefreshClaims {
    pub fn new(user_id: u64, email: String, expiration_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            sub: email,
            exp: (now + Duration::hours(expiration_hours)).timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "refresh".to_string(),
        }
    }
}

pub fn hash_password(password: &str) -> AppResult<String> {
    hash_password_with_cost(password, DEFAULT_COST)
}

/// 使用指定 cost 因子哈希密码（cost 来自配置，默认 12，压测可降至 10）
pub fn hash_password_with_cost(password: &str, cost: u32) -> AppResult<String> {
    hash(password, cost).map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))
}

pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    verify(password, hash).map_err(|e| AppError::internal(format!("Failed to verify password: {}", e)))
}

/// bcrypt 并发限流信号量：限制为 CPU 核心数，避免阻塞线程池过度订阅导致 CPU 抢占
/// （bcrypt 是纯 CPU 密集型，并发数超过核心数只会增加上下文切换开销，不会提升吞吐）
static BCRYPT_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

fn bcrypt_semaphore() -> &'static Semaphore {
    BCRYPT_SEMAPHORE.get_or_init(|| {
        let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        Semaphore::new(cpus)
    })
}

/// 在阻塞线程池中哈希密码（bcrypt 是 CPU 密集型，放到阻塞线程池执行，并用信号量限制并发为核心数）
#[tracing::instrument(skip(password), fields(cost = cost))]
pub async fn hash_password_async(password: String, cost: u32) -> AppResult<String> {
    let _permit = bcrypt_semaphore().acquire().await
        .map_err(|e| AppError::internal(format!("Semaphore closed: {}", e)))?;
    tokio::task::spawn_blocking(move || hash_password_with_cost(&password, cost))
        .await
        .map_err(|e| AppError::internal(format!("Blocking task failed: {}", e)))?
}

/// 在阻塞线程池中验证密码（同上，避免阻塞 tokio worker 线程）
#[tracing::instrument(skip(password, hash))]
pub async fn verify_password_async(password: String, hash: String) -> AppResult<bool> {
    let _permit = bcrypt_semaphore().acquire().await
        .map_err(|e| AppError::internal(format!("Semaphore closed: {}", e)))?;
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::internal(format!("Blocking task failed: {}", e)))?
}

/// 密码验证错误
#[derive(Debug)]
pub struct PasswordValidationError {
    pub message: String,
}

impl std::fmt::Display for PasswordValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// 验证密码强度
/// - 最少 8 个字符
/// - 包含至少一个大写字母
/// - 包含至少一个小写字母
/// - 包含至少一个数字
pub fn validate_password(password: &str) -> Result<(), PasswordValidationError> {
    if password.len() < 8 {
        return Err(PasswordValidationError {
            message: "Password must be at least 8 characters long".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        return Err(PasswordValidationError {
            message: "Password must contain at least one uppercase letter".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_lowercase()) {
        return Err(PasswordValidationError {
            message: "Password must contain at least one lowercase letter".to_string(),
        });
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(PasswordValidationError {
            message: "Password must contain at least one digit".to_string(),
        });
    }

    Ok(())
}

pub fn generate_jwt(claims: &Claims, secret: &str) -> AppResult<String> {
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), claims, &encoding_key)
        .map_err(|e| AppError::authentication(format!("Failed to generate JWT: {}", e)))
}

pub fn validate_jwt(token: &str, secret: &str) -> AppResult<Claims> {
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::authentication(format!("Invalid JWT: {}", e)))
}

/// 生成 refresh token
pub fn generate_refresh_token(claims: RefreshClaims, secret: &str) -> AppResult<String> {
    let header = Header::default();
    encode(&header, &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::internal(format!("Failed to generate refresh token: {}", e)))
}

/// 验证 refresh token
pub fn validate_refresh_token(token: &str, secret: &str) -> AppResult<RefreshClaims> {
    decode::<RefreshClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|data| data.claims)
        .map_err(|e| AppError::Authentication(format!("Invalid refresh token: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "test_password_123";
        let hashed = hash_password(password).unwrap();

        assert!(verify_password(password, &hashed).unwrap());
        assert!(!verify_password("wrong_password", &hashed).unwrap());
    }

    #[test]
    fn test_jwt_generate_and_validate() {
        let secret = "test_secret_key";
        let claims = Claims::new(12345, "test@example.com".to_string(), 24);

        let token = generate_jwt(&claims, secret).unwrap();
        let validated_claims = validate_jwt(&token, secret).unwrap();

        assert_eq!(validated_claims.user_id, 12345);
        assert_eq!(validated_claims.sub, "test@example.com");
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("Test1234").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        let result = validate_password("Test1");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("at least 8 characters"));
    }

    #[test]
    fn test_validate_password_no_uppercase() {
        let result = validate_password("test1234");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("uppercase letter"));
    }

    #[test]
    fn test_validate_password_no_lowercase() {
        let result = validate_password("TEST1234");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("lowercase letter"));
    }

    #[test]
    fn test_validate_password_no_digit() {
        let result = validate_password("TestTest");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("digit"));
    }

    #[test]
    fn test_refresh_token_generate_and_validate() {
        let secret = "test_secret_key";
        let claims = RefreshClaims::new(12345, "test@example.com".to_string(), 24);

        let token = generate_refresh_token(claims, secret).unwrap();
        let validated_claims = validate_refresh_token(&token, secret).unwrap();

        assert_eq!(validated_claims.user_id, 12345);
        assert_eq!(validated_claims.sub, "test@example.com");
        assert_eq!(validated_claims.token_type, "refresh");
    }
}
