-- ============================================
-- Simple Trade - V1: Initial Schema
-- ============================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================
-- 1. Users Table
-- ============================================
CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    nickname VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

COMMENT ON TABLE users IS '用户表';
COMMENT ON COLUMN users.id IS '雪花算法生成的用户ID';
COMMENT ON COLUMN users.email IS '用户邮箱';
COMMENT ON COLUMN users.password_hash IS 'bcrypt加密后的密码';
COMMENT ON COLUMN users.nickname IS '用户昵称';

-- ============================================
-- 2. Categories Table
-- ============================================
CREATE TABLE IF NOT EXISTS categories (
    id BIGINT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    parent_id BIGINT REFERENCES categories(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_id);

COMMENT ON TABLE categories IS '商品分类表';

-- ============================================
-- 3. Products Table
-- ============================================
CREATE TABLE IF NOT EXISTS products (
    id BIGINT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(12, 2) NOT NULL,
    category_id BIGINT NOT NULL REFERENCES categories(id),
    stock INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_price ON products(price);

COMMENT ON TABLE products IS '商品表';

-- ============================================
-- 4. Inventory Table
-- ============================================
CREATE TABLE IF NOT EXISTS inventory (
    product_id BIGINT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    quantity INT NOT NULL DEFAULT 0,
    reserved_quantity INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE inventory IS '库存表';

-- ============================================
-- 5. Order Status ENUM
-- ============================================
DO $$ BEGIN
    CREATE TYPE order_status AS ENUM (
        'PENDING',
        'PAID',
        'SHIPPED',
        'COMPLETED',
        'CANCELLED',
        'REFUNDED'
    );
EXCEPTION WHEN duplicate_object THEN null;
END $$;

-- ============================================
-- 6. Orders Table
-- ============================================
CREATE TABLE IF NOT EXISTS orders (
    id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    total_amount DECIMAL(12, 2) NOT NULL,
    status order_status NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_orders_user ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_created ON orders(created_at);

COMMENT ON TABLE orders IS '订单表';

-- ============================================
-- 7. Order Items Table
-- ============================================
CREATE TABLE IF NOT EXISTS order_items (
    id BIGINT PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL,
    quantity INT NOT NULL,
    unit_price DECIMAL(12, 2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_product ON order_items(product_id);

COMMENT ON TABLE order_items IS '订单商品项表';

-- ============================================
-- 8. Create updated_at trigger function
-- ============================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================
-- 9. Apply triggers (DROP IF EXISTS for idempotency)
-- ============================================
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_categories_updated_at ON categories;
CREATE TRIGGER update_categories_updated_at BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_products_updated_at ON products;
CREATE TRIGGER update_products_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_inventory_updated_at ON inventory;
CREATE TRIGGER update_inventory_updated_at BEFORE UPDATE ON inventory
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS update_orders_updated_at ON orders;
CREATE TRIGGER update_orders_updated_at BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- 10. Insert Initial Data
-- ============================================

-- Insert default categories
INSERT INTO categories (id, name, parent_id) VALUES
    (1001, '电子产品', NULL),
    (1002, '手机', 1001),
    (1003, '电脑', 1001),
    (1004, '服装', NULL),
    (1005, '书籍', NULL)
ON CONFLICT DO NOTHING;

-- Insert sample products
INSERT INTO products (id, name, description, price, category_id, stock) VALUES
    (2001, 'Rust 编程指南', '学习 Rust 编程语言的最佳书籍', 99.00, 1005, 100),
    (2002, 'DDD 领域驱动设计', '深入理解领域驱动设计', 128.00, 1005, 50),
    (2003, '高性能笔记本电脑', '16GB RAM, 512GB SSD, i7', 6999.00, 1003, 20)
ON CONFLICT DO NOTHING;

-- Insert inventory for products
INSERT INTO inventory (product_id, quantity, reserved_quantity) VALUES
    (2001, 100, 0),
    (2002, 50, 0),
    (2003, 20, 0)
ON CONFLICT DO NOTHING;
