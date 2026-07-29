use super::Inventory;
use common::AppResult;

#[async_trait::async_trait]
pub trait InventoryRepository: Send + Sync + 'static {
    async fn create(&self, inventory: Inventory) -> AppResult<Inventory>;
    async fn find_by_product_id(&self, product_id: u64) -> AppResult<Option<Inventory>>;
    async fn update(&self, inventory: Inventory) -> AppResult<Inventory>;
    async fn batch_find_by_product_ids(&self, product_ids: Vec<u64>) -> AppResult<Vec<Inventory>>;

    /// 原子扣减库存（防止超卖）
    /// 使用 `UPDATE ... SET quantity = quantity - $2 WHERE quantity - reserved_quantity >= $2`
    async fn atomic_deduct_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory>;

    /// 原子预留库存
    async fn atomic_reserve_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory>;

    /// 原子释放预留库存
    async fn atomic_release_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory>;
}
