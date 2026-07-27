//! 接口层（Interface）。
//!
//! 负责将外部协议（gRPC）请求转换为 application 层的命令调用，
//! 并将 application 层返回的 DTO 转换为 gRPC 响应。
//! 接口层不包含业务逻辑，只做协议适配与类型转换。

pub mod payment_service;

pub use payment_service::PaymentServiceImpl;
