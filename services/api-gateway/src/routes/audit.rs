//! 审计日志查询路由（需 admin 角色）

use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use common::{AppError, Claims};

use crate::dto::audit::{AuditLogListDto, AuditLogQueryDto, AuditLogResponseDto};
use crate::repository::audit::{self, AuditLogRecord};
use crate::response::ApiResponse;
use crate::state::AppState;

pub fn audit_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/logs", get(list_audit_logs_handler))
        .route("/export", get(export_audit_logs_handler))
}

/// 校验当前用户是否为 admin，否则返回 403
fn require_admin(claims: &Claims) -> Result<(), AppError> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("Admin access required"));
    }
    Ok(())
}

/// 从 AppState 获取数据库连接池
fn get_pool(state: &AppState) -> Result<&sqlx::PgPool, AppError> {
    state
        .db_pool
        .as_ref()
        .ok_or_else(|| AppError::internal("Database pool not available"))
}

/// GET /logs — 分页查询审计日志（JSON）
async fn list_audit_logs_handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Arc<Claims>>,
    Query(query): Query<AuditLogQueryDto>,
) -> Result<Json<ApiResponse<AuditLogListDto>>, AppError> {
    require_admin(&claims)?;

    let pool = get_pool(&state)?;
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);

    let (records, total) = audit::query_audit_logs(pool, &query).await?;

    let items: Vec<AuditLogResponseDto> = records
        .iter()
        .map(AuditLogRecord::to_response_dto)
        .collect();

    Ok(Json(ApiResponse::success(AuditLogListDto {
        items,
        total,
        page,
        page_size,
    })))
}

/// GET /export — CSV 导出审计日志（admin）
async fn export_audit_logs_handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Arc<Claims>>,
    Query(query): Query<AuditLogQueryDto>,
) -> Result<Response, AppError> {
    require_admin(&claims)?;

    let pool = get_pool(&state)?;
    let records = audit::query_audit_logs_for_export(pool, &query).await?;
    let csv = build_csv(&records);

    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/csv; charset=utf-8"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"audit_logs.csv\""),
            ),
        ],
        csv,
    )
        .into_response())
}

/// 手动拼接 CSV 字符串（不依赖外部 CSV 库）
fn build_csv(records: &[AuditLogRecord]) -> String {
    let header = "id,timestamp,user_id,action,resource_type,resource_id,\
service_name,request_id,ip_address,user_agent,details,status,error_message,created_at\n";
    let mut csv = String::from(header);
    for r in records {
        let row = vec![
            r.id.to_string(),
            r.timestamp.to_rfc3339(),
            r.user_id.map_or_else(String::new, |id| id.to_string()),
            csv_escape(&r.action),
            opt_csv(r.resource_type.as_deref()),
            opt_csv(r.resource_id.as_deref()),
            csv_escape(&r.service_name),
            opt_csv(r.request_id.as_deref()),
            opt_csv(r.ip_address.as_deref()),
            opt_csv(r.user_agent.as_deref()),
            csv_escape(&r.details.to_string()),
            csv_escape(&r.status),
            opt_csv(r.error_message.as_deref()),
            r.created_at.to_rfc3339(),
        ];
        csv.push_str(&row.join(","));
        csv.push('\n');
    }
    csv
}

/// CSV 字段转义：包含逗号/引号/换行时用双引号包裹，内部引号翻倍
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Option<&str> -> CSV 字段（None 输出空字符串）
fn opt_csv(val: Option<&str>) -> String {
    match val {
        Some(s) => csv_escape(s),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn test_csv_escape_quote() {
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn test_csv_escape_newline() {
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn test_opt_csv_none() {
        assert_eq!(opt_csv(None), "");
    }

    #[test]
    fn test_opt_csv_some() {
        assert_eq!(opt_csv(Some("ok")), "ok");
    }

    #[test]
    fn test_build_csv_empty() {
        let csv = build_csv(&[]);
        assert!(csv.starts_with("id,timestamp"));
        assert_eq!(csv.matches('\n').count(), 1);
    }
}
