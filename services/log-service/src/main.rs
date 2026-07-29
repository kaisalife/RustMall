//! Log Service - 分布式日志消费服务
//!
//! 消费 Kafka 中的 app_log 和 audit_log 事件，批量写入 PostgreSQL。
//! 每 2 秒或累积 100 条（ whichever comes first）执行一次批量写入。

mod repository;

use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

use common::load_config;
use event_bus::consumer::parse_event;
use event_bus::{EventEnvelope, EventPayload};

/// 批量写入的阈值
const BATCH_SIZE: usize = 100;
/// 最大刷新间隔
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() {
    let config = load_config().expect("Failed to load config");

    common::init_tracing(
        "log-service",
        config.tracing.otlp_endpoint.as_deref(),
        "log_service=info",
    );

    tracing::info!("========================================");
    tracing::info!("  Simple Trade - Log Service");
    tracing::info!("========================================");

    // 初始化数据库（连接 + 迁移）
    let pool = db_migration::setup_database(&config.database)
        .await
        .expect("Failed to setup database");
    tracing::info!("Database connected, migrations applied");

    // 初始化 Kafka 消费者
    let app_log_topic = format!("{}.app_log", config.kafka.topic_prefix);
    let audit_log_topic = format!("{}.audit_log", config.kafka.topic_prefix);
    let topics = vec![app_log_topic.as_str(), audit_log_topic.as_str()];

    let consumer = match event_bus::EventBusConsumer::new(&config.kafka.brokers, "log-service") {
        Ok(c) => {
            if let Err(e) = c.subscribe(&topics) {
                tracing::error!("Failed to subscribe to topics {:?}: {}", topics, e);
                panic!("Kafka subscription failed");
            }
            tracing::info!("Kafka consumer subscribed to: {:?}", topics);
            c
        }
        Err(e) => {
            tracing::error!("Failed to create Kafka consumer: {}", e);
            panic!("Kafka consumer init failed");
        }
    };

    tracing::info!("Log Service started, consuming events...");

    // 批量缓冲区
    let mut app_log_batch: Vec<EventEnvelope> = Vec::with_capacity(BATCH_SIZE);
    let mut audit_log_batch: Vec<EventEnvelope> = Vec::with_capacity(BATCH_SIZE);
    let mut last_flush = Instant::now();

    let mut stream = consumer.stream();

    loop {
        // 带超时的接收，确保定期刷新
        tokio::select! {
            result = stream.next() => {
                match result {
                    Some(Ok(msg)) => {
                        if let Ok(envelope) = parse_event(&msg) {
                            match &envelope.payload {
                                EventPayload::AppLog { .. } => {
                                    app_log_batch.push(envelope);
                                }
                                EventPayload::AuditLog { .. } => {
                                    audit_log_batch.push(envelope);
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("Kafka consume error: {}", e);
                    }
                    None => {
                        tracing::warn!("Kafka stream ended, reconnecting...");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                // 超时检查
            }
        }

        // 检查是否需要刷新
        let should_flush_size =
            app_log_batch.len() >= BATCH_SIZE || audit_log_batch.len() >= BATCH_SIZE;
        let should_flush_time = last_flush.elapsed() >= FLUSH_INTERVAL
            && (!app_log_batch.is_empty() || !audit_log_batch.is_empty());

        if should_flush_size || should_flush_time {
            flush(&pool, &mut app_log_batch, &mut audit_log_batch).await;
            last_flush = Instant::now();
        }
    }
}

/// 批量刷新日志到 PostgreSQL
async fn flush(
    pool: &sqlx::PgPool,
    app_logs: &mut Vec<EventEnvelope>,
    audit_logs: &mut Vec<EventEnvelope>,
) {
    let app_count = app_logs.len();
    let audit_count = audit_logs.len();

    if !app_logs.is_empty() {
        if let Err(e) = repository::batch_insert_app_logs(pool, app_logs).await {
            tracing::error!("Failed to flush {} app_logs: {}", app_count, e);
        }
        app_logs.clear();
    }

    if !audit_logs.is_empty() {
        if let Err(e) = repository::batch_insert_audit_logs(pool, audit_logs).await {
            tracing::error!("Failed to flush {} audit_logs: {}", audit_count, e);
        }
        audit_logs.clear();
    }

    if app_count > 0 || audit_count > 0 {
        tracing::info!("Flushed {} app_logs, {} audit_logs", app_count, audit_count);
    }
}
