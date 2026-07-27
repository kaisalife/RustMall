//! 事件总线 crate
//!
//! 基于 rdkafka（librdkafka 的 Rust 绑定）实现高吞吐事件处理。
//!
//! ## 设计要点
//!
//! - **长连接**：rdkafka 内部维护到 broker 的长连接，自动重连、心跳保活
//! - **Producer**：`FutureProducer` 异步发送，内部缓冲 + 批量推送
//! - **Consumer**：`StreamConsumer` 消息流，消费者组自动 rebalance
//! - **事件类型**：用 enum + serde 序列化，topic 按 `{prefix}.{event_type}` 命名
//!
//! ## 使用方式
//!
//! ### Producer（发布事件）
//! ```ignore
//! let producer = EventBusProducer::new(&kafka_config).await?;
//! producer.publish(&PaymentSucceeded { payment_id: 123 }).await?;
//! ```
//!
//! ### Consumer（订阅事件）
//! ```ignore
//! let consumer = EventBusConsumer::new(&kafka_config, "payment-group").await?;
//! let mut stream = consumer.subscribe(&["simple_trade.payment.succeeded"]);
//! while let Some(msg) = stream.next().await {
//!     let event: PaymentSucceeded = serde_json::from_slice(&msg.payload)?;
//!     // 处理事件...
//! }
//! ```

pub mod event;
pub mod producer;
pub mod consumer;

pub use event::{EventPayload, EventType, OrderItemEvent, EventEnvelope};
pub use producer::EventBusProducer;
pub use consumer::EventBusConsumer;
