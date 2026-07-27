# WSL2 编译 + Docker 部署方案
# 适用于 Docker Hub 镜像下不下来的情况
#
# 原理：
#   1. WSL2 是 Linux 环境，可以编译 Linux 二进制
#   2. 只需要拉取 debian-slim（约 40MB），不需要 rust 镜像（1.2GB）
#   3. 编译好的二进制直接丢进 Docker 运行
#
# 前提：
#   - WSL2 已安装（Windows 10/11 自带）
#   - WSL2 中已安装 Rust（curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh）
#   - Docker Desktop 已配置 WSL2 集成

param(
    [string[]]$Services = @("auth-service", "product-service", "order-service", "inventory-service", "email-service", "payment-service", "api-gateway")
)

$ErrorActionPreference = "Stop"
Write-Host "=== WSL2 编译 + Docker 部署 ===" -ForegroundColor Cyan

# ============================================================
# 步骤 1: 在 WSL2 中编译所有服务
# ============================================================

Write-Host "`n[1/3] 在 WSL2 中编译 Rust 项目..." -ForegroundColor Yellow

$wslCommands = @"
cd /mnt/d/Coding/simple_trade

# 确保 Rust 已安装
if ! command -v cargo &> /dev/null; then
    echo '安装 Rust...'
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# 安装编译依赖
sudo apt-get update -qq
sudo apt-get install -y -qq protobuf-compiler cmake pkg-config build-essential libssl-dev 2>/dev/null

# 编译
echo '开始编译（release 模式，约 5-8 分钟）...'
cargo build --release --workspace

echo '编译完成！'
ls -la target/release/auth-service target/release/product-service target/release/order-service target/release/inventory-service target/release/email-service target/release/payment-service target/release/api-gateway
"@

wsl bash -c $wslCommands
if ($LASTEXITCODE -ne 0) {
    Write-Host "编译失败，请检查 WSL2 环境" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] 编译完成" -ForegroundColor Green

# ============================================================
# 步骤 2: 构建极简 Docker 镜像（只拉 debian-slim 40MB）
# ============================================================

Write-Host "`n[2/3] 构建极简 Docker 镜像..." -ForegroundColor Yellow

# 先拉取 debian-slim（很小，约 40MB）
Write-Host "拉取 debian-slim 镜像（约 40MB）..."
docker pull docker.1ms.run/library/debian:bookworm-slim 2>$null
if ($LASTEXITCODE -ne 0) {
    docker pull debian:bookworm-slim
}
docker tag docker.1ms.run/library/debian:bookworm-slim debian:bookworm-slim 2>$null

# 为每个服务构建镜像
foreach ($svc in $Services) {
    Write-Host "  构建 $svc ..."
    docker build -f Dockerfile.simple -t "simple-trade/${svc}:latest" --build-arg "SERVICE=${svc}" .
}

Write-Host "[OK] 镜像构建完成" -ForegroundColor Green

# ============================================================
# 步骤 3: 启动服务
# ============================================================

Write-Host "`n[3/3] 启动 Docker 服务..." -ForegroundColor Yellow

# 先启动基础设施
docker-compose up -d postgres redis kafka
Write-Host "等待基础设施就绪..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# 启动所有微服务
docker-compose up -d

Write-Host "`n=== 部署完成 ===" -ForegroundColor Green
Write-Host "API Gateway: http://localhost:8080" -ForegroundColor Cyan
Write-Host "查看状态: docker-compose ps" -ForegroundColor Cyan
