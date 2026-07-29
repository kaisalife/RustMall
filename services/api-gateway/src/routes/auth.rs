use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use common::AppError;

use crate::dto::auth::{
    LoginRequest, LoginResponseDto, RefreshTokenRequest, RegisterRequest, UpdatePasswordRequest,
    UpdatePasswordResponseDto, UserDto,
};
use crate::response::ApiResponse;
use crate::state::AppState;

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_token_handler))
}

pub fn user_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id", get(get_user_handler))
        .route("/password", put(update_password_handler))
}

async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<UserDto>>, AppError> {
    let request = proto::auth::v1::RegisterRequest {
        email: req.email,
        password: req.password,
        nickname: req.nickname,
    };
    let response = state
        .clients
        .call_auth(|mut client| async move { client.register(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UserDto {
        user_id: inner.user_id,
        email: inner.email,
        nickname: inner.nickname,
        created_at: inner.created_at,
    })))
}

async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseDto>>, AppError> {
    let cache_key = login_cache_key(&req.email, &req.password);

    // 先查缓存（TTL 30s，避免重复 bcrypt 验证）
    if let Some(ref cache) = state.cache {
        if let Ok(Some(cached)) = cache.get_json::<LoginResponseDto>(&cache_key).await {
            tracing::debug!("Login cache hit");
            return Ok(Json(ApiResponse::success(cached)));
        }
    }

    let request = proto::auth::v1::LoginRequest {
        email: req.email,
        password: req.password,
    };
    let response = state
        .clients
        .call_auth(|mut client| async move { client.login(request).await })
        .await?;
    let inner = response.into_inner();

    let dto = LoginResponseDto {
        user_id: inner.user_id,
        email: inner.email,
        token: inner.token,
        refresh_token: inner.refresh_token,
    };

    // 写入缓存（TTL 30s，密码变更后 key 自然不同，无需显式失效）
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache
            .set_json(&cache_key, &dto, Duration::from_secs(30))
            .await
        {
            tracing::warn!("Failed to write login to cache: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(dto)))
}

async fn refresh_token_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseDto>>, AppError> {
    let request = proto::auth::v1::RefreshTokenRequest {
        refresh_token: req.refresh_token,
    };
    let response = state
        .clients
        .call_auth(|mut client| async move { client.refresh_token(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(LoginResponseDto {
        user_id: inner.user_id,
        email: inner.email,
        token: inner.token,
        refresh_token: inner.refresh_token,
    })))
}

async fn get_user_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<UserDto>>, AppError> {
    let request = proto::auth::v1::GetUserRequest { user_id: id };
    let response = state
        .clients
        .call_auth(|mut client| async move { client.get_user(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UserDto {
        user_id: inner.user_id,
        email: inner.email,
        nickname: inner.nickname,
        created_at: inner.created_at,
    })))
}

async fn update_password_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdatePasswordRequest>,
) -> Result<Json<ApiResponse<UpdatePasswordResponseDto>>, AppError> {
    let request = proto::auth::v1::UpdatePasswordRequest {
        user_id: req.user_id,
        old_password: req.old_password,
        new_password: req.new_password,
    };
    let response = state
        .clients
        .call_auth(|mut client| async move { client.update_password(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UpdatePasswordResponseDto {
        success: inner.success,
    })))
}

/// 生成登录缓存 key：hash(email:password)，不存储明文密码
fn login_cache_key(email: &str, password: &str) -> String {
    let mut hasher = DefaultHasher::new();
    email.hash(&mut hasher);
    password.hash(&mut hasher);
    format!("login:{:x}", hasher.finish())
}
