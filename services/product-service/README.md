# Product Service

## 服务概述

商品管理服务（Product Service）是 Simple Trade 电商系统的商品领域服务，负责商品的完整生命周期管理。

**核心功能：**
- 商品创建、查询、更新、删除（CRUD）
- 商品分类管理（支持多级分类）
- 商品列表分页查询
- 价格区间过滤
- 按分类筛选商品

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 编程语言 |
| Tokio | 1.35 | 异步运行时 |
| Tonic | 0.10 | gRPC 框架 |
| SQLx | 0.7 | 数据库 ORM（PostgreSQL） |
| serde | 1.0 | 序列化/反序列化 |
| validator | 0.16 | 输入校验 |
| figment | 0.10 | 配置管理 |
| tracing | 0.1 | 日志与分布式追踪 |
| common (workspace) | - | 公共库（配置、ID 生成等） |
| proto (workspace) | - | gRPC 生成代码 |

## 架构设计

本服务采用 DDD（领域驱动设计）四层架构：

```
┌─────────────────────────────────────────┐
│           Interface Layer               │  gRPC 接口实现层
│  (interface/mod.rs - ProductServiceImpl)│  接收 gRPC 请求，转换为应用层调用
├─────────────────────────────────────────┤
│          Application Layer              │  应用服务层
│  (application/ - ProductApplicationSvc) │  业务编排：商品 CRUD、分类管理
│  - service.rs: 核心业务逻辑              │
│  - command.rs: 命令对象与查询对象        │
│  - dto.rs: 数据传输对象                 │
├─────────────────────────────────────────┤
│            Domain Layer                 │  领域层
│  (domain/)                              │  核心业务模型与规则
│  - product.rs: Product 实体（聚合根）    │
│  - category.rs: Category 实体           │
│  - repository.rs: 仓储接口（Trait）      │
├─────────────────────────────────────────┤
│         Infrastructure Layer            │  基础设施层
│  (infrastructure/)                      │  技术实现细节
│  - database.rs: 数据库连接池             │
│  - repository.rs: 仓储实现（PostgreSQL） │
└─────────────────────────────────────────┘
```

## API 文档

### gRPC 接口定义

Proto 文件：`proto/product.proto`，包名 `product`

| RPC 方法 | 请求消息 | 响应消息 | 说明 |
|----------|----------|----------|------|
| `CreateProduct` | `CreateProductRequest` | `ProductResponse` | 创建商品 |
| `GetProduct` | `GetProductRequest` | `ProductResponse` | 查询单个商品 |
| `UpdateProduct` | `UpdateProductRequest` | `ProductResponse` | 更新商品信息 |
| `DeleteProduct` | `DeleteProductRequest` | `DeleteProductResponse` | 删除商品 |
| `ListProducts` | `ListProductsRequest` | `ListProductsResponse` | 分页查询商品列表 |

### 消息定义

#### CreateProductRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| name | string | 商品名称 |
| description | string | 商品描述 |
| price | double | 商品价格（必须大于 0） |
| category_id | uint64 | 分类 ID |
| stock | int32 | 初始库存（不能为负） |

#### GetProductRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |

#### UpdateProductRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| name | optional string | 商品名称 |
| description | optional string | 商品描述 |
| price | optional double | 商品价格 |
| category_id | optional uint64 | 分类 ID |

#### DeleteProductRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |

#### DeleteProductResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否删除成功 |

#### ProductResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| name | string | 商品名称 |
| description | string | 商品描述 |
| price | double | 商品价格 |
| category_id | uint64 | 分类 ID |
| created_at | string | 创建时间（RFC3339） |
| updated_at | string | 更新时间（RFC3339） |

#### ListProductsRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| category_id | optional uint64 | 按分类筛选 |
| min_price | optional double | 最低价格 |
| max_price | optional double | 最高价格 |
| page | int32 | 页码（从 1 开始） |
| page_size | int32 | 每页数量 |

#### ListProductsResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| products | repeated ProductResponse | 商品列表 |
| total | int32 | 总记录数 |
| page | int32 | 当前页码 |
| page_size | int32 | 每页数量 |

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 服务配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[product_service] host` | `127.0.0.1` | 服务监听地址 |
| `[product_service] port` | `50052` | gRPC 监听端口 |
| `[product_service] worker_id` | `2` | Snowflake ID 生成器工作节点 ID |

### 数据库配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[database] host` | `127.0.0.1` | PostgreSQL 地址 |
| `[database] port` | `5432` | PostgreSQL 端口 |
| `[database] username` | `postgres` | 数据库用户名 |
| `[database] password` | `postgres` | 数据库密码（**生产环境必须覆盖**） |
| `[database] database` | `simple_trade` | 数据库名 |
| `[database] max_connections` | `20` | 最大连接数 |
| `[database] min_connections` | `5` | 最小连接数 |

### 环境变量

| 环境变量 | 说明 |
|----------|------|
| `APP_DATABASE__PASSWORD` | 覆盖数据库密码 |

## 本地开发

### 前置依赖

- Rust 工具链（stable）
- PostgreSQL 数据库

### 启动命令

```bash
# 1. 启动 PostgreSQL
docker-compose up -d postgres

# 2. 运行数据库迁移
cargo run --bin migrate

# 3. 启动商品服务
cargo run --bin product-service
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| Product Service | 50052 | gRPC |
| PostgreSQL | 5432 | TCP |

## 目录结构

```
services/product-service/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：配置加载、依赖注入、gRPC 服务启动
    ├── domain/                # 领域层
    │   ├── mod.rs             # 模块导出
    │   ├── product.rs         # Product 聚合根实体
    │   ├── category.rs        # Category 实体
    │   └── repository.rs      # 仓储接口（Trait）
    ├── application/           # 应用层
    │   ├── mod.rs             # 模块导出
    │   ├── service.rs         # ProductApplicationService 业务编排
    │   ├── command.rs         # 命令对象与查询对象
    │   └── dto.rs             # 数据传输对象
    ├── infrastructure/        # 基础设施层
    │   ├── mod.rs             # 模块导出
    │   ├── database.rs        # 数据库连接池
    │   └── repository.rs      # 仓储实现（PostgreSQL）
    └── interface/             # 接口层
        └── mod.rs             # ProductServiceImpl（gRPC 服务实现）
```
