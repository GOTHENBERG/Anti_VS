[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  Сборка Anti_VS и упаковка в ZIP для отправки другу  " -ForegroundColor Yellow
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Компиляция в Release-режиме
Write-Host "[1/3] Компиляция в Release-режиме (cargo build --release)..." -ForegroundColor Green
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[ОШИБКА] Сборка завершилась с ошибкой!" -ForegroundColor Red
    exit 1
}

# 2. Подготовка файлов релиза
Write-Host ""
Write-Host "[2/3] Подготовка файлов в папке dist\Anti_VS_Game..." -ForegroundColor Green
$distDir = Join-Path $PSScriptRoot "dist"
$gameDir = Join-Path $distDir "Anti_VS_Game"
$zipPath = Join-Path $distDir "Anti_VS_Release.zip"

if (Test-Path $distDir) {
    Remove-Item -Path $distDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $gameDir -Force | Out-Null

Copy-Item (Join-Path $PSScriptRoot "target\release\Anti_VS.exe") -Destination (Join-Path $gameDir "Anti_VS.exe") -Force
Copy-Item (Join-Path $PSScriptRoot "assets") -Destination (Join-Path $gameDir "assets") -Recurse -Force
Copy-Item (Join-Path $PSScriptRoot "models") -Destination (Join-Path $gameDir "models") -Recurse -Force

# 3. Сжатие в готовый архив
Write-Host ""
Write-Host "[3/3] Сжатие в готовый архив dist\Anti_VS_Release.zip..." -ForegroundColor Green
Compress-Archive -Path "$gameDir\*" -DestinationPath $zipPath -Force

$zipSizeMb = [math]::Round((Get-Item $zipPath).Length / 1MB, 1)

Write-Host ""
Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "  УСПЕШНО! Архив для друга готов:" -ForegroundColor Green
Write-Host "  $zipPath ($zipSizeMb МБ)" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Другу нужно лишь:" -ForegroundColor White
Write-Host "  1. Распаковать полученный ZIP-архив в любую папку." -ForegroundColor Gray
Write-Host "  2. Запустить Anti_VS.exe" -ForegroundColor Gray
Write-Host "======================================================" -ForegroundColor Cyan
