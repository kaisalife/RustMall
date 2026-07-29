//! 服务实例

use std::collections::HashMap;

/// 服务实例信息
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    /// 服务名称（如 "auth-service"）
    pub service_name: String,
    /// IP 地址
    pub ip: String,
    /// 端口
    pub port: u16,
    /// 元数据（版本、权重等）
    pub metadata: HashMap<String, String>,
}

impl ServiceInstance {
    pub fn new(service_name: &str, ip: &str, port: u16) -> Self {
        Self {
            service_name: service_name.to_string(),
            ip: ip.to_string(),
            port,
            metadata: HashMap::new(),
        }
    }

    /// 构建 Nacos 地址字符串
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

/// 从 Nacos ServiceInstance 转换
impl From<nacos_sdk::api::naming::ServiceInstance> for ServiceInstance {
    fn from(nacos_instance: nacos_sdk::api::naming::ServiceInstance) -> Self {
        Self {
            service_name: nacos_instance.service_name.unwrap_or_default(),
            ip: nacos_instance.ip,
            port: nacos_instance.port as u16,
            metadata: nacos_instance.metadata,
        }
    }
}

/// 转换为 Nacos ServiceInstance
impl From<ServiceInstance> for nacos_sdk::api::naming::ServiceInstance {
    fn from(instance: ServiceInstance) -> Self {
        nacos_sdk::api::naming::ServiceInstance {
            ip: instance.ip,
            port: instance.port as i32,
            service_name: Some(instance.service_name),
            metadata: instance.metadata,
            ..Default::default()
        }
    }
}
