//! 基础设施层
//! 
//! 提供邮件发送的具体实现和数据库存储

pub mod email_sender;
pub mod repository;

pub use email_sender::EmailSender;
pub use repository::EmailRepositoryImpl;
