use std::sync::Arc;

use super::command::{
    AddStockCommand, DeductStockCommand, ReleaseStockCommand, ReserveStockCommand,
};
use super::dto::{AddStockResult, DeductStockResult, InventoryDto};
use crate::domain::{Inventory, InventoryRepository};
use common::{AppError, AppResult};

#[derive(Clone)]
pub struct InventoryApplicationService {
    inventory_repository: Arc<dyn InventoryRepository>,
}

impl InventoryApplicationService {
    pub fn new(inventory_repository: Arc<dyn InventoryRepository>) -> Self {
        Self {
            inventory_repository,
        }
    }

    pub async fn deduct_stock(&self, command: DeductStockCommand) -> AppResult<DeductStockResult> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        // 原子扣减预留库存（预扣减 -> 实际扣减）
        let inventory = self
            .inventory_repository
            .atomic_deduct_reserved(command.product_id, command.quantity)
            .await?
            .ok_or_else(|| AppError::invalid_input("Insufficient reserved stock"))?;

        Ok(DeductStockResult {
            success: true,
            remaining: inventory.available_quantity(),
        })
    }

    pub async fn add_stock(&self, command: AddStockCommand) -> AppResult<AddStockResult> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        let mut inventory = match self
            .inventory_repository
            .find_by_product_id(command.product_id)
            .await?
        {
            Some(inv) => inv,
            None => {
                let new_inv = Inventory::new(command.product_id, command.quantity);
                let created = self.inventory_repository.create(new_inv).await?;
                return Ok(AddStockResult {
                    success: true,
                    total: created.quantity,
                });
            }
        };

        inventory
            .add_stock(command.quantity)
            .map_err(AppError::invalid_input)?;
        self.inventory_repository.update(&inventory).await?;

        Ok(AddStockResult {
            success: true,
            total: inventory.quantity,
        })
    }

    pub async fn get_stock(&self, product_id: u64) -> AppResult<InventoryDto> {
        let inventory = self
            .inventory_repository
            .find_by_product_id(product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Inventory not found for product"))?;

        Ok(Self::inventory_to_dto(&inventory))
    }

    pub async fn batch_get_stock(&self, product_ids: Vec<u64>) -> AppResult<Vec<InventoryDto>> {
        let inventories = self
            .inventory_repository
            .batch_find_by_product_ids(product_ids)
            .await?;
        Ok(inventories
            .into_iter()
            .map(|inv| Self::inventory_to_dto(&inv))
            .collect())
    }

    /// 预扣减库存（原子操作，1 次 DB 往返，无重试）
    pub async fn reserve_stock(&self, command: ReserveStockCommand) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        let inventory = self
            .inventory_repository
            .atomic_reserve(command.product_id, command.quantity)
            .await?
            .ok_or_else(|| AppError::invalid_input("Insufficient available stock"))?;

        Ok(Self::inventory_to_dto(&inventory))
    }

    /// 释放预留库存（原子操作）
    pub async fn release_reserved_stock(
        &self,
        command: ReleaseStockCommand,
    ) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        let inventory = self
            .inventory_repository
            .atomic_release(command.product_id, command.quantity)
            .await?
            .ok_or_else(|| AppError::invalid_input("Insufficient reserved stock"))?;

        Ok(Self::inventory_to_dto(&inventory))
    }

    /// 批量预扣减库存（全成功或全回滚）
    /// 任一商品预扣减失败则回滚已成功的，返回错误
    pub async fn batch_reserve_stock(
        &self,
        items: Vec<(u64, i32)>,
    ) -> AppResult<Vec<InventoryDto>> {
        let mut reserved = Vec::with_capacity(items.len());
        for (product_id, quantity) in &items {
            match self
                .reserve_stock(ReserveStockCommand {
                    product_id: *product_id,
                    quantity: *quantity,
                })
                .await
            {
                Ok(dto) => reserved.push(dto),
                Err(e) => {
                    // 回滚已预扣减的
                    for dto in &reserved {
                        let _ = self
                            .release_reserved_stock(ReleaseStockCommand {
                                product_id: dto.product_id,
                                quantity: items
                                    .iter()
                                    .find(|(pid, _)| *pid == dto.product_id)
                                    .map(|(_, qty)| *qty)
                                    .unwrap_or(0),
                            })
                            .await;
                    }
                    return Err(AppError::invalid_input(format!(
                        "Insufficient stock for product {}: {}",
                        product_id, e
                    )));
                }
            }
        }
        Ok(reserved)
    }

    /// 批量释放预留库存（best-effort，失败只记日志）
    pub async fn batch_release_stock(&self, items: Vec<(u64, i32)>) {
        for (product_id, quantity) in items {
            if let Err(e) = self
                .release_reserved_stock(ReleaseStockCommand {
                    product_id,
                    quantity,
                })
                .await
            {
                tracing::error!("Failed to release stock for product {}: {}", product_id, e);
            }
        }
    }

    fn inventory_to_dto(inventory: &Inventory) -> InventoryDto {
        InventoryDto {
            product_id: inventory.product_id,
            quantity: inventory.quantity,
            reserved_quantity: inventory.reserved_quantity,
            updated_at: inventory.updated_at.to_rfc3339(),
        }
    }
}
