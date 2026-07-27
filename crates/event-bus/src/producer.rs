//! Kafka Producer（长连接 + 异步发送）
//!
//! 基于 rdkafka 的 `FutureProducer`，内部维护到 broker 的长连接。
//!
//! ## 长连接机制
//!
//! rdkafka 内部维护连接池，具备以下特性：
//! - **自动重连**：broker 断连后自动重试（`reconnect.backoff.ms`）
//! - **心跳保活**：后台线程定期发送心跳（`session.timeout.ms`）
//! - **元数据刷新**：定期拉取集群元数据，自动发现新 broker/分区
//! - **批量推送**：内部缓冲消息，按 `batch.num.messages` / `linger.ms` 批量发送
//!
//! ## 使用方式
//!
//! ```ignore
//! let producer = EventBusProducer::new(brokers, "payment-service").await?;
//! producer.publish(event).await?;
//! ```

use std::sync::Arc;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::util::Timeout;

use common::{AppError, AppResult, SnowflakeIdGenerator};

use super::event::{EventEnvelope, EventPayload};

/// 事件总线 Producer
///
/// 封装 rdkafka FutureProducer，提供长连接 + 异步发送能力。
/// 通过 `Arc<EventBusProducer>` 共享，内部连接由 rdkafka 管理。
#[derive(Clone)]
pub struct EventBusProducer {
    /// rdkafka 异步 producer（内部维护长连接）
    producer: FutureProducer,
    /// topic 前缀（如 "simple_trade"）
    topic_prefix: String,
    /// 事件来源服务名
    source: String,
    /// 雪花 ID 生成器（生成 event_id）
    id_generator: Arc<SnowflakeIdGenerator>,
}

impl EventBusProducer {
    /// 创建 Producer 并建立长连接
    ///
    /// `brokers`: Kafka broker 地址，如 "kafka:9092"
    /// `source`: 事件来源服务名，如 "payment-service"
    /// `topic_prefix`: topic 前缀，如 "simple_trade"
    /// `id_generator`: 雪花 ID 生成器（Arc 共享）
    pub fn new(
        brokers: &str,
        source: &str,
        topic_prefix: &str,
        id_generator: Arc<SnowflakeIdGenerator>,
    ) -> AppResult<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            // 长连接保活配置
            .set("connections.max.idle.ms", "540000") // 9分钟（小于 broker 的 10 分钟默认值）
            .set("reconnect.backoff.ms", "1000")     // 重连初始退避
            .set("reconnect.backoff.max.ms", "10000") // 重连最大退避
            // 批量发送配置（吞吐优化）
            .set("linger.ms", "10")                   // 等待 10ms 攒批
            .set("batch.num.messages", "10000")       // 单批最大消息数
            .set("batch.size", "1048576")             // 单批最大字节数（1MB）
            // 可靠性配置
            .set("enable.idempotence", "true")        // 幂等 producer（防重复）
            .set("acks", "all")                       // 等待所有副本确认
            .set("message.timeout.ms", "30000")       // 发送超时 30s
            // 日志级别
            .set("log.connection.close", "false")     // 不记录连接关闭日志（减少噪音）
            .create()
            .map_err(|e| AppError::internal(format!("创建 Kafka producer 失败: {}", e)))?;

        Ok(Self {
            producer,
            topic_prefix: topic_prefix.to_string(),
            source: source.to_string(),
            id_generator,
        })
    }

    /// 发布事件（异步）
    ///
    /// 自动完成：
    /// 1. 生成 event_id（雪花算法）
    /// 2. 包装为 EventEnvelope（含元数据）
    /// 3. 序列化为 JSON
    /// 4. 发送到对应 topic
    /// 5. 等待 broker 确认（ack = all）
    pub async fn publish(&self, payload: EventPayload) -> AppResult<()> {
        let topic = payload.topic_name(&self.topic_prefix);
        let event_id = self.id_generator.generate()?;

        let envelope = EventEnvelope {
            event_id,
            event_type: match &payload {
                EventPayload::PaymentSucceeded { .. } => super::EventType::PaymentSucceeded,
                EventPayload::PaymentFailed { .. } => super::EventType::PaymentFailed,
                EventPayload::RefundInitiated { .. } => super::EventType::RefundInitiated,
                EventPayload::RefundCompleted { .. } => super::EventType::RefundCompleted,
                EventPayload::OrderCreated { .. } => super::EventType::OrderCreated,
                EventPayload::InventoryDeducted { .. } => super::EventType::InventoryDeducted,
                EventPayload::AppLog { .. } => super::EventType::AppLog,
                EventPayload::AuditLog { .. } => super::EventType::AuditLog,
            },
            timestamp: chrono::Utc::now(),
            source: self.source.clone(),
            payload,
        };

        let json = serde_json::to_string(&envelope)
            .map_err(|e| AppError::internal(format!("事件序列化失败: {}", e)))?;

        tracing::debug!(topic = %topic, event_id = event_id, "发布事件");

        // 发送并等待确认（timeout 30s）
        let delivery_result = self
            .producer
            .send(
                FutureRecord::to(&topic)
                    .payload(&json)
                    .key(&event_id.to_string()),
                Timeout::After(Duration::from_secs(30)),
            )
            .await;

        match delivery_result {
            Ok((partition, offset)) => {
                tracing::info!(
                    topic = %topic,
                    partition = partition,
                    offset = offset,
                    event_id = event_id,
                    "事件发布成功"
                );
                Ok(())
            }
            Err((e, _)) => {
                tracing::error!(topic = %topic, event_id = event_id, error = ?e, "事件发布失败");
                Err(AppError::internal(format!("Kafka 发送失败: {}", e)))
            }
        }
    }

    /// 获取内部 producer 引用（用于 flush 等高级操作）
    pub fn inner(&self) -> &FutureProducer {
        &self.producer
    }

    /// 刷新所有缓冲消息（优雅关闭时调用）
    ///
    /// 阻塞等待所有未确认消息发送完毕。
    pub fn flush(&self, timeout: Duration) {
        let _ = self.producer.flush(Timeout::After(timeout));
    }
}
