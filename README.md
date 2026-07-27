# Simple Trade - Rust 微服务电商平台

基于 DDD 领域驱动设计的 Rust 微服务电商平台，使用 axum、tonic、sqlx 等现代 Rust 技术栈构建。

---

## ✨ 核心特性

- **🦀 Rust 技术栈** - 编译期内存安全、极致性能
- **🏗️ DDD 分层架构** - 领域层、基础设施层、应用层、接口层
- **🔗 gRPC 通信** - 微服务之间使用 Protobuf + gRPC 通信
- **💽 SQLx 编译期 ORM** - 编译时 SQL 语法检查与类型校验
- **🔄 嵌入式数据库迁移** - sqlx::migrate! 编译时嵌入迁移脚本
- **❄️ 雪花算法 ID** - 分布式唯一 ID 生成
- **🔐 JWT 认证** - 无状态认证
- **🔍 统一错误处理** - 结构化错误类型

---

## 🚀 快速开始

### 前置要求

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| **Rust** | 1.70+ | 建议使用 rustup 安装 |
| **Docker Desktop** | 最新版 | 运行 PostgreSQL 数据库 |

> 💡 安装 Rust: https://www.rust-lang.org/tools/install

---

### 一键启动开发环境

```powershell
# 进入项目根目录
cd simple_trade

# 一键启动（PostgreSQL + pgAdmin + 自动运行迁移）
.\scripts\start-dev.ps1
```

启动成功后会显示：
- ✅ PostgreSQL 连接信息
- ✅ 已应用的数据库迁移列表
- ✅ pgAdmin Web UI 访问地址

---

## 📦 项目结构

```
simple_trade/
├── crates/                      # 公共 Crates
│   ├── common/                 # 通用工具
│   │   ├── src/
│   │   │   ├── id.rs          # 雪花算法 ID 生成器
│   │   │   ├── error.rs       # 统一错误处理
│   │   │   ├── config.rs      # 配置加载 (figment)
│   │   │   └── crypto.rs      # 密码/JWT 工具
│   ├── db-migration/          # 数据库迁移工具
│   │   ├── migrations/        # SQL 迁移脚本
│   │   │   ├── V1__initial_schema.sql
│   │   │   └── V1__initial_schema.down.sql
│   │   └── src/
│   │       ├── lib.rs         # 迁移库函数
│   │       └── main.rs        # 命令行工具
│   └── tower-middleware/      # 中间件
│       └── src/
│           ├── logger.rs
│           ├── rate_limit.rs
│           └── auth.rs
├── services/                    # 微服务
│   ├── api-gateway/           # API 网关 (axum)
│   ├── auth-service/          # 用户认证服务 (gRPC)
│   ├── product-service/       # 商品服务 (gRPC)
│   ├── order-service/         # 订单服务 (gRPC)
│   ├── inventory-service/     # 库存服务 (gRPC)
│   └── email-service/         # 邮件服务 (gRPC)
├── proto/                      # gRPC Proto 定义
├── config/                     # 配置文件
│   └── base.toml
├── scripts/                    # 工具脚本
│   └── start-dev.ps1
├── docker-compose.yml          # Docker 编排
└── Cargo.toml                  # 工作空间配置
```

---

## 💾 数据库迁移系统

### 迁移文件位置

迁移文件位于 `crates/db-migration/migrations/`，遵循 sqlx 命名规范：

```
V{version}__{name}.sql        # Up 迁移
V{version}__{name}.down.sql   # Down 迁移（可选）
```

### 常用命令

```powershell
# ✅ 运行所有待执行的迁移
cargo run --bin migrate

# ✅ 使用自定义 DATABASE_URL 运行
$env:DATABASE_URL='postgres://user:pass@host:5432/db'
cargo run --bin migrate

# ✅ 启动服务时自动运行迁移（默认）
cargo run --bin auth-service

# ⏭️ 启动服务时跳过迁移
$env:SKIP_MIGRATIONS='true'
cargo run --bin auth-service
```

### 工作原理

