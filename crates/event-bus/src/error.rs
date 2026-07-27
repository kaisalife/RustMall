//! 事件总线错误类型
//!
//! 统一封装 Kafka、序列化、初始化等错误，便于上层用 `?` 传播。

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("Kafka error: {0}")]
    Kafka(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Producer not initialized")]
    ProducerNotInitialized,

    #[error("Consumer not initialized")]
    ConsumerNotInitialized,
}

pub type EventBusResult<T> = Result<T, EventBusError>;

// 实现 From<rdkafka::error::KafkaError>
impl From<rdkafka::error::KafkaError> for EventBusError {
    fn from(e: rdkafka::error::KafkaError) -> Self {
        EventBusError::Kafka(e.to_string())
    }
}

impl From<serde_json::Error> for EventBusError {
    fn from(e: serde_json::Error) -> Self {
        EventBusError::Serialization(e.to_string())
    }
}
