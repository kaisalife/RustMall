# ============================================
# Simple Trade - Development Environment Setup
# ============================================

param(
    [switch]$NoPgAdmin,
    [switch]$NoMigration,
    [switch]$OnlyDB
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Simple Trade - Development Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if Docker is running
Write-Host "🐳 Checking Docker..." -ForegroundColor Yellow
try {
    $dockerVersion = docker version --format '{{.Server.Version}}' 2>$null
    if (-not $dockerVersion) {
        throw "Docker is not running or not installed"
    }
    Write-Host "   ✅ Docker version: $dockerVersion" -ForegroundColor Green
} catch {
    Write-Host "   ❌ $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Please install Docker Desktop and start it first." -ForegroundColor Yellow
    Write-Host "   Download: https://www.docker.com/products/docker-desktop" -ForegroundColor Gray
    exit 1
}

Write-Host ""

# Start PostgreSQL
if ($NoPgAdmin) {
    Write-Host "🚀 Starting PostgreSQL..." -ForegroundColor Green
    docker-compose up -d postgres
} else {
    Write-Host "🚀 Starting PostgreSQL + pgAdmin..." -ForegroundColor Green
    docker-compose up -d
}

Write-Host ""
Write-Host "⏳ Waiting for PostgreSQL to be ready..." -ForegroundColor Yellow

# Wait for PostgreSQL to be healthy
$maxRetries = 30
$retryCount = 0
$isReady = $false

while ($retryCount -lt $maxRetries -and -not $isReady) {
    $health = docker inspect --format '{{.State.Health.Status}}' simple_trade_postgres 2>$null
    
    if ($health -eq "healthy") {
        $isReady = $true
        Write-Host "   ✅ PostgreSQL is ready!" -ForegroundColor Green
    } else {
        $retryCount++
        Write-Host "   [$retryCount/$maxRetries] Waiting..." -ForegroundColor Gray
        Start-Sleep -Seconds 2
    }
}

if (-not $isReady) {
    Write-Host "   ❌ PostgreSQL failed to start in time" -ForegroundColor Red
    exit 1
}

Write-Host ""

# Run database migrations using Rust migration tool
if (-not $NoMigration) {
    Write-Host "🔧 Running database migrations..." -ForegroundColor Yellow
    Write-Host ""

    Push-Location "$PSScriptRoot\.."
    try {
        cargo run --bin migrate
        if ($LASTEXITCODE -ne 0) {
            throw "Migration failed with exit code $LASTEXITCODE"
        }
    } catch {
        Write-Host ""
        Write-Host "   ❌ Migration failed: $_" -ForegroundColor Red
        Write-Host "   Please check the error above and fix any issues." -ForegroundColor Yellow
        Pop-Location
        exit 1
    }
    Pop-Location
} else {
    Write-Host "   ⏭️ Skipping database migrations" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Environment Ready! 🎉" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "📋 Connection Details:" -ForegroundColor Yellow
Write-Host "   PostgreSQL:" -ForegroundColor Cyan
Write-Host "     Host:     localhost" -ForegroundColor Gray
Write-Host "     Port:     5432" -ForegroundColor Gray
Write-Host "     User:     postgres" -ForegroundColor Gray
Write-Host "     Password: postgres" -ForegroundColor Gray
Write-Host "     Database: simple_trade" -ForegroundColor Gray
Write-Host ""

if (-not $NoPgAdmin) {
    Write-Host "   pgAdmin (Web UI):" -ForegroundColor Cyan
    Write-Host "     URL:      http://localhost:5050" -ForegroundColor Gray
    Write-Host "     Email:    admin@simple-trade.com" -ForegroundColor Gray
    Write-Host "     Password: admin123" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "🚀 Useful Commands:" -ForegroundColor Yellow
Write-Host "   Start Auth Service:  cargo run --bin auth-service" -ForegroundColor Gray
Write-Host "   Run Migrations:      cargo run --bin migrate" -ForegroundColor Gray
Write-Host "   View Logs:           docker-compose logs -f" -ForegroundColor Gray
Write-Host "   Stop Services:       docker-compose down" -ForegroundColor Gray
Write-Host ""

# Next steps
Write-Host "📝 Next Steps:" -ForegroundColor Yellow
Write-Host "   1. Start the auth service: cargo run --bin auth-service" -ForegroundColor Gray
Write-Host "   2. Run API Gateway (if implemented): cargo run --bin api-gateway" -ForegroundColor Gray
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Happy coding! 🦀" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
