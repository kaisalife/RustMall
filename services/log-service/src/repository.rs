//! 日志仓储层：批量写入 app_logs + audit_logs

use common::AppResult;
use sqlx::{PgPool, query};
use event_bus::{EventPayload, EventEnvelope};

/// 批量写入应用日志
pub async fn batch_insert_app_logs(pool: &PgPool, logs: &[EventEnvelope]) -> AppResult<u64> {
    if logs.is_empty() {
        return Ok(0);
    }

    let mut rows_affected = 0u64;
    for envelope in logs {
        if let EventPayload::AppLog {
            level, message, request_id, trace_id, span_id, user_id, fields, file, line,
        } = &envelope.payload
        {
            let result = query(
                r#"INSERT INTO app_logs (timestamp, level, service_name, message, request_id, trace_id, span_id, user_id, fields, file, line)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
            )
            .bind(envelope.timestamp)
            .bind(level)
            .bind(&envelope.source)
            .bind(message)
            .bind(request_id.as_deref())
            .bind(trace_id.as_deref())
            .bind(span_id.as_deref())
            .bind(user_id.map(|id| id as i64))
            .bind(fields)
            .bind(file.as_deref())
            .bind(line.map(|l| l as i32))
            .execute(pool)
            .await;

            match result {
                Ok(r) => rows_affected += r.rows_affected(),
                Err(e) => tracing::error!("Failed to insert app_log: {}", e),
            }
        }
    }

    tracing::debug!("Inserted {} app_logs", rows_affected);
    Ok(rows_affected)
}

/// 批量写入审计日志
pub async fn batch_insert_audit_logs(pool: &PgPool, logs: &[EventEnvelope]) -> AppResult<u64> {
    if logs.is_empty() {
        return Ok(0);
    }

    let mut rows_affected = 0u64;
    for envelope in logs {
        if let EventPayload::AuditLog {
            user_id, action, resource_type, resource_id, request_id, ip_address, user_agent, details, status, error_message,
        } = &envelope.payload
        {
            let result = query(
                r#"INSERT INTO audit_logs (timestamp, user_id, action, resource_type, resource_id, service_name, request_id, ip_address, user_agent, details, status, error_message)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
            )
            .bind(envelope.timestamp)
            .bind(user_id.map(|id| id as i64))
            .bind(action)
            .bind(resource_type.as_deref())
            .bind(resource_id.as_deref())
            .bind(&envelope.source)
            .bind(request_id.as_deref())
            .bind(ip_address.as_deref())
            .bind(user_agent.as_deref())
            .bind(details)
            .bind(status)
            .bind(error_message.as_deref())
            .execute(pool)
            .await;

            match result {
                Ok(r) => rows_affected += r.rows_affected(),
                Err(e) => tracing::error!("Failed to insert audit_log: {}", e),
            }
        }
    }

    tracing::debug!("Inserted {} audit_logs", rows_affected);
    Ok(rows_affected)
}
