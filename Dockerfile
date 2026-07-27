# Build stage - 使用 Ubuntu 基础镜像 + 手动安装 Rust
# 避免下载 1.2GB 的 rust 镜像，ubuntu 镜像仅 77MB
FROM ubuntu:22.04 AS builder

# 避免交互式安装卡住
ENV DEBIAN_FRONTEND=noninteractive

# 安装编译依赖 + Rust 工具链
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.75.0

ENV PATH="/root/.cargo/bin:${PATH}"

ARG SERVICE
WORKDIR /app

# 创建空白项目以缓存依赖
COPY Cargo.toml .
RUN mkdir -p crates/common/src && \
    echo "pub fn placeholder() {}" > crates/common/src/lib.rs && \
    mkdir -p crates/proto/src && \
    echo "pub fn placeholder() {}" > crates/proto/src/lib.rs && \
    mkdir -p crates/tower-middleware/src && \
    echo "pub fn placeholder() {}" > crates/tower-middleware/src/lib.rs && \
    mkdir -p crates/redis-cache/src && \
    echo "pub fn placeholder() {}" > crates/redis-cache/src/lib.rs && \
    mkdir -p crates/metrics/src && \
    echo "pub fn placeholder() {}" > crates/metrics/src/lib.rs && \
    mkdir -p crates/idempotency/src && \
    echo "pub fn placeholder() {}" > crates/idempotency/src/lib.rs && \
    mkdir -p crates/event-bus/src && \
    echo "pub fn placeholder() {}" > crates/event-bus/src/lib.rs && \
    mkdir -p services/api-gateway/src && \
    echo "fn main() {}" > services/api-gateway/src/main.rs && \
    mkdir -p services/auth-service/src && \
    echo "fn main() {}" > services/auth-service/src/main.rs && \
    mkdir -p services/product-service/src && \
    echo "fn main() {}" > services/product-service/src/main.rs && \
    mkdir -p services/order-service/src && \
    echo "fn main() {}" > services/order-service/src/main.rs && \
    mkdir -p services/inventory-service/src && \
    echo "fn main() {}" > services/inventory-service/src/main.rs && \
    mkdir -p services/email-service/src && \
    echo "fn main() {}" > services/email-service/src/main.rs && \
    mkdir -p services/payment-service/src && \
    echo "fn main() {}" > services/payment-service/src/main.rs

# 复制各个 crate 的 Cargo.toml
COPY crates/common/Cargo.toml crates/common/
COPY crates/proto/Cargo.toml crates/proto/
COPY crates/tower-middleware/Cargo.toml crates/tower-middleware/
COPY crates/redis-cache/Cargo.toml crates/redis-cache/
COPY crates/metrics/Cargo.toml crates/metrics/
COPY crates/idempotency/Cargo.toml crates/idempotency/
COPY crates/event-bus/Cargo.toml crates/event-bus/
COPY services/api-gateway/Cargo.toml services/api-gateway/
COPY services/auth-service/Cargo.toml services/auth-service/
COPY services/product-service/Cargo.toml services/product-service/
COPY services/order-service/Cargo.toml services/order-service/
COPY services/inventory-service/Cargo.toml services/inventory-service/
COPY services/email-service/Cargo.toml services/email-service/
COPY services/payment-service/Cargo.toml services/payment-service/

# 复制 proto 文件
COPY proto/ ./proto/

# 构建依赖缓存（首次约 8 分钟，后续缓存命中约 30 秒）
RUN cargo build --release --workspace 2>/dev/null || true

# 复制源代码
COPY . .

# 构建指定服务
RUN cargo build --release --bin ${SERVICE}

# Final stage - 极简运行镜像
FROM debian:bookworm-slim

ARG SERVICE
ENV SERVICE_NAME=${SERVICE}

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY config/ ./config/
COPY --from=builder /app/target/release/${SERVICE} ./app

RUN chmod +x ./app

EXPOSE 8080 50051 50052 50053 50054 50055

CMD ["./app"]
