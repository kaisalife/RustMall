pub mod audit;
pub mod auth;
pub mod auth_v2;
pub mod bench;
pub mod health;
pub mod inventory;
pub mod order;
pub mod product;

pub use audit::audit_routes;
pub use auth::{auth_routes, user_routes};
pub use auth_v2::auth_v2_routes;
pub use bench::{echo_handler, ping_handler};
pub use health::health_check_handler;
pub use inventory::inventory_routes;
pub use order::order_routes;
pub use product::product_routes;
