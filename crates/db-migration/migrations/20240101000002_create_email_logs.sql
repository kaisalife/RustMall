-- 邮件日志表
CREATE TABLE IF NOT EXISTS email_logs (
    id BIGINT PRIMARY KEY,
    to_email VARCHAR(255) NOT NULL,
    username VARCHAR(100),
    subject VARCHAR(500) NOT NULL,
    html_content TEXT NOT NULL,
    email_type VARCHAR(50) NOT NULL DEFAULT 'Custom',
    status VARCHAR(20) NOT NULL DEFAULT 'Pending',
    message_id VARCHAR(255),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_email_logs_to_email ON email_logs(to_email);
CREATE INDEX IF NOT EXISTS idx_email_logs_status ON email_logs(status);
CREATE INDEX IF NOT EXISTS idx_email_logs_created_at ON email_logs(created_at);

-- 自动更新 updated_at
CREATE OR REPLACE TRIGGER update_email_logs_updated_at
    BEFORE UPDATE ON email_logs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE email_logs IS '邮件发送日志';
COMMENT ON COLUMN email_logs.email_type IS '邮件类型: Verification, OrderNotification, PasswordReset, Custom';
COMMENT ON COLUMN email_logs.status IS '邮件状态: Pending, Sent, Failed';
