use tonic::{Request, Response, Status};

use crate::application::{command::*, AuthApplicationService};

use proto::auth::{
    auth_service_server::AuthService, GetUserRequest, LoginRequest, LoginResponse,
    RefreshTokenRequest, RegisterRequest, RegisterResponse, UpdatePasswordRequest,
    UpdatePasswordResponse, UserResponse,
};

#[derive(Clone)]
pub struct AuthServiceImpl {
    service: AuthApplicationService,
}

impl AuthServiceImpl {
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

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
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

    async fn update_password(
        &self,
        request: Request<UpdatePasswordRequest>,
    ) -> Result<Response<UpdatePasswordResponse>, Status> {
        let req = request.into_inner();

        let command = UpdatePasswordCommand {
            user_id: req.user_id,
            old_password: req.old_password,
            new_password: req.new_password,
        };

        let success = self
            .service
            .update_password(command)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(UpdatePasswordResponse { success }))
    }
}
