# Inventory Service

## 服务概述

库存管理服务（Inventory Service）是 Simple Trade 电商系统的库存领域服务，负责商品库存的原子操作与管理。

**核心功能：**
- 库存扣减（原子操作，防止超卖）
- 库存增加
- 单个/批量库存查询
- 库存预留（Reserve）
- 库存释放（Release）
- 可用库存计算（总库存 - 预留库存）

**库存模型：**
- `quantity`：总库存数量
- `reserved_quantity`：已预留数量
- `available_quantity`：可用数量 = `quantity - reserved_quantity`

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
| common (workspace) | - | 公共库（配置、错误处理等） |
| proto (workspace) | - | gRPC 生成代码 |

## 架构设计

本服务采用 DDD（领域驱动设计）四层架构：

```
┌─────────────────────────────────────────────┐
│              Interface Layer                │  gRPC 接口实现层
│  (interface/mod.rs - InventoryServiceImpl)  │  接收 gRPC 请求，转换为应用层调用
├─────────────────────────────────────────────┤
│            Application Layer                │  应用服务层
│  (application/ - InventoryApplicationSvc)   │  业务编排：库存扣减、增加、预留、释放
│  - service.rs: 核心业务逻辑                  │
│  - dto.rs: 数据传输对象                     │
├─────────────────────────────────────────────┤
│              Domain Layer                   │  领域层
│  (domain/)                                  │  核心业务模型与规则
│  - inventory.rs: Inventory 实体              │
│    （库存扣减/预留/释放的业务规则）           │
│  - repository.rs: InventoryRepository 接口   │
├─────────────────────────────────────────────┤
│           Infrastructure Layer              │  基础设施层
│  (infrastructure/)                          │  技术实现细节
│  - database.rs: 数据库连接池                 │
│  - repository.rs: InventoryRepositoryImpl   │
│    （含原子操作 SQL 实现）                    │
└─────────────────────────────────────────────┘
```

## API 文档

### gRPC 接口定义

Proto 文件：`proto/inventory.proto`，包名 `inventory`

| RPC 方法 | 请求消息 | 响应消息 | 说明 |
|----------|----------|----------|------|
| `DeductStock` | `DeductStockRequest` | `DeductStockResponse` | 扣减库存（原子操作） |
| `AddStock` | `AddStockRequest` | `AddStockResponse` | 增加库存 |
| `GetStock` | `GetStockRequest` | `StockResponse` | 查询单个商品库存 |
| `BatchGetStock` | `BatchGetStockRequest` | `BatchGetStockResponse` | 批量查询库存 |
| `ReserveStock` | `ReserveStockRequest` | `ReserveStockResponse` | 预留库存 |
| `ReleaseStock` | `ReleaseStockRequest` | `ReleaseStockResponse` | 释放预留库存 |

### 消息定义

#### DeductStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 扣减数量（必须为正数） |

#### DeductStockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否扣减成功 |
| remaining | int32 | 剩余库存 |

#### AddStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 增加数量（必须为正数） |

#### AddStockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否增加成功 |
| total | int32 | 增加后总库存 |

#### GetStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |

#### StockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 总库存数量 |
| reserved_quantity | int32 | 已预留数量 |
| updated_at | string | 更新时间（RFC3339） |

#### BatchGetStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_ids | repeated uint64 | 商品 ID 列表 |

#### BatchGetStockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| stocks | repeated StockResponse | 库存信息列表 |

#### ReserveStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 预留数量（必须为正数） |

#### ReserveStockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否预留成功 |
| reserved | int32 | 已预留数量 |

#### ReleaseStockRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| product_id | uint64 | 商品 ID |
| quantity | int32 | 释放数量（必须为正数） |

#### ReleaseStockResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否释放成功 |
| released | int32 | 已释放数量 |

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 服务配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[inventory_service] host` | `127.0.0.1` | 服务监听地址 |
| `[inventory_service] port` | `50054` | gRPC 监听端口 |
| `[inventory_service] worker_id` | `4` | Snowflake ID 生成器工作节点 ID |

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

# 3. 启动库存服务
cargo run --bin inventory-service
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| Inventory Service | 50054 | gRPC |
| PostgreSQL | 5432 | TCP |

## 目录结构

```
services/inventory-service/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：配置加载、依赖注入、gRPC 服务启动
    ├── domain/                # 领域层
    │   ├── mod.rs             # 模块导出
    │   ├── inventory.rs       # Inventory 实体（库存操作业务规则）
    │   └── repository.rs      # InventoryRepository 仓储接口（Trait）
    ├── application/           # 应用层
    │   ├── mod.rs             # 模块导出
    │   ├── service.rs         # InventoryApplicationService 业务编排
    │   └── dto.rs             # 数据传输对象
    ├── infrastructure/        # 基础设施层
    │   ├── mod.rs             # 模块导出
    │   ├── database.rs        # 数据库连接池
    │   └── repository.rs      # InventoryRepositoryImpl（PostgreSQL + 原子操作）
    └── interface/             # 接口层
        └── mod.rs             # InventoryServiceImpl（gRPC 服务实现）
```
