pub mod auth;
pub mod product;
pub mod order;
pub mod inventory;
pub mod health;
pub mod bench;

pub use auth::{auth_routes, user_routes};
pub use product::product_routes;
pub use order::order_routes;
pub use inventory::inventory_routes;
pub use health::health_check_handler;
pub use bench::{ping_handler, echo_handler};
