use sqlx::{query, query_as, query_scalar, FromRow, PgPool};

use crate::domain::{Category, CategoryRepository, Product, ProductRepository};
use common::{AppError, AppResult};

#[derive(Clone)]
pub struct ProductRepositoryImpl {
    pool: PgPool,
}

impl ProductRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ProductRepository for ProductRepositoryImpl {
    async fn create(&self, product: Product) -> AppResult<Product> {
        let record = query_as::<_, ProductRecord>(
            r#"
            INSERT INTO products (id, name, description, price, category_id, stock, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, description, price::DOUBLE PRECISION AS price, category_id, stock, created_at, updated_at
            "#,
        )
        .bind(product.id as i64)
        .bind(&product.name)
        .bind(&product.description)
        .bind(product.price)
        .bind(product.category_id as i64)
        .bind(product.stock)
        .bind(product.created_at)
        .bind(product.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Product>> {
        let record = query_as::<_, ProductRecord>(
            r#"
            SELECT id, name, description, price::DOUBLE PRECISION AS price, category_id, stock, created_at, updated_at
            FROM products
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn update(&self, product: Product) -> AppResult<Product> {
        let record = query_as::<_, ProductRecord>(
            r#"
            UPDATE products
            SET name = $2, description = $3, price = $4, category_id = $5, stock = $6, updated_at = $7
            WHERE id = $1
            RETURNING id, name, description, price::DOUBLE PRECISION AS price, category_id, stock, created_at, updated_at
            "#,
        )
        .bind(product.id as i64)
        .bind(&product.name)
        .bind(&product.description)
        .bind(product.price)
        .bind(product.category_id as i64)
        .bind(product.stock)
        .bind(product.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn delete(&self, id: u64) -> AppResult<()> {
        query(
            r#"
            DELETE FROM products WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list(
        &self,
        category_id: Option<u64>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        page: i32,
        page_size: i32,
    ) -> AppResult<(Vec<Product>, i64)> {
        let offset = (page - 1) * page_size;

        // 构建动态查询
        let mut sql = String::from(
            "SELECT id, name, description, price::DOUBLE PRECISION AS price, category_id, stock, created_at, updated_at FROM products WHERE 1=1"
        );
        let mut count_sql = String::from("SELECT COUNT(*) as count FROM products WHERE 1=1");

        let mut params: Vec<String> = Vec::new();
        let mut param_idx = 1;

        if let Some(cid) = category_id {
            let cond = format!(" AND category_id = ${}", param_idx);
            sql.push_str(&cond);
            count_sql.push_str(&cond);
            params.push(cid.to_string());
            param_idx += 1;
        }
        if let Some(min) = min_price {
            let cond = format!(" AND price >= ${}", param_idx);
            sql.push_str(&cond);
            count_sql.push_str(&cond);
            params.push(min.to_string());
            param_idx += 1;
        }
        if let Some(max) = max_price {
            let cond = format!(" AND price <= ${}", param_idx);
            sql.push_str(&cond);
            count_sql.push_str(&cond);
            params.push(max.to_string());
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY id LIMIT ${} OFFSET ${}",
            param_idx,
            param_idx + 1
        ));

        // 执行计数查询
        let mut count_query = query_scalar::<_, i64>(&count_sql);
        for p in &params {
            count_query = count_query.bind(p.parse::<i64>().unwrap_or(0));
        }
        let total = count_query.fetch_one(&self.pool).await?;

        // 执行主查询
        let mut product_query = query_as::<_, ProductRecord>(&sql);
        for p in &params {
            // 尝试解析为 f64，如果失败则解析为 i64
            if let Ok(f) = p.parse::<f64>() {
                product_query = product_query.bind(f);
            } else if let Ok(i) = p.parse::<i64>() {
                product_query = product_query.bind(i);
            }
        }
        product_query = product_query.bind(page_size as i64).bind(offset as i64);

        let records = product_query.fetch_all(&self.pool).await?;

        Ok((
            records.into_iter().map(|r| r.into_domain()).collect(),
            total,
        ))
    }
}

#[derive(Clone)]
pub struct CategoryRepositoryImpl {
    pool: PgPool,
}

impl CategoryRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CategoryRepository for CategoryRepositoryImpl {
    async fn create(&self, category: Category) -> AppResult<Category> {
        let record = query_as::<_, CategoryRecord>(
            r#"
            INSERT INTO categories (id, name, parent_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(category.id as i64)
        .bind(&category.name)
        .bind(category.parent_id.map(|id| id as i64))
        .bind(category.created_at)
        .bind(category.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<Category>> {
        let record = query_as::<_, CategoryRecord>(
            r#"
            SELECT id, name, parent_id, created_at, updated_at
            FROM categories
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn update(&self, category: Category) -> AppResult<Category> {
        let record = query_as::<_, CategoryRecord>(
            r#"
            UPDATE categories
            SET name = $2, parent_id = $3, updated_at = $4
            WHERE id = $1
            RETURNING id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(category.id as i64)
        .bind(&category.name)
        .bind(category.parent_id.map(|id| id as i64))
        .bind(category.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn delete(&self, id: u64) -> AppResult<()> {
        query(
            r#"
            DELETE FROM categories WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list(&self) -> AppResult<Vec<Category>> {
        let records = query_as::<_, CategoryRecord>(
            r#"
            SELECT id, name, parent_id, created_at, updated_at
            FROM categories
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records.into_iter().map(|r| r.into_domain()).collect())
    }
}

#[derive(FromRow, Clone, Debug)]
struct ProductRecord {
    id: i64,
    name: String,
    description: String,
    price: f64,
    category_id: i64,
    stock: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl ProductRecord {
    fn into_domain(self) -> Product {
        Product {
            id: self.id as u64,
            name: self.name,
            description: self.description,
            price: self.price,
            category_id: self.category_id as u64,
            stock: self.stock,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(FromRow, Clone, Debug)]
struct CategoryRecord {
    id: i64,
    name: String,
    parent_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl CategoryRecord {
    fn into_domain(self) -> Category {
        Category {
            id: self.id as u64,
            name: self.name,
            parent_id: self.parent_id.map(|id| id as u64),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
