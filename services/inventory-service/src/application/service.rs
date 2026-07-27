use std::sync::Arc;

use common::{AppResult, AppError};
use crate::domain::{Inventory, InventoryRepository};
use super::command::{DeductStockCommand, AddStockCommand, ReserveStockCommand, ReleaseStockCommand};
use super::dto::{InventoryDto, DeductStockResult, AddStockResult};

#[derive(Clone)]
pub struct InventoryApplicationService {
    inventory_repository: Arc<dyn InventoryRepository>,
}

impl InventoryApplicationService {
    pub fn new(inventory_repository: Arc<dyn InventoryRepository>) -> Self {
        Self { inventory_repository }
    }

    pub async fn deduct_stock(&self, command: DeductStockCommand) -> AppResult<DeductStockResult> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        // 使用原子操作防止超卖
        let updated = self.inventory_repository.atomic_deduct_stock(command.product_id, command.quantity).await?;

        Ok(DeductStockResult {
            success: true,
            remaining: updated.available_quantity(),
        })
    }

    pub async fn add_stock(&self, command: AddStockCommand) -> AppResult<AddStockResult> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        let inventory = match self.inventory_repository.find_by_product_id(command.product_id).await? {
            Some(mut inv) => {
                inv.add_stock(command.quantity).map_err(|e| AppError::invalid_input(e))?;
                self.inventory_repository.update(inv).await?
            }
            None => {
                let new_inv = Inventory::new(command.product_id, command.quantity);
                self.inventory_repository.create(new_inv).await?
            }
        };

        Ok(AddStockResult {
            success: true,
            total: inventory.quantity,
        })
    }

    pub async fn get_stock(&self, product_id: u64) -> AppResult<InventoryDto> {
        let inventory = self.inventory_repository.find_by_product_id(product_id).await?
            .ok_or_else(|| AppError::not_found("Inventory not found for product"))?;

        Ok(Self::inventory_to_dto(inventory))
    }

    pub async fn batch_get_stock(&self, product_ids: Vec<u64>) -> AppResult<Vec<InventoryDto>> {
        let inventories = self.inventory_repository.batch_find_by_product_ids(product_ids).await?;
        Ok(inventories.into_iter().map(Self::inventory_to_dto).collect())
    }

    pub async fn reserve_stock(&self, command: ReserveStockCommand) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        // 使用原子操作防止超卖
        let inventory = self.inventory_repository.atomic_reserve_stock(command.product_id, command.quantity).await?;

        Ok(Self::inventory_to_dto(inventory))
    }

    pub async fn release_reserved_stock(&self, command: ReleaseStockCommand) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        // 使用原子操作释放预留
        let inventory = self.inventory_repository.atomic_release_stock(command.product_id, command.quantity).await?;

        Ok(Self::inventory_to_dto(inventory))
    }

    /// 批量预留库存，返回每个商品的预留结果
    pub async fn batch_reserve_stock(&self, items: Vec<(u64, i32)>) -> Vec<(u64, AppResult<InventoryDto>)> {
        let mut results = Vec::with_capacity(items.len());
        for (product_id, quantity) in items {
            let result = self.reserve_stock(ReserveStockCommand { product_id, quantity }).await;
            results.push((product_id, result));
        }
        results
    }

    /// 批量释放预留库存（best-effort，失败只记日志）
    pub async fn batch_release_stock(&self, items: Vec<(u64, i32)>) -> Vec<(u64, AppResult<()>)> {
        let mut results = Vec::with_capacity(items.len());
        for (product_id, quantity) in items {
            let result = self.release_reserved_stock(ReleaseStockCommand { product_id, quantity })
                .await
                .map(|_| ());
            if let Err(ref e) = result {
                tracing::error!("Failed to release stock for product {}: {}", product_id, e);
            }
            results.push((product_id, result));
        }
        results
    }

    fn inventory_to_dto(inventory: Inventory) -> InventoryDto {
        InventoryDto {
            product_id: inventory.product_id,
            quantity: inventory.quantity,
            reserved_quantity: inventory.reserved_quantity,
            available_quantity: inventory.available_quantity(),
            updated_at: inventory.updated_at.to_rfc3339(),
        }
    }
}
