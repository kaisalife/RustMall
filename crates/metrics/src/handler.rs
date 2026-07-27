use axum::body::Body;
use axum::response::Response;
use http::StatusCode;
use prometheus::Encoder;

/// Prometheus 指标暴露端点
///
/// 返回 Prometheus 文本格式的指标数据。
pub async fn metrics_handler() -> Response<Body> {
    let metric_families = prometheus::gather();
    let mut buffer = String::new();
    let encoder = prometheus::TextEncoder::new();
    encoder.encode_utf8(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}
