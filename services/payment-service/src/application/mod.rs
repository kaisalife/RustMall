//! 支付服务应用层（Application Layer）
//!
//! 应用层负责业务用例的编排，协调领域对象、仓储和基础设施适配器完成完整业务流程。
//! - 不包含业务规则（业务规则在 domain 层）
//! - 不关心技术细节（技术细节在 infrastructure 层）
//! - 对外暴露应用服务、命令对象、DTO 等类型，供 interface 层调用

pub mod service;
pub mod command;
pub mod dto;
pub mod idempotency;
pub mod routing;

pub use service::PaymentApplicationService;
pub use command::*;
pub use dto::*;
