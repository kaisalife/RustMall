# API Gateway

## 服务概述

API 网关（API Gateway）是 Simple Trade 电商系统的统一 HTTP 入口，负责将外部 RESTful 请求路由到内部 gRPC 微服务，并提供认证、限流、缓存等横切关注点。

**核心功能：**
- RESTful API 端点，代理后端 gRPC 微服务
- JWT 认证中间件（Bearer Token 校验）
- 分布式限流（令牌桶算法，区分普通/严格策略）
- Redis 缓存（商品查询缓存，服务不可用时降级返回旧数据）
- Saga 编排（创建订单时：预留库存 -> 创建订单 -> 扣减库存，失败时补偿释放）
- Prometheus 指标暴露（`/metrics` 端点）
- 健康检查（`/health` 端点，检测后端服务连通性）
- CORS 跨域支持
- 请求日志中间件
- 优雅关闭（Graceful Shutdown）

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 编程语言 |
| Tokio | 1.35 | 异步运行时 |
| Axum | 0.7 | Web 框架（HTTP2 支持） |
| Tonic | 0.10 | gRPC 客户端 |
| Tower | 0.4 | 中间件框架 |
| Tower-HTTP | 0.5 | HTTP 中间件（CORS/Trace/Limit/Timeout） |
| Hyper | 1.0 | HTTP 底层库 |
| serde / serde_json | 1.0 | 序列化/反序列化 |
| figment | 0.10 | 配置管理 |
| tracing | 0.1 | 日志与分布式追踪 |
| common (workspace) | - | 公共库（配置、错误处理等） |
| proto (workspace) | - | gRPC 生成代码 |
| tower-middleware (workspace) | - | JWT 认证、限流、日志中间件 |
| metrics (workspace) | - | Prometheus 指标中间件 |
| redis-cache (workspace) | - | Redis 缓存封装 |

## 架构设计

API Gateway 作为系统的统一入口层，采用分层路由设计：

```
                        ┌──────────────────┐
                        │   HTTP Client    │
                        └────────┬─────────┘
                                 │
                    ┌────────────▼────────────┐
                    │      API Gateway        │
                    │       (Axum 0.7)        │
                    ├─────────────────────────┤
                    │  全局中间件链：           │
                    │  CORS -> Logger ->      │
                    │  Metrics                │
                    ├─────────────────────────┤
                    │  公共路由（无需认证）      │
                    │  /api/auth/*            │
                    │  （严格限流：10 次/分钟）  │
                    ├─────────────────────────┤
                    │  受保护路由（需 JWT 认证） │
                    │  /api/users/*           │
                    │  /api/products/*        │
                    │  /api/orders/*          │
                    │  /api/inventory/*       │
                    │  （默认限流：60 次/分钟）  │
                    ├─────────────────────────┤
                    │  /health   健康检查      │
                    │  /metrics  Prometheus   │
                    └────────────┬────────────┘
                                 │ gRPC
              ┌──────────────────┼──────────────────┐
              │                  │                  │
     ┌────────▼───────┐ ┌───────▼────────┐ ┌───────▼────────┐
     │  Auth Service  │ │ Product Service│ │  Order Service │
     │   (50051)      │ │   (50052)      │ │   (50053)      │
     └────────────────┘ └────────────────┘ └────────────────┘
                                              │
                                     ┌────────▼────────┐
                                     │Inventory Service│
                                     │   (50054)       │
                                     └─────────────────┘
```

### Saga 编排模式（创建订单）

创建订单时，API Gateway 编排跨服务事务：

1. **预留库存**：对每个订单项调用 `ReserveStock`
2. **创建订单**：调用 Order Service 创建订单
3. **确认扣减**：订单创建成功后，调用 `DeductStock` 确认扣减
4. **补偿回滚**：如果订单创建失败，调用 `ReleaseStock` 释放已预留的库存

## API 文档

### HTTP 接口

#### 认证接口（公共，无需认证）

| 方法 | 路径 | 说明 | 请求体 |
|------|------|------|--------|
| POST | `/api/auth/register` | 用户注册 | `{ email, password, nickname }` |
| POST | `/api/auth/login` | 用户登录 | `{ email, password }` |
| POST | `/api/auth/refresh` | 刷新 Token | `{ refresh_token }` |

#### 用户接口（需认证）

| 方法 | 路径 | 说明 | 请求/参数 |
|------|------|------|-----------|
| GET | `/api/users/:id` | 获取用户信息 | Path: `id` |
| PUT | `/api/users/password` | 修改密码 | `{ user_id, old_password, new_password }` |

#### 商品接口（需认证）

