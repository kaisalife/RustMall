//! 共享 Proto 定义
//!
//! 这个 crate 统一编译所有 gRPC proto 文件，
//! 各个服务通过依赖此 crate 来获取 gRPC server/client 代码。

/// 认证服务 proto
pub mod auth {
    pub mod v1 {
        tonic::include_proto!("auth.v1");
    }
    pub mod v2 {
        tonic::include_proto!("auth.v2");
    }
}

/// 邮件服务 proto
pub mod email {
    pub mod v1 {
        tonic::include_proto!("email.v1");
    }
}

/// 商品服务 proto
pub mod product {
    pub mod v1 {
        tonic::include_proto!("product.v1");
    }
}

/// 订单服务 proto
pub mod order {
    pub mod v1 {
        tonic::include_proto!("order.v1");
    }
}

/// 库存服务 proto
pub mod inventory {
    pub mod v1 {
        tonic::include_proto!("inventory.v1");
    }
}

/// 支付服务 proto
pub mod payment {
    pub mod v1 {
        tonic::include_proto!("payment.v1");
    }
}
