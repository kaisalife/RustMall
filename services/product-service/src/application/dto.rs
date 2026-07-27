#[derive(Debug, Clone)]
pub struct ProductDto {
    pub product_id: u64,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category_id: u64,
    pub stock: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProductListDto {
    pub products: Vec<ProductDto>,
    pub total: i32,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone)]
pub struct CategoryDto {
    pub category_id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
}
