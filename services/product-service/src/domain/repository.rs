use common::AppResult;

use super::{Category, Product};

#[async_trait::async_trait]
pub trait ProductRepository: Send + Sync + 'static {
    async fn create(&self, product: Product) -> AppResult<Product>;
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Product>>;
    async fn update(&self, product: Product) -> AppResult<Product>;
    async fn delete(&self, id: u64) -> AppResult<()>;
    async fn list(
        &self,
        category_id: Option<u64>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        page: i32,
        page_size: i32,
    ) -> AppResult<(Vec<Product>, i64)>;
}

#[async_trait::async_trait]
pub trait CategoryRepository: Send + Sync + 'static {
    async fn create(&self, category: Category) -> AppResult<Category>;
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Category>>;
    async fn update(&self, category: Category) -> AppResult<Category>;
    async fn delete(&self, id: u64) -> AppResult<()>;
    async fn list(&self) -> AppResult<Vec<Category>>;
}
