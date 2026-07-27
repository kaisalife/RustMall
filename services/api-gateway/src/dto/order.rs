use serde::{Deserialize, Serialize};

use super::product::{default_page, default_page_size};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderItemDto {
    pub product_id: u64,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    pub user_id: u64,
    pub items: Vec<OrderItemDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub status: i32,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub user_id: u64,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_page_size")]
    pub page_size: i32,
}

#[derive(Debug, Serialize)]
pub struct OrderDto {
    pub order_id: u64,
    pub user_id: u64,
    pub total_amount: f64,
    pub status: i32,
    pub items: Vec<OrderItemDto>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListOrdersResponseDto {
    pub orders: Vec<OrderDto>,
    pub total: i32,
    pub page: i32,
    pub page_size: i32,
}
