//! Tower 中间件集合
//!
//! 提供 API 网关所需的中间件：
//! - JWT 认证中间件
//! - 限流中间件（令牌桶算法）
//! - 日志中间件
//! - CORS 配置
//! - 幂等性中间件（Redis 分布式锁）

pub mod auth;
pub mod rate_limit;
pub mod logger;
pub mod idempotency;
pub mod audit;

pub use auth::{
    create_auth_middleware,
    create_optional_auth_middleware,
    get_user_claims,
    JwtValidator,
    AuthError,
};
pub use rate_limit::{
    RateLimiter,
    create_rate_limit_middleware,
    create_default_rate_limiter,
    create_strict_rate_limiter,
    RateLimitError,
};
pub use logger::{
    logger_middleware,
    verbose_logger_middleware,
    create_cors_layer,
};
pub use idempotency::{
    idempotency_middleware,
    IDEMPOTENCY_HEADER,
};
pub use audit::{AuditLayer, AuditMiddleware};
