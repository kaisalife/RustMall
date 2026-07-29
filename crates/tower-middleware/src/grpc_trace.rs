//! gRPC trace context 传播（W3C TraceContext）
//!
//! 客户端侧（网关）：把当前 tracing span 的 OpenTelemetry context 注入到出站 gRPC
//! 请求的 metadata（W3C `traceparent` header）。
//! 服务端侧（后端服务）：从入站 gRPC 请求的 metadata 提取父 context，关联到当前
//! tracing span，使分布式追踪跨服务串联。
//!
//! 通过 tonic [`Interceptor`](tonic::service::Interceptor) 实现，仅处理请求 metadata，
//! 不修改响应。需要全局注册 W3C `TraceContextPropagator`（见 `common::init_tracing`），
//! 否则 inject/extract 为 noop。

use std::str::FromStr;

use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use tonic::service::Interceptor;
use tonic::Request;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// tonic metadata 注入器（客户端侧），实现 opentelemetry [`Injector`]。
struct MetadataInjector<'a>(&'a mut tonic::metadata::MetadataMap);

impl<'a> Injector for MetadataInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(metadata_key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(metadata_value) = tonic::metadata::MetadataValue::from_str(&value) {
                self.0.insert(metadata_key, metadata_value);
            }
        }
    }
}

/// tonic metadata 提取器（服务端侧），实现 opentelemetry [`Extractor`]。
struct MetadataExtractor<'a>(&'a tonic::metadata::MetadataMap);

impl<'a> Extractor for MetadataExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0
            .keys()
            .map(|k| match k {
                tonic::metadata::KeyRef::Ascii(k) => k.as_str(),
                tonic::metadata::KeyRef::Binary(k) => k.as_str(),
            })
            .collect()
    }
}

/// 客户端 interceptor：把当前 tracing span 的 OpenTelemetry context 注入到出站 gRPC
/// 请求的 metadata。
///
/// 用法：`AuthServiceClient::with_interceptor(channel, TraceContextInjector)`
#[derive(Debug, Clone, Copy)]
pub struct TraceContextInjector;

impl Interceptor for TraceContextInjector {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, tonic::Status> {
        let context = tracing::span::Span::current().context();
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut MetadataInjector(req.metadata_mut()));
        });
        Ok(req)
    }
}

/// 服务端 interceptor：从入站 gRPC 请求的 metadata 提取父 OpenTelemetry context，
/// 并关联到当前 tracing span。
///
/// 用法：`AuthServiceServer::with_interceptor(impl, TraceContextExtractor)`
#[derive(Debug, Clone, Copy)]
pub struct TraceContextExtractor;

impl Interceptor for TraceContextExtractor {
    fn call(&mut self, req: Request<()>) -> Result<Request<()>, tonic::Status> {
        let parent_context = global::get_text_map_propagator(|propagator| {
            propagator.extract(&MetadataExtractor(req.metadata()))
        });
        tracing::span::Span::current().set_parent(parent_context);
        Ok(req)
    }
}

/// 带 trace 注入 interceptor 的 gRPC channel 服务类型。
///
/// 网关侧 client 类型形如 `AuthServiceClient<TracedChannel>`，
/// 由 `AuthServiceClient::with_interceptor(channel, TraceContextInjector)` 构造。
pub type TracedChannel = tonic::service::interceptor::InterceptedService<
    tonic::transport::Channel,
    TraceContextInjector,
>;
