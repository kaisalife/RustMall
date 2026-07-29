use crate::application::{OrderApplicationService, OrderDto, OrderItemDto};
use tonic::{Request, Response, Status};

use proto::order::{
    order_service_server::OrderService, CreateOrderRequest, GetOrderRequest, ListOrdersRequest,
    ListOrdersResponse, OrderItem as ProtoOrderItem, OrderResponse, OrderStatus,
    UpdateOrderStatusRequest,
};

#[derive(Clone)]
pub struct OrderServiceImpl {
    service: OrderApplicationService,
}

impl OrderServiceImpl {
    pub fn new(service: OrderApplicationService) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn create_order(
        &self,
        request: Request<CreateOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let req = request.into_inner();

        let items: Vec<OrderItemDto> = req
            .items
            .into_iter()
            .map(|i| OrderItemDto {
                product_id: i.product_id,
                quantity: i.quantity,
                unit_price: i.unit_price,
            })
            .collect();

        let result = self
            .service
            .create_order(req.user_id, items)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(Self::dto_to_proto(result)))
    }

    async fn get_order(
        &self,
        request: Request<GetOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .get_order(req.order_id)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(Self::dto_to_proto(result)))
    }

    async fn list_orders(
        &self,
        request: Request<ListOrdersRequest>,
    ) -> Result<Response<ListOrdersResponse>, Status> {
        let req = request.into_inner();

        let (orders, total) = self
            .service
            .list_orders(req.user_id, req.page, req.page_size)
            .await
            .map_err(app_error_to_status)?;

        let orders: Vec<OrderResponse> = orders.into_iter().map(Self::dto_to_proto).collect();

        Ok(Response::new(ListOrdersResponse {
            orders,
            total,
            page: req.page,
            page_size: req.page_size,
        }))
    }

    async fn update_order_status(
        &self,
        request: Request<UpdateOrderStatusRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let req = request.into_inner();

        let status = match OrderStatus::try_from(req.status) {
            Ok(OrderStatus::Pending) => "PENDING",
            Ok(OrderStatus::Paid) => "PAID",
            Ok(OrderStatus::Shipped) => "SHIPPED",
            Ok(OrderStatus::Completed) => "COMPLETED",
            Ok(OrderStatus::Cancelled) => "CANCELLED",
            Err(_) => "PENDING",
        }
        .to_string();

        let result = self
            .service
            .update_order_status(req.order_id, status)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(Self::dto_to_proto(result)))
    }
}

impl OrderServiceImpl {
    fn dto_to_proto(dto: OrderDto) -> OrderResponse {
        let status = match dto.status.as_str() {
            "PAID" => OrderStatus::Paid,
            "SHIPPED" => OrderStatus::Shipped,
            "COMPLETED" => OrderStatus::Completed,
            "CANCELLED" => OrderStatus::Cancelled,
            _ => OrderStatus::Pending,
        };
        OrderResponse {
            order_id: dto.order_id,
            user_id: dto.user_id,
            total_amount: dto.total_amount,
            status: status as i32,
            items: dto
                .items
                .into_iter()
                .map(|i| ProtoOrderItem {
                    product_id: i.product_id,
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }
}

fn app_error_to_status(error: common::AppError) -> Status {
    match error {
        common::AppError::NotFound(msg) => Status::not_found(msg),
        common::AppError::InvalidInput(msg) => Status::invalid_argument(msg),
        common::AppError::Database(e) => Status::internal(e.to_string()),
        common::AppError::Internal(msg) => Status::internal(msg),
        _ => Status::internal(error.to_string()),
    }
}
