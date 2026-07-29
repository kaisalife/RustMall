use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Inventory {
    pub product_id: u64,
    pub quantity: i32,
    pub reserved_quantity: i32,
    /// 乐观锁版本号，与数据库 version 列对应
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

impl Inventory {
    pub fn new(product_id: u64, quantity: i32) -> Self {
        Self {
            product_id,
            quantity,
            reserved_quantity: 0,
            version: 0,
            updated_at: Utc::now(),
        }
    }

    /// 增加库存（仅 add_stock 应用方法使用）
    pub fn add_stock(&mut self, quantity: i32) -> Result<(), &'static str> {
        if quantity < 0 {
            return Err("Quantity cannot be negative");
        }
        self.quantity += quantity;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 可用库存 = 总库存 - 已预留
    pub fn available_quantity(&self) -> i32 {
        self.quantity - self.reserved_quantity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_inventory() {
        let inv = Inventory::new(1, 100);
        assert_eq!(inv.product_id, 1);
        assert_eq!(inv.quantity, 100);
        assert_eq!(inv.reserved_quantity, 0);
        assert_eq!(inv.available_quantity(), 100);
    }

    #[test]
    fn test_add_stock() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.add_stock(50).is_ok());
        assert_eq!(inv.quantity, 150);
    }

    #[test]
    fn test_add_stock_negative() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.add_stock(-10).is_err());
        assert_eq!(inv.quantity, 100);
    }

    #[test]
    fn test_available_quantity() {
        let inv = Inventory::new(1, 100);
        assert_eq!(inv.available_quantity(), 100);
    }
}
