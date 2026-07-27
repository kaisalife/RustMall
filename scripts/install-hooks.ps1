# scripts/install-hooks.ps1
# 安装本地 git hooks（使用 core.hooksPath 指向 githooks/ 目录）
# 使用方式：powershell -ExecutionPolicy Bypass -File scripts/install-hooks.ps1

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Installing Local Git Hooks (CI/CD)      " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 检查是否在 git 仓库中
$gitDir = git rev-parse --git-dir 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Not a git repository!" -ForegroundColor Red
    exit 1
}

# 设置 hooks 路径为项目根目录的 githooks/
git config core.hooksPath githooks
Write-Host "[OK] core.hooksPath set to: githooks/" -ForegroundColor Green

# 列出已安装的 hooks
Write-Host ""
Write-Host "Installed hooks:" -ForegroundColor Yellow
$hooks = Get-ChildItem -Path "githooks" -File
foreach ($hook in $hooks) {
    Write-Host "  - $($hook.Name)" -ForegroundColor Green
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Git Hooks Installed Successfully!        " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Now every 'git commit' will trigger:" -ForegroundColor White
Write-Host "  1. commit-msg  - commit message format check" -ForegroundColor Gray
Write-Host "  2. pre-commit  - cargo fmt + clippy + test" -ForegroundColor Gray
Write-Host ""
Write-Host "To skip hooks temporarily:" -ForegroundColor Yellow
Write-Host "  git commit --no-verify" -ForegroundColor Gray
Write-Host ""
Write-Host "To uninstall:" -ForegroundColor Yellow
Write-Host "  git config --unset core.hooksPath" -ForegroundColor Gray
Write-Host ""
