//! 资金流水实体
//!
//! 流水记录采用 **append-only** 设计：只 INSERT，永不 UPDATE/DELETE。
//! 这是支付系统的基本原则，确保资金变动可追溯、不可篡改。
//!
//! 如需更正，新增一条冲正流水（正负抵消），而非修改原记录。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 流水类型
///
/// 每笔资金变动都会产生一条流水记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// 支付扣款
    Pay,
    /// 退款
    Refund,
    /// 手续费
    Fee,
}

impl TransactionType {
    /// 转为字符串（存储到 DB VARCHAR 列）
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Pay => "PAY",
            TransactionType::Refund => "REFUND",
            TransactionType::Fee => "FEE",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "PAY" => TransactionType::Pay,
            "REFUND" => TransactionType::Refund,
            "FEE" => TransactionType::Fee,
            _ => TransactionType::Pay, // 默认
        }
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 资金流水
///
/// 记录每一笔资金变动，是支付系统的"账本"。
/// 流水表无 currency 列，金额使用 `Decimal`（货币信息从 payment_orders 关联获取）。
#[derive(Debug, Clone)]
pub struct Transaction {
    /// 流水 ID（雪花算法生成）
    pub id: u64,
    /// 关联的支付订单 ID
    pub payment_order_id: u64,
    /// 流水类型
    pub txn_type: TransactionType,
    /// 金额（正数为收入，负数为支出）
    pub amount: Decimal,
    /// 操作后余额（可选，用于账户余额场景）
    pub balance_after: Option<Decimal>,
    /// 渠道交易号
    pub channel_txn_id: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl Transaction {
    /// 创建流水记录
    pub fn new(
        id: u64,
        payment_order_id: u64,
        txn_type: TransactionType,
        amount: Decimal,
    ) -> Self {
        Self {
            id,
            payment_order_id,
            txn_type,
            amount,
            balance_after: None,
            channel_txn_id: None,
            created_at: Utc::now(),
        }
    }

    /// 设置操作后余额
    pub fn with_balance(mut self, balance: Decimal) -> Self {
        self.balance_after = Some(balance);
        self
    }

    /// 设置渠道交易号
    pub fn with_channel_txn(mut self, txn_id: String) -> Self {
        self.channel_txn_id = Some(txn_id);
        self
    }
}
