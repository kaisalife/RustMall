//! Service Discovery crate
//!
//! 提供服务注册与发现能力，基于 Nacos。
//!
//! # 使用方式
//!
//! ## 服务注册（后端服务）
//! ```ignore
//! use service_discovery::{NacosRegistry, NacosConfig, ServiceInstance, ServiceRegistry};
//!
//! let config = NacosConfig::default();
//! let registry = NacosRegistry::new(&config).await?;
//! registry.register(ServiceInstance::new("auth-service", "0.0.0.0", 50051)).await?;
//! ```
//!
//! ## 服务发现（网关）
//! ```ignore
//! let instances = registry.discover("auth-service").await?;
//! // instances: Vec<ServiceInstance>，用 tonic Channel::balance_list 创建负载均衡连接
//! ```

pub mod instance;
pub mod registry;

pub use instance::ServiceInstance;
pub use registry::{ServiceRegistry, NacosRegistry, NacosConfig};
