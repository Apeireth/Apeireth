# start_tauri_dev.ps1
$ErrorActionPreference = 'Stop'
Set-Location D:\apx\apeireth-rust

Write-Host "=== 1. Ensuring Sidecar is Staged for dev ==="
powershell -ExecutionPolicy Bypass -File .\packaging\stage-sidecar.ps1 -Profile release -Target x86_64-pc-windows-msvc

Write-Host "`n=== 2. Starting Tauri Dev ==="
Set-Location frontend\companion-desktop
cmd.exe /c pnpm tauri dev
