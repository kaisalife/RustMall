#[derive(Debug, Clone)]
pub struct CreateProductCommand {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category_id: u64,
    pub stock: i32,
}

#[derive(Debug, Clone)]
pub struct UpdateProductCommand {
    pub product_id: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub category_id: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ListProductsQuery {
    pub category_id: Option<u64>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub page: i32,
    pub page_size: i32,
}
