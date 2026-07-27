use sqlx::PgPool;

use crate::domain::{Inventory, InventoryRepository};
use common::{AppResult, AppError};

#[derive(Clone)]
pub struct InventoryRepositoryImpl {
    pool: PgPool,
}

impl InventoryRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl InventoryRepository for InventoryRepositoryImpl {
    async fn create(&self, inventory: Inventory) -> AppResult<Inventory> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            INSERT INTO inventory (product_id, quantity, reserved_quantity, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (product_id) DO UPDATE SET
                quantity = inventory.quantity + EXCLUDED.quantity,
                updated_at = EXCLUDED.updated_at
            RETURNING product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(inventory.product_id as i64)
        .bind(inventory.quantity)
        .bind(inventory.reserved_quantity)
        .bind(inventory.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn find_by_product_id(&self, product_id: u64) -> AppResult<Option<Inventory>> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            SELECT product_id, quantity, reserved_quantity, updated_at
            FROM inventory
            WHERE product_id = $1
            "#,
        )
        .bind(product_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn update(&self, inventory: Inventory) -> AppResult<Inventory> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            UPDATE inventory
            SET quantity = $2, reserved_quantity = $3, updated_at = $4
            WHERE product_id = $1
            RETURNING product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(inventory.product_id as i64)
        .bind(inventory.quantity)
        .bind(inventory.reserved_quantity)
        .bind(inventory.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn batch_find_by_product_ids(&self, product_ids: Vec<u64>) -> AppResult<Vec<Inventory>> {
        let ids: Vec<i64> = product_ids.into_iter().map(|id| id as i64).collect();

        let records = sqlx::query_as::<_, InventoryRecord>(
            r#"
            SELECT product_id, quantity, reserved_quantity, updated_at
            FROM inventory
            WHERE product_id = ANY($1)
            "#,
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(records.into_iter().map(|r| r.into_domain()).collect())
    }

    async fn atomic_deduct_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            UPDATE inventory
            SET quantity = quantity - $2, updated_at = NOW()
            WHERE product_id = $1 AND (quantity - reserved_quantity) >= $2
            RETURNING product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(product_id as i64)
        .bind(quantity)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::invalid_input("Insufficient stock or product not found"))?;

        Ok(record.into_domain())
    }

    async fn atomic_reserve_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            UPDATE inventory
            SET reserved_quantity = reserved_quantity + $2, updated_at = NOW()
            WHERE product_id = $1 AND (quantity - reserved_quantity) >= $2
            RETURNING product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(product_id as i64)
        .bind(quantity)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::invalid_input("Insufficient available stock or product not found"))?;

        Ok(record.into_domain())
    }

    async fn atomic_release_stock(&self, product_id: u64, quantity: i32) -> AppResult<Inventory> {
        let record = sqlx::query_as::<_, InventoryRecord>(
            r#"
            UPDATE inventory
            SET reserved_quantity = reserved_quantity - $2, updated_at = NOW()
            WHERE product_id = $1 AND reserved_quantity >= $2
            RETURNING product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(product_id as i64)
        .bind(quantity)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::invalid_input("Not enough reserved stock or product not found"))?;

        Ok(record.into_domain())
    }
}

#[derive(sqlx::FromRow)]
struct InventoryRecord {
    product_id: i64,
    quantity: i32,
    reserved_quantity: i32,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl InventoryRecord {
    fn into_domain(self) -> Inventory {
        Inventory {
            product_id: self.product_id as u64,
            quantity: self.quantity,
            reserved_quantity: self.reserved_quantity,
            updated_at: self.updated_at,
        }
    }
}
