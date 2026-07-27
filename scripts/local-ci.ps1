# scripts/local-ci.ps1
# 本地 CI 检查脚本 - 手动运行或被 git hooks 调用
# 使用方式：powershell -ExecutionPolicy Bypass -File scripts/local-ci.ps1
# 参数：-SkipTest（跳过测试，只做 fmt + clippy）

param(
    [switch]$SkipTest,
    [switch]$SkipClippy,
    [switch]$Fix  # 自动修复格式问题
)

$ErrorActionPreference = "Stop"
$failed = $false

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Local CI Check (same as pre-commit)     " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 代码格式检查
Write-Host "--- [1/3] cargo fmt ---" -ForegroundColor Yellow
if ($Fix) {
    Write-Host "Auto-fixing format..." -ForegroundColor Gray
    cargo fmt --all
    Write-Host "Format fixed." -ForegroundColor Green
} else {
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FORMAT CHECK FAILED!" -ForegroundColor Red
        Write-Host "Run: cargo fmt --all" -ForegroundColor Gray
        $failed = $true
    } else {
        Write-Host "Format check passed." -ForegroundColor Green
    }
}
Write-Host ""

# 2. Clippy 静态分析
if (-not $SkipClippy) {
    Write-Host "--- [2/3] cargo clippy ---" -ForegroundColor Yellow
    cargo clippy --workspace --all-targets -- -D warnings 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CLIPPY FAILED!" -ForegroundColor Red
        $failed = $true
    } else {
        Write-Host "Clippy check passed." -ForegroundColor Green
    }
    Write-Host ""
}

# 3. 测试
if (-not $SkipTest) {
    Write-Host "--- [3/3] cargo test ---" -ForegroundColor Yellow
    cargo test --workspace 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "TESTS FAILED!" -ForegroundColor Red
        $failed = $true
    } else {
        Write-Host "All tests passed." -ForegroundColor Green
    }
    Write-Host ""
}

# 结果
Write-Host "==========================================" -ForegroundColor Cyan
if ($failed) {
    Write-Host "  CI CHECK FAILED!" -ForegroundColor Red
    Write-Host "==========================================" -ForegroundColor Cyan
    exit 1
} else {
    Write-Host "  ALL CHECKS PASSED!" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Cyan
    exit 0
}
