use tonic::{Request, Response, Status};

use crate::application::command::{
    AddStockCommand, DeductStockCommand, ReleaseStockCommand, ReserveStockCommand,
};
use crate::application::InventoryApplicationService;

use proto::inventory::v1::{
    inventory_service_server::InventoryService, AddStockRequest, AddStockResponse,
    BatchGetStockRequest, BatchGetStockResponse, BatchReleaseStockRequest,
    BatchReleaseStockResponse, BatchReserveStockRequest, BatchReserveStockResponse,
    BatchReserveStockResult, DeductStockRequest, DeductStockResponse, GetStockRequest,
    ReleaseStockRequest, ReleaseStockResponse, ReserveStockRequest, ReserveStockResponse,
    StockResponse,
};

#[derive(Clone)]
pub struct InventoryServiceImpl {
    service: InventoryApplicationService,
}

impl InventoryServiceImpl {
    pub fn new(service: InventoryApplicationService) -> Self {
        Self { service }
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

#[tonic::async_trait]
impl InventoryService for InventoryServiceImpl {
    async fn deduct_stock(
        &self,
        request: Request<DeductStockRequest>,
    ) -> Result<Response<DeductStockResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .deduct_stock(DeductStockCommand {
                product_id: req.product_id,
                quantity: req.quantity,
            })
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(DeductStockResponse {
            success: result.success,
            remaining: result.remaining,
        }))
    }

    async fn add_stock(
        &self,
        request: Request<AddStockRequest>,
    ) -> Result<Response<AddStockResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .add_stock(AddStockCommand {
                product_id: req.product_id,
                quantity: req.quantity,
            })
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(AddStockResponse {
            success: result.success,
            total: result.total,
        }))
    }

    async fn get_stock(
        &self,
        request: Request<GetStockRequest>,
    ) -> Result<Response<StockResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .service
            .get_stock(req.product_id)
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(StockResponse {
            product_id: result.product_id,
            quantity: result.quantity,
            reserved_quantity: result.reserved_quantity,
            updated_at: result.updated_at,
        }))
    }

    async fn batch_get_stock(
        &self,
        request: Request<BatchGetStockRequest>,
    ) -> Result<Response<BatchGetStockResponse>, Status> {
        let req = request.into_inner();

        let results = self
            .service
            .batch_get_stock(req.product_ids)
            .await
            .map_err(app_error_to_status)?;

        let stocks = results
            .into_iter()
            .map(|r| StockResponse {
                product_id: r.product_id,
                quantity: r.quantity,
                reserved_quantity: r.reserved_quantity,
                updated_at: r.updated_at,
            })
            .collect();

        Ok(Response::new(BatchGetStockResponse { stocks }))
    }

    async fn reserve_stock(
        &self,
        request: Request<ReserveStockRequest>,
    ) -> Result<Response<ReserveStockResponse>, Status> {
        let req = request.into_inner();

        let inventory = self
            .service
            .reserve_stock(ReserveStockCommand {
                product_id: req.product_id,
                quantity: req.quantity,
            })
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(ReserveStockResponse {
            success: true,
            reserved: inventory.reserved_quantity,
        }))
    }

    async fn release_stock(
        &self,
        request: Request<ReleaseStockRequest>,
    ) -> Result<Response<ReleaseStockResponse>, Status> {
        let req = request.into_inner();

        let inventory = self
            .service
            .release_reserved_stock(ReleaseStockCommand {
                product_id: req.product_id,
                quantity: req.quantity,
            })
            .await
            .map_err(app_error_to_status)?;

        Ok(Response::new(ReleaseStockResponse {
            success: true,
            released: inventory.reserved_quantity,
        }))
    }

    async fn batch_reserve_stock(
        &self,
        request: Request<BatchReserveStockRequest>,
    ) -> Result<Response<BatchReserveStockResponse>, Status> {
        let req = request.into_inner();
        let items: Vec<(u64, i32)> = req
            .items
            .iter()
            .map(|i| (i.product_id, i.quantity))
            .collect();

        match self.service.batch_reserve_stock(items).await {
            Ok(_) => Ok(Response::new(BatchReserveStockResponse {
                all_success: true,
                results: req
                    .items
                    .iter()
                    .map(|i| BatchReserveStockResult {
                        product_id: i.product_id,
                        success: true,
                        error: String::new(),
                    })
                    .collect(),
            })),
            Err(e) => {
                let error_msg = e.to_string();
                Ok(Response::new(BatchReserveStockResponse {
                    all_success: false,
                    results: req
                        .items
                        .iter()
                        .map(|i| BatchReserveStockResult {
                            product_id: i.product_id,
                            success: false,
                            error: error_msg.clone(),
                        })
                        .collect(),
                }))
            }
        }
    }

    async fn batch_release_stock(
        &self,
        request: Request<BatchReleaseStockRequest>,
    ) -> Result<Response<BatchReleaseStockResponse>, Status> {
        let req = request.into_inner();
        let items: Vec<(u64, i32)> = req
            .items
            .iter()
            .map(|i| (i.product_id, i.quantity))
            .collect();
        self.service.batch_release_stock(items).await;

        Ok(Response::new(BatchReleaseStockResponse {
            all_success: true,
        }))
    }
}
