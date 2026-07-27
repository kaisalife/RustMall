# Auth Service

## 服务概述

认证授权服务（Auth Service）是 Simple Trade 电商系统的核心安全基础设施，负责用户认证与授权管理。

**核心功能：**
- 用户注册与登录
- JWT 双 Token 机制（Access Token + Refresh Token）
- 用户信息查询
- 密码修改
- 注册时异步发送验证邮件（可选）

**安全特性：**
- 密码策略验证：最少 8 字符，需包含大小写字母和数字
- bcrypt 加密存储用户密码（DEFAULT_COST）
- JWT 双 Token 认证：Access Token（短期）+ Refresh Token（长期）
- 邮箱唯一性校验，防止重复注册

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 编程语言 |
| Tokio | 1.35 | 异步运行时 |
| Tonic | 0.10 | gRPC 框架 |
| SQLx | 0.7 | 数据库 ORM（PostgreSQL） |
| jsonwebtoken | 9.2 | JWT 生成与验证 |
| bcrypt | 0.15 | 密码哈希加密 |
| validator | 0.16 | 输入校验 |
| figment | 0.10 | 配置管理 |
| tracing | 0.1 | 日志与分布式追踪 |
| common (workspace) | - | 公共库（配置、加密、ID 生成等） |
| proto (workspace) | - | gRPC 生成代码 |
| db-migration (workspace) | - | 数据库迁移 |

## 架构设计

本服务采用 DDD（领域驱动设计）四层架构：

```
┌─────────────────────────────────────────┐
│           Interface Layer               │  gRPC 接口实现层
│  (interface/mod.rs - AuthServiceImpl)   │  接收 gRPC 请求，转换为应用层调用
├─────────────────────────────────────────┤
│          Application Layer              │  应用服务层
│  (application/ - AuthApplicationService)│  业务编排：注册、登录、Token 刷新等
│  - service.rs: 核心业务逻辑              │
│  - command.rs: 命令对象（CQRS 写模型）   │
│  - dto.rs: 数据传输对象                 │
├─────────────────────────────────────────┤
│            Domain Layer                 │  领域层
│  (domain/)                              │  核心业务模型与规则
│  - user.rs: User 实体（聚合根）          │
│  - repository.rs: UserRepository 仓储接口 │
├─────────────────────────────────────────┤
│         Infrastructure Layer            │  基础设施层
│  (infrastructure/)                      │  技术实现细节
│  - database.rs: 数据库连接与迁移         │
│  - repository.rs: UserRepositoryImpl     │
│  - email_client.rs: EmailService 客户端  │
└─────────────────────────────────────────┘
```

## API 文档

### gRPC 接口定义

Proto 文件：`proto/auth.proto`，包名 `auth`

| RPC 方法 | 请求消息 | 响应消息 | 说明 |
|----------|----------|----------|------|
| `Register` | `RegisterRequest` | `RegisterResponse` | 用户注册 |
| `Login` | `LoginRequest` | `LoginResponse` | 用户登录，返回双 Token |
| `RefreshToken` | `RefreshTokenRequest` | `LoginResponse` | 刷新 Access Token |
| `GetUser` | `GetUserRequest` | `UserResponse` | 获取用户信息 |
| `UpdatePassword` | `UpdatePasswordRequest` | `UpdatePasswordResponse` | 修改密码 |

### 消息定义

#### RegisterRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| email | string | 用户邮箱 |
| password | string | 密码（需满足密码策略） |
| nickname | string | 用户昵称 |

#### RegisterResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID（Snowflake） |
| email | string | 用户邮箱 |
| nickname | string | 用户昵称 |
| created_at | string | 创建时间（RFC3339） |

#### LoginRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| email | string | 用户邮箱 |
| password | string | 密码 |

#### LoginResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |
| email | string | 用户邮箱 |
| token | string | Access Token（默认 1 小时过期） |
| refresh_token | string | Refresh Token（默认 168 小时/7 天过期） |

#### RefreshTokenRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| refresh_token | string | Refresh Token |

#### GetUserRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |

#### UserResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |
| email | string | 用户邮箱 |
| nickname | string | 用户昵称 |
| created_at | string | 创建时间（RFC3339） |

#### UpdatePasswordRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | uint64 | 用户 ID |
| old_password | string | 旧密码 |
| new_password | string | 新密码（需满足密码策略） |

#### UpdatePasswordResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否修改成功 |

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 服务配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[auth_service] host` | `127.0.0.1` | 服务监听地址 |
| `[auth_service] port` | `50051` | gRPC 监听端口 |
| `[auth_service] worker_id` | `1` | Snowflake ID 生成器工作节点 ID |

### JWT 配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[jwt] secret` | `your-super-secret-...` | JWT 签名密钥（**生产环境必须覆盖**） |
| `[jwt] expiration_hours` | `1` | Access Token 过期时间（小时） |
| `[jwt] refresh_expiration_hours` | `168` | Refresh Token 过期时间（小时，7 天） |

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
| `APP_JWT__SECRET` | 覆盖 JWT 密钥（`__` 表示嵌套层级） |
| `APP_DATABASE__PASSWORD` | 覆盖数据库密码 |
| `SKIP_MIGRATIONS` | 设为 `true` 跳过启动时数据库迁移 |

## 本地开发

### 前置依赖

- Rust 工具链（stable）
- PostgreSQL 数据库
- （可选）email-service，用于注册时发送验证邮件

### 启动命令

```bash
# 1. 启动 PostgreSQL（使用 Docker Compose）
docker-compose up -d postgres

# 2. 运行数据库迁移
cargo run --bin migrate

# 3. 启动认证服务
cargo run --bin auth-service
```

或使用开发脚本：

```powershell
# 一键启动开发环境（PostgreSQL + 迁移）
.\scripts\start-dev.ps1
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| Auth Service | 50051 | gRPC |
| PostgreSQL | 5432 | TCP |
| Email Service（可选） | 50055 | gRPC |

## 目录结构

```
services/auth-service/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：配置加载、依赖注入、gRPC 服务启动
    ├── domain/                # 领域层
    │   ├── mod.rs             # 模块导出
    │   ├── user.rs            # User 聚合根实体
    │   └── repository.rs      # UserRepository 仓储接口（Trait）
    ├── application/           # 应用层
    │   ├── mod.rs             # 模块导出
    │   ├── service.rs         # AuthApplicationService 业务编排
    │   ├── command.rs         # 命令对象（Register/Login/UpdatePassword）
    │   └── dto.rs             # 数据传输对象
    ├── infrastructure/        # 基础设施层
    │   ├── mod.rs             # 模块导出
    │   ├── database.rs        # 数据库连接池与迁移
    │   ├── repository.rs      # UserRepositoryImpl（PostgreSQL 实现）
    │   └── email_client.rs    # EmailService gRPC 客户端封装
    └── interface/             # 接口层
        └── mod.rs             # AuthServiceImpl（gRPC 服务实现）
```