1. **编译期嵌入** - `sqlx::migrate!("./migrations")` 宏在编译时将所有 `.sql` 文件嵌入二进制
2. **自动版本追踪** - 首次运行自动创建 `_sqlx_migrations` 表追踪已应用的迁移
3. **幂等执行** - 重复执行不会出错，仅执行未应用的迁移

---

## 🗄️ 数据库表结构

启动迁移后自动创建以下表：

| 表名 | 说明 |
|------|------|
| `users` | 用户表（邮箱、密码哈希、昵称） |
| `categories` | 商品分类表（树状结构） |
| `products` | 商品表（名称、描述、价格、库存） |
| `inventory` | 库存表（可预留库存） |
| `orders` | 订单表（订单状态、总金额） |
| `order_items` | 订单商品项表 |
| `_sqlx_migrations` | sqlx 迁移版本追踪表（自动创建） |

---

## 🎯 本地开发流程

### 完整启动流程

```powershell
# 1. 启动数据库并运行迁移
.\scripts\start-dev.ps1

# 2. 启动认证服务（端口 50051）
cargo run --bin auth-service

# 3. 启动商品服务（端口 50052）
cargo run --bin product-service

# 4. 启动 API 网关（端口 8080）
cargo run --bin api-gateway
```

### 独立迁移命令

```powershell
# 仅运行数据库迁移（无需启动服务）
cargo run --bin migrate
```

### pgAdmin 管理界面

启动后访问 http://localhost:5050：
- **邮箱**: admin@simple-trade.com
- **密码**: admin123
- **数据库连接**: 主机名 `postgres`，端口 `5432`

---

## 🛠️ 添加新的数据库迁移

需要修改表结构时，创建新的迁移文件：

```powershell
# 1. 在 crates/db-migration/migrations/ 下创建
V2__add_new_column.sql
V2__add_new_column.down.sql  # 可选（用于回滚）
```

**V2__add_new_column.sql**
```sql
ALTER TABLE users ADD COLUMN phone VARCHAR(20);
```

**V2__add_new_column.down.sql**
```sql
ALTER TABLE users DROP COLUMN phone;
```

然后运行迁移：
```powershell
cargo run --bin migrate
```

---

## ⚙️ 配置说明

配置文件位于 `config/base.toml`，支持环境变量覆盖：

```toml
[database]
host = "localhost"
port = 5432
username = "postgres"
password = "postgres"
database = "simple_trade"

[jwt]
secret = "your-super-secret-key"
expiration_hours = 24
```

或者使用环境变量：
```powershell
$env:DATABASE_URL = "postgres://user:pass@localhost:5432/simple_trade"
```

---

## 📝 添加新服务

遵循以下步骤添加新的微服务：

1. 创建 `services/new-service/` 目录
2. 实现 DDD 四层架构
3. 在 `proto/` 中添加 `.proto` 定义
4. 更新 `Cargo.toml` 添加依赖
5. 添加 `db-migration` 依赖以支持迁移
6. 在 `main.rs` 中使用 `DatabaseConnection::new_with_migration()`

---

## 🔍 常见问题

### Q: 如何重置数据库？
```powershell
# 停止并删除数据库卷
docker-compose down -v

# 重新启动（会自动重新运行迁移）
.\scripts\start-dev.ps1
```

### Q: 迁移失败怎么办？
```powershell
# 1. 查看详细错误
cargo run --bin migrate 2>&1

# 2. 重置数据库后重试
docker-compose down -v
docker-compose up -d
cargo run --bin migrate
```

### Q: 如何查看已应用的迁移？
```sql
-- 连接到数据库后执行
SELECT version, description, applied_at
FROM _sqlx_migrations
ORDER BY version;
```

---

## 📚 学习资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [SQLx 文档](https://docs.rs/sqlx)
- [Tonic (gRPC) 文档](https://docs.rs/tonic)
- [Axum 文档](https://docs.rs/axum)
- [DDD 领域驱动设计参考](https://martinfowler.com/tags/domain%20driven%20design.html)

---

## 🤝 贡献指南

1. Fork 本仓库
2. 创建特性分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

---

## 📄 License

MIT License

---

**Happy coding with Rust! 🦀**
