#[derive(Debug, Clone)]
pub struct OrderDto {
    pub order_id: u64,
    pub user_id: u64,
    pub total_amount: f64,
    pub status: String,
    pub items: Vec<OrderItemDto>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct OrderItemDto {
    pub product_id: u64,
    pub quantity: i32,
    pub unit_price: f64,
}
