# oha 基准测试脚本
# 需要先安装 oha: cargo install oha
#
# 用法:
#   .\scripts\bench\bench-gateway.ps1           # 全套压测
#   .\scripts\bench\bench-gateway.ps1 -Quick     # 快速压测（10秒）
#   .\scripts\bench\bench-gateway.ps1 -Stress    # 极限压测

param(
    [switch]$Quick,
    [switch]$Stress
)

$ErrorActionPreference = "Stop"
$BaseUrl = "http://localhost:8080"

function Write-Bench($title, $cmd) {
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "  $title" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "> $cmd" -ForegroundColor Yellow
    Write-Host ""
    Invoke-Expression $cmd
}

# 压测参数
if ($Quick) {
    # 快速压测：10秒，低并发
    $duration = "10s"
    $concurrent = @(10, 50, 100)
    $totalReqs = "10000"
} elseif ($Stress) {
    # 极限压测：持续 30s，高并发
    $duration = "30s"
    $concurrent = @(500, 1000, 2000)
    $totalReqs = "1000000"
} else {
    # 标准压测
    $duration = "15s"
    $concurrent = @(50, 100, 200, 500)
    $totalReqs = "100000"
}

Write-Host @"
============================================
  simple_trade oha 基准测试
  模式: $(if ($Quick) {"快速"} elseif ($Stress) {"极限"} else {"标准"})
  目标: $BaseUrl
============================================
"@ -ForegroundColor Green

# ============================================================
# 1. 网关纯框架开销（/bench/ping）
# ============================================================

Write-Bench "1. 网关纯框架开销 - 单并发基线" "oha -n 1000 -c 1 `"$BaseUrl/bench/ping`""

foreach ($c in $concurrent) {
    Write-Bench "1.$c 网关纯框架开销 - ${c}并发 持续${duration}" "oha -z $duration -c $c `"$BaseUrl/bench/ping`""
}

# ============================================================
# 2. 路径解析开销（/bench/echo/:id）
# ============================================================

Write-Bench "2. 路径参数解析 - 100并发" "oha -n $totalReqs -c 100 `"$BaseUrl/bench/echo/12345`""

# ============================================================
# 3. 健康检查（含 gRPC 并发探测）
# ============================================================

Write-Bench "3. 健康检查端点（含4个gRPC调用）- 50并发" "oha -z 10s -c 50 `"$BaseUrl/health`""

# ============================================================
# 4. 认证接口（POST + JSON 解析）
# ============================================================

Write-Bench "4. 登录接口压测 - 50并发" @"
oha -z 10s -c 50 -m POST -d '{\"email\":\"bench@test.com\",\"password\":\"123456\"}' -H 'Content-Type: application/json' `"$BaseUrl/api/auth/login`"
"@

# ============================================================
# 5. QPS 限流测试
# ============================================================

Write-Bench "5. QPS 限流测试 - 1000 QPS 持续10秒" "oha -z 10s -q 1000 -c 100 `"$BaseUrl/bench/ping`""

# ============================================================
# 汇总
# ============================================================

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  压测完成！查看上方数据。" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host @"
指标解读:
  - Requests/sec:  每秒请求数（越高越好）
  - Latency avg:   平均延迟（越低越好）
  - Latency p99:   99% 请求的延迟（越低越好）
  - p99 < 10ms 为优秀，< 50ms 为良好，> 100ms 需优化
"@ -ForegroundColor Yellow
