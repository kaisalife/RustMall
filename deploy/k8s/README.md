# Simple Trade - Kubernetes 部署指南

## 目录结构

```
deploy/k8s/
├── base/                          # 通用基础配置（所有环境共享）
│   ├── namespace.yaml             # Namespace: simple-trade
│   ├── configmap.yaml             # 共享配置（Nacos/Kafka/Redis/DB/JWT）
│   ├── secrets.yaml               # 占位符（<REPLACE_ME>，生产必须替换）
│   ├── external-services.yaml     # ExternalName Service（PG/Redis/Kafka/Nacos）
│   ├── priority-class.yaml        # 3 级优先级（critical/high/standard）
│   ├── network-policy.yaml        # 默认拒绝 + 白名单
│   ├── pod-disruption-budgets.yaml # 7 个 PDB（保证最小可用）
│   ├── services/                  # 8 个微服务 Deployment + Service
│   ├── api-gateway/ingress.yaml   # Nginx Ingress
│   ├── hpa/                       # 3 个 HPA（自动扩缩容）
│   └── kustomization.yaml
│
├── overlays/
│   ├── dev/                       # 开发环境
│   │   ├── kustomization.yaml
│   │   ├── external-services-patch.yaml  # 指向 localhost
│   │   └── config-patch.yaml             # ConfigMap 指向 localhost
│   │
│   └── prod/                      # 生产环境
│       ├── kustomization.yaml
│       ├── external-services-patch.yaml  # 占位符（替换为云托管地址）
│       ├── config-patch.yaml             # 占位符（替换为生产配置）
│       ├── secrets-patch.yaml            # 占位符（替换为真实凭据）
│       └── replicas-patch.yaml           # 提高副本数
│
└── README.md                      # 本文件
```

## 快速部署

### 开发环境

```bash
# 前提：本地已通过 docker-compose 启动 PG/Redis/Kafka/Nacos
kubectl apply -k deploy/k8s/overlays/dev/
```

### 生产环境

**第 1 步：替换配置**

编辑 `deploy/k8s/overlays/prod/` 下的 4 个 patch 文件：

| 文件 | 需要替换的内容 |
|------|---------------|
| `external-services-patch.yaml` | PG/Redis/Kafka/Nacos 的 ExternalName 地址 |
| `config-patch.yaml` | ConfigMap 中的 DB_HOST/REDIS_URL/KAFKA_BROKERS 等 |
| `secrets-patch.yaml` | DB 密码/JWT Secret/SMTP 密码 |
| `replicas-patch.yaml` | 副本数（可选，已有默认值） |

**第 2 步：部署**

```bash
kubectl apply -k deploy/k8s/overlays/prod/
```

**第 3 步：验证**

```bash
kubectl -n simple-trade get pods
kubectl -n simple-trade get hpa
kubectl -n simple-trade get ingress
```

## 设计决策

### 1. 基础设施不在 K8s 内运行

PostgreSQL、Redis、Kafka、Nacos 通过 **ExternalName Service** 指向集群外部。

**原因**：K8s 擅长管理无状态应用，不擅长管理有状态数据库。数据库的备份、主从切换、Point-in-Time 恢复应由云托管服务（AWS RDS、ElastiCache、MSK）处理。

### 2. Kustomize Overlay 分层

- `base/` 包含所有环境共享的配置
- `overlays/dev/` 覆盖地址为 localhost
- `overlays/prod/` 覆盖地址为云托管服务

生产部署只需修改 `overlays/prod/` 下的 4 个文件。

### 3. Secrets 管理

当前使用 `stringData` 占位符 `<REPLACE_ME>`。

**生产环境推荐方案**（按安全等级递增）：
- Sealed Secrets：加密后可安全存入 Git
- External Secrets Operator：从 AWS Secrets Manager / HashiCorp Vault 拉取
- Bitnami Sealed Secrets + ESO 组合

### 4. 可靠性保障

| 机制 | 说明 |
|------|------|
| **PodDisruptionBudget** | 7 个微服务各设 `minAvailable: 1`，节点维护时保证至少 1 个 Pod 可用 |
| **NetworkPolicy** | 默认拒绝所有入站，仅允许 api-gateway 被外部访问、gateway 访问后端、order 访问 inventory |
| **PriorityClass** | 3 级优先级：critical(api-gateway) > high(auth/product/order/inventory) > standard(email/payment/log) |
| **init container** | 每个微服务启动前等待 PostgreSQL 就绪 |
| **滚动更新** | `maxUnavailable: 0, maxSurge: 1`，确保更新期间不减少可用 Pod |
| **HPA** | api-gateway(2-10)、auth/product(2-6)，CPU 70% + Memory 80% 触发扩容 |
| **gRPC Probe** | 后端服务使用 gRPC readiness/liveness probe（K8s 1.24+） |

### 5. 监控栈

监控栈（Prometheus + Loki + Tempo + Grafana）不在 K8s manifests 中。

**推荐安装方式**：
```bash
# 一键安装 kube-prometheus-stack
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install monitoring prometheus-community/kube-prometheus-stack -n monitoring --create-namespace

# 应用微服务通过 /metrics 暴露 Prometheus 指标
# 日志通过 Promtail 采集到 Loki
# 追踪通过 OTLP 发送到 Tempo
```

## 从 CI/CD 部署

GitHub Actions CD 流水线（`.github/workflows/cd.yml`）支持两种部署方式：

### Docker Compose（默认）

打 tag `v*.*.*` 时自动触发，SSH 到服务器执行 `docker-compose up -d`。

### Kubernetes

在 GitHub Secrets 中配置 `KUBECONFIG`（base64 编码的 kubeconfig），CD 会执行：

```bash
kubectl apply -k deploy/k8s/overlays/prod/
```

详见 `.github/workflows/cd.yml` 中的 `deploy-k8s` job。

## 常用命令

```bash
# 预览渲染结果（不实际部署）
kubectl kustomize deploy/k8s/overlays/dev/
kubectl kustomize deploy/k8s/overlays/prod/

# 部署
kubectl apply -k deploy/k8s/overlays/dev/

# 查看资源
kubectl -n simple-trade get all
kubectl -n simple-trade get pdb
kubectl -n simple-trade get networkpolicy

# 查看 HPA 状态
kubectl -n simple-trade get hpa

# 滚动重启
kubectl -n simple-trade rollout restart deployment/api-gateway

# 查看滚动更新状态
kubectl -n simple-trade rollout status deployment/api-gateway

# 删除
kubectl delete -k deploy/k8s/overlays/dev/
```
