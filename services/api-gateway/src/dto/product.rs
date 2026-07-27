use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category_id: u64,
    pub stock: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub category_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    pub category_id: Option<u64>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_page_size")]
    pub page_size: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductDto {
    pub product_id: u64,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category_id: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteProductResponseDto {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ListProductsResponseDto {
    pub products: Vec<ProductDto>,
    pub total: i32,
    pub page: i32,
    pub page_size: i32,
}

pub fn default_page() -> i32 {
    1
}

pub fn default_page_size() -> i32 {
    10
}
