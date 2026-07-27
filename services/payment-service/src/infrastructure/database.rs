//! 支付服务数据库连接封装。
//!
//! `PaymentDatabase` 仅持有 `PgPool`，供各仓储共享。
//! 连接池的创建在 main.rs 中通过 `common::create_pool` 完成，
//! 随后传入本结构体，保持与基础设施层其他服务一致的职责划分。

use sqlx::PgPool;

/// 支付服务数据库访问入口。
///
/// 包装 `PgPool`，提供统一的连接池获取入口，
/// 各 PostgreSQL 仓储实现通过 `pool()` 拿到连接池后执行查询。
#[derive(Clone)]
pub struct PaymentDatabase {
    pool: PgPool,
}

impl PaymentDatabase {
    /// 由已创建的连接池构造数据库访问对象。
    ///
    /// 连接池的生命周期由调用方（main.rs）管理，
    /// 这里只做持有，避免重复创建池。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取底层 PostgreSQL 连接池引用。
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
