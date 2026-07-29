//! 基础设施层（Infrastructure）。
//!
//! 负责与外部系统交互的技术实现细节，包含：
//! - `database`：数据库连接池封装
//! - `repository`：domain 层仓储 trait 的 PostgreSQL 实现
//! - `channels`：第三方支付渠道适配器（微信、支付宝、测试桩）
//!
//! 基础设施层向上依赖 domain 层（实现 domain 定义的 trait），
//! 不被 domain/application 层反向依赖，保证依赖方向单一。

pub mod channels;
pub mod database;
pub mod repository;

pub use channels::{PaymentChannelAdapter, StubChannelAdapter};
pub use database::PaymentDatabase;
pub use repository::{PgPaymentRepository, PgRefundRepository, PgTransactionRepository};
