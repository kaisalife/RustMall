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

    /// 原子预扣减：reserved_quantity += qty，校验可用量
    /// 返回更新后的 Inventory，库存不足返回 None
    async fn atomic_reserve(&self, product_id: u64, quantity: i32) -> AppResult<Option<Inventory>>;

    /// 原子释放预留：reserved_quantity -= qty，校验已预留量
    async fn atomic_release(&self, product_id: u64, quantity: i32) -> AppResult<Option<Inventory>>;

    /// 原子扣减预留：quantity -= qty, reserved_quantity -= qty
    /// 用于订单支付成功后将预扣减转为实际扣减
    async fn atomic_deduct_reserved(
        &self,
        product_id: u64,
        quantity: i32,
    ) -> AppResult<Option<Inventory>>;
}
