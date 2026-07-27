use chrono::{DateTime, Utc};

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
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description,
            price,
            category_id,
            stock,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn update_price(&mut self, price: f64) {
        self.price = price;
        self.updated_at = Utc::now();
    }

    pub fn update_category(&mut self, category_id: u64) {
        self.category_id = category_id;
        self.updated_at = Utc::now();
    }

    pub fn add_stock(&mut self, quantity: i32) {
        self.stock += quantity;
        self.updated_at = Utc::now();
    }

    pub fn deduct_stock(&mut self, quantity: i32) -> Result<(), &'static str> {
        if self.stock < quantity {
            return Err("Insufficient stock");
        }
        self.stock -= quantity;
        self.updated_at = Utc::now();
        Ok(())
    }
}
