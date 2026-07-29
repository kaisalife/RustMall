use std::sync::Arc;

use common::{
    generate_jwt, generate_refresh_token, hash_password_async, validate_refresh_token,
    verify_password_async, AppError, AppResult, Claims, JwtConfig, RefreshClaims,
    SnowflakeIdGenerator,
};

use crate::domain::{User, UserRepository};
use crate::infrastructure::EmailServiceClientWrapper;

use super::command::{LoginCommand, RegisterCommand, UpdatePasswordCommand};
use super::dto::{LoginResponseDto, RegisterResponseDto, UserDto};

#[derive(Clone)]
pub struct AuthApplicationService {
    user_repository: Arc<dyn UserRepository>,
    id_generator: Arc<SnowflakeIdGenerator>,
    jwt_config: Arc<JwtConfig>,
    email_client: Option<EmailServiceClientWrapper>,
}

impl AuthApplicationService {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        id_generator: Arc<SnowflakeIdGenerator>,
        jwt_config: Arc<JwtConfig>,
        email_client: Option<EmailServiceClientWrapper>,
    ) -> Self {
        Self {
            user_repository,
            id_generator,
            jwt_config,
            email_client,
        }
    }

    #[tracing::instrument(skip(self, command), fields(email = %command.email))]
    pub async fn register(&self, command: RegisterCommand) -> AppResult<RegisterResponseDto> {
        // 验证密码强度
        common::validate_password(&command.password)
            .map_err(|e| AppError::invalid_input(e.message))?;

        // 验证邮箱是否已存在
        if self
            .user_repository
            .find_by_email(&command.email)
            .await?
            .is_some()
        {
            return Err(AppError::conflict("Email already exists"));
        }

        // 生成密码哈希（bcrypt 是 CPU 密集型，放到阻塞线程池执行）
        let password_hash =
            hash_password_async(command.password.clone(), self.jwt_config.bcrypt_cost).await?;

        // 生成用户ID
        let user_id = self.id_generator.generate().map_err(AppError::internal)?;

        // 创建用户实体
        let user = User::new(
            user_id,
            command.email.clone(),
            password_hash,
            command.nickname.clone(),
        );

        // 保存用户
        let saved_user = self.user_repository.create(user).await?;

        // 异步发送验证邮件（不阻塞注册流程）
        if let Some(mut email_client) = self.email_client.clone() {
            let to_email = saved_user.email.clone();
            let username = saved_user.nickname.clone();
            let verification_code = "123456".to_string(); // 实际应用中应该生成随机码
            tracing::info!("Verification email queued for {}", to_email);
            tokio::spawn(async move {
                if let Err(e) = email_client
                    .send_verification_email(to_email, username, verification_code)
                    .await
                {
                    tracing::error!("Failed to send verification email: {}", e);
                }
            });
        }

        Ok(RegisterResponseDto {
            user_id: saved_user.id,
            email: saved_user.email,
            nickname: saved_user.nickname,
            created_at: saved_user.created_at.to_rfc3339(),
        })
    }

    #[tracing::instrument(skip(self, command), fields(email = %command.email))]
    pub async fn login(&self, command: LoginCommand) -> AppResult<LoginResponseDto> {
        // 查找用户
        let user = self
            .user_repository
            .find_by_email(&command.email)
            .await?
            .ok_or_else(|| AppError::authentication("Invalid email or password"))?;

        // 验证密码（bcrypt 是 CPU 密集型，放到阻塞线程池执行）
        if !verify_password_async(command.password.clone(), user.password_hash.clone()).await? {
            return Err(AppError::authentication("Invalid email or password"));
        }

        // 生成 access token
        let access_claims = Claims::new(
            user.id,
            user.email.clone(),
            self.jwt_config.expiration_hours,
            "user".to_string(),
        );
        let token = generate_jwt(&access_claims, &self.jwt_config.secret)?;

        // 生成 refresh token
        let refresh_claims = RefreshClaims::new(
            user.id,
            user.email.clone(),
            self.jwt_config.refresh_expiration_hours,
        );
        let refresh_token = generate_refresh_token(refresh_claims, &self.jwt_config.secret)?;

        Ok(LoginResponseDto {
            user_id: user.id,
            email: user.email,
            token,
            refresh_token,
        })
    }

    pub async fn refresh_token(&self, refresh_token: String) -> AppResult<(u64, String, String)> {
        // 验证 refresh token
        let refresh_claims = validate_refresh_token(&refresh_token, &self.jwt_config.secret)?;

        if refresh_claims.token_type != "refresh" {
            return Err(AppError::authentication("Invalid token type"));
        }

        // 生成新的 access token
        let access_claims = Claims::new(
            refresh_claims.user_id,
            refresh_claims.sub.clone(),
            self.jwt_config.expiration_hours,
            "user".to_string(),
        );
        let access_token = generate_jwt(&access_claims, &self.jwt_config.secret)?;

        Ok((refresh_claims.user_id, refresh_claims.sub, access_token))
    }

    pub async fn get_user(&self, user_id: u64) -> AppResult<UserDto> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(UserDto {
            user_id: user.id,
            email: user.email,
            nickname: user.nickname,
            created_at: user.created_at.to_rfc3339(),
        })
    }

    pub async fn update_password(&self, command: UpdatePasswordCommand) -> AppResult<bool> {
        let mut user = self
            .user_repository
            .find_by_id(command.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // 验证旧密码（bcrypt 是 CPU 密集型，放到阻塞线程池执行）
        if !verify_password_async(command.old_password.clone(), user.password_hash.clone()).await? {
            return Err(AppError::authentication("Invalid old password"));
        }

        // 验证新密码强度
        common::validate_password(&command.new_password)
            .map_err(|e| AppError::invalid_input(e.message))?;

        // 生成新密码哈希（bcrypt 是 CPU 密集型，放到阻塞线程池执行）
        let new_password_hash =
            hash_password_async(command.new_password.clone(), self.jwt_config.bcrypt_cost).await?;

        // 更新密码
        user.update_password(new_password_hash);

        // 保存
        self.user_repository.update(user).await?;

        Ok(true)
    }

    pub async fn update_profile(&self, user_id: u64, nickname: Option<String>) -> AppResult<bool> {
        let mut user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if let Some(n) = nickname {
            user.update_nickname(n);
        }

        self.user_repository.update(user).await?;

        Ok(true)
    }

    pub async fn delete_user(&self, user_id: u64) -> AppResult<bool> {
        self.user_repository.delete(user_id).await?;
        Ok(true)
    }
}
