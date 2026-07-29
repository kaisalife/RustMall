use std::sync::Arc;

use common::{AppError, AppResult, SnowflakeIdGenerator};

use crate::domain::{Category, CategoryRepository, Product, ProductRepository};

use super::command::{CreateProductCommand, ListProductsQuery, UpdateProductCommand};
use super::dto::{CategoryDto, ProductDto, ProductListDto};

#[derive(Clone)]
pub struct ProductApplicationService {
    product_repository: Arc<dyn ProductRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    id_generator: Arc<SnowflakeIdGenerator>,
}

impl ProductApplicationService {
    pub fn new(
        product_repository: Arc<dyn ProductRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        id_generator: Arc<SnowflakeIdGenerator>,
    ) -> Self {
        Self {
            product_repository,
            category_repository,
            id_generator,
        }
    }

    pub async fn create_product(&self, command: CreateProductCommand) -> AppResult<ProductDto> {
        // 验证分类是否存在
        if self
            .category_repository
            .find_by_id(command.category_id)
            .await?
            .is_none()
        {
            return Err(AppError::not_found("Category not found"));
        }

        // 生成产品ID
        let product_id = self.id_generator.generate().map_err(AppError::internal)?;

        // 创建产品实体（价格与库存校验由域模型内部完成）
        let product = Product::new(
            product_id,
            command.name,
            command.description,
            command.price,
            command.category_id,
            command.stock,
        )?;

        // 保存产品
        let saved_product = self.product_repository.create(product).await?;

        Ok(Self::product_to_dto(saved_product))
    }

    pub async fn get_product(&self, product_id: u64) -> AppResult<ProductDto> {
        let product = self
            .product_repository
            .find_by_id(product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product not found"))?;

        Ok(Self::product_to_dto(product))
    }

    pub async fn update_product(&self, command: UpdateProductCommand) -> AppResult<ProductDto> {
        let mut product = self
            .product_repository
            .find_by_id(command.product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product not found"))?;

        // 更新字段
        if let Some(name) = command.name {
            product.update_name(name);
        }
        if let Some(description) = command.description {
            product.update_description(description);
        }
        if let Some(price) = command.price {
            // 价格校验由域模型内部完成
            product.update_price(price)?;
        }
        if let Some(category_id) = command.category_id {
            if self
                .category_repository
                .find_by_id(category_id)
                .await?
                .is_none()
            {
                return Err(AppError::not_found("Category not found"));
            }
            product.update_category(category_id);
        }

        // 保存更新
        let updated_product = self.product_repository.update(product).await?;

        Ok(Self::product_to_dto(updated_product))
    }

    pub async fn delete_product(&self, product_id: u64) -> AppResult<bool> {
        self.product_repository.delete(product_id).await?;
        Ok(true)
    }

    pub async fn list_products(&self, query: ListProductsQuery) -> AppResult<ProductListDto> {
        let (products, total) = self
            .product_repository
            .list(
                query.category_id,
                query.min_price,
                query.max_price,
                query.page,
                query.page_size,
            )
            .await?;

        let product_dtos = products.into_iter().map(Self::product_to_dto).collect();

        Ok(ProductListDto {
            products: product_dtos,
            total: total as i32,
            page: query.page,
            page_size: query.page_size,
        })
    }

    pub async fn create_category(
        &self,
        name: String,
        parent_id: Option<u64>,
    ) -> AppResult<CategoryDto> {
        let category_id = self.id_generator.generate().map_err(AppError::internal)?;

        let category = Category::new(category_id, name, parent_id);
        let saved_category = self.category_repository.create(category).await?;

        Ok(Self::category_to_dto(saved_category))
    }

    pub async fn list_categories(&self) -> AppResult<Vec<CategoryDto>> {
        let categories = self.category_repository.list().await?;
        Ok(categories.into_iter().map(Self::category_to_dto).collect())
    }

    pub async fn update_category(
        &self,
        id: u64,
        name: Option<String>,
        parent_id: Option<Option<u64>>,
    ) -> AppResult<()> {
        let mut category = self
            .category_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("Category not found"))?;
        if let Some(n) = name {
            category.update_name(n);
        }
        if let Some(p) = parent_id {
            category.update_parent(p);
        }
        self.category_repository.update(category).await?;
        Ok(())
    }

    fn product_to_dto(product: Product) -> ProductDto {
        ProductDto {
            product_id: product.id,
            name: product.name,
            description: product.description,
            price: product.price,
            category_id: product.category_id,
            stock: product.stock,
            created_at: product.created_at.to_rfc3339(),
            updated_at: product.updated_at.to_rfc3339(),
        }
    }

    fn category_to_dto(category: Category) -> CategoryDto {
        CategoryDto {
            category_id: category.id,
            name: category.name,
            parent_id: category.parent_id,
            created_at: category.created_at.to_rfc3339(),
            updated_at: category.updated_at.to_rfc3339(),
        }
    }
}
