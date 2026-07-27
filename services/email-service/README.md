# Email Service

## 服务概述

邮件发送服务（Email Service）是 Simple Trade 电商系统的通知领域服务，负责各类邮件的发送与日志记录。

**核心功能：**
- 发送验证码邮件（用户注册）
- 发送订单通知邮件（订单状态变更）
- 发送密码重置邮件
- 发送自定义邮件
- 邮件日志持久化（记录发送状态与结果）
- 开发模式控制台输出（无需 SMTP 配置）

**邮件状态管理：**
- `Pending`（待发送）-> `Sent`（发送成功）/ `Failed`（发送失败）

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 编程语言 |
| Tokio | 1.35 | 异步运行时 |
| Tonic | 0.10 | gRPC 框架 |
| SQLx | 0.7 | 数据库 ORM（PostgreSQL） |
| Lettre | 0.11 | SMTP 邮件发送库 |
| uuid | 1.6 | UUID 生成 |
| chrono | 0.4 | 时间处理 |
| figment | 0.10 | 配置管理 |
| tracing | 0.1 | 日志与分布式追踪 |
| common (workspace) | - | 公共库（配置、ID 生成等） |
| db-migration (workspace) | - | 数据库迁移 |
| proto (workspace) | - | gRPC 生成代码 |

## 架构设计

本服务采用 DDD（领域驱动设计）四层架构：

```
┌─────────────────────────────────────────┐
│           Interface Layer               │  gRPC 接口实现层
│  (interface/mod.rs - EmailServiceImpl)  │  接收 gRPC 请求，转换为应用层调用
├─────────────────────────────────────────┤
│          Application Layer              │  应用服务层
│  (application/ - EmailApplicationSvc)   │  业务编排：邮件创建、发送、状态更新
│  - service.rs: 核心业务逻辑              │
├─────────────────────────────────────────┤
│            Domain Layer                 │  领域层
│  (domain/)                              │  核心业务模型与规则
│  - model.rs: Email 实体、EmailType、     │
│    EmailStatus 枚举、HTML 模板生成       │
│  - repository.rs: EmailRepository 接口   │
├─────────────────────────────────────────┤
│         Infrastructure Layer            │  基础设施层
│  (infrastructure/)                      │  技术实现细节
│  - email_sender.rs: EmailSender         │
│    （SMTP 发送 + 开发模式控制台输出）     │
│  - repository.rs: EmailRepositoryImpl   │
│    （PostgreSQL 邮件日志持久化）          │
└─────────────────────────────────────────┘
```

## API 文档

### gRPC 接口定义

Proto 文件：`proto/email.proto`，包名 `email`

| RPC 方法 | 请求消息 | 响应消息 | 说明 |
|----------|----------|----------|------|
| `SendVerificationEmail` | `SendVerificationEmailRequest` | `SendEmailResponse` | 发送验证码邮件 |
| `SendOrderNotification` | `SendOrderNotificationRequest` | `SendEmailResponse` | 发送订单通知邮件 |
| `SendPasswordResetEmail` | `SendPasswordResetEmailRequest` | `SendEmailResponse` | 发送密码重置邮件 |
| `SendCustomEmail` | `SendCustomEmailRequest` | `SendEmailResponse` | 发送自定义邮件 |

### 消息定义

#### SendVerificationEmailRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| to_email | string | 收件人邮箱 |
| username | string | 用户名 |
| verification_code | string | 验证码 |

#### SendOrderNotificationRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| to_email | string | 收件人邮箱 |
| username | string | 用户名 |
| order_id | uint64 | 订单 ID |
| total_amount | double | 订单总金额 |
| status | string | 订单状态 |

#### SendPasswordResetEmailRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| to_email | string | 收件人邮箱 |
| username | string | 用户名 |
| reset_token | string | 密码重置令牌 |

#### SendCustomEmailRequest
| 字段 | 类型 | 说明 |
|------|------|------|
| to_email | string | 收件人邮箱 |
| subject | string | 邮件主题 |
| html_content | string | HTML 邮件内容 |
| username | optional string | 用户名（可选） |

#### SendEmailResponse
| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | 是否发送成功 |
| message_id | string | 邮件消息 ID |

## 配置说明

配置文件位于 `config/base.toml`，支持通过环境变量（`APP_` 前缀）覆盖。

### 服务配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[email_service] host` | `127.0.0.1` | 服务监听地址 |
| `[email_service] port` | `50055` | gRPC 监听端口 |
| `[email_service] worker_id` | `5` | Snowflake ID 生成器工作节点 ID |

### 邮件配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `[email] smtp_host` | `smtp.gmail.com` | SMTP 服务器地址 |
| `[email] smtp_port` | `587` | SMTP 服务器端口 |
| `[email] smtp_username` | `your-email@gmail.com` | SMTP 用户名 |
| `[email] smtp_password` | `your-app-password` | SMTP 密码（**生产环境必须覆盖**） |
| `[email] from_address` | `no-reply@simple-trade.com` | 发件人地址 |

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
| `APP_EMAIL__SMTP_PASSWORD` | 覆盖 SMTP 密码 |
| `APP_DATABASE__PASSWORD` | 覆盖数据库密码 |

### 开发模式

在 debug 编译模式下（`cfg!(debug_assertions)`），服务自动使用模拟邮件发送器，邮件内容输出到控制台，无需配置真实 SMTP 服务器。生产编译模式下使用真实 SMTP 发送。

## 本地开发

### 前置依赖

- Rust 工具链（stable）
- PostgreSQL 数据库
- （生产模式）SMTP 服务器

### 启动命令

```bash
# 1. 启动 PostgreSQL
docker-compose up -d postgres

# 2. 运行数据库迁移
cargo run --bin migrate

# 3. 启动邮件服务
cargo run --bin email-service
```

### 端口信息

| 服务 | 端口 | 协议 |
|------|------|------|
| Email Service | 50055 | gRPC |
| PostgreSQL | 5432 | TCP |
| SMTP（生产模式） | 587 | TCP |

## 目录结构

```
services/email-service/
├── Cargo.toml                 # 包配置与依赖
├── README.md                  # 本文档
└── src/
    ├── main.rs                # 服务入口：配置加载、依赖注入、gRPC 服务启动
    ├── lib.rs                 # 库入口
    ├── domain/                # 领域层
    │   ├── mod.rs             # 模块导出
    │   ├── model.rs           # Email 实体、EmailType/EmailStatus 枚举、HTML 模板
    │   └── repository.rs      # EmailRepository 仓储接口（Trait）
    ├── application/           # 应用层
    │   ├── mod.rs             # 模块导出
    │   └── service.rs         # EmailApplicationService 业务编排
    ├── infrastructure/        # 基础设施层
    │   ├── mod.rs             # 模块导出
    │   ├── email_sender.rs    # EmailSender（SMTP + 开发模式控制台输出）
    │   └── repository.rs      # EmailRepositoryImpl（PostgreSQL 日志持久化）
    └── interface/             # 接口层
        └── mod.rs             # EmailServiceImpl（gRPC 服务实现）
```
