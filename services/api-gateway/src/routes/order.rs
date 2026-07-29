use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use std::sync::Arc;

use common::AppError;

use crate::dto::order::{
    CreateOrderRequest, ListOrdersQuery, ListOrdersResponseDto, OrderDto, OrderItemDto,
    UpdateOrderStatusRequest,
};
use crate::response::ApiResponse;
use crate::state::AppState;

pub fn order_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_order_handler))
        .route("/:id", get(get_order_handler))
        .route("/", get(list_orders_handler))
        .route("/:id/status", put(update_order_status_handler))
}

#[tracing::instrument(skip(state, req), fields(user_id = req.user_id, items = req.items.len()))]
async fn create_order_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<ApiResponse<OrderDto>>, AppError> {
    // 步骤1: 批量预留库存（单次 gRPC 替代 N 次逐项调用）
    let stock_items: Vec<proto::inventory::StockItem> = req
        .items
        .iter()
        .map(|item| proto::inventory::StockItem {
            product_id: item.product_id,
            quantity: item.quantity,
        })
        .collect();

    let reserve_resp = state
        .clients
        .call_inventory(|mut client| async move {
            client
                .batch_reserve_stock(proto::inventory::BatchReserveStockRequest {
                    items: stock_items,
                })
                .await
        })
        .await
        .map_err(|e| {
            let app_err: AppError = e.into();
            tracing::error!("Failed to batch reserve stock: {}", app_err);
            app_err
        })?;

    let reserve_inner = reserve_resp.into_inner();
    if !reserve_inner.all_success {
        // 部分预留失败，释放已成功的预留
        let succeeded: std::collections::HashSet<u64> = reserve_inner
            .results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.product_id)
            .collect();
        let release_items: Vec<proto::inventory::StockItem> = req
            .items
            .iter()
            .filter(|i| succeeded.contains(&i.product_id))
            .map(|i| proto::inventory::StockItem {
                product_id: i.product_id,
                quantity: i.quantity,
            })
            .collect();
        if !release_items.is_empty() {
            let _ = state
                .clients
                .call_inventory(|mut client| async move {
                    client
                        .batch_release_stock(proto::inventory::BatchReleaseStockRequest {
                            items: release_items,
                        })
                        .await
                })
                .await;
        }
        let errors: Vec<_> = reserve_inner
            .results
            .iter()
            .filter(|r| !r.success)
            .map(|r| format!("product {}: {}", r.product_id, r.error))
            .collect();
        return Err(AppError::invalid_input(format!(
            "Insufficient stock: {}",
            errors.join("; ")
        )));
    }

    // 步骤2: 创建订单
    let items = req
        .items
        .iter()
        .map(|item| proto::order::OrderItem {
            product_id: item.product_id,
            quantity: item.quantity,
            unit_price: item.unit_price,
        })
        .collect();
    let order_request = proto::order::CreateOrderRequest {
        user_id: req.user_id,
        items,
    };

    let order_result = state
        .clients
        .call_order(|mut client| async move { client.create_order(order_request).await })
        .await;

    match order_result {
        Ok(resp) => {
            // 订单创建成功，库存扣减由 inventory-service 异步消费 OrderCreated 事件完成
            let order = resp.into_inner();
            Ok(Json(ApiResponse::success(order_to_dto(order))))
        }
        Err(e) => {
            // 补偿 - 批量释放已预留的库存
            let app_err: AppError = e.into();
            tracing::error!(
                "Order creation failed, compensating by releasing reserved stock: {}",
                app_err
            );
            let release_items: Vec<proto::inventory::StockItem> = req
                .items
                .iter()
                .map(|item| proto::inventory::StockItem {
                    product_id: item.product_id,
                    quantity: item.quantity,
                })
                .collect();
            let _ = state
                .clients
                .call_inventory(|mut client| async move {
                    client
                        .batch_release_stock(proto::inventory::BatchReleaseStockRequest {
                            items: release_items,
                        })
                        .await
                })
                .await;
            Err(app_err)
        }
    }
}

async fn get_order_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderDto>>, AppError> {
    let request = proto::order::GetOrderRequest { order_id: id };
    let response = state
        .clients
        .call_order(|mut client| async move { client.get_order(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(order_to_dto(inner))))
}

async fn list_orders_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListOrdersQuery>,
) -> Result<Json<ApiResponse<ListOrdersResponseDto>>, AppError> {
    let request = proto::order::ListOrdersRequest {
        user_id: query.user_id,
        page: query.page,
        page_size: query.page_size,
    };
    let response = state
        .clients
        .call_order(|mut client| async move { client.list_orders(request).await })
        .await?;
    let inner = response.into_inner();

    let orders = inner.orders.into_iter().map(order_to_dto).collect();

    Ok(Json(ApiResponse::success(ListOrdersResponseDto {
        orders,
        total: inner.total,
        page: inner.page,
        page_size: inner.page_size,
    })))
}

async fn update_order_status_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateOrderStatusRequest>,
) -> Result<Json<ApiResponse<OrderDto>>, AppError> {
    let request = proto::order::UpdateOrderStatusRequest {
        order_id: id,
        status: req.status,
    };
    let response = state
        .clients
        .call_order(|mut client| async move { client.update_order_status(request).await })
        .await?;
    let inner = response.into_inner();

    Ok(Json(ApiResponse::success(order_to_dto(inner))))
}

fn order_to_dto(o: proto::order::OrderResponse) -> OrderDto {
    let items = o
        .items
        .into_iter()
        .map(|item| OrderItemDto {
            product_id: item.product_id,
            quantity: item.quantity,
            unit_price: item.unit_price,
        })
        .collect();

    OrderDto {
        order_id: o.order_id,
        user_id: o.user_id,
        total_amount: o.total_amount,
        status: o.status,
        items,
        created_at: o.created_at,
        updated_at: o.updated_at,
    }
}
