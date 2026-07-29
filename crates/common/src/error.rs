use std::fmt;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("ID generation error: {0}")]
    IdGenerationError(String),
}

impl AppError {
    pub fn internal<T: fmt::Display>(msg: T) -> Self {
        AppError::Internal(msg.to_string())
    }

    pub fn invalid_input<T: fmt::Display>(msg: T) -> Self {
        AppError::InvalidInput(msg.to_string())
    }

    pub fn not_found<T: fmt::Display>(msg: T) -> Self {
        AppError::NotFound(msg.to_string())
    }

    pub fn unauthorized<T: fmt::Display>(msg: T) -> Self {
        AppError::Unauthorized(msg.to_string())
    }

    pub fn forbidden<T: fmt::Display>(msg: T) -> Self {
        AppError::Forbidden(msg.to_string())
    }

    pub fn conflict<T: fmt::Display>(msg: T) -> Self {
        AppError::Conflict(msg.to_string())
    }

    pub fn authentication<T: fmt::Display>(msg: T) -> Self {
        AppError::Authentication(msg.to_string())
    }

    pub fn grpc<T: fmt::Display>(msg: T) -> Self {
        AppError::Grpc(msg.to_string())
    }
}

impl From<tonic::Status> for AppError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::NotFound => AppError::not_found(status.message()),
            tonic::Code::AlreadyExists => AppError::conflict(status.message()),
            tonic::Code::InvalidArgument => AppError::invalid_input(status.message()),
            tonic::Code::Unauthenticated => AppError::unauthorized(status.message()),
            tonic::Code::PermissionDenied => AppError::forbidden(status.message()),
            _ => AppError::grpc(status.message()),
        }
    }
}

impl From<AppError> for tonic::Status {
    fn from(error: AppError) -> Self {
        match error {
            AppError::NotFound(msg) => tonic::Status::not_found(msg),
            AppError::Conflict(msg) => tonic::Status::already_exists(msg),
            AppError::InvalidInput(msg) => tonic::Status::invalid_argument(msg),
            AppError::Unauthorized(msg) => tonic::Status::unauthenticated(msg),
            AppError::Forbidden(msg) => tonic::Status::permission_denied(msg),
            AppError::Authentication(msg) => tonic::Status::unauthenticated(msg),
            _ => tonic::Status::internal(error.to_string()),
        }
    }
}

impl AppError {
    /// 将错误映射为 HTTP 状态码
    pub fn http_status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) | AppError::Authentication(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Database(_) | AppError::Grpc(_) | AppError::IdGenerationError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.http_status_code();
        let message = self.to_string();

        // 对于内部错误，不暴露具体细节
        let safe_message = match &self {
            AppError::Database(_) | AppError::Config(_) | AppError::IdGenerationError(_) => {
                "Internal server error".to_string()
            }
            AppError::Internal(_) if status == StatusCode::INTERNAL_SERVER_ERROR => {
                "Internal server error".to_string()
            }
            _ => message,
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": safe_message,
            }
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_status_code_mapping() {
        assert_eq!(
            AppError::not_found("test").http_status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::conflict("test").http_status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::invalid_input("test").http_status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::unauthorized("test").http_status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::authentication("test").http_status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::forbidden("test").http_status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::internal("test").http_status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::grpc("test").http_status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::Config("test".to_string()).http_status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::IdGenerationError("test".to_string()).http_status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_into_response() {
        // NotFound -> 404 with safe message
        let response = AppError::not_found("resource missing").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // InvalidInput -> 400
        let response = AppError::invalid_input("bad input").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Unauthorized -> 401
        let response = AppError::unauthorized("no token").into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Forbidden -> 403
        let response = AppError::forbidden("no permission").into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Conflict -> 409
        let response = AppError::conflict("dup").into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // Internal error -> 500, safe message hides details
        let response = AppError::internal("secret internal error").into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Config error -> 500, safe message hides details
        let response = AppError::Config("db url missing".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
