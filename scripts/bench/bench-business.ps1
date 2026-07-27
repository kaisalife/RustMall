# 真实业务接口压测脚本
# 包含数据初始化 + 多场景压测
#
# 用法:
#   .\scripts\bench\bench-business.ps1                # 标准压测
#   .\scripts\bench\bench-business.ps1 -Quick         # 快速压测
#   .\scripts\bench\bench-business.ps1 -InitOnly      # 只初始化数据

param(
    [switch]$Quick,
    [switch]$InitOnly
)

$ErrorActionPreference = "Stop"
$BaseUrl = "http://localhost:8080"

# 测试账号
$TestEmail = "bench@test.com"
$TestPassword = "Bench@123456"
$TestNickname = "BenchUser"

# 压测参数
if ($Quick) {
    $Duration = "5s"
    $Concurrent = @(10, 50)
} else {
    $Duration = "10s"
    $Concurrent = @(50, 100, 200)
}

function Write-Step($msg) {
    Write-Host "`n=== $msg ===" -ForegroundColor Cyan
}

function Write-Ok($msg) {
    Write-Host "[OK] $msg" -ForegroundColor Green
}

function Write-Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
}

# ============================================================
# 阶段 1: 数据初始化
# ============================================================

Write-Step "阶段 1: 数据初始化"

