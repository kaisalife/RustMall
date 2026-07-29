//! JWT 认证中间件

use axum::{
    body::Body, http::Request, http::StatusCode, middleware::Next, response::IntoResponse,
    response::Response, Json,
};
use common::Claims;
use serde_json::json;
use std::sync::Arc;

/// JWT 认证错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing authorization header")]
    MissingToken,

    #[error("Invalid token format")]
    InvalidFormat,

    #[error("Token expired or invalid")]
    InvalidToken,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing authorization header" })),
            )
                .into_response(),
            AuthError::InvalidFormat => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid token format" })),
            )
                .into_response(),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Token expired or invalid" })),
            )
                .into_response(),
            AuthError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": msg })),
            )
                .into_response(),
        }
    }
}

/// JWT 验证器（需要密钥）
#[derive(Clone)]
pub struct JwtValidator {
    secret: String,
}

impl JwtValidator {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn validate(&self, token: &str) -> common::AppResult<Claims> {
        common::validate_jwt(token, &self.secret)
    }
}

/// 认证中间件工厂函数
pub fn create_auth_middleware(
    secret: String,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, AuthError>> + Send + 'static>,
> + Clone
       + Send
       + Sync
       + 'static {
    let validator = Arc::new(JwtValidator::new(secret));

    move |request: Request<Body>, next: Next| {
        let validator = validator.clone();
        Box::pin(async move {
            // 从 Authorization header 获取 token
            let auth_header = request
                .headers()
                .get("Authorization")
                .ok_or(AuthError::MissingToken)?;

            let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidFormat)?;

            // 验证 Bearer 格式
            let token = auth_str
                .strip_prefix("Bearer ")
                .ok_or(AuthError::InvalidFormat)?;

            // 验证并解析 JWT
            let claims = validator
                .validate(token)
                .map_err(|_| AuthError::InvalidToken)?;

            // 将用户信息存入 request extensions
            let (mut parts, body) = request.into_parts();
            parts.extensions.insert(Arc::new(claims));
            let request = Request::from_parts(parts, body);

            Ok(next.run(request).await)
        })
    }
}

/// 可选认证中间件工厂函数
pub fn create_optional_auth_middleware(
    secret: String,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, AuthError>> + Send + 'static>,
> + Clone
       + Send
       + Sync
       + 'static {
    let validator = Arc::new(JwtValidator::new(secret));

    move |mut request: Request<Body>, next: Next| {
        let validator = validator.clone();
        Box::pin(async move {
            // 尝试从 Authorization header 获取 token
            if let Some(auth_header) = request.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        if let Ok(claims) = validator.validate(token) {
                            request.extensions_mut().insert(Arc::new(claims));
                        }
                    }
                }
            }

            Ok(next.run(request).await)
        })
    }
}

/// 从 request 中获取用户 claims
pub fn get_user_claims<B>(request: &Request<B>) -> Option<Arc<Claims>> {
    request.extensions().get::<Arc<Claims>>().cloned()
}
