use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化分布式追踪
///
/// 如果 `otlp_endpoint` 为 `None` 或连接失败，则只使用本地 tracing。
pub fn init_tracing(service_name: &str, otlp_endpoint: Option<&str>, env_filter: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    // 注册 W3C TraceContext propagator，使 gRPC/HTTP 跨服务 trace context 可传播。
    // 否则全局默认为 NoopTextMapPropagator，inject/extract 均不生效。
    opentelemetry::global::set_text_map_propagator(
        opentelemetry::sdk::propagation::TraceContextPropagator::new(),
    );

    // 尝试初始化 OpenTelemetry
    let otlp_layer = otlp_endpoint.and_then(|endpoint| {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
                opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                    "service.name",
                    service_name.to_string(),
                )]),
            ))
            .install_batch(opentelemetry::sdk::runtime::Tokio)
            .ok()?;

        Some(OpenTelemetryLayer::new(tracer))
    });

    match otlp_layer {
        Some(layer) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_file(true)
                        .with_line_number(true),
                )
                .with(layer)
                .init();
            tracing::info!(
                "OpenTelemetry tracing initialized for service: {}",
                service_name
            );
        }
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_file(true)
                        .with_line_number(true),
                )
                .init();
            tracing::info!(
                "Local tracing initialized (no OTLP endpoint) for service: {}",
                service_name
            );
        }
    }
}
