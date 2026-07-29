//! 审计日志查询仓储：动态构建 WHERE 条件并分页查询 audit_logs 表

use chrono::{DateTime, Utc};
use sqlx::{PgPool, QueryBuilder};

use common::{AppError, AppResult};

use crate::dto::audit::{AuditLogQueryDto, AuditLogResponseDto};

/// audit_logs 表的数据库行映射
#[derive(sqlx::FromRow)]
pub struct AuditLogRecord {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<i64>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub service_name: String,
    pub request_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AuditLogRecord {
    /// 转换为 API 响应 DTO（时间戳格式化为 RFC 3339 字符串）
    pub fn to_response_dto(&self) -> AuditLogResponseDto {
        AuditLogResponseDto {
            id: self.id,
            timestamp: self.timestamp.to_rfc3339(),
            user_id: self.user_id,
            action: self.action.clone(),
            resource_type: self.resource_type.clone(),
            resource_id: self.resource_id.clone(),
            service_name: self.service_name.clone(),
            request_id: self.request_id.clone(),
            ip_address: self.ip_address.clone(),
            user_agent: self.user_agent.clone(),
            details: self.details.clone(),
            status: self.status.clone(),
            error_message: self.error_message.clone(),
            created_at: self.created_at.to_rfc3339(),
        }
    }
}

/// 审计日志列表列名（data 查询与 count 查询共用）
const AUDIT_LOG_COLUMNS: &str = "id, timestamp, user_id, action, resource_type, resource_id, \
     service_name, request_id, ip_address, user_agent, details, status, error_message, created_at";

/// 解析 ISO 8601 时间字符串为 UTC DateTime
fn parse_time(s: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AppError::invalid_input(format!("Invalid time format '{}': {}", s, e)))
}

/// 分页查询审计日志，返回（记录列表, 总数）
///
/// 支持按 user_id / action / status / 时间范围动态过滤，按 timestamp 倒序排列。
pub async fn query_audit_logs(
    pool: &PgPool,
    query: &AuditLogQueryDto,
) -> AppResult<(Vec<AuditLogRecord>, i64)> {
    let start_time = match &query.start_time {
        Some(s) => Some(parse_time(s)?),
        None => None,
    };
    let end_time = match &query.end_time {
        Some(s) => Some(parse_time(s)?),
        None => None,
    };

    // 限制分页范围，防止过大查询
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // 构建数据查询
    let mut data_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(format!(
        "SELECT {} FROM audit_logs WHERE 1=1",
        AUDIT_LOG_COLUMNS
    ));
    push_filters(&mut data_qb, query, &start_time, &end_time);
    data_qb
        .push(" ORDER BY timestamp DESC LIMIT ")
        .push_bind(page_size);
    data_qb.push(" OFFSET ").push_bind(offset);

    let records: Vec<AuditLogRecord> = data_qb
        .build_query_as::<AuditLogRecord>()
        .fetch_all(pool)
        .await?;

    // 构建计数查询（复用相同 WHERE 条件）
    let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM audit_logs WHERE 1=1");
    push_filters(&mut count_qb, query, &start_time, &end_time);

    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    Ok((records, total))
}

/// 向 QueryBuilder 追加过滤条件（data 查询与 count 查询共用）
///
/// 生命周期 `'a` 将 QueryBuilder 与 query 绑定，确保 push_bind 借用的字符串引用
/// 在 QueryBuilder 存活期间有效。
fn push_filters<'a>(
    qb: &mut QueryBuilder<'a, sqlx::Postgres>,
    query: &'a AuditLogQueryDto,
    start_time: &Option<DateTime<Utc>>,
    end_time: &Option<DateTime<Utc>>,
) {
    if let Some(uid) = query.user_id {
        qb.push(" AND user_id = ").push_bind(uid);
    }
    if let Some(ref action) = query.action {
        qb.push(" AND action = ").push_bind(action.as_str());
    }
    if let Some(ref status) = query.status {
        qb.push(" AND status = ").push_bind(status.as_str());
    }
    if let Some(start) = start_time {
        qb.push(" AND timestamp >= ").push_bind(*start);
    }
    if let Some(end) = end_time {
        qb.push(" AND timestamp <= ").push_bind(*end);
    }
}

/// 查询审计日志用于 CSV 导出（不分页，上限 10000 条防止内存溢出）
pub async fn query_audit_logs_for_export(
    pool: &PgPool,
    query: &AuditLogQueryDto,
) -> AppResult<Vec<AuditLogRecord>> {
    let start_time = match &query.start_time {
        Some(s) => Some(parse_time(s)?),
        None => None,
    };
    let end_time = match &query.end_time {
        Some(s) => Some(parse_time(s)?),
        None => None,
    };

    let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(format!(
        "SELECT {} FROM audit_logs WHERE 1=1",
        AUDIT_LOG_COLUMNS
    ));
    push_filters(&mut qb, query, &start_time, &end_time);
    qb.push(" ORDER BY timestamp DESC LIMIT 10000");

    let records: Vec<AuditLogRecord> = qb
        .build_query_as::<AuditLogRecord>()
        .fetch_all(pool)
        .await?;

    Ok(records)
}
