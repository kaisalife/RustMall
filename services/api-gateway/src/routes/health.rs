use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct HealthStatus {
    status: String,
    services: Vec<ServiceHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceHealth {
    name: String,
    status: String,
}

async fn check_grpc<F, T>(fut: F) -> bool
where
    F: std::future::Future<Output = Result<T, tonic::Status>> + Send,
    T: Send,
{
    match tokio::time::timeout(Duration::from_secs(10), fut).await {
        Ok(Ok(_)) => true,
        Ok(Err(status)) => status.code() != tonic::Code::Unavailable,
        Err(_) => false,
    }
}

pub async fn health_check_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache_key = "health:status";

    // 先查缓存（TTL 3s，避免高频健康检查打满 gRPC）
    if let Some(ref cache) = state.cache {
        if let Ok(Some(cached)) = cache.get_json::<HealthStatus>(cache_key).await {
            return (axum::http::StatusCode::OK, Json(cached));
        }
    }

    let (auth_ok, product_ok, order_ok, inventory_ok) = tokio::join!(
        check_grpc(state.clients.call_auth(|mut client| async move {
            client
                .get_user(proto::auth::GetUserRequest { user_id: 0 })
                .await
        })),
        check_grpc(state.clients.call_product(|mut client| async move {
            client
                .get_product(proto::product::GetProductRequest { product_id: 0 })
                .await
        })),
        check_grpc(state.clients.call_order(|mut client| async move {
            client
                .get_order(proto::order::GetOrderRequest { order_id: 0 })
                .await
        })),
        check_grpc(state.clients.call_inventory(|mut client| async move {
            client
                .get_stock(proto::inventory::GetStockRequest { product_id: 0 })
                .await
        })),
    );

    let services = vec![
        ServiceHealth {
            name: "auth-service".to_string(),
            status: if auth_ok { "up" } else { "down" }.to_string(),
        },
        ServiceHealth {
            name: "product-service".to_string(),
            status: if product_ok { "up" } else { "down" }.to_string(),
        },
        ServiceHealth {
            name: "order-service".to_string(),
            status: if order_ok { "up" } else { "down" }.to_string(),
        },
        ServiceHealth {
            name: "inventory-service".to_string(),
            status: if inventory_ok { "up" } else { "down" }.to_string(),
        },
    ];

    let all_healthy = services.iter().all(|s| s.status == "up");
    let status = if all_healthy { "healthy" } else { "degraded" };

    let health = HealthStatus {
        status: status.to_string(),
        services,
    };

    // 写入缓存
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache
            .set_json(cache_key, &health, Duration::from_secs(3))
            .await
        {
            tracing::warn!("Failed to write health to cache: {}", e);
        }
    }

    let code = if all_healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (code, Json(health))
}
