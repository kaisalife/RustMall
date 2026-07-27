//! Kafka Consumer（长连接 + 消息流）
//!
//! 基于 rdkafka 的 `StreamConsumer`，消费者组模式。
//!
//! ## 长连接机制
//!
//! - **消费者组**：多个消费者实例共享 group.id，自动 rebalance 分区
//! - **心跳保活**：后台线程定期发送心跳（`session.timeout.ms`）
//! - **自动提交**：定期提交消费 offset（`auto.commit.interval.ms`）
//! - **自动重连**：broker 断连后自动重试
//!
//! ## 使用方式
//!
//! ```ignore
//! let consumer = EventBusConsumer::new(brokers, "order-group")?;
//! consumer.subscribe(&["simple_trade.payment_succeeded"])?;
//!
//! let mut stream = consumer.stream();
//! while let Some(message) = stream.next().await {
//!     let event: EventEnvelope = serde_json::from_slice(&message.payload)?;
//!     // 处理事件...
//! }
//! ```

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;

use common::{AppError, AppResult};

use super::event::EventEnvelope;

/// 事件总线 Consumer
///
/// 封装 rdkafka StreamConsumer，提供长连接 + 消息流能力。
/// 消费者组模式，支持自动 rebalance。
pub struct EventBusConsumer {
    /// rdkafka 流式 consumer（内部维护长连接）
    consumer: StreamConsumer,
}

impl EventBusConsumer {
    /// 创建 Consumer 并建立长连接
    ///
    /// `brokers`: Kafka broker 地址，如 "kafka:9092"
    /// `group_id`: 消费者组 ID，同组的消费者自动 rebalance 分区
    pub fn new(brokers: &str, group_id: &str) -> AppResult<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            // 长连接保活配置
            .set("session.timeout.ms", "30000")       // 心跳超时 30s
            .set("heartbeat.interval.ms", "10000")    // 心跳间隔 10s
            .set("connections.max.idle.ms", "540000") // 空闲超时 9 分钟
            // 消费者组配置
            .set("auto.offset.reset", "latest")       // 新消费者从最新消息开始
            .set("enable.auto.commit", "true")        // 自动提交 offset
            .set("auto.commit.interval.ms", "5000")   // 每 5s 提交一次
            // 读取配置
            .set("fetch.min.bytes", "1")              // 最少拉取 1 字节
            .set("fetch.max.wait.ms", "500")          // 最大等待 500ms
            // 分区分配策略
            .set("partition.assignment.strategy", "roundrobin") // 轮询分配
            .create()
            .map_err(|e| AppError::internal(format!("创建 Kafka consumer 失败: {}", e)))?;

        Ok(Self { consumer })
    }

    /// 订阅 topic
    ///
    /// 订阅后通过 `stream()` 获取消息流。
    /// 消费者组内自动 rebalance 分区分配。
    pub fn subscribe(&self, topics: &[&str]) -> AppResult<()> {
        self.consumer
            .subscribe(topics)
            .map_err(|e| AppError::internal(format!("Kafka 订阅失败: {}", e)))?;

        tracing::info!(topics = ?topics, "Kafka consumer 已订阅");
        Ok(())
    }

    /// 获取消息流
    ///
    /// 返回一个异步 Stream，每次 yield 一条 Kafka 消息。
    /// 消息的 payload 是 JSON 字符串，需要反序列化为 `EventEnvelope`。
    ///
    /// ```ignore
    /// use tokio_stream::StreamExt;
    ///
    /// let mut stream = consumer.stream();
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(msg) => {
    ///             let payload = msg.payload().unwrap_or(b"");
    ///             let event: EventEnvelope = serde_json::from_slice(payload)?;
    ///             // 处理事件...
    ///         }
    ///         Err(e) => {
    ///             tracing::error!("Kafka 消费错误: {}", e);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn stream(&self) -> impl tokio_stream::Stream<Item = Result<rdkafka::message::BorrowedMessage<'_>, rdkafka::error::KafkaError>> {
        self.consumer.stream()
    }

    /// 手动提交 offset（关闭 auto.commit 时使用）
    ///
    /// `partition`/`offset`: 提交到指定位置
    pub async fn commit_offset(
        &self,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> AppResult<()> {
        use rdkafka::TopicPartitionList;

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(offset))
            .map_err(|e| AppError::internal(format!("设置 offset 失败: {}", e)))?;

        self.consumer
            .commit(&tpl, rdkafka::consumer::CommitMode::Async)
            .map_err(|e| AppError::internal(format!("提交 offset 失败: {}", e)))?;

        Ok(())
    }

    /// 获取内部 consumer 引用（用于高级操作）
    pub fn inner(&self) -> &StreamConsumer {
        &self.consumer
    }
}

/// 消费消息并反序列化为 EventEnvelope 的辅助函数
///
/// ```ignore
/// let mut stream = consumer.stream();
/// while let Some(result) = stream.next().await {
///     if let Ok(msg) = result {
///         if let Ok(event) = parse_event(&msg) {
///             // 处理事件...
///         }
///     }
/// }
/// ```
pub fn parse_event(msg: &rdkafka::message::BorrowedMessage<'_>) -> AppResult<EventEnvelope> {
    let payload = msg.payload().unwrap_or(b"[]");
    let envelope: EventEnvelope = serde_json::from_slice(payload)
        .map_err(|e| AppError::internal(format!("事件反序列化失败: {}", e)))?;
    Ok(envelope)
}
