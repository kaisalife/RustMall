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

        const MAX_RETRIES: usize = 3;
        for attempt in 0..MAX_RETRIES {
            let mut inventory = self
                .inventory_repository
                .find_by_product_id(command.product_id)
                .await?
                .ok_or_else(|| AppError::not_found("Inventory not found for product"))?;

            inventory
                .deduct_stock(command.quantity)
                .map_err(AppError::invalid_input)?;

            match self.inventory_repository.update(&inventory).await {
                Ok(()) => {
                    inventory.increment_version();
                    return Ok(DeductStockResult {
                        success: true,
                        remaining: inventory.available_quantity(),
                    });
                }
                Err(AppError::Conflict(_)) if attempt < MAX_RETRIES - 1 => continue,
                Err(e) => return Err(e),
            }
        }
        Err(AppError::conflict(
            "Inventory deduct concurrency conflict, retries exhausted",
        ))
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
        inventory.increment_version();

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

    pub async fn reserve_stock(&self, command: ReserveStockCommand) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        const MAX_RETRIES: usize = 3;
        for attempt in 0..MAX_RETRIES {
            let mut inventory = self
                .inventory_repository
                .find_by_product_id(command.product_id)
                .await?
                .ok_or_else(|| AppError::not_found("Inventory not found for product"))?;

            inventory
                .reserve_stock(command.quantity)
                .map_err(AppError::invalid_input)?;

            match self.inventory_repository.update(&inventory).await {
                Ok(()) => {
                    inventory.increment_version();
                    return Ok(Self::inventory_to_dto(&inventory));
                }
                Err(AppError::Conflict(_)) if attempt < MAX_RETRIES - 1 => continue,
                Err(e) => return Err(e),
            }
        }
        Err(AppError::conflict(
            "Inventory reserve concurrency conflict, retries exhausted",
        ))
    }

    pub async fn release_reserved_stock(
        &self,
        command: ReleaseStockCommand,
    ) -> AppResult<InventoryDto> {
        if command.quantity <= 0 {
            return Err(AppError::invalid_input("Quantity must be positive"));
        }

        const MAX_RETRIES: usize = 3;
        for attempt in 0..MAX_RETRIES {
            let mut inventory = self
                .inventory_repository
                .find_by_product_id(command.product_id)
                .await?
                .ok_or_else(|| AppError::not_found("Inventory not found for product"))?;

            inventory
                .release_reserved(command.quantity)
                .map_err(AppError::invalid_input)?;

            match self.inventory_repository.update(&inventory).await {
                Ok(()) => {
                    inventory.increment_version();
                    return Ok(Self::inventory_to_dto(&inventory));
                }
                Err(AppError::Conflict(_)) if attempt < MAX_RETRIES - 1 => continue,
                Err(e) => return Err(e),
            }
        }
        Err(AppError::conflict(
            "Inventory release concurrency conflict, retries exhausted",
        ))
    }

    /// 批量预留库存，返回每个商品的预留结果
    pub async fn batch_reserve_stock(
        &self,
        items: Vec<(u64, i32)>,
    ) -> Vec<(u64, AppResult<InventoryDto>)> {
        let mut results = Vec::with_capacity(items.len());
        for (product_id, quantity) in items {
            let result = self
                .reserve_stock(ReserveStockCommand {
                    product_id,
                    quantity,
                })
                .await;
            results.push((product_id, result));
        }
        results
    }

    /// 批量释放预留库存（best-effort，失败只记日志）
    pub async fn batch_release_stock(&self, items: Vec<(u64, i32)>) -> Vec<(u64, AppResult<()>)> {
        let mut results = Vec::with_capacity(items.len());
        for (product_id, quantity) in items {
            let result = self
                .release_reserved_stock(ReleaseStockCommand {
                    product_id,
                    quantity,
                })
                .await
                .map(|_| ());
            if let Err(ref e) = result {
                tracing::error!("Failed to release stock for product {}: {}", product_id, e);
            }
            results.push((product_id, result));
        }
        results
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
