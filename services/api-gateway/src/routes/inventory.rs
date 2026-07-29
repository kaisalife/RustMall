use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use std::time::Duration;

use common::AppError;

use crate::dto::inventory::{
    AddStockRequest, AddStockResponseDto, DeductStockRequest, DeductStockResponseDto, StockDto,
};
use crate::response::ApiResponse;
use crate::state::AppState;

pub fn inventory_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:product_id", get(get_inventory_handler))
        .route("/:product_id/deduct", post(deduct_inventory_handler))
        .route("/:product_id/add", post(add_inventory_handler))
}

async fn get_inventory_handler(
    State(state): State<Arc<AppState>>,
    Path(product_id): Path<u64>,
) -> Result<Json<ApiResponse<StockDto>>, AppError> {
    let cache_key = format!("inventory:{}", product_id);

    // 先查缓存
    if let Some(ref cache) = state.cache {
        if let Ok(Some(cached)) = cache.get_json::<StockDto>(&cache_key).await {
            return Ok(Json(ApiResponse::success(cached)));
        }
    }

    let mut client = state.clients.inventory.clone();
    let request = proto::inventory::GetStockRequest { product_id };
    let response = client.get_stock(request).await?;
    let inner = response.into_inner();

    let dto = StockDto {
        product_id: inner.product_id,
        quantity: inner.quantity,
        reserved_quantity: inner.reserved_quantity,
        updated_at: inner.updated_at,
    };

    // 写入缓存（TTL 5s，库存变化频繁）
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache
            .set_json(&cache_key, &dto, Duration::from_secs(5))
            .await
        {
            tracing::warn!("Failed to write inventory {} to cache: {}", product_id, e);
        }
    }

    Ok(Json(ApiResponse::success(dto)))
}

async fn deduct_inventory_handler(
    State(state): State<Arc<AppState>>,
    Path(product_id): Path<u64>,
    Json(req): Json<DeductStockRequest>,
) -> Result<Json<ApiResponse<DeductStockResponseDto>>, AppError> {
    let mut client = state.clients.inventory.clone();
    let request = proto::inventory::DeductStockRequest {
        product_id,
        quantity: req.quantity,
    };
    let response = client.deduct_stock(request).await?;
    let inner = response.into_inner();

    // 扣减后失效缓存
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache.delete(&format!("inventory:{}", product_id)).await {
            tracing::warn!(
                "Failed to invalidate cache for inventory {}: {}",
                product_id,
                e
            );
        }
    }

    Ok(Json(ApiResponse::success(DeductStockResponseDto {
        success: inner.success,
        remaining: inner.remaining,
    })))
}

async fn add_inventory_handler(
    State(state): State<Arc<AppState>>,
    Path(product_id): Path<u64>,
    Json(req): Json<AddStockRequest>,
) -> Result<Json<ApiResponse<AddStockResponseDto>>, AppError> {
    let mut client = state.clients.inventory.clone();
    let request = proto::inventory::AddStockRequest {
        product_id,
        quantity: req.quantity,
    };
    let response = client.add_stock(request).await?;
    let inner = response.into_inner();

    // 添加后失效缓存
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache.delete(&format!("inventory:{}", product_id)).await {
            tracing::warn!(
                "Failed to invalidate cache for inventory {}: {}",
                product_id,
                e
            );
        }
    }

    Ok(Json(ApiResponse::success(AddStockResponseDto {
        success: inner.success,
        total: inner.total,
    })))
}