# 1.1 注册用户
Write-Host "注册测试用户..." -ForegroundColor Yellow
$registerBody = @{ email = $TestEmail; password = $TestPassword; nickname = $TestNickname } | ConvertTo-Json
try {
    $registerResult = Invoke-RestMethod -Uri "$BaseUrl/api/auth/register" -Method POST -Body $registerBody -ContentType "application/json" -ErrorAction Stop
    Write-Ok "用户注册成功: $($registerResult.data.user_id)"
} catch {
    if ($_.Exception.Response.StatusCode.value__ -eq 409) {
        Write-Host "用户已存在，继续..." -ForegroundColor Yellow
    } else {
        Write-Host "注册失败（非 409），继续尝试登录... 错误: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

# 1.2 登录获取 token
Write-Host "登录获取 JWT token..." -ForegroundColor Yellow
$loginBody = @{ email = $TestEmail; password = $TestPassword } | ConvertTo-Json
$loginResult = Invoke-RestMethod -Uri "$BaseUrl/api/auth/login" -Method POST -Body $loginBody -ContentType "application/json" -ErrorAction Stop
$Token = $loginResult.data.token
$UserId = $loginResult.data.user_id
$TokenPreview = if ($Token) { $Token.Substring(0, [Math]::Min(20, $Token.Length)) } else { "<empty>" }
Write-Ok "登录成功，user_id=$UserId, token=$TokenPreview..."

# 1.2.1 确保测试分类存在（categories 无 HTTP/gRPC 接口，直接走 DB）
# 注意：服务跑在 Docker 里，连的是容器内 postgres；宿主机 5432 可能被本地 postgres 占用
# 所以用 docker exec 进入容器执行 psql，确保命中同一个 DB
$CategoryId = 1
try {
    $sql = "INSERT INTO categories (id, name, parent_id, created_at, updated_at) VALUES ($CategoryId, 'Bench Category', NULL, NOW(), NOW()) ON CONFLICT (id) DO NOTHING;"
    $dockerOk = docker exec simple_trade_postgres psql -U postgres -d simple_trade -c $sql 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Ok "分类已就绪: category_id=$CategoryId"
    } else {
        Write-Host "docker exec 失败: $dockerOk" -ForegroundColor Yellow
        Write-Host "将尝试宿主机 psql（可能命中错误的 DB 实例）..." -ForegroundColor Yellow
        $env:PGPASSWORD = "postgres"
        psql -h 127.0.0.1 -p 5432 -U postgres -d simple_trade -c $sql 2>&1 | Out-Null
        Remove-Item Env:\PGPASSWORD -ErrorAction SilentlyContinue
        if ($LASTEXITCODE -eq 0) { Write-Ok "分类已就绪(宿主机): category_id=$CategoryId" }
    }
} catch {
    Write-Host "分类初始化异常: $($_.Exception.Message)" -ForegroundColor Yellow
}

# 1.3 创建商品
Write-Host "创建测试商品..." -ForegroundColor Yellow
$productBody = @{
    name = "Bench Test Product"
    description = "压测专用商品"
    price = 99.99
    category_id = $CategoryId
    stock = 100000
} | ConvertTo-Json
$productResult = Invoke-RestMethod -Uri "$BaseUrl/api/products" -Method POST -Body $productBody -ContentType "application/json" -Headers @{ Authorization = "Bearer $Token" } -ErrorAction SilentlyContinue
if ($productResult) {
    $ProductId = $productResult.data.product_id
    Write-Ok "商品创建成功: product_id=$ProductId"
} else {
    Write-Host "商品创建失败，使用默认 ID=1" -ForegroundColor Yellow
    $ProductId = 1
}

# 1.4 添加库存
Write-Host "添加库存..." -ForegroundColor Yellow
$stockBody = @{ quantity = 1000000 } | ConvertTo-Json
try {
    Invoke-RestMethod -Uri "$BaseUrl/api/inventory/$ProductId/add" -Method POST -Body $stockBody -ContentType "application/json" -Headers @{ Authorization = "Bearer $Token" } -ErrorAction Stop | Out-Null
    Write-Ok "库存添加成功"
} catch {
    Write-Host "库存添加跳过（可能已存在）" -ForegroundColor Yellow
}

if ($InitOnly) {
    Write-Ok "数据初始化完成，退出"
    exit 0
}

# ============================================================
# 阶段 2: 压测
# ============================================================

# 报告输出目录与结果收集
$ReportDir = Join-Path $PSScriptRoot "..\..\reports\bench"
if (-not (Test-Path $ReportDir)) { New-Item -ItemType Directory -Path $ReportDir -Force | Out-Null }
$ReportTimestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$script:Results = @()

Write-Host @"
`n============================================
  真实业务接口压测
  目标: $BaseUrl
  用户: $TestEmail (ID: $UserId)
  商品: ID=$ProductId
  持续: $Duration per scenario
============================================
"@ -ForegroundColor Green

function Run-Bench($title, $cmd) {
    Write-Host "`n----------------------------------------" -ForegroundColor Cyan
    Write-Host "  $title" -ForegroundColor Cyan
    Write-Host "----------------------------------------" -ForegroundColor Cyan
    # 用 JSON 输出便于解析指标；合并多行为单行后追加 --output-format json
    $cmdNorm = (($cmd -split "`n").Trim() -join " ")
    $jsonCmd = "$cmdNorm --output-format json"
    $raw = Invoke-Expression $jsonCmd 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $raw) {
        Write-Err "压测失败或无输出 (exit=$LASTEXITCODE)"
        $script:Results += [pscustomobject]@{ Title=$title; Ok=$false; RPS="-"; OkRps="-"; OkRate="-"; Avg="-"; P50="-"; P95="-"; P99="-"; Codes="-" }
        return
    }
    try { $data = $raw | ConvertFrom-Json }
    catch { Write-Err "JSON 解析失败: $_"; return }

    $lat = $data.metrics.latency_ms
    $rps = [math]::Round($data.summary.requestsPerSec, 1)
    $p50 = if ($lat.p50) { [math]::Round($lat.p50, 2) } else { "-" }
    $p95 = if ($lat.p95) { [math]::Round($lat.p95, 2) } else { "-" }
    $p99 = if ($lat.p99) { [math]::Round($lat.p99, 2) } else { "-" }
    $avg = if ($lat.mean) { [math]::Round($lat.mean, 2) } else { "-" }
    # 由状态码分布计算真实 2xx 成功率（oha 的 successRate 仅反映连接成功，429 也算成功）
    $codeProps = $data.statusCodeDistribution.PSObject.Properties
    $totalReq = 0; $okReq = 0
    foreach ($p in $codeProps) { $totalReq += [int]$p.Value; if ($p.Name -like "2*") { $okReq += [int]$p.Value } }
    $okRate = if ($totalReq -gt 0) { [math]::Round($okReq / $totalReq * 100, 2) } else { 0 }
    $okRps = if ($totalReq -gt 0) { [math]::Round($rps * $okReq / $totalReq, 1) } else { 0 }
    $codes = ($codeProps | ForEach-Object { "$($_.Name)×$($_.Value)" }) -join ", "
    Write-Host ("  RPS={0} (2xx={1})  2xx率={2}%  avg={3}ms  p50={4}ms  p95={5}ms  p99={6}ms  [{7}]" -f $rps,$okRps,$okRate,$avg,$p50,$p95,$p99,$codes) -ForegroundColor Green
    $script:Results += [pscustomobject]@{
        Title=$title; Ok=$true; RPS=$rps; OkRps=$okRps; OkRate=$okRate
        Avg=$avg; P50=$p50; P95=$p95; P99=$p99; Codes=$codes
    }
}

