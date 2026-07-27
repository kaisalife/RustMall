//! 仓储 trait 定义
//!
//! 定义数据访问接口，由 infrastructure 层实现。
//! domain 层只定义接口，不关心具体存储方式（PostgreSQL/MySQL 等）。

use async_trait::async_trait;
use common::AppResult;

use super::payment::{Payment, PaymentStatus};
use super::refund::Refund;
use super::transaction::Transaction;

/// 支付订单仓储
///
/// 提供支付订单的 CRUD 操作。
/// `find_by_idempotency_key` 用于幂等控制，`find_by_order_id` 用于关联查询。
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// 创建支付订单
    async fn create(&self, payment: Payment) -> AppResult<Payment>;

    /// 根据 ID 查询
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Payment>>;

    /// 根据幂等 key 查询（幂等控制核心方法）
    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Payment>>;

    /// 根据业务订单 ID 查询
    async fn find_by_order_id(&self, order_id: u64) -> AppResult<Option<Payment>>;

    /// 更新支付状态和渠道交易号
    async fn update_status(
        &self,
        id: u64,
        status: &PaymentStatus,
        channel_txn_id: Option<&str>,
    ) -> AppResult<()>;
}

/// 资金流水仓储
///
/// 流水表是 append-only 的，只支持创建和查询，不支持更新和删除。
#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// 创建流水记录（append-only）
    async fn create(&self, txn: Transaction) -> AppResult<Transaction>;

    /// 查询某支付订单的所有流水
    async fn find_by_payment_id(&self, payment_id: u64) -> AppResult<Vec<Transaction>>;
}

/// 退款仓储
///
/// 提供退款单的 CRUD 操作，支持幂等控制。
#[async_trait]
pub trait RefundRepository: Send + Sync {
    /// 创建退款单
    async fn create(&self, refund: Refund) -> AppResult<Refund>;

    /// 根据 ID 查询
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Refund>>;

    /// 根据幂等 key 查询（防重复退款）
    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Refund>>;

    /// 更新退款状态和渠道交易号
    async fn update_status(
        &self,
        id: u64,
        status: &PaymentStatus,
        channel_txn_id: Option<&str>,
    ) -> AppResult<()>;
}
