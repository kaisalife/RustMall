use crate::domain::{Order, OrderItem, OrderRepository, OrderStatus};
use common::{AppError, AppResult, SnowflakeIdGenerator};
use sqlx::{query, query_as, query_scalar, FromRow, PgPool};
use std::sync::Arc;

#[derive(Clone)]
pub struct OrderRepositoryImpl {
    pool: PgPool,
    id_generator: Arc<SnowflakeIdGenerator>,
}

impl OrderRepositoryImpl {
    pub fn new(pool: PgPool, id_generator: Arc<SnowflakeIdGenerator>) -> Self {
        Self { pool, id_generator }
    }
}

#[async_trait::async_trait]
impl OrderRepository for OrderRepositoryImpl {
    async fn create(&self, order: Order) -> AppResult<Order> {
        let mut tx = self.pool.begin().await?;

        // 创建订单
        let status_str = Self::status_to_string(&order.status);
        query(
            r#"
            INSERT INTO orders (id, user_id, total_amount, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4::order_status, $5, $6)
            "#,
        )
        .bind(order.id as i64)
        .bind(order.user_id as i64)
        .bind(order.total_amount)
        .bind(status_str)
        .bind(order.created_at)
        .bind(order.updated_at)
        .execute(&mut *tx)
        .await?;

        // 批量创建订单项（单条 SQL 替代逐条 INSERT）
        let item_ids: Vec<i64> = (0..order.items.len())
            .map(|_| self.id_generator.generate().map(|id| id as i64))
            .collect::<Result<_, _>>()
            .map_err(|e| AppError::internal(e))?;
        let product_ids: Vec<i64> = order.items.iter().map(|i| i.product_id as i64).collect();
        let quantities: Vec<i32> = order.items.iter().map(|i| i.quantity).collect();
        let unit_prices: Vec<f64> = order.items.iter().map(|i| i.unit_price).collect();

        query(
            r#"
            INSERT INTO order_items (id, order_id, product_id, quantity, unit_price)
            SELECT id, $2, product_id, quantity, unit_price
            FROM unnest($1::bigint[], $3::bigint[], $4::int[], $5::float8[])
            AS t(id, product_id, quantity, unit_price)
            "#,
        )
        .bind(&item_ids)
        .bind(order.id as i64)
        .bind(&product_ids)
        .bind(&quantities)
        .bind(&unit_prices)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(order)
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Order>> {
        let record = query_as::<_, OrderRecord>(
            r#"
            SELECT id, user_id, total_amount::DOUBLE PRECISION AS total_amount, status::TEXT AS status, created_at, updated_at
            FROM orders WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        let order = match record {
            Some(r) => {
                let items = query_as::<_, OrderItemRecord>(
                    r#"
                    SELECT order_id, product_id, quantity, unit_price::DOUBLE PRECISION AS unit_price
                    FROM order_items WHERE order_id = $1
                    "#,
                )
                .bind(id as i64)
                .fetch_all(&self.pool)
                .await?;

                Some(Order {
                    id: r.id as u64,
                    user_id: r.user_id as u64,
                    total_amount: r.total_amount,
                    status: Self::string_to_status(&r.status),
                    items: items.into_iter().map(|i| i.into_domain()).collect(),
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            }
            None => None,
        };

        Ok(order)
    }

    async fn update(&self, order: Order) -> AppResult<Order> {
        let status_str = Self::status_to_string(&order.status);
        query(
            r#"
            UPDATE orders SET status = $2::order_status, updated_at = $3 WHERE id = $1
            "#,
        )
        .bind(order.id as i64)
        .bind(status_str)
        .bind(order.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(order)
    }

    async fn list_by_user(
        &self,
        user_id: u64,
        page: i32,
        page_size: i32,
    ) -> AppResult<(Vec<Order>, i64)> {
        let offset = (page - 1) * page_size;

        let records = query_as::<_, OrderRecord>(
            r#"
            SELECT id, user_id, total_amount::DOUBLE PRECISION AS total_amount, status::TEXT AS status, created_at, updated_at
            FROM orders WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id as i64)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = query_scalar(
            r#"
            SELECT COUNT(*) FROM orders WHERE user_id = $1
            "#,
        )
        .bind(user_id as i64)
        .fetch_one(&self.pool)
        .await?;

        if records.is_empty() {
            return Ok((vec![], total));
        }

        // 批量查询所有订单项（替代逐单查询的 N+1）
        let order_ids: Vec<i64> = records.iter().map(|r| r.id).collect();
        let all_items = query_as::<_, OrderItemRecord>(
            r#"
            SELECT order_id, product_id, quantity, unit_price::DOUBLE PRECISION AS unit_price
            FROM order_items WHERE order_id = ANY($1)
            "#,
        )
        .bind(&order_ids)
        .fetch_all(&self.pool)
        .await?;

        // 按 order_id 分组
        use std::collections::HashMap;
        let mut items_map: HashMap<i64, Vec<OrderItem>> = HashMap::new();
        for item in all_items {
            items_map
                .entry(item.order_id)
                .or_default()
                .push(item.into_domain());
        }

        let orders = records
            .into_iter()
            .map(|r| Order {
                id: r.id as u64,
                user_id: r.user_id as u64,
                total_amount: r.total_amount,
                status: Self::string_to_status(&r.status),
                items: items_map.remove(&r.id).unwrap_or_default(),
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect();

        Ok((orders, total))
    }
}

impl OrderRepositoryImpl {
    fn status_to_string(status: &OrderStatus) -> String {
        match status {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Paid => "PAID",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
        }
        .to_string()
    }

    fn string_to_status(s: &str) -> OrderStatus {
        match s {
            "PENDING" => OrderStatus::Pending,
            "PAID" => OrderStatus::Paid,
            "SHIPPED" => OrderStatus::Shipped,
            "COMPLETED" => OrderStatus::Completed,
            "CANCELLED" => OrderStatus::Cancelled,
            _ => OrderStatus::Pending,
        }
    }
}

#[derive(FromRow, Clone, Debug)]
struct OrderRecord {
    id: i64,
    user_id: i64,
    total_amount: f64,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow, Clone, Debug)]
struct OrderItemRecord {
    order_id: i64,
    product_id: i64,
    quantity: i32,
    unit_price: f64,
}

impl OrderItemRecord {
    fn into_domain(self) -> OrderItem {
        OrderItem {
            product_id: self.product_id as u64,
            quantity: self.quantity,
            unit_price: self.unit_price,
        }
    }
}