# 2.1 登录接口压测（POST + DB 读写）
foreach ($c in $Concurrent) {
    Run-Bench "登录接口 - ${c}并发 x ${Duration}" @"
oha -z $Duration -c $c -m POST -d '{\"email\":\"$TestEmail\",\"password\":\"$TestPassword\"}' -H 'Content-Type: application/json' "$BaseUrl/api/auth/login"
"@
}

# 2.2 查询商品（GET + DB 读 + JWT 验证）
foreach ($c in $Concurrent) {
    Run-Bench "查询商品 - ${c}并发 x ${Duration}" "oha -z $Duration -c $c -H 'Authorization: Bearer $Token' `"$BaseUrl/api/products/$ProductId`""
}

# 2.3 查询库存（GET + DB 读）
foreach ($c in $Concurrent) {
    Run-Bench "查询库存 - ${c}并发 x ${Duration}" "oha -z $Duration -c $c -H 'Authorization: Bearer $Token' `"$BaseUrl/api/inventory/$ProductId`""
}

# 2.4 商品列表（GET + DB 分页查询）
foreach ($c in $Concurrent) {
    Run-Bench "商品列表 - ${c}并发 x ${Duration}" "oha -z $Duration -c $c -H 'Authorization: Bearer $Token' `"$BaseUrl/api/products?page=1&page_size=20`""
}

# 2.5 创建订单（POST + DB 写 + Saga + 库存扣减）
Run-Bench "创建订单 - 10并发 x ${Duration}（写操作降并发）" @"
oha -z $Duration -c 10 -m POST -d '{\"user_id\":$UserId,\"items\":[{\"product_id\":$ProductId,\"quantity\":1,\"unit_price\":99.99}]}' -H 'Content-Type: application/json' -H 'Authorization: Bearer $Token' "$BaseUrl/api/orders"
"@

# 2.6 健康检查（并发探测 4 个 gRPC）
Run-Bench "健康检查 - 50并发 x ${Duration}" "oha -z $Duration -c 50 `"$BaseUrl/health`""

# ============================================================
# 汇总
# ============================================================

# ============================================================
# 阶段 3: 生成报告
# ============================================================

$ReportFile = Join-Path $ReportDir "bench-business-$ReportTimestamp.md"
$now = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$mode = if ($Quick) { "快速" } else { "标准" }

$md = @()
$md += "# 压测报告 - bench-business"
$md += ""
$md += "- 生成时间: $now"
$md += "- 目标: $BaseUrl"
$md += "- 模式: $mode (Duration=$Duration, Concurrent=$($Concurrent -join ','))"
$md += "- 测试用户: $TestEmail (ID: $UserId)"
$md += "- 商品 ID: $ProductId"
$md += ""
$md += "## 汇总结果"
$md += ""
$md += "| 场景 | RPS | 2xxRPS | 2xx率(%) | 平均(ms) | p50(ms) | p95(ms) | p99(ms) | 状态码 |"
$md += "|------|-----|--------|----------|----------|---------|---------|---------|--------|"
foreach ($r in $script:Results) {
    $md += "| $($r.Title) | $($r.RPS) | $($r.OkRps) | $($r.OkRate) | $($r.Avg) | $($r.P50) | $($r.P95) | $($r.P99) | $($r.Codes) |"
}
$md += ""
$md += "## 场景说明"
$md += ""
$md += "- 登录: POST + DB读写 + JWT生成（CPU密集）"
$md += "- 查询商品: GET + DB读 + JWT验证"
$md += "- 查询库存: GET + DB读 + JWT验证"
$md += "- 商品列表: GET + DB分页查询 + JWT验证"
$md += "- 创建订单: POST + DB写 + Saga编排 + 库存扣减（最重）"
$md += "- 健康检查: GET + 4个gRPC并发探测"
$md += ""
$md += "## 关注指标"
$md += ""
$md += "- 登录 RPS: JWT 签名 + DB 读 的综合性能"
$md += "- 查询 p99: DB读 + 网络往返 的尾延迟"
$md += "- 创建订单 RPS: Saga + 库存扣减 的写入性能"
$md += "- 健康检查 RPS: gRPC 并发探测的开销"
$md += ""

$md | Set-Content -Path $ReportFile -Encoding UTF8

Write-Host @"
`n============================================
  压测完成
============================================
共 $($script:Results.Count) 个场景，报告已生成:
  $ReportFile
"@ -ForegroundColor Green
