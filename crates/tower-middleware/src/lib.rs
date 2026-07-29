//! Tower 中间件集合
//!
//! 提供 API 网关所需的中间件：
//! - JWT 认证中间件
//! - 限流中间件（令牌桶算法）
//! - 日志中间件
//! - CORS 配置
//! - 幂等性中间件（Redis 分布式锁）

pub mod audit;
pub mod auth;
pub mod grpc_trace;
pub mod idempotency;
pub mod logger;
pub mod rate_limit;

pub use audit::{AuditLayer, AuditMiddleware};
pub use auth::{
    create_auth_middleware, create_optional_auth_middleware, get_user_claims, AuthError,
    JwtValidator,
};
pub use grpc_trace::{TraceContextExtractor, TraceContextInjector, TracedChannel};
pub use idempotency::{idempotency_middleware, IDEMPOTENCY_HEADER};
pub use logger::{create_cors_layer, logger_middleware, verbose_logger_middleware};
pub use rate_limit::{
    create_default_rate_limiter, create_rate_limit_middleware, create_strict_rate_limiter,
    RateLimitError, RateLimiter,
};
