use super::Inventory;
use common::AppResult;

#[async_trait::async_trait]
pub trait InventoryRepository: Send + Sync + 'static {
    async fn create(&self, inventory: Inventory) -> AppResult<Inventory>;
    async fn find_by_product_id(&self, product_id: u64) -> AppResult<Option<Inventory>>;

    /// Update inventory with optimistic locking.
    /// Returns Ok(()) if successful, Err(AppError::Conflict) if version mismatch.
    async fn update(&self, inventory: &Inventory) -> AppResult<()>;

    async fn batch_find_by_product_ids(&self, product_ids: Vec<u64>) -> AppResult<Vec<Inventory>>;
}
