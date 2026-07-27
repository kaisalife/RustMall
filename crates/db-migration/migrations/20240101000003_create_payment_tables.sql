-- ============================================
-- Simple Trade - V3: Payment Tables
-- ============================================
-- 金额统一使用 NUMERIC(18, 8)，避免浮点精度丢失
-- 状态/渠道/类型等枚举以 VARCHAR 存储，由应用层负责序列化

-- 支付订单表
CREATE TABLE IF NOT EXISTS payment_orders (
    id BIGINT PRIMARY KEY,
    idempotency_key VARCHAR(128) UNIQUE NOT NULL,
    user_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    amount NUMERIC(18, 8) NOT NULL,
    fee NUMERIC(18, 8) DEFAULT 0,
    currency VARCHAR(8) NOT NULL DEFAULT 'CNY',
    channel VARCHAR(32) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'PENDING',
    channel_txn_id VARCHAR(128),
    pay_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 资金流水表（append-only）
CREATE TABLE IF NOT EXISTS payment_transactions (
    id BIGINT PRIMARY KEY,
    payment_order_id BIGINT NOT NULL REFERENCES payment_orders(id),
    txn_type VARCHAR(16) NOT NULL,
    amount NUMERIC(18, 8) NOT NULL,
    balance_after NUMERIC(18, 8),
    channel_txn_id VARCHAR(128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 退款表
CREATE TABLE IF NOT EXISTS payment_refunds (
    id BIGINT PRIMARY KEY,
    idempotency_key VARCHAR(128) UNIQUE NOT NULL,
    payment_id BIGINT NOT NULL REFERENCES payment_orders(id),
    refund_amount NUMERIC(18, 8) NOT NULL,
    currency VARCHAR(8) NOT NULL DEFAULT 'CNY',
    reason VARCHAR(256),
    status VARCHAR(16) NOT NULL DEFAULT 'PENDING',
    channel_txn_id VARCHAR(128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 幂等记录表
CREATE TABLE IF NOT EXISTS idempotency_records (
    idempotency_key VARCHAR(128) PRIMARY KEY,
    status VARCHAR(16) NOT NULL,
    response_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expired_at TIMESTAMPTZ NOT NULL
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_payment_orders_order_id ON payment_orders(order_id);
CREATE INDEX IF NOT EXISTS idx_payment_orders_user_id ON payment_orders(user_id);
CREATE INDEX IF NOT EXISTS idx_payment_transactions_payment_id ON payment_transactions(payment_order_id);
CREATE INDEX IF NOT EXISTS idx_payment_refunds_payment_id ON payment_refunds(payment_id);

-- 触发器
CREATE TRIGGER update_payment_orders_updated_at BEFORE UPDATE ON payment_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payment_refunds_updated_at BEFORE UPDATE ON payment_refunds
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
