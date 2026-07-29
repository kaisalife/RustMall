-- Add version column for optimistic locking
ALTER TABLE inventory ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN inventory.version IS '乐观锁版本号，每次更新递增';
