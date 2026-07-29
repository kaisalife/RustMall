use super::Order;
use common::AppResult;

#[async_trait::async_trait]
pub trait OrderRepository: Send + Sync + 'static {
    async fn create(&self, order: Order) -> AppResult<Order>;
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Order>>;
    async fn update(&self, order: Order) -> AppResult<Order>;
    async fn list_by_user(
        &self,
        user_id: u64,
        page: i32,
        page_size: i32,
    ) -> AppResult<(Vec<Order>, i64)>;
}
