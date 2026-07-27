//! 事件类型定义
//!
//! 所有跨服务事件在此定义，用 enum 统一管理。
//! 事件通过 serde 序列化为 JSON 发送到 Kafka。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 订单商品项（事件负载用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemEvent {
    pub product_id: u64,
    pub quantity: i32,
    pub unit_price_cents: i64,
}

/// 事件类型枚举
///
/// 每个变体对应一个 Kafka topic：`{prefix}.{snake_case_name}`
/// 例如 `simple_trade.payment_succeeded`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum EventPayload {
    /// 支付成功
    PaymentSucceeded {
        payment_id: u64,
        order_id: u64,
        user_id: u64,
        amount_cents: i64,
        currency: String,
        channel: String,
    },
    /// 支付失败
    PaymentFailed {
        payment_id: u64,
        order_id: u64,
        reason: String,
    },
    /// 退款发起
    RefundInitiated {
        refund_id: u64,
        payment_id: u64,
        amount_cents: i64,
    },
    /// 退款完成
    RefundCompleted {
        refund_id: u64,
        payment_id: u64,
        amount_cents: i64,
    },
    /// 订单创建
    OrderCreated {
        order_id: u64,
        user_id: u64,
        total_amount_cents: i64,
        /// 订单商品项（inventory-service 消费后据此扣减库存）
        items: Vec<OrderItemEvent>,
    },
    /// 库存扣减
    InventoryDeducted {
        product_id: u64,
        quantity: i64,
    },
}

impl EventPayload {
    /// 获取事件对应的 topic 名称
    ///
    /// 格式：`{prefix}.{event_name}`
    pub fn topic_name(&self, prefix: &str) -> String {
        let name = match self {
            EventPayload::PaymentSucceeded { .. } => "payment_succeeded",
            EventPayload::PaymentFailed { .. } => "payment_failed",
            EventPayload::RefundInitiated { .. } => "refund_initiated",
            EventPayload::RefundCompleted { .. } => "refund_completed",
            EventPayload::OrderCreated { .. } => "order_created",
            EventPayload::InventoryDeducted { .. } => "inventory_deducted",
        };
        format!("{}.{}", prefix, name)
    }
}

/// 事件信封（包装事件 + 元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// 事件唯一 ID（雪花算法生成）
    pub event_id: u64,
    /// 事件类型
    pub event_type: EventType,
    /// 事件发生时间
    pub timestamp: DateTime<Utc>,
    /// 事件来源服务
    pub source: String,
    /// 事件负载数据
    pub payload: EventPayload,
}

/// 事件类型字符串标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PaymentSucceeded,
    PaymentFailed,
    RefundInitiated,
    RefundCompleted,
    OrderCreated,
    InventoryDeducted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_name() {
        let event = EventPayload::PaymentSucceeded {
            payment_id: 1,
            order_id: 2,
            user_id: 3,
            amount_cents: 9999,
            currency: "CNY".to_string(),
            channel: "WeChat".to_string(),
        };
        assert_eq!(event.topic_name("simple_trade"), "simple_trade.payment_succeeded");
    }

    #[test]
    fn test_event_serialization() {
        let event = EventPayload::PaymentFailed {
            payment_id: 1,
            order_id: 2,
            reason: "Insufficient balance".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("payment_failed") || json.contains("PaymentFailed"));
    }

    #[test]
    fn test_event_envelope_serialization() {
        let envelope = EventEnvelope {
            event_id: 123456789,
            event_type: EventType::PaymentSucceeded,
            timestamp: Utc::now(),
            source: "payment-service".to_string(),
            payload: EventPayload::PaymentSucceeded {
                payment_id: 1,
                order_id: 2,
                user_id: 3,
                amount_cents: 9999,
                currency: "CNY".to_string(),
                channel: "WeChat".to_string(),
            },
        };
        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.event_id, 123456789);
    }
}
