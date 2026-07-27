use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeductStockRequest {
    pub quantity: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddStockRequest {
    pub quantity: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockDto {
    pub product_id: u64,
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct DeductStockResponseDto {
    pub success: bool,
    pub remaining: i32,
}

#[derive(Debug, Serialize)]
pub struct AddStockResponseDto {
    pub success: bool,
    pub total: i32,
}
