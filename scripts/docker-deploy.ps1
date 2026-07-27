# simple_trade Docker 一键部署脚本
# 用法：
#   .\scripts\docker-deploy.ps1              # 构建并启动所有服务
#   .\scripts\docker-deploy.ps1 -Build       # 只构建镜像
#   .\scripts\docker-deploy.ps1 -Up          # 只启动服务
#   .\scripts\docker-deploy.ps1 -Down        # 停止所有服务
#   .\scripts\docker-deploy.ps1 -Logs        # 查看所有日志
#   .\scripts\docker-deploy.ps1 -Clean       # 停止并删除所有镜像和数据
#   .\scripts\docker-deploy.ps1 -Service auth-service  # 只操作单个服务

param(
    [switch]$Build,
    [switch]$Up,
    [switch]$Down,
    [switch]$Logs,
    [switch]$Clean,
    [string]$Service = ""
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
    Write-Host "`n=== $msg ===" -ForegroundColor Cyan
}

function Write-Ok($msg) {
    Write-Host "[OK] $msg" -ForegroundColor Green
}

function Write-Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
}

# 显示帮助
if (-not $Build -and -not $Up -and -not $Down -and -not $Logs -and -not $Clean) {
    Write-Host @"
simple_trade Docker 部署脚本

用法:
  .\scripts\docker-deploy.ps1              构建并启动所有服务
  .\scripts\docker-deploy.ps1 -Build       只构建镜像
  .\scripts\docker-deploy.ps1 -Up          只启动服务
  .\scripts\docker-deploy.ps1 -Down        停止所有服务
  .\scripts\docker-deploy.ps1 -Logs        查看所有日志
  .\scripts\docker-deploy.ps1 -Clean       停止并删除所有镜像和数据
  .\scripts\docker-deploy.ps1 -Service auth-service  只操作单个服务
"@ -ForegroundColor Yellow
    exit 0
}

# 清理
if ($Clean) {
    Write-Step "清理所有容器、镜像和数据卷"
    docker-compose down -v --rmi all
    Write-Ok "清理完成"
    exit 0
}

# 停止
if ($Down) {
    Write-Step "停止所有服务"
    docker-compose down
    Write-Ok "所有服务已停止"
    exit 0
}

# 查看日志
if ($Logs) {
    if ($Service) {
        Write-Step "查看 $Service 日志"
        docker-compose logs -f $Service
    } else {
        Write-Step "查看所有日志"
        docker-compose logs -f
    }
    exit 0
}

# 构建
if ($Build -or (-not $Up -and -not $Down -and -not $Logs)) {
    if ($Service) {
        Write-Step "构建 $Service"
        docker-compose build $Service
    } else {
        Write-Step "构建所有镜像"
        docker-compose build
    }
    if ($LASTEXITCODE -ne 0) {
        Write-Err "构建失败"
        exit 1
    }
    Write-Ok "构建完成"
}

# 启动
if ($Up -or (-not $Build -and -not $Up -and -not $Down -and -not $Logs)) {
    if (-not $Service) {
        Write-Step "启动基础设施（PostgreSQL + Redis + Kafka）"
        docker-compose up -d postgres redis kafka
        Write-Host "等待基础设施就绪..." -ForegroundColor Yellow
        Start-Sleep -Seconds 10
        Write-Ok "基础设施已启动"
    }

    Write-Step "启动服务"
    if ($Service) {
        docker-compose up -d $Service
    } else {
        docker-compose up -d
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Err "启动失败"
        exit 1
    }

    Write-Step "服务状态"
    docker-compose ps
    Write-Ok "部署完成"
    Write-Host "`nAPI Gateway: http://localhost:8080" -ForegroundColor Yellow
    Write-Host "Kafka UI:    http://localhost:9090" -ForegroundColor Yellow
    Write-Host "pgAdmin:     http://localhost:5050" -ForegroundColor Yellow
}
