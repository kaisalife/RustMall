use chrono::{DateTime, Utc};
use common::AppError;

#[derive(Debug, Clone)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub category_id: u64,
    pub stock: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    pub fn new(
        id: u64,
        name: String,
        description: String,
        price: f64,
        category_id: u64,
        stock: i32,
    ) -> Result<Self, AppError> {
        // 域模型内部校验：价格必须大于 0
        if price <= 0.0 {
            return Err(AppError::invalid_input("Price must be greater than zero"));
        }
        // 域模型内部校验：库存不能为负
        if stock < 0 {
            return Err(AppError::invalid_input("Stock cannot be negative"));
        }

        let now = Utc::now();
        Ok(Self {
            id,
            name,
            description,
            price,
            category_id,
            stock,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn update_price(&mut self, price: f64) -> Result<(), AppError> {
        // 域模型内部校验：价格必须大于 0
        if price <= 0.0 {
            return Err(AppError::invalid_input("Price must be greater than zero"));
        }
        self.price = price;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_category(&mut self, category_id: u64) {
        self.category_id = category_id;
        self.updated_at = Utc::now();
    }
}
