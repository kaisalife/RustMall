# Simple Trade — Kubernetes 部署

本目录包含 simple_trade 项目的完整 Kubernetes 编排 manifests，使用 Kustomize 进行一键部署。

## 目录结构

```
deploy/k8s/
├── namespace.yaml              # Namespace 定义
├── configmap.yaml              # 全局非敏感配置
├── secrets.yaml                # 敏感配置（base64 编码，生产环境务必替换）
├── kustomization.yaml          # Kustomize 入口
├── postgres/                   # PostgreSQL 15 StatefulSet + PVC
├── redis/                      # Redis 7 Deployment
├── kafka/                      # Kafka 3.7 (KRaft 模式) Deployment
├── nacos/                      # Nacos 2.4 (standalone) Deployment
├── auth-service/               # 认证服务 (gRPC :50051, 2 副本)
├── product-service/            # 商品服务 (gRPC :50052, 2 副本)
├── order-service/              # 订单服务 (gRPC :50053, 2 副本)
├── inventory-service/          # 库存服务 (gRPC :50054, 2 副本)
├── email-service/              # 邮件服务 (gRPC :50055, 1 副本)
├── payment-service/            # 支付服务 (gRPC :50056, 1 副本)
├── api-gateway/                # API 网关 (HTTP :8080, 2 副本, LoadBalancer + Ingress)
├── hpa/                        # 水平 Pod 自动扩缩容
└── README.md                   # 本文件
```

## 前置条件

1. **Kubernetes 集群** — 1.24+（gRPC probe 稳定特性）
2. **kubectl** — 已配置集群访问
3. **metrics-server** — 集群中已安装（HPA 依赖）
4. **Ingress Controller** — 如 nginx-ingress（可选，用于 Ingress 路由）
5. **GHCR 镜像访问** — 集群节点可拉取 `ghcr.io/kaisalife/rustmall/*` 镜像

## 快速部署

```bash
# 一键部署所有资源
kubectl apply -k deploy/k8s/

# 或使用 kustomize CLI
kustomize build deploy/k8s/ | kubectl apply -f -
```

## 查看状态

```bash
# 查看所有资源
kubectl get all -n simple-trade

# 查看 Pod 状态
kubectl get pods -n simple-trade -o wide

# 查看服务
kubectl get svc -n simple-trade

# 查看 Ingress
kubectl get ingress -n simple-trade

# 查看 HPA 状态
kubectl get hpa -n simple-trade

# 查看 ConfigMap 和 Secret
kubectl get configmap,secret -n simple-trade

# 查看 PVC
kubectl get pvc -n simple-trade

# 查看某个 Pod 的日志
kubectl logs -f -n simple-trade deployment/api-gateway

# 进入 Pod 调试
kubectl exec -it -n simple-trade deployment/api-gateway -- /bin/bash
```

## 访问服务

### 通过 LoadBalancer

```bash
# 获取 api-gateway 的外部 IP
kubectl get svc api-gateway -n simple-trade
# 使用 EXTERNAL-IP:8080 访问
```

### 通过 Ingress

将域名 `api.simple-trade.com` 解析到 Ingress Controller 的 LoadBalancer IP，然后访问 `http://api.simple-trade.com`。

### 通过 port-forward（本地调试）

```bash
kubectl port-forward -n simple-trade svc/api-gateway 8080:8080
# 访问 http://localhost:8080
```

## 配置说明

### 环境变量映射

项目使用 [figment](https://crates.io/crates/figment) 加载配置，环境变量前缀为 `APP_`，分隔符为 `__`：

| 环境变量                          | 配置路径                |
| --------------------------------- | ----------------------- |
| `APP_DATABASE__HOST`              | `database.host`         |
| `APP_AUTH_SERVICE__PORT`          | `auth_service.port`     |
| `APP_NACOS__SERVER_ADDR`          | `nacos.server_addr`     |
| `APP_KAFKA__BROKERS`              | `kafka.brokers`         |

### ConfigMap（非敏感配置）

`configmap.yaml` 包含所有服务共享的配置：Nacos 地址、Kafka brokers、Redis URL、Tracing endpoint、worker_id、JWT 过期时间、限流开关等。

### Secrets（敏感配置）

`secrets.yaml` 包含 base64 编码的敏感数据：

| Secret Key                | 说明                  | 默认值（需替换）      |
| ------------------------- | --------------------- | --------------------- |
| `APP_DATABASE__USERNAME`  | PostgreSQL 用户名      | `postgres`            |
| `APP_DATABASE__PASSWORD`  | PostgreSQL 密码        | `postgres`            |
| `APP_DATABASE__DATABASE`  | PostgreSQL 数据库名    | `simple_trade`        |
| `APP_JWT__SECRET`         | JWT 签名密钥           | 需替换为强随机值      |
| `APP_EMAIL__SMTP_PASSWORD`| SMTP 邮箱密码          | 需替换为实际密码      |

> **生产环境务必替换所有 Secret 值！**
>
> ```bash
> # 生成 base64 编码的值
> echo -n 'your-password' | base64
> ```

### 各服务数据库连接池大小

| 服务                | max_connections |
| ------------------- | --------------- |
| auth-service        | 20              |
| product-service     | 40              |
| order-service       | 30              |
| inventory-service   | 30              |
| email-service       | 10              |
| payment-service     | 10              |
| api-gateway         | 50（配置默认值） |

## HPA 自动扩缩容

| 服务             | 最小副本 | 最大副本 | 扩缩容指标               |
| ---------------- | -------- | -------- | ------------------------ |
| api-gateway      | 2        | 10       | CPU 70% + Memory 80%    |
| auth-service     | 2        | 6        | CPU 70%                  |
| product-service  | 2        | 6        | CPU 70%                  |

> HPA 依赖集群中已安装 metrics-server。

## 更新部署

```bash
# 更新镜像版本（以 auth-service 为例）
kubectl set image deployment/auth-service \
  auth-service=ghcr.io/kaisalife/rustmall/auth-service:v1.2.0 \
  -n simple-trade

# 滚动更新状态
kubectl rollout status deployment/auth-service -n simple-trade

# 回滚
kubectl rollout undo deployment/auth-service -n simple-trade
```

## 清理

```bash
# 删除所有资源（保留 Namespace 外的 PV）
kubectl delete -k deploy/k8s/

# 强制删除 Namespace（包括所有资源）
kubectl delete namespace simple-trade
```

## 注意事项

1. **gRPC Probe** — 微服务使用 gRPC 健康检查，需要 Kubernetes 1.24+
2. **镜像拉取** — Docker 镜像从 GHCR 拉取，确保集群节点有访问权限
3. **PostgreSQL 数据** — 使用 PVC 持久化，删除 StatefulSet 不会删除 PVC
4. **Kafka KRaft** — 单节点 KRaft 模式，适合开发/测试，生产环境建议使用多副本
5. **Nacos standalone** — 单节点模式，生产环境建议集群部署
6. **可观测性** — Tracing endpoint 默认指向 `tempo:4317`，需单独部署可观测性栈
