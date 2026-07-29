//! 支付服务 PostgreSQL 仓储实现。
//!
//! 实现 domain 层定义的三个仓储 trait：
//! - [`PaymentRepository`](crate::domain::PaymentRepository)：支付订单
//! - [`TransactionRepository`](crate::domain::TransactionRepository)：资金流水（append-only）
//! - [`RefundRepository`](crate::domain::RefundRepository)：退款单
//!
//! ## 关键设计说明
//!
//! 1. **金额类型映射**：金额使用 `rust_decimal::Decimal`，PostgreSQL 列为 `NUMERIC(18, 8)`。
//!    由于当前 workspace 的 sqlx 未启用 `rust_decimal` feature，NUMERIC 列以 `String`
//!    读写，再通过 `Decimal::from_str` / `Decimal::to_string` 转换，避免 f64 精度丢失。
//!    （后续可启用 sqlx `rust_decimal` feature 以直接绑定 Decimal。）
//!
//! 2. **枚举序列化**：`PaymentStatus` / `PaymentChannel` / `TransactionType` 枚举
//!    序列化为 VARCHAR 存储，字符串值与 proto 定义保持一致
//!    （如 `PENDING`、`WECHAT`、`PAY`）。
//!
//! 3. **主键映射**：domain 层 ID 为 `u64`，PostgreSQL 以 `BIGINT`（`i64`）存储，
//!    读写时做 `as i64` / `as u64` 转换。
//!
//! ## domain 类型假设
//!
//! 本文件假设 domain 层提供如下类型（字段为公开）。部分字段类型已由 application 层
//! 的 DTO `From` 实现确认（如 `channel_txn_id: String`），其余为合理推断：
//! - `Money { amount: Decimal, currency: Currency }`，含 `Money::new(amount, currency)`
//! - `Currency`：枚举，实现 `as_str() -> &str` 与 `FromStr`
//! - `PaymentStatus` 变体：`Pending / Processing / Success / Failed / Refunding /
//!   Refunded / PartialRefunded / Closed`
//! - `PaymentChannel` 变体：`Unknown / WeChat / Alipay / BankCard / Stub`
//! - `TransactionType` 变体：`Pay / Refund`
//! - `Payment`：`channel_txn_id: String`、`pay_url: String`（非 Option，DB 列可空，
//!   读取时用空串兜底）
//! - `Refund`：`channel_txn_id: String`（同上）；`reason: Option<String>`
//! - `Transaction`：流水表无 currency 列，`amount: Decimal`、`balance_after: Option<Decimal>`、
//!   `channel_txn_id: Option<String>`
//!
//! 仓储 trait 方法签名假设如下（若 domain 层定义不同，需相应调整 impl）：
//! - `PaymentRepository`: `create / find_by_id / find_by_idempotency_key /
//!   find_by_order_id / update`
//! - `TransactionRepository`: `create / find_by_payment_id`
//! - `RefundRepository`: `create / find_by_id / find_by_idempotency_key /
//!   find_by_payment_id / update`

use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::domain::{
    Currency, Money, Payment, PaymentChannel, PaymentRepository, PaymentStatus, Refund,
    RefundRepository, Transaction, TransactionRepository, TransactionType,
};
use common::{AppError, AppResult};

// ============================================================================
// PaymentRepository 实现
// ============================================================================

/// 支付订单的 PostgreSQL 仓储实现。
#[derive(Clone)]
pub struct PgPaymentRepository {
    pool: PgPool,
}