| 方法 | 路径 | 说明 | 请求/参数 |
|------|------|------|-----------|
| POST | `/api/products/` | 创建商品 | `{ name, description, price, category_id, stock }` |
| GET | `/api/products/:id` | 查询商品（含缓存） | Path: `id` |
| PUT | `/api/products/:id` | 更新商品 | `{ name?, description?, price?, category_id? }` |
| DELETE | `/api/products/:id` | 删除商品 | Path: `id` |
| GET | `/api/products/` | 商品列表（分页） | Query: `category_id?, min_price?, max_price?, page, page_size` |

#### 订单接口（需认证）

| 方法 | 路径 | 说明 | 请求/参数 |
|------|------|------|-----------|
| POST | `/api/orders/` | 创建订单（Saga 编排） | `{ user_id, items: [{ product_id, quantity, unit_price }] }` |
| GET | `/api/orders/:id` | 查询订单 | Path: `id` |
| GET | `/api/orders/` | 订单列表（分页） | Query: `user_id, page, page_size` |
| PUT | `/api/orders/:id/status` | 更新订单状态 | `{ status }` |

#### 库存接口（需认证）

| 方法 | 路径 | 说明 | 请求/参数 |
|------|------|------|-----------|
| GET | `/api/inventory/:product_id` | 查询库存 | Path: `product_id` |
| POST | `/api/inventory/:product_id/deduct` | 扣减库存 | `{ quantity }` |
| POST | `/api/inventory/:product_id/add` | 增加库存 | `{ quantity }` |

#### 系统接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查（检测后端服务连通性） |
| GET | `/metrics` | Prometheus 指标端点 |

### 统一响应格式

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

### 认证方式

受保护接口需要在请求头中携带 JWT Token：

```
Authorization: Bearer <access_token>
```

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 网关配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[gateway] host` | `127.0.0.1` | HTTP 监听地址 |
| `[gateway] port` | `8080` | HTTP 监听端口 |
| `[gateway] cors_origins` | `["http://localhost:3000", ...]` | CORS 允许的源 |

### 后端服务地址

| 配置项 | 默认端口 | 说明 |
|--------|----------|------|
| `[auth_service]` | `50051` | 认证服务 gRPC 地址 |
| `[product_service]` | `50052` | 商品服务 gRPC 地址 |
| `[order_service]` | `50053` | 订单服务 gRPC 地址 |
| `[inventory_service]` | `50054` | 库存服务 gRPC 地址 |

### Redis 配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[redis] url` | `redis://localhost:6379` | Redis 连接地址（可选，连接失败时降级运行） |

### JWT 配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[jwt] secret` | `your-super-secret-...` | JWT 签名密钥（**生产环境必须覆盖**） |
| `[jwt] expiration_hours` | `1` | Access Token 过期时间 |
| `[jwt] refresh_expiration_hours` | `168` | Refresh Token 过期时间 |

### 限流策略

| 策略 | 限制 | 适用范围 |
|------|------|----------|
| 默认限流 | 60 次/分钟 | 受保护路由（用户/商品/订单/库存） |
| 严格限流 | 10 次/分钟 | 认证接口（注册/登录/刷新） |

### 环境变量

| 环境变量 | 说明 |
|----------|------|
| `APP_JWT__SECRET` | 覆盖 JWT 密钥 |
| `APP_GATEWAY__PORT` | 覆盖网关端口 |
| `APP_REDIS__URL` | 覆盖 Redis 地址 |

## 本地开发

### 前置依赖

- Rust 工具链（stable）
- 后端微服务：auth-service、product-service、order-service、inventory-service
- PostgreSQL 数据库
- Redis（可选，连接失败时降级运行）

### 启动命令

```bash
# 1. 启动 PostgreSQL
docker-compose up -d postgres

# 2. 运行数据库迁移
cargo run --bin migrate

# 3. 启动后端微服务（各开一个终端）
cargo run --bin auth-service
cargo run --bin product-service
cargo run --bin order-service
cargo run --bin inventory-service

# 4. 启动 API 网关
cargo run --bin api-gateway
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| API Gateway | 8080 | HTTP |
| Auth Service | 50051 | gRPC |
| Product Service | 50052 | gRPC |
| Order Service | 50053 | gRPC |
| Inventory Service | 50054 | gRPC |
| PostgreSQL | 5432 | TCP |
| Redis | 6379 | TCP |

## 目录结构

```
services/api-gateway/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：路由组装、中间件链、gRPC 客户端初始化
    ├── routes.rs              # 路由定义、请求/响应 DTO、所有 Handler 实现
    │                          #   - 认证 Handler（register/login/refresh）
    │                          #   - 用户 Handler（get_user/update_password）
    │                          #   - 商品 Handler（CRUD + 缓存降级）
    │                          #   - 订单 Handler（CRUD + Saga 编排）
    │                          #   - 库存 Handler（查询/扣减/增加）
    │                          #   - 健康检查 Handler
    └── grpc_clients.rs        # gRPC 客户端封装（连接池、超时配置）
```
