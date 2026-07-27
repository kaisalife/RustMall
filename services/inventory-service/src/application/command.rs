#[derive(Debug, Clone)]
pub struct DeductStockCommand {
    pub product_id: u64,
    pub quantity: i32,
}

#[derive(Debug, Clone)]
pub struct AddStockCommand {
    pub product_id: u64,
    pub quantity: i32,
}

#[derive(Debug, Clone)]
pub struct ReserveStockCommand {
    pub product_id: u64,
    pub quantity: i32,
}

#[derive(Debug, Clone)]
pub struct ReleaseStockCommand {
    pub product_id: u64,
    pub quantity: i32,
}
