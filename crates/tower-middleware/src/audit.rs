//! 审计日志中间件
//!
//! 自动捕获 HTTP 请求的审计信息（method, path, user_id, IP, status），
//! 通过 Kafka 发布审计日志，由 log-service 消费写入 PostgreSQL。
//!
//! 使用方式（在 gateway main.rs）:
//! ```ignore
//! use tower_middleware::audit::AuditLayer;
//! // producer: Option<EventBusProducer>
//! let app = Router::new()
//!     ...
//!     .layer(AuditLayer::new(producer));
//! ```

use axum::{body::Body, extract::Request, http::Method, response::Response};
use common::request_context::RequestId;

/// 审计中间件层
#[derive(Clone)]
pub struct AuditLayer {
    producer: Option<event_bus::EventBusProducer>,
}

impl AuditLayer {
    pub fn new(producer: Option<event_bus::EventBusProducer>) -> Self {
        Self { producer }
    }
}

impl<S> tower::Layer<S> for AuditLayer {
    type Service = AuditMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuditMiddleware {
            inner,
            producer: self.producer.clone(),
        }
    }
}

/// 审计中间件服务
#[derive(Clone)]
pub struct AuditMiddleware<S> {
    inner: S,
    producer: Option<event_bus::EventBusProducer>,
}

impl<S> tower::Service<Request<Body>> for AuditMiddleware<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let producer = self.producer.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let method = request.method().clone();
            let path = request.uri().path().to_string();
            let headers = request.headers().clone();

            // 从请求扩展中获取 request_id（logger 中间件注入）
            let request_id = request
                .extensions()
                .get::<RequestId>()
                .map(|rid| rid.0.clone());

            // 只审计写操作
            let should_audit = matches!(
                method,
                Method::POST | Method::PUT | Method::DELETE | Method::PATCH
            );

            // 提取请求信息
            let user_id =
                extract_user_id(headers.get("authorization").and_then(|v| v.to_str().ok()));
            let ip_address = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
                .or_else(|| {
                    headers
                        .get("x-real-ip")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string())
                });
            let user_agent = headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // 执行请求
            let response = inner.call(request).await?;

            // 异步发布审计日志
            if should_audit {
                if let Some(ref producer) = producer {
                    let status = response.status();
                    let status_str = if status.is_success() {
                        "success"
                    } else {
                        "failure"
                    };
                    let action = format!("{} {}", method, path);
                    let resource_type = path.split('/').nth(2).map(|s| s.to_string());

                    let audit_event = event_bus::EventPayload::AuditLog {
                        user_id,
                        action,
                        resource_type,
                        resource_id: None,
                        request_id,
                        ip_address,
                        user_agent,
                        details: serde_json::json!({
                            "method": method.as_str(),
                            "path": path,
                            "status_code": status.as_u16(),
                        }),
                        status: status_str.to_string(),
                        error_message: if status.is_success() {
                            None
                        } else {
                            Some(format!("HTTP {}", status.as_u16()))
                        },
                    };

                    // 非阻塞发布，失败只记日志
                    if let Err(e) = producer.publish(audit_event).await {
                        tracing::warn!("Failed to publish audit log: {}", e);
                    }
                }
            }

            Ok(response)
        })
    }
}

/// 从 Authorization header 解析 user_id（不验证签名，仅解码 JWT payload）
fn extract_user_id(auth_header: Option<&str>) -> Option<u64> {
    use base64::Engine;
    let token = auth_header?.strip_prefix("Bearer ")?;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    // JWT payload 是 base64url 编码的第二段
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims.get("sub")?.as_str()?.parse().ok()
}
