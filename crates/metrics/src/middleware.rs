use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::body::Body;
use axum::http::{Request, Response};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

static HTTP_REQUESTS_TOTAL: OnceLock<prometheus::CounterVec> = OnceLock::new();
static HTTP_REQUEST_DURATION: OnceLock<prometheus::HistogramVec> = OnceLock::new();

fn http_requests_total() -> &'static prometheus::CounterVec {
    HTTP_REQUESTS_TOTAL.get_or_init(|| {
        prometheus::register_counter_vec!(
            "http_requests_total",
            "Total number of HTTP requests",
            &["method", "path", "status"]
        )
        .unwrap()
    })
}

fn http_request_duration() -> &'static prometheus::HistogramVec {
    HTTP_REQUEST_DURATION.get_or_init(|| {
        prometheus::register_histogram_vec!(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
            &["method", "path"],
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )
        .unwrap()
    })
}

/// 记录一次 HTTP 请求的指标
pub fn record_request(method: &str, path: &str, status: u16, duration: f64) {
    http_requests_total()
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    http_request_duration()
        .with_label_values(&[method, path])
        .observe(duration);
}

/// Prometheus 指标中间件层
///
/// 记录 HTTP 请求的 QPS、延迟、状态码分布。
#[derive(Clone)]
pub struct MetricsMiddleware;

impl MetricsMiddleware {
    pub fn new() -> Self {
        // 触发指标注册
        let _ = http_requests_total();
        let _ = http_request_duration();
        Self
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for MetricsMiddleware {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MetricsService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let start = Instant::now();

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let response = inner.call(req).await?;

            let duration = start.elapsed().as_secs_f64();
            let status = response.status().as_u16();

            record_request(method.as_str(), &path, status, duration);

            Ok(response)
        })
    }
}
