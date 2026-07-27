use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct OrderItem {
    pub product_id: u64,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub user_id: u64,
    pub total_amount: f64,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    pub fn new(id: u64, user_id: u64, items: Vec<OrderItem>) -> Self {
        let total_amount = items.iter().map(|item| item.unit_price * item.quantity as f64).sum();
        Self {
            id,
            user_id,
            total_amount,
            status: OrderStatus::Pending,
            items,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn mark_as_paid(&mut self) {
        self.status = OrderStatus::Paid;
        self.updated_at = Utc::now();
    }

    pub fn mark_as_shipped(&mut self) {
        self.status = OrderStatus::Shipped;
        self.updated_at = Utc::now();
    }

    pub fn mark_as_completed(&mut self) {
        self.status = OrderStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) -> Result<(), &'static str> {
        match self.status {
            OrderStatus::Pending | OrderStatus::Paid => {
                self.status = OrderStatus::Cancelled;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err("Cannot cancel order in current status"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<OrderItem> {
        vec![
            OrderItem {
                product_id: 1,
                quantity: 2,
                unit_price: 10.0,
            },
            OrderItem {
                product_id: 2,
                quantity: 3,
                unit_price: 20.0,
            },
        ]
    }

    #[test]
    fn test_new_order() {
        let order = Order::new(1, 100, sample_items());
        // total = 2*10 + 3*20 = 80
        assert_eq!(order.total_amount, 80.0);
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_new_order_empty_items() {
        let order = Order::new(1, 100, vec![]);
        assert_eq!(order.total_amount, 0.0);
        assert_eq!(order.items.len(), 0);
    }

    #[test]
    fn test_mark_as_paid() {
        let mut order = Order::new(1, 100, sample_items());
        assert_eq!(order.status, OrderStatus::Pending);
        order.mark_as_paid();
        assert_eq!(order.status, OrderStatus::Paid);
    }

    #[test]
    fn test_mark_as_shipped() {
        let mut order = Order::new(1, 100, sample_items());
        order.mark_as_paid();
        order.mark_as_shipped();
        assert_eq!(order.status, OrderStatus::Shipped);
    }

    #[test]
    fn test_mark_as_completed() {
        let mut order = Order::new(1, 100, sample_items());
        order.mark_as_paid();
        order.mark_as_shipped();
        order.mark_as_completed();
        assert_eq!(order.status, OrderStatus::Completed);
    }

    #[test]
    fn test_cancel_from_pending() {
        let mut order = Order::new(1, 100, sample_items());
        assert!(order.cancel().is_ok());
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_cancel_from_paid() {
        let mut order = Order::new(1, 100, sample_items());
        order.mark_as_paid();
        assert!(order.cancel().is_ok());
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_cancel_from_shipped() {
        let mut order = Order::new(1, 100, sample_items());
        order.mark_as_paid();
        order.mark_as_shipped();
        assert!(order.cancel().is_err());
        assert_eq!(order.status, OrderStatus::Shipped);
    }

    #[test]
    fn test_cancel_from_completed() {
        let mut order = Order::new(1, 100, sample_items());
        order.mark_as_paid();
        order.mark_as_shipped();
        order.mark_as_completed();
        assert!(order.cancel().is_err());
        assert_eq!(order.status, OrderStatus::Completed);
    }
}
