use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use std::time::Duration;

use common::AppError;

use crate::dto::product::{
    CreateProductRequest, DeleteProductResponseDto, ListProductsQuery, ListProductsResponseDto,
    ProductDto, UpdateProductRequest,
};
use crate::response::ApiResponse;
use crate::state::AppState;

pub fn product_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_product_handler))
        .route("/:id", get(get_product_handler))
        .route("/:id", put(update_product_handler))
        .route("/:id", delete(delete_product_handler))
        .route("/", get(list_products_handler))
}

async fn create_product_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<ApiResponse<ProductDto>>, AppError> {
    let request = proto::product::v1::CreateProductRequest {
        name: req.name,
        description: req.description,
        price: req.price,
        category_id: req.category_id,
        stock: req.stock,
    };
    let response = state
        .clients
        .call_product(|mut client| async move { client.create_product(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(product_to_dto(inner))))
}

async fn get_product_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<ProductDto>>, AppError> {
    let cache_key = format!("product:{}", id);

    // 先查缓存
    if let Some(ref cache) = state.cache {
        if let Ok(Some(cached)) = cache.get_json::<ProductDto>(&cache_key).await {
            tracing::debug!("Cache hit for product {}", id);
            return Ok(Json(ApiResponse::success(cached)));
        }
    }

    // 未命中缓存，调用 gRPC
    match state
        .clients
        .call_product(|mut client| async move {
            client
                .get_product(proto::product::v1::GetProductRequest { product_id: id })
                .await
        })
        .await
    {
        Ok(resp) => {
            let product = resp.into_inner();
            let dto = product_to_dto(product);

            // 写入缓存（TTL 5 分钟）
            if let Some(ref cache) = state.cache {
                if let Err(e) = cache
                    .set_json(&cache_key, &dto, Duration::from_secs(300))
                    .await
                {
                    tracing::warn!("Failed to write product {} to cache: {}", id, e);
                }
            }

            Ok(Json(ApiResponse::success(dto)))
        }
        Err(e) => match e.code() {
            tonic::Code::NotFound => Err(AppError::not_found(format!("Product {} not found", id))),
            tonic::Code::Unavailable => {
                tracing::error!("Product service unavailable: {}", e.message());
                // 降级：尝试从缓存返回旧数据
                if let Some(ref cache) = state.cache {
                    if let Ok(Some(cached)) = cache.get_json::<ProductDto>(&cache_key).await {
                        tracing::info!("Serving stale cache for product {}", id);
                        return Ok(Json(ApiResponse::success(cached)));
                    }
                }
                Err(AppError::internal(
                    "Product service is temporarily unavailable. Please try again later.",
                ))
            }
            _ => {
                let app_err: AppError = e.into();
                Err(app_err)
            }
        },
    }
}

async fn update_product_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ApiResponse<ProductDto>>, AppError> {
    let request = proto::product::v1::UpdateProductRequest {
        product_id: id,
        name: req.name,
        description: req.description,
        price: req.price,
        category_id: req.category_id,
    };
    let response = state
        .clients
        .call_product(|mut client| async move { client.update_product(request).await })
        .await?;
    let inner = response.into_inner();

    // 更新后失效缓存
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache.delete(&format!("product:{}", id)).await {
            tracing::warn!("Failed to invalidate cache for product {}: {}", id, e);
        }
    }

    Ok(Json(ApiResponse::success(product_to_dto(inner))))
}

async fn delete_product_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<DeleteProductResponseDto>>, AppError> {
    let request = proto::product::v1::DeleteProductRequest { product_id: id };
    let response = state
        .clients
        .call_product(|mut client| async move { client.delete_product(request).await })
        .await?;
    let inner = response.into_inner();

    // 删除后失效缓存
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache.delete(&format!("product:{}", id)).await {
            tracing::warn!("Failed to invalidate cache for product {}: {}", id, e);
        }
    }

    Ok(Json(ApiResponse::success(DeleteProductResponseDto {
        success: inner.success,
    })))
}

async fn list_products_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListProductsQuery>,
) -> Result<Json<ApiResponse<ListProductsResponseDto>>, AppError> {
    let request = proto::product::v1::ListProductsRequest {
        category_id: query.category_id,
        min_price: query.min_price,
        max_price: query.max_price,
        page: query.page,
        page_size: query.page_size,
    };
    match state
        .clients
        .call_product(|mut client| async move { client.list_products(request).await })
        .await
    {
        Ok(resp) => {
            let resp = resp.into_inner();
            let products: Vec<ProductDto> = resp.products.into_iter().map(product_to_dto).collect();
            Ok(Json(ApiResponse::success(ListProductsResponseDto {
                products,
                total: resp.total,
                page: resp.page,
                page_size: resp.page_size,
            })))
        }
        Err(e) if e.code() == tonic::Code::Unavailable => {
            tracing::error!("Product service unavailable: {}", e.message());
            // 降级：返回空列表而非错误
            Ok(Json(ApiResponse::success(ListProductsResponseDto {
                products: vec![],
                total: 0,
                page: query.page,
                page_size: query.page_size,
            })))
        }
        Err(e) => {
            let app_err: AppError = e.into();
            Err(app_err)
        }
    }
}

fn product_to_dto(p: proto::product::v1::ProductResponse) -> ProductDto {
    ProductDto {
        product_id: p.product_id,
        name: p.name,
        description: p.description,
        price: p.price,
        category_id: p.category_id,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}
