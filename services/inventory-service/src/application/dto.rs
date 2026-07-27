#[derive(Debug, Clone)]
pub struct InventoryDto {
    pub product_id: u64,
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub available_quantity: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct DeductStockResult {
    pub success: bool,
    pub remaining: i32,
}

#[derive(Debug, Clone)]
pub struct AddStockResult {
    pub success: bool,
    pub total: i32,
}
