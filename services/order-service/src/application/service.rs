use crate::domain::{Order, OrderItem, OrderRepository, OrderStatus};
use common::{AppError, AppResult, SnowflakeIdGenerator};
use std::sync::Arc;

use super::dto::{OrderDto, OrderItemDto};

use proto::inventory::v1::{inventory_service_client::InventoryServiceClient, StockItem};
use tonic::transport::Channel;

#[derive(Clone)]
pub struct OrderApplicationService {
    order_repository: Arc<dyn OrderRepository>,
    id_generator: Arc<SnowflakeIdGenerator>,
    event_producer: Option<event_bus::EventBusProducer>,
    inventory_client: Option<InventoryServiceClient<Channel>>,
}

impl OrderApplicationService {
    pub fn new(
        order_repository: Arc<dyn OrderRepository>,
        id_generator: Arc<SnowflakeIdGenerator>,
    ) -> Self {
        Self {
            order_repository,
            id_generator,
            event_producer: None,
            inventory_client: None,
        }
    }

    pub fn with_event_producer(mut self, producer: event_bus::EventBusProducer) -> Self {
        self.event_producer = Some(producer);
        self
    }

    pub fn with_inventory_client(mut self, client: InventoryServiceClient<Channel>) -> Self {
        self.inventory_client = Some(client);
        self
    }

    pub async fn create_order(
        &self,
        user_id: u64,
        items: Vec<OrderItemDto>,
    ) -> AppResult<OrderDto> {
        if items.is_empty() {
            return Err(AppError::invalid_input("Order must have at least one item"));
        }

        // 同步预扣减库存（防超卖）
        if let Some(client) = &self.inventory_client {
            let reserve_items: Vec<StockItem> = items
                .iter()
                .map(|i| StockItem {
                    product_id: i.product_id,
                    quantity: i.quantity,
                })
                .collect();

            let response = client
                .clone()
                .batch_reserve_stock(proto::inventory::v1::BatchReserveStockRequest {
                    items: reserve_items,
                })
                .await
                .map_err(|e| AppError::internal(format!("Inventory service error: {}", e)))?;

            let resp = response.into_inner();
            if !resp.all_success {
                // 预扣减失败（库存不足），application 层已自动回滚
                let errors: Vec<_> = resp
                    .results
                    .iter()
                    .filter(|r| !r.success)
                    .map(|r| format!("product {}: {}", r.product_id, r.error))
                    .collect();
                return Err(AppError::invalid_input(format!(
                    "Insufficient stock: {}",
                    errors.join("; ")
                )));
            }
        }

        let order_items: Vec<OrderItem> = items
            .into_iter()
            .map(|item| OrderItem {
                product_id: item.product_id,
                quantity: item.quantity,
                unit_price: item.unit_price,
            })
            .collect();

        let order_id = self.id_generator.generate().map_err(AppError::internal)?;
        let order = Order::new(order_id, user_id, order_items);
        let saved_order = self.order_repository.create(order).await?;

        // 发布 OrderCreated 事件（inventory-service 异步消费，将预扣减转为实际扣减）
        if let Some(ref producer) = self.event_producer {
            let event_items: Vec<event_bus::OrderItemEvent> = saved_order
                .items
                .iter()
                .map(|i| event_bus::OrderItemEvent {
                    product_id: i.product_id,
                    quantity: i.quantity,
                    unit_price_cents: (i.unit_price * 100.0) as i64,
                })
                .collect();
            let event = event_bus::EventPayload::OrderCreated {
                order_id: saved_order.id,
                user_id: saved_order.user_id,
                total_amount_cents: (saved_order.total_amount * 100.0) as i64,
                items: event_items,
            };
            if let Err(e) = producer.publish(event).await {
                tracing::error!("Failed to publish OrderCreated event: {}", e);
                // 事件发送失败不阻塞订单创建（库存已预扣减，可通过补偿机制回滚）
            }
        }

        Ok(Self::order_to_dto(saved_order))
    }

    pub async fn get_order(&self, order_id: u64) -> AppResult<OrderDto> {
        let order = self
            .order_repository
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| AppError::not_found("Order not found"))?;

        Ok(Self::order_to_dto(order))
    }

    pub async fn list_orders(
        &self,
        user_id: u64,
        page: i32,
        page_size: i32,
    ) -> AppResult<(Vec<OrderDto>, i32)> {
        let (orders, total) = self
            .order_repository
            .list_by_user(user_id, page, page_size)
            .await?;

        let order_dtos = orders.into_iter().map(Self::order_to_dto).collect();

        Ok((order_dtos, total as i32))
    }

    pub async fn update_order_status(&self, order_id: u64, status: String) -> AppResult<OrderDto> {
        let mut order = self
            .order_repository
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| AppError::not_found("Order not found"))?;

        match status.as_str() {
            "PAID" => order.mark_as_paid(),
            "SHIPPED" => order.mark_as_shipped(),
            "COMPLETED" => order.mark_as_completed(),
            "CANCELLED" => {
                // 释放预扣减库存
                if let Some(client) = &self.inventory_client {
                    let release_items: Vec<proto::inventory::v1::StockItem> = order
                        .items
                        .iter()
                        .map(|i| proto::inventory::v1::StockItem {
                            product_id: i.product_id,
                            quantity: i.quantity,
                        })
                        .collect();
                    if let Err(e) = client
                        .clone()
                        .batch_release_stock(proto::inventory::v1::BatchReleaseStockRequest {
                            items: release_items,
                        })
                        .await
                    {
                        tracing::error!("Failed to release stock on cancel: {}", e);
                    }
                }
                order.cancel().map_err(AppError::invalid_input)?
            }
            _ => return Err(AppError::invalid_input("Invalid order status")),
        }

        let updated_order = self.order_repository.update(order).await?;

        Ok(Self::order_to_dto(updated_order))
    }

    fn order_to_dto(order: Order) -> OrderDto {
        let status = match order.status {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Paid => "PAID",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
        }
        .to_string();

        OrderDto {
            order_id: order.id,
            user_id: order.user_id,
            total_amount: order.total_amount,
            status,
            items: order
                .items
                .into_iter()
                .map(|i| OrderItemDto {
                    product_id: i.product_id,
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
            created_at: order.created_at.to_rfc3339(),
            updated_at: order.updated_at.to_rfc3339(),
        }
    }
}
