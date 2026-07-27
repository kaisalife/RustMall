-- ============================================
-- Simple Trade - V4: Distributed Log Tables
-- ============================================

-- ============================================
-- 1. App Logs (通用应用日志)
-- ============================================
CREATE TABLE IF NOT EXISTS app_logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    level VARCHAR(20) NOT NULL,
    service_name VARCHAR(50) NOT NULL,
    message TEXT NOT NULL,
    request_id VARCHAR(64),
    trace_id VARCHAR(64),
    span_id VARCHAR(32),
    user_id BIGINT,
    fields JSONB DEFAULT '{}'::jsonb,
    file VARCHAR(255),
    line INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_app_logs_timestamp ON app_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_app_logs_level ON app_logs(level);
CREATE INDEX IF NOT EXISTS idx_app_logs_service ON app_logs(service_name);
CREATE INDEX IF NOT EXISTS idx_app_logs_request_id ON app_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_app_logs_user_id ON app_logs(user_id);

COMMENT ON TABLE app_logs IS '分布式应用日志表（Kafka 消费写入）';

-- ============================================
-- 2. Audit Logs (审计日志 - 用户操作记录)
-- ============================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    user_id BIGINT,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50),
    resource_id VARCHAR(64),
    service_name VARCHAR(50) NOT NULL,
    request_id VARCHAR(64),
    ip_address VARCHAR(45),
    user_agent TEXT,
    details JSONB DEFAULT '{}'::jsonb,
    status VARCHAR(20) NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_request_id ON audit_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_status ON audit_logs(status);

COMMENT ON TABLE audit_logs IS '审计日志表（记录用户敏感操作）';
