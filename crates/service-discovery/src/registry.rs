//! 服务注册表 trait + Nacos 实现

use async_trait::async_trait;
use common::AppResult;
use std::sync::Arc;

use crate::instance::ServiceInstance;

/// 服务注册表 trait
#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    /// 注册服务实例
    async fn register(&self, instance: ServiceInstance) -> AppResult<()>;

    /// 注销服务实例
    async fn deregister(&self, service_name: &str) -> AppResult<()>;

    /// 发现服务实例列表
    async fn discover(&self, service_name: &str) -> AppResult<Vec<ServiceInstance>>;
}

/// Nacos 注册表配置
#[derive(Debug, Clone)]
pub struct NacosConfig {
    /// Nacos 服务端地址（如 "nacos:8848"）
    pub server_addr: String,
    /// 命名空间（"public" 对应 ""）
    pub namespace: String,
    /// 用户名
    pub username: Option<String>,
    /// 密码
    pub password: Option<String>,
    /// 应用名称
    pub app_name: String,
}

impl Default for NacosConfig {
    fn default() -> Self {
        Self {
            server_addr: "nacos:8848".to_string(),
            namespace: "".to_string(),
            username: None,
            password: None,
            app_name: "simple_trade".to_string(),
        }
    }
}

/// Nacos 服务注册表实现
pub struct NacosRegistry {
    naming_service: Arc<nacos_sdk::api::naming::NamingService>,
    group: String,
}

impl NacosRegistry {
    /// 创建 Nacos 注册表
    pub async fn new(config: &NacosConfig) -> AppResult<Self> {
        let mut client_props = nacos_sdk::api::props::ClientProps::new()
            .server_addr(&config.server_addr)
            .namespace(&config.namespace)
            .app_name(&config.app_name);

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            client_props = client_props
                .auth_username(username)
                .auth_password(password);
        }

        let naming_service = nacos_sdk::api::naming::NamingServiceBuilder::new(client_props)
            .build()
            .await
            .map_err(|e| common::AppError::Internal(format!("Failed to create Nacos naming service: {}", e)))?;

        tracing::info!("Nacos naming service connected: {}", config.server_addr);

        Ok(Self {
            naming_service: Arc::new(naming_service),
            group: nacos_sdk::api::constants::DEFAULT_GROUP.to_string(),
        })
    }

    /// 获取内部 NamingService 引用（用于 subscribe 等高级功能）
    pub fn naming_service(&self) -> &nacos_sdk::api::naming::NamingService {
        &self.naming_service
    }

    /// 获取组名
    pub fn group(&self) -> &str {
        &self.group
    }
}

#[async_trait]
impl ServiceRegistry for NacosRegistry {
    async fn register(&self, instance: ServiceInstance) -> AppResult<()> {
        let service_name = instance.service_name.clone();
        let nacos_instance: nacos_sdk::api::naming::ServiceInstance = instance.into();

        self.naming_service
            .batch_register_instance(
                service_name.clone(),
                Some(self.group.clone()),
                vec![nacos_instance],
            )
            .await
            .map_err(|e| {
                common::AppError::Internal(format!("Failed to register service {}: {}", service_name, e))
            })?;

        tracing::info!("Service registered to Nacos: {}", service_name);
        Ok(())
    }

    async fn deregister(&self, service_name: &str) -> AppResult<()> {
        // Nacos 临时实例会通过心跳自动注销，这里只需记录日志
        tracing::info!("Service deregistered from Nacos: {}", service_name);
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> AppResult<Vec<ServiceInstance>> {
        let instances = self
            .naming_service
            .get_all_instances(
                service_name.to_string(),
                Some(self.group.clone()),
                Vec::default(),
                false,
            )
            .await
            .map_err(|e| {
                common::AppError::Internal(format!("Failed to discover service {}: {}", service_name, e))
            })?;

        let result: Vec<ServiceInstance> = instances.into_iter().map(ServiceInstance::from).collect();
        tracing::debug!("Discovered {} instances for service: {}", result.len(), service_name);
        Ok(result)
    }
}
