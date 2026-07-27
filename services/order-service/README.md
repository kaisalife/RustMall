# Order Service

## 服务概述

订单管理服务（Order Service）是 Simple Trade 电商系统的订单领域服务，负责订单的创建、查询和状态管理。

**核心功能：**
- 订单创建（自动计算总金额）
- 订单查询（单个/列表分页）
- 订单状态流转管理
- 订单状态机（Pending → Paid → Shipped → Completed / Cancelled）

**订单状态流转规则：**
- `Pending`（待支付）→ `Paid`（已支付）→ `Shipped`（已发货）→ `Completed`（已完成）
- `Pending` 或 `Paid` 状态可取消为 `Cancelled`（已取消）
- `Shipped` 和 `Completed` 状态不可取消

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 编程语言 |
| Tokio | 1.35 | 异步运行时 |
| Tonic | 0.10 | gRPC 框架 |
| SQLx | 0.7 | 数据库 ORM（PostgreSQL） |
| serde | 1.0 | 序列化/反序列化 |
| figment | 0.10 | 配置管理 |
| tracing | 0.1 | 日志与分布式追踪 |
| common (workspace) | - | 公共库（配置、ID 生成等） |
| proto (workspace) | - | gRPC 生成代码 |

## 架构设计

本服务采用 DDD（领域驱动设计）四层架构：

```
┌─────────────────────────────────────────┐
│           Interface Layer               │  gRPC 接口实现层
│  (interface/mod.rs - OrderServiceImpl)  │  接收 gRPC 请求，转换为应用层调用
├─────────────────────────────────────────┤
│          Application Layer              │  应用服务层
│  (application/ - OrderApplicationSvc)   │  业务编排：订单创建、查询、状态更新
│  - service.rs: 核心业务逻辑 + DTO 定义   │
├─────────────────────────────────────────┤
│            Domain Layer                 │  领域层
│  (domain/)                              │  核心业务模型与规则
│  - order.rs: Order 实体、OrderItem、     │
│    OrderStatus 状态机                   │
│  - repository.rs: OrderRepository 接口   │
├─────────────────────────────────────────┤
│         Infrastructure Layer            │  基础设施层
│  (infrastructure/)                      │  技术实现细节
│  - database.rs: 数据库连接池             │
│  - repository.rs: OrderRepositoryImpl   │
└─────────────────────────────────────────┘
```

## API 文档

### gRPC 接口定义

Proto 文件：`proto/order.proto`，包名 `order`

| RPC 方法 | 请求消息 | 响应消息 | 说明 |
|----------|----------|----------|------|
| `CreateOrder` | `CreateOrderRequest` | `OrderResponse` | 创建订单 |
| `GetOrder` | `GetOrderRequest` | `OrderResponse` | 查询单个订单 |
| `ListOrders` | `ListOrdersRequest` | `ListOrdersResponse` | 分页查询用户订单 |
| `UpdateOrderStatus` | `UpdateOrderStatusRequest` | `OrderResponse` | 更新订单状态 |

### 消息定义

#### OrderStatus 枚举

| 值 | 名称 | 说明 |
|----|------|------|
| 0 | PENDING | 待支付 |
| 1 | PAID | 已支付 |
| 2 | SHIPPED | 已发货 |
| 3 | COMPLETED | 已完成 |
| 4 | CANCELLED | 已取消 |

#### OrderItem
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 购买数量 |
| unit_price | double | 单价 |

#### CreateOrderRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |
| items | repeated OrderItem | 订单商品列表（不能为空） |

#### GetOrderRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| order_id | uint64 | 订单 ID |

#### ListOrdersRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |
| page | int32 | 页码（从 1 开始） |
| page_size | int32 | 每页数量 |

#### UpdateOrderStatusRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| order_id | uint64 | 订单 ID |
| status | OrderStatus | 目标状态 |

#### OrderResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| order_id | uint64 | 订单 ID |
| user_id | uint64 | 用户 ID |
| total_amount | double | 订单总金额（自动计算） |
| status | OrderStatus | 订单状态 |
| items | repeated OrderItem | 订单商品列表 |
| created_at | string | 创建时间（RFC3339） |
| updated_at | string | 更新时间（RFC3339） |

#### ListOrdersResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| orders | repeated OrderResponse | 订单列表 |
| total | int32 | 总记录数 |
| page | int32 | 当前页码 |
| page_size | int32 | 每页数量 |

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 服务配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[order_service] host` | `127.0.0.1` | 服务监听地址 |
| `[order_service] port` | `50053` | gRPC 监听端口 |
| `[order_service] worker_id` | `3` | Snowflake ID 生成器工作节点 ID |

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

# 3. 启动订单服务
cargo run --bin order-service
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| Order Service | 50053 | gRPC |
| PostgreSQL | 5432 | TCP |

## 目录结构

```
services/order-service/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：配置加载、依赖注入、gRPC 服务启动
    ├── domain/                # 领域层
    │   ├── mod.rs             # 模块导出
    │   ├── order.rs           # Order 实体、OrderItem、OrderStatus 状态机
    │   └── repository.rs      # OrderRepository 仓储接口（Trait）
    ├── application/           # 应用层
    │   ├── mod.rs             # 模块导出
    │   └── service.rs         # OrderApplicationService 业务编排 + DTO
    ├── infrastructure/        # 基础设施层
    │   ├── mod.rs             # 模块导出
    │   ├── database.rs        # 数据库连接池
    │   └── repository.rs      # OrderRepositoryImpl（PostgreSQL 实现）
    └── interface/             # 接口层
        └── mod.rs             # OrderServiceImpl（gRPC 服务实现）
```
