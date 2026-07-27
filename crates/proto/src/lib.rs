//! 共享 Proto 定义
//!
//! 这个 crate 统一编译所有 gRPC proto 文件，
//! 各个服务通过依赖此 crate 来获取 gRPC server/client 代码。

/// 认证服务 proto
pub mod auth {
    tonic::include_proto!("auth");
}

/// 邮件服务 proto
pub mod email {
    tonic::include_proto!("email");
}

/// 商品服务 proto
pub mod product {
    tonic::include_proto!("product");
}

/// 订单服务 proto
pub mod order {
    tonic::include_proto!("order");
}

/// 库存服务 proto
pub mod inventory {
    tonic::include_proto!("inventory");
}

/// 支付服务 proto
pub mod payment {
    tonic::include_proto!("payment");
}
