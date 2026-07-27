# ============================================
# Simple Trade - Database Migration Script
# ============================================

param(
    [string]$DB_HOST = "localhost",
    [int]$DB_PORT = 5432,
    [string]$DB_USER = "postgres",
    [string]$DB_NAME = "simple_trade",
    [string]$DB_PASSWORD = "postgres"
)

# Set environment variable for psql password
$env:PGPASSWORD = $DB_PASSWORD

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Simple Trade - Database Migration" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Host:     $DB_HOST" -ForegroundColor Gray
Write-Host "Port:     $DB_PORT" -ForegroundColor Gray
Write-Host "User:     $DB_USER" -ForegroundColor Gray
Write-Host "Database: $DB_NAME" -ForegroundColor Gray
Write-Host ""

try {
    # Check if psql is available
    $psqlCheck = Get-Command psql -ErrorAction SilentlyContinue
    if (-not $psqlCheck) {
        Write-Host "❌ psql not found! Please install PostgreSQL and add to PATH." -ForegroundColor Red
        Write-Host "   Alternatively, use Docker to run PostgreSQL:" -ForegroundColor Yellow
        Write-Host "   docker exec -i simple_trade_postgres psql ..." -ForegroundColor Yellow
        exit 1
    }

    # 1. Create database if not exists
    Write-Host "🔍 Checking if database exists..." -ForegroundColor Yellow
    $dbExists = psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -t -c "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" 2>$null
    
    if ($dbExists.Trim() -ne "1") {
        Write-Host "📦 Creating database '$DB_NAME'..." -ForegroundColor Green
        psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c "CREATE DATABASE $DB_NAME" 2>$null
        Write-Host "✅ Database created successfully!" -ForegroundColor Green
    } else {
        Write-Host "✅ Database already exists" -ForegroundColor Green
    }

    # 2. Run migration scripts
    Write-Host ""
    Write-Host "📄 Running migration scripts..." -ForegroundColor Yellow
    
    $migrationDir = Join-Path $PSScriptRoot "..\crates\db-migration\migrations"
    $migrationFiles = Get-ChildItem -Path $migrationDir -Filter "*.sql" | Sort-Object Name
    
    foreach ($file in $migrationFiles) {
        Write-Host "   Applying: $($file.Name)" -ForegroundColor Gray
        
        $result = psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f $file.FullName 2>&1
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "   ✅ $($file.Name) applied successfully" -ForegroundColor Green
        } else {
            Write-Host "   ❌ Error applying $($file.Name):" -ForegroundColor Red
            Write-Host "      $result" -ForegroundColor Red
        }
    }

    Write-Host ""
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host "  Migration completed successfully! 🎉" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host ""

    # 3. Verify tables
    Write-Host "📊 Database Tables:" -ForegroundColor Yellow
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "\dt" 2>$null

} catch {
    Write-Host "❌ Error: $_" -ForegroundColor Red
    exit 1
} finally {
    Remove-Item Env:\PGPASSWORD -ErrorAction SilentlyContinue
}
