use tonic::{Request, Response, Status};

use crate::application::{ProductApplicationService, command::*};

use proto::product::{
    product_service_server::ProductService,
    CreateProductRequest, ProductResponse,
    GetProductRequest, UpdateProductRequest, DeleteProductRequest,
    ListProductsRequest, ListProductsResponse,
};

#[derive(Clone)]
pub struct ProductServiceImpl {
    service: ProductApplicationService,
}

impl ProductServiceImpl {
    pub fn new(service: ProductApplicationService) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl ProductService for ProductServiceImpl {
    async fn create_product(
        &self,
        request: Request<CreateProductRequest>,
    ) -> Result<Response<ProductResponse>, Status> {
        let req = request.into_inner();

        let command = CreateProductCommand {
            name: req.name,
            description: req.description,
            price: req.price,
            category_id: req.category_id,
            stock: req.stock,
        };

        let result = self.service.create_product(command).await.map_err(app_error_to_status)?;

        Ok(Response::new(ProductResponse {
            product_id: result.product_id,
            name: result.name,
            description: result.description,
            price: result.price,
            category_id: result.category_id,
            created_at: result.created_at,
            updated_at: result.updated_at,
        }))
    }

    async fn get_product(
        &self,
        request: Request<GetProductRequest>,
    ) -> Result<Response<ProductResponse>, Status> {
        let req = request.into_inner();

        let result = self.service.get_product(req.product_id).await.map_err(app_error_to_status)?;

        Ok(Response::new(ProductResponse {
            product_id: result.product_id,
            name: result.name,
            description: result.description,
            price: result.price,
            category_id: result.category_id,
            created_at: result.created_at,
            updated_at: result.updated_at,
        }))
    }

    async fn update_product(
        &self,
        request: Request<UpdateProductRequest>,
    ) -> Result<Response<ProductResponse>, Status> {
        let req = request.into_inner();

        let command = UpdateProductCommand {
            product_id: req.product_id,
            name: req.name,
            description: req.description,
            price: req.price,
            category_id: req.category_id,
        };

        let result = self.service.update_product(command).await.map_err(app_error_to_status)?;

        Ok(Response::new(ProductResponse {
            product_id: result.product_id,
            name: result.name,
            description: result.description,
            price: result.price,
            category_id: result.category_id,
            created_at: result.created_at,
            updated_at: result.updated_at,
        }))
    }

    async fn delete_product(
        &self,
        request: Request<DeleteProductRequest>,
    ) -> Result<Response<proto::product::DeleteProductResponse>, Status> {
        let req = request.into_inner();

        let success = self.service.delete_product(req.product_id).await.map_err(app_error_to_status)?;

        Ok(Response::new(proto::product::DeleteProductResponse { success }))
    }

    async fn list_products(
        &self,
        request: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        let req = request.into_inner();

        let query = ListProductsQuery {
            category_id: req.category_id,
            min_price: req.min_price,
            max_price: req.max_price,
            page: req.page,
            page_size: req.page_size,
        };

        let result = self.service.list_products(query).await.map_err(app_error_to_status)?;

        let products = result.products
            .into_iter()
            .map(|p| ProductResponse {
                product_id: p.product_id,
                name: p.name,
                description: p.description,
                price: p.price,
                category_id: p.category_id,
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect();

        Ok(Response::new(ListProductsResponse {
            products,
            total: result.total,
            page: result.page,
            page_size: result.page_size,
        }))
    }
}

/// 将 AppError 转换为 tonic Status
pub fn app_error_to_status(error: common::AppError) -> Status {
    match error {
        common::AppError::NotFound(msg) => Status::not_found(msg),
        common::AppError::Conflict(msg) => Status::already_exists(msg),
        common::AppError::InvalidInput(msg) => Status::invalid_argument(msg),
        common::AppError::Authentication(msg) => Status::unauthenticated(msg),
        common::AppError::Forbidden(msg) => Status::permission_denied(msg),
        common::AppError::Database(e) => Status::internal(e.to_string()),
        common::AppError::Internal(msg) => Status::internal(msg),
        _ => Status::internal(error.to_string()),
    }
}
