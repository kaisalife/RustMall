//! Auth v2 HTTP 路由
//!
//! 提供 v2 用户 profile 管理接口：
//! - GET    /:id/profile  -> GetUserProfile
//! - PUT    /:id/avatar   -> UpdateUserAvatar
//! - PUT    /:id/profile  -> UpdateUserProfile

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use common::AppError;

use crate::response::ApiResponse;
use crate::state::AppState;

/// v2 用户 profile 响应 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfileDto {
    pub user_id: u64,
    pub email: String,
    pub nickname: String,
    pub avatar_url: String,
    pub bio: String,
    pub mfa_enabled: bool,
    pub created_at: String,
}

/// 更新头像请求 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAvatarRequest {
    pub avatar_url: String,
}

/// 更新 profile 请求 DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserProfileRequest {
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

pub fn auth_v2_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/profile", get(get_user_profile_handler))
        .route("/:id/avatar", put(update_user_avatar_handler))
        .route("/:id/profile", put(update_user_profile_handler))
}

async fn get_user_profile_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<UserProfileDto>>, AppError> {
    let request = proto::auth::v2::GetUserProfileRequest { user_id: id };
    let response = state
        .clients
        .call_auth_v2(|mut client| async move { client.get_user_profile(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UserProfileDto {
        user_id: inner.user_id,
        email: inner.email,
        nickname: inner.nickname,
        avatar_url: inner.avatar_url,
        bio: inner.bio,
        mfa_enabled: inner.mfa_enabled,
        created_at: inner.created_at,
    })))
}

async fn update_user_avatar_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateAvatarRequest>,
) -> Result<Json<ApiResponse<UserProfileDto>>, AppError> {
    let request = proto::auth::v2::UpdateAvatarRequest {
        user_id: id,
        avatar_url: req.avatar_url,
    };
    let response = state
        .clients
        .call_auth_v2(|mut client| async move { client.update_user_avatar(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UserProfileDto {
        user_id: inner.user_id,
        email: inner.email,
        nickname: inner.nickname,
        avatar_url: inner.avatar_url,
        bio: inner.bio,
        mfa_enabled: inner.mfa_enabled,
        created_at: inner.created_at,
    })))
}

async fn update_user_profile_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateUserProfileRequest>,
) -> Result<Json<ApiResponse<UserProfileDto>>, AppError> {
    let request = proto::auth::v2::UpdateUserProfileRequest {
        user_id: id,
        nickname: req.nickname,
        avatar_url: req.avatar_url,
        bio: req.bio,
    };
    let response = state
        .clients
        .call_auth_v2(|mut client| async move { client.update_user_profile(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(UserProfileDto {
        user_id: inner.user_id,
        email: inner.email,
        nickname: inner.nickname,
        avatar_url: inner.avatar_url,
        bio: inner.bio,
        mfa_enabled: inner.mfa_enabled,
        created_at: inner.created_at,
    })))
}
