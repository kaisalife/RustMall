//! common crate 集成测试
//!
//! 测试 AppError 到 HTTP 响应的转换、密码验证流程和 JWT 完整生命周期。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::{
    generate_jwt, generate_refresh_token, hash_password, validate_jwt, validate_password,
    validate_refresh_token, verify_password, AppError, Claims, RefreshClaims,
};

#[tokio::test]
async fn test_app_error_to_http_response() {
    use http_body_util::BodyExt;

    // NotFound -> 404，响应体包含正确的 error 结构
    let response = AppError::not_found("resource missing").into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], 404);

    // InvalidInput -> 400
    let response = AppError::invalid_input("bad input").into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Conflict -> 409
    let response = AppError::conflict("duplicate").into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Forbidden -> 403
    let response = AppError::forbidden("no permission").into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Internal -> 500（不暴露内部细节）
    let response = AppError::internal("secret internal error").into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["message"], "Internal server error");
}

#[test]
fn test_app_error_not_found_returns_404() {
    let response = AppError::not_found("resource not found").into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_app_error_conflict_returns_409() {
    let response = AppError::conflict("resource already exists").into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_app_error_unauthorized_returns_401() {
    let response = AppError::unauthorized("missing or invalid token").into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_password_validation_integration() {
    // 合法密码
    assert!(validate_password("ValidPass1").is_ok());

    // 太短
    assert!(validate_password("Short1").is_err());
    // 无大写字母
    assert!(validate_password("validpass1").is_err());
    // 无小写字母
    assert!(validate_password("VALIDPASS1").is_err());
    // 无数字
    assert!(validate_password("ValidPass").is_err());

    // 哈希和验证
    let hashed = hash_password("ValidPass1").unwrap();
    assert!(verify_password("ValidPass1", &hashed).unwrap());
    assert!(!verify_password("WrongPass1", &hashed).unwrap());
}

#[test]
fn test_jwt_full_lifecycle() {
    let secret = "test_secret_key";

    // 1. 生成 access token
    let claims = Claims::new(12345, "test@example.com".to_string(), 1, "user".to_string());
    let access_token = generate_jwt(&claims, secret).unwrap();

    // 2. 验证 access token
    let validated_claims = validate_jwt(&access_token, secret).unwrap();
    assert_eq!(validated_claims.user_id, 12345);
    assert_eq!(validated_claims.sub, "test@example.com");
    assert_eq!(validated_claims.token_type, "access");

    // 3. 生成 refresh token
    let refresh_claims = RefreshClaims::new(12345, "test@example.com".to_string(), 24);
    let refresh_token = generate_refresh_token(refresh_claims, secret).unwrap();

    // 4. 验证 refresh token
    let validated_refresh = validate_refresh_token(&refresh_token, secret).unwrap();
    assert_eq!(validated_refresh.user_id, 12345);
    assert_eq!(validated_refresh.sub, "test@example.com");
    assert_eq!(validated_refresh.token_type, "refresh");

    // 5. 使用错误密钥验证应失败
    assert!(validate_jwt(&access_token, "wrong_secret").is_err());
}
