use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub gateway: GatewayConfig,
    pub auth_service: ServiceConfig,
    pub product_service: ServiceConfig,
    pub order_service: ServiceConfig,
    pub inventory_service: ServiceConfig,
    pub email_service: ServiceConfig,
    pub payment_service: PaymentServiceConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub email: EmailConfig,
    pub kafka: KafkaConfig,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub nacos: NacosConfigSection,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    /// gRPC 调用超时（秒），网关->下游服务的请求超时
    #[serde(default = "default_grpc_timeout")]
    pub grpc_timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub worker_id: u64,
    /// 注册到服务发现的可访问地址（Docker 中为服务名，如 "auth-service"）
    /// 未设置时使用 host
    #[serde(default)]
    pub advertise_host: Option<String>,
}

impl ServiceConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// 获取注册到服务发现的可访问地址
    pub fn advertise_address(&self) -> String {
        let host = self.advertise_host.as_ref().unwrap_or(&self.host);
        format!("{}:{}", host, self.port)
    }

    /// 获取注册到服务发现的可访问 IP
    pub fn advertise_ip(&self) -> &str {
        self.advertise_host.as_ref().unwrap_or(&self.host)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_minutes: u64,
    pub max_lifetime_minutes: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
    pub refresh_expiration_hours: i64,
    /// bcrypt cost 因子（4-31），默认 12，压测/开发可降至 10
    #[serde(default = "default_bcrypt_cost")]
    pub bcrypt_cost: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TracingConfig {
    pub otlp_endpoint: Option<String>,
    pub jaeger_endpoint: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub whitelist_ips: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            whitelist_ips: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PaymentServiceConfig {
    pub host: String,
    pub port: u16,
    pub worker_id: u64,
    #[serde(default)]
    pub advertise_host: Option<String>,
}

impl PaymentServiceConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn advertise_ip(&self) -> &str {
        self.advertise_host.as_ref().unwrap_or(&self.host)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic_prefix: String,
    pub consumer_group: String,
}

pub fn load_config() -> AppResult<AppConfig> {
    let figment = Figment::new()
        .merge(Toml::file("config/base.toml"))
        .merge(Env::prefixed("APP_").split("__"));

    let config: AppConfig = figment
        .extract()
        .map_err(|e| AppError::Config(format!("Failed to load config: {}", e)))?;

    // 安全校验：如果 JWT secret 是默认值，在非 debug 模式下警告
    if config.jwt.secret.contains("change-this-in-production") && !cfg!(debug_assertions) {
        tracing::warn!(
            "JWT secret appears to be the default value. Please change it in production!"
        );
    }

    Ok(config)
}

fn default_grpc_timeout() -> u64 {
    10
}

fn default_bcrypt_cost() -> u32 {
    12
}

/// Nacos 服务注册与发现配置
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct NacosConfigSection {
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
    /// 是否启用服务注册（开发环境可关闭）
    pub enabled: bool,
}

impl Default for NacosConfigSection {
    fn default() -> Self {
        Self {
            server_addr: "nacos:8848".to_string(),
            namespace: "".to_string(),
            username: None,
            password: None,
            app_name: "simple_trade".to_string(),
            enabled: false,
        }
    }
}
