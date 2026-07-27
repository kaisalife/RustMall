use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Inventory {
    pub product_id: u64,
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub updated_at: DateTime<Utc>,
}

impl Inventory {
    pub fn new(product_id: u64, quantity: i32) -> Self {
        Self {
            product_id,
            quantity,
            reserved_quantity: 0,
            updated_at: Utc::now(),
        }
    }

    pub fn add_stock(&mut self, quantity: i32) -> Result<(), &'static str> {
        if quantity < 0 {
            return Err("Quantity cannot be negative");
        }
        self.quantity += quantity;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn deduct_stock(&mut self, quantity: i32) -> Result<(), &'static str> {
        if quantity < 0 {
            return Err("Quantity cannot be negative");
        }
        let available = self.quantity - self.reserved_quantity;
        if available < quantity {
            return Err("Insufficient stock");
        }
        self.quantity -= quantity;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn reserve_stock(&mut self, quantity: i32) -> Result<(), &'static str> {
        if quantity < 0 {
            return Err("Quantity cannot be negative");
        }
        let available = self.quantity - self.reserved_quantity;
        if available < quantity {
            return Err("Insufficient available stock");
        }
        self.reserved_quantity += quantity;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn release_reserved(&mut self, quantity: i32) -> Result<(), &'static str> {
        if quantity < 0 {
            return Err("Quantity cannot be negative");
        }
        if self.reserved_quantity < quantity {
            return Err("Not enough reserved stock");
        }
        self.reserved_quantity -= quantity;
        self.updated_at = Utc::now();
        Ok(())
    }

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
    fn test_deduct_stock_success() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.deduct_stock(30).is_ok());
        assert_eq!(inv.quantity, 70);
    }

    #[test]
    fn test_deduct_stock_insufficient() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.deduct_stock(150).is_err());
        assert_eq!(inv.quantity, 100);
    }

    #[test]
    fn test_deduct_stock_negative() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.deduct_stock(-10).is_err());
        assert_eq!(inv.quantity, 100);
    }

    #[test]
    fn test_reserve_stock_success() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.reserve_stock(20).is_ok());
        assert_eq!(inv.reserved_quantity, 20);
        assert_eq!(inv.quantity, 100);
    }

    #[test]
    fn test_reserve_stock_insufficient() {
        let mut inv = Inventory::new(1, 100);
        assert!(inv.reserve_stock(150).is_err());
        assert_eq!(inv.reserved_quantity, 0);
    }

    #[test]
    fn test_release_reserved_success() {
        let mut inv = Inventory::new(1, 100);
        inv.reserve_stock(20).unwrap();
        assert!(inv.release_reserved(10).is_ok());
        assert_eq!(inv.reserved_quantity, 10);
    }

    #[test]
    fn test_release_reserved_too_much() {
        let mut inv = Inventory::new(1, 100);
        inv.reserve_stock(20).unwrap();
        assert!(inv.release_reserved(30).is_err());
        assert_eq!(inv.reserved_quantity, 20);
    }

    #[test]
    fn test_available_quantity() {
        let mut inv = Inventory::new(1, 100);
        assert_eq!(inv.available_quantity(), 100);
        inv.reserve_stock(30).unwrap();
        assert_eq!(inv.available_quantity(), 70);
    }

    #[test]
    fn test_deduct_with_reserved() {
        let mut inv = Inventory::new(1, 100);
        inv.reserve_stock(20).unwrap();
        // available = 100 - 20 = 80
        // deduct 90 should fail (90 > 80)
        assert!(inv.deduct_stock(90).is_err());
        assert_eq!(inv.quantity, 100);
        // deduct 80 should succeed (80 <= 80)
        assert!(inv.deduct_stock(80).is_ok());
        assert_eq!(inv.quantity, 20);
    }
}
