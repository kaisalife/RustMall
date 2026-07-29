//! 应用层
//!
//! 邮件发送业务编排

pub mod command;
pub mod service;

pub use service::EmailApplicationService;
