//! 邮件领域层
//! 
//! 定义邮件发送的核心业务规则和接口

pub mod model;
pub mod repository;

pub use model::{Email, EmailType, EmailStatus};
pub use repository::EmailRepository;
