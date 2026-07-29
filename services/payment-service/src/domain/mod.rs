//! 支付服务领域层
//!
//! 包含支付系统的核心领域模型：金额类型、支付订单、资金流水、退款单。
//! 遵循 DDD 原则，领域层不依赖基础设施，只定义业务规则和状态机。

pub mod money;
pub mod payment;
pub mod refund;
pub mod repository;
pub mod transaction;

pub use money::{Currency, Money};
pub use payment::{Payment, PaymentChannel, PaymentStatus};
pub use refund::Refund;
pub use repository::{PaymentRepository, RefundRepository, TransactionRepository};
pub use transaction::{Transaction, TransactionType};
