//! 审计日志查询相关 DTO

use serde::{Deserialize, Serialize};

/// 审计日志查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogQueryDto {
    /// 按用户 ID 过滤
    pub user_id: Option<i64>,
    /// 按操作类型过滤
    pub action: Option<String>,
    /// 按状态过滤
    pub status: Option<String>,
    /// 起始时间（ISO 8601 / RFC 3339）
    pub start_time: Option<String>,
    /// 结束时间（ISO 8601 / RFC 3339）
    pub end_time: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

/// 单条审计日志响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponseDto {
    pub id: i64,
    /// 事件发生时间（RFC 3339）
    pub timestamp: String,
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
    /// 记录写入时间（RFC 3339）
    pub created_at: String,
}

/// 分页列表响应
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogListDto {
    pub items: Vec<AuditLogResponseDto>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}
