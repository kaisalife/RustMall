//! Auth Service v2 gRPC 接口实现
//!
//! 实现 `auth.v2::AuthService` trait：
//! - v1 兼容方法（Register/Login/RefreshToken/GetUser）委托给 AuthApplicationService
//! - v2 新增方法（GetUserProfile/UpdateUserAvatar/UpdateUserProfile）
//!   返回扩展的 UserProfileResponse

use tonic::{Request, Response, Status};

use crate::application::{command::*, AuthApplicationService};

use proto::auth::v2::{
    auth_service_server::AuthService, GetUserProfileRequest, GetUserRequest, LoginRequest,
    LoginResponse, RefreshTokenRequest, RegisterRequest, RegisterResponse, UpdateAvatarRequest,
    UpdateUserProfileRequest, UserProfileResponse, UserResponse,
};

#[derive(Clone)]
pub struct AuthServiceV2Impl {
    service: AuthApplicationService,
}

impl AuthServiceV2Impl {
    pub fn new(service: AuthApplicationService) -> Self {
        Self { service }
    }
}

/// Convert AppError to tonic Status
fn app_error_to_status(error: common::AppError) -> Status {
    match error {
        common::AppError::NotFound(msg) => Status::not_found(msg),
        common::AppError::Conflict(msg) => Status::already_exists(msg),
        common::AppError::InvalidInput(msg) => Status::invalid_argument(msg),
        common::AppError::Authentication(msg) => Status::unauthenticated(msg),
        common::AppError::Forbidden(msg) => Status::permission_denied(msg),
        _ => Status::internal(error.to_string()),
    }
}

/// 将 UserDto 转换为 UserProfileResponse
///
/// 当前 User 实体尚未包含 avatar_url/bio/mfa_enabled 字段，
/// 这些字段使用默认值，待数据模型扩展后填充真实数据。
fn user_dto_to_profile(dto: &crate::application::dto::UserDto) -> UserProfileResponse {
    UserProfileResponse {
        user_id: dto.user_id,
        email: dto.email.clone(),
        nickname: dto.nickname.clone(),
        avatar_url: String::new(),
        bio: String::new(),
        mfa_enabled: false,
        created_at: dto.created_at.clone(),
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceV2Impl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();

        let command = RegisterCommand {
            email: req.email,
            password: req.password,
            nickname: req.nickname,
        };

        let result = self
            .service
            .register(command)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(RegisterResponse {
            user_id: result.user_id,
            email: result.email,
            nickname: result.nickname,
            created_at: result.created_at,
        }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let command = LoginCommand {
            email: req.email,
            password: req.password,
        };

        let result = self
            .service
            .login(command)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(LoginResponse {
            user_id: result.user_id,
            email: result.email,
            token: result.token,
            refresh_token: result.refresh_token,
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let (user_id, email, access_token) = self
            .service
            .refresh_token(req.refresh_token.clone())
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(LoginResponse {
            user_id,
            email,
            token: access_token,
            refresh_token: req.refresh_token,
        }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .get_user(req.user_id)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(UserResponse {
            user_id: result.user_id,
            email: result.email,
            nickname: result.nickname,
            created_at: result.created_at,
        }))
    }

    async fn get_user_profile(
        &self,
        request: Request<GetUserProfileRequest>,
    ) -> Result<Response<UserProfileResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .get_user(req.user_id)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(user_dto_to_profile(&result)))
    }

    async fn update_user_avatar(
        &self,
        request: Request<UpdateAvatarRequest>,
    ) -> Result<Response<UserProfileResponse>, Status> {
        let req = request.into_inner();

        // 验证用户存在
        let result = self
            .service
            .get_user(req.user_id)
            .await
            .map_err(app_error_to_status)?;

        // TODO: 待 User 实体扩展 avatar_url 字段后持久化
        let mut profile = user_dto_to_profile(&result);
        profile.avatar_url = req.avatar_url;

        Ok(Response::new(profile))
    }

    async fn update_user_profile(
        &self,
        request: Request<UpdateUserProfileRequest>,
    ) -> Result<Response<UserProfileResponse>, Status> {
        let req = request.into_inner();

        // 如果提供了 nickname，则更新
        if let Some(ref nickname) = req.nickname {
            if !nickname.is_empty() {
                self.service
                    .update_profile(req.user_id, Some(nickname.clone()))
                    .await
                    .map_err(app_error_to_status)?;
            }
        }

        // 返回更新后的 profile
        let result = self
            .service
            .get_user(req.user_id)
            .await
            .map_err(app_error_to_status)?;

        let mut profile = user_dto_to_profile(&result);
        // TODO: 待 User 实体扩展 avatar_url/bio 字段后持久化
        if let Some(avatar_url) = req.avatar_url {
            profile.avatar_url = avatar_url;
        }
        if let Some(bio) = req.bio {
            profile.bio = bio;
        }

        Ok(Response::new(profile))
    }
}