impl PgPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PaymentRepository for PgPaymentRepository {
    async fn create(&self, payment: Payment) -> AppResult<Payment> {
        // 枚举/金额序列化：存为 VARCHAR / NUMERIC(经 String)
        let status_str = status_to_string(&payment.status);
        let channel_str = channel_to_string(&payment.channel);
        let amount_str = payment.amount.amount.to_string();
        let fee_str = payment.fee.amount.to_string();
        let currency_str = payment.amount.currency.as_str();

        let _sql = r#"
            INSERT INTO payment_orders
                (id, idempotency_key, user_id, order_id, amount, fee, currency,
                 channel, status, channel_txn_id, pay_url, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING
                id, idempotency_key, user_id, order_id, amount, fee, currency,
                channel, status, channel_txn_id, pay_url, created_at, updated_at
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, PaymentRecord>(_sql)
        //     .bind(payment.id as i64)
        //     .bind(&payment.idempotency_key)
        //     .bind(payment.user_id as i64)
        //     .bind(payment.order_id as i64)
        //     .bind(&amount_str)                      // NUMERIC 经 String 传递
        //     .bind(&fee_str)
        //     .bind(currency_str)                     // VARCHAR
        //     .bind(&channel_str)                     // VARCHAR
        //     .bind(&status_str)                      // VARCHAR
        //     .bind(&payment.channel_txn_id)          // VARCHAR（domain 为 String）
        //     .bind(&payment.pay_url)                 // TEXT（domain 为 String）
        //     .bind(payment.created_at)
        //     .bind(payment.updated_at)
        //     .fetch_one(&self.pool)
        //     .await?;
        // Ok(record.into_domain()?)
        let _ = (
            &status_str,
            &channel_str,
            &amount_str,
            &fee_str,
            currency_str,
            payment.id,
            payment.user_id,
            payment.order_id,
            &payment.idempotency_key,
            &payment.channel_txn_id,
            &payment.pay_url,
            payment.created_at,
            payment.updated_at,
        );
        todo!("绑定参数并执行 INSERT payment_orders")
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Payment>> {
        let _sql = r#"
            SELECT id, idempotency_key, user_id, order_id, amount, fee, currency,
                   channel, status, channel_txn_id, pay_url, created_at, updated_at
            FROM payment_orders
            WHERE id = $1
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, PaymentRecord>(_sql)
        //     .bind(id as i64)
        //     .fetch_optional(&self.pool)
        //     .await?;
        // Ok(record.map(|r| r.into_domain()).transpose()?)
        let _ = id;
        todo!("绑定参数并执行 SELECT payment_orders by id")
    }

    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Payment>> {
        let _sql = r#"
            SELECT id, idempotency_key, user_id, order_id, amount, fee, currency,
                   channel, status, channel_txn_id, pay_url, created_at, updated_at
            FROM payment_orders
            WHERE idempotency_key = $1
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, PaymentRecord>(_sql)
        //     .bind(key)
        //     .fetch_optional(&self.pool)
        //     .await?;
        // Ok(record.map(|r| r.into_domain()).transpose()?)
        let _ = key;
        todo!("绑定参数并执行 SELECT payment_orders by idempotency_key")
    }

    async fn find_by_order_id(&self, order_id: u64) -> AppResult<Option<Payment>> {
        let _sql = r#"
            SELECT id, idempotency_key, user_id, order_id, amount, fee, currency,
                   channel, status, channel_txn_id, pay_url, created_at, updated_at
            FROM payment_orders
            WHERE order_id = $1
            ORDER BY created_at DESC
            LIMIT 1
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, PaymentRecord>(_sql)
        //     .bind(order_id as i64)
        //     .fetch_optional(&self.pool)
        //     .await?;
        // Ok(record.map(|r| r.into_domain()).transpose()?)
        let _ = order_id;
        todo!("绑定参数并执行 SELECT payment_orders by order_id")
    }

    async fn update_status(
        &self,
        id: u64,
        status: &PaymentStatus,
        channel_txn_id: Option<&str>,
    ) -> AppResult<()> {
        let status_str = status_to_string(status);

        let _sql = r#"
            UPDATE payment_orders
            SET status = $2, channel_txn_id = $3, updated_at = NOW()
            WHERE id = $1
        "#;
        // TODO: 绑定参数并执行
        // sqlx::query(_sql)
        //     .bind(id as i64)
        //     .bind(&status_str)                      // VARCHAR
        //     .bind(channel_txn_id)                   // 可空 VARCHAR
        //     .execute(&self.pool)
        //     .await?;
        let _ = (id, &status_str, channel_txn_id);
        todo!("绑定参数并执行 UPDATE payment_orders status")
    }
}

// ============================================================================
// TransactionRepository 实现
// ============================================================================

/// 资金流水的 PostgreSQL 仓储实现。流水表为 append-only，仅支持写入与查询。
#[derive(Clone)]
pub struct PgTransactionRepository {
    pool: PgPool,
}

impl PgTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TransactionRepository for PgTransactionRepository {
    async fn create(&self, txn: Transaction) -> AppResult<Transaction> {
        let txn_type_str = txn_type_to_string(&txn.txn_type);
        let amount_str = txn.amount.to_string();
        let balance_after_str = txn.balance_after.map(|d| d.to_string());

        let _sql = r#"
            INSERT INTO payment_transactions
                (id, payment_order_id, txn_type, amount, balance_after, channel_txn_id, created_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, payment_order_id, txn_type, amount, balance_after, channel_txn_id, created_at
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, TransactionRecord>(_sql)
        //     .bind(txn.id as i64)
        //     .bind(txn.payment_order_id as i64)
        //     .bind(&txn_type_str)                    // VARCHAR
        //     .bind(&amount_str)                      // NUMERIC 经 String
        //     .bind(balance_after_str.as_deref())     // 可空 NUMERIC
        //     .bind(txn.channel_txn_id.as_deref())    // 可空 VARCHAR
        //     .bind(txn.created_at)
        //     .fetch_one(&self.pool)
        //     .await?;
        // Ok(record.into_domain()?)
        let _ = (
            &txn_type_str,
            &amount_str,
            &balance_after_str,
            txn.id,
            txn.payment_order_id,
            &txn.channel_txn_id,
            txn.created_at,
        );
        todo!("绑定参数并执行 INSERT payment_transactions")
    }

    async fn find_by_payment_id(&self, payment_id: u64) -> AppResult<Vec<Transaction>> {
        let _sql = r#"
            SELECT id, payment_order_id, txn_type, amount, balance_after, channel_txn_id, created_at
            FROM payment_transactions
            WHERE payment_order_id = $1
            ORDER BY created_at ASC
        "#;
        // TODO: 绑定参数并执行
        // let records = sqlx::query_as::<_, TransactionRecord>(_sql)
        //     .bind(payment_id as i64)
        //     .fetch_all(&self.pool)
        //     .await?;
        // records.into_iter().map(|r| r.into_domain()).collect()
        let _ = payment_id;
        todo!("绑定参数并执行 SELECT payment_transactions by payment_order_id")
    }
}

// ============================================================================
// RefundRepository 实现
// ============================================================================

/// 退款单的 PostgreSQL 仓储实现。
#[derive(Clone)]
pub struct PgRefundRepository {
    pool: PgPool,
}

impl PgRefundRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl RefundRepository for PgRefundRepository {
    async fn create(&self, refund: Refund) -> AppResult<Refund> {
        let status_str = status_to_string(&refund.status);
        let amount_str = refund.refund_amount.amount.to_string();
        let currency_str = refund.refund_amount.currency.as_str();

        let _sql = r#"
            INSERT INTO payment_refunds
                (id, idempotency_key, payment_id, refund_amount, currency,
                 reason, status, channel_txn_id, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, idempotency_key, payment_id, refund_amount, currency,
                      reason, status, channel_txn_id, created_at, updated_at
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, RefundRecord>(_sql)
        //     .bind(refund.id as i64)
        //     .bind(&refund.idempotency_key)
        //     .bind(refund.payment_id as i64)
        //     .bind(&amount_str)                      // NUMERIC 经 String
        //     .bind(currency_str)                     // VARCHAR
        //     .bind(refund.reason.as_deref())        // 可空 VARCHAR
        //     .bind(&status_str)                      // VARCHAR
        //     .bind(&refund.channel_txn_id)           // VARCHAR（domain 为 String）
        //     .bind(refund.created_at)
        //     .bind(refund.updated_at)
        //     .fetch_one(&self.pool)
        //     .await?;
        // Ok(record.into_domain()?)
        let _ = (
            &status_str,
            &amount_str,
            currency_str,
            refund.id,
            refund.payment_id,
            &refund.idempotency_key,
            &refund.reason,
            &refund.channel_txn_id,
            refund.created_at,
            refund.updated_at,
        );
        todo!("绑定参数并执行 INSERT payment_refunds")
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Refund>> {
        let _sql = r#"
            SELECT id, idempotency_key, payment_id, refund_amount, currency,
                   reason, status, channel_txn_id, created_at, updated_at
            FROM payment_refunds
            WHERE id = $1
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, RefundRecord>(_sql)
        //     .bind(id as i64)
        //     .fetch_optional(&self.pool)
        //     .await?;
        // Ok(record.map(|r| r.into_domain()).transpose()?)
        let _ = id;
        todo!("绑定参数并执行 SELECT payment_refunds by id")
    }

    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Refund>> {
        let _sql = r#"
            SELECT id, idempotency_key, payment_id, refund_amount, currency,
                   reason, status, channel_txn_id, created_at, updated_at
            FROM payment_refunds
            WHERE idempotency_key = $1
        "#;
        // TODO: 绑定参数并执行
        // let record = sqlx::query_as::<_, RefundRecord>(_sql)
        //     .bind(key)
        //     .fetch_optional(&self.pool)
        //     .await?;
        // Ok(record.map(|r| r.into_domain()).transpose()?)
        let _ = key;
        todo!("绑定参数并执行 SELECT payment_refunds by idempotency_key")
    }

    async fn update_status(
        &self,
        id: u64,
        status: &PaymentStatus,
        channel_txn_id: Option<&str>,
    ) -> AppResult<()> {
        let status_str = status_to_string(status);

        let _sql = r#"
            UPDATE payment_refunds
            SET status = $2, channel_txn_id = $3, updated_at = NOW()
            WHERE id = $1
        "#;
        // TODO: 绑定参数并执行
        // sqlx::query(_sql)
        //     .bind(id as i64)
        //     .bind(&status_str)                      // VARCHAR
        //     .bind(channel_txn_id)                   // 可空 VARCHAR
        //     .execute(&self.pool)
        //     .await?;
        let _ = (id, &status_str, channel_txn_id);
        todo!("绑定参数并执行 UPDATE payment_refunds status")
    }
}

// ============================================================================
// 数据库行记录与 domain 转换
// ============================================================================

/// payment_orders 表行记录。
///
/// amount/fee 以 String 读取 NUMERIC，再解析为 Decimal。
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PaymentRecord {
    id: i64,
    idempotency_key: String,
    user_id: i64,
    order_id: i64,
    amount: String,
    fee: String,
    currency: String,
    channel: String,
    status: String,
    channel_txn_id: Option<String>,
    pay_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PaymentRecord {
    #[allow(dead_code)]
    fn into_domain(self) -> AppResult<Payment> {
        let amount = parse_decimal(&self.amount)?;
        let fee = parse_decimal(&self.fee)?;
        let currency = Currency::from_str(&self.currency).map_err(|e| {
            AppError::internal(format!("invalid currency '{}': {}", self.currency, e))
        })?;
        Ok(Payment {
            id: self.id as u64,
            idempotency_key: self.idempotency_key,
            user_id: self.user_id as u64,
            order_id: self.order_id as u64,
            amount: Money::new(amount, currency),
            fee: Money::new(fee, currency),
            channel: string_to_channel(&self.channel),
            status: string_to_status(&self.status),
            channel_txn_id: self.channel_txn_id.unwrap_or_default(),
            pay_url: self.pay_url.unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// payment_transactions 表行记录。
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct TransactionRecord {
    id: i64,
    payment_order_id: i64,
    txn_type: String,
    amount: String,
    balance_after: Option<String>,
    channel_txn_id: Option<String>,
    created_at: DateTime<Utc>,
}

impl TransactionRecord {
    #[allow(dead_code)]
    fn into_domain(self) -> AppResult<Transaction> {
        let amount = parse_decimal(&self.amount)?;
        let balance_after = self
            .balance_after
            .as_deref()
            .map(parse_decimal)
            .transpose()?;
        // 流水表无 currency 列，amount/balance_after 直接使用 Decimal
        Ok(Transaction {
            id: self.id as u64,
            payment_order_id: self.payment_order_id as u64,
            txn_type: string_to_txn_type(&self.txn_type),
            amount,
            balance_after,
            channel_txn_id: self.channel_txn_id,
            created_at: self.created_at,
        })
    }
}

/// payment_refunds 表行记录。
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct RefundRecord {
    id: i64,
    idempotency_key: String,
    payment_id: i64,
    refund_amount: String,
    currency: String,
    reason: Option<String>,
    status: String,
    channel_txn_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RefundRecord {
    #[allow(dead_code)]
    fn into_domain(self) -> AppResult<Refund> {
        let amount = parse_decimal(&self.refund_amount)?;
        let currency = Currency::from_str(&self.currency).map_err(|e| {
            AppError::internal(format!("invalid currency '{}': {}", self.currency, e))
        })?;
        Ok(Refund {
            id: self.id as u64,
            idempotency_key: self.idempotency_key,
            payment_id: self.payment_id as u64,
            refund_amount: Money::new(amount, currency),
            reason: self.reason,
            status: string_to_status(&self.status),
            channel_txn_id: self.channel_txn_id.unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// ============================================================================
// 枚举序列化辅助函数（VARCHAR <-> domain 枚举）
// ============================================================================

/// 将支付状态序列化为 VARCHAR，字符串值与 proto 定义一致。
fn status_to_string(status: &PaymentStatus) -> String {
    match status {
        PaymentStatus::Pending => "PENDING",
        PaymentStatus::Processing => "PROCESSING",
        PaymentStatus::Success => "SUCCESS",
        PaymentStatus::Failed => "FAILED",
        PaymentStatus::Refunding => "REFUNDING",
        PaymentStatus::Refunded => "REFUNDED",
        PaymentStatus::PartialRefunded => "PARTIAL_REFUNDED",
        PaymentStatus::Closed => "CLOSED",
    }
    .to_string()
}

/// 将 VARCHAR 反序列化为支付状态，未知值降级为 Pending。
fn string_to_status(s: &str) -> PaymentStatus {
    match s {
        "PENDING" => PaymentStatus::Pending,
        "PROCESSING" => PaymentStatus::Processing,
        "SUCCESS" => PaymentStatus::Success,
        "FAILED" => PaymentStatus::Failed,
        "REFUNDING" => PaymentStatus::Refunding,
        "REFUNDED" => PaymentStatus::Refunded,
        "PARTIAL_REFUNDED" => PaymentStatus::PartialRefunded,
        "CLOSED" => PaymentStatus::Closed,
        _ => PaymentStatus::Pending,
    }
}

/// 将支付渠道序列化为 VARCHAR。
fn channel_to_string(channel: &PaymentChannel) -> String {
    match channel {
        PaymentChannel::Unknown => "UNKNOWN_CHANNEL",
        PaymentChannel::WeChat => "WECHAT",
        PaymentChannel::Alipay => "ALIPAY",
        PaymentChannel::BankCard => "BANK_CARD",
        PaymentChannel::Stub => "STUB",
    }
    .to_string()
}

/// 将 VARCHAR 反序列化为支付渠道，未知值降级为 Unknown。
fn string_to_channel(s: &str) -> PaymentChannel {
    match s {
        "WECHAT" => PaymentChannel::WeChat,
        "ALIPAY" => PaymentChannel::Alipay,
        "BANK_CARD" => PaymentChannel::BankCard,
        "STUB" => PaymentChannel::Stub,
        _ => PaymentChannel::Unknown,
    }
}

/// 将交易类型序列化为 VARCHAR。
fn txn_type_to_string(t: &TransactionType) -> String {
    match t {
        TransactionType::Pay => "PAY",
        TransactionType::Refund => "REFUND",
        TransactionType::Fee => "FEE",
    }
    .to_string()
}

/// 将 VARCHAR 反序列化为交易类型，未知值降级为 Pay。
fn string_to_txn_type(s: &str) -> TransactionType {
    match s {
        "REFUND" => TransactionType::Refund,
        _ => TransactionType::Pay,
    }
}

/// 将 NUMERIC 列读出的字符串解析为 Decimal，失败返回内部错误。
fn parse_decimal(s: &str) -> AppResult<Decimal> {
    Decimal::from_str(s).map_err(|e| AppError::internal(format!("invalid decimal '{}': {}", s, e)))
}
