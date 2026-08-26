# install.ps1 — 把 apeireth-tui 装成 `apeireth` 命令 (像 claude code / gemini cli)
#
# 装完效果: 在任何 PowerShell 窗口敲 `apeireth` 就启动 R19 TUI
#
# 跑法 (PowerShell):
#   .\install.ps1           # 装到 ~/bin
#   .\install.ps1 -Uninstall # 卸载

param(
    [switch]$Uninstall,
    [switch]$Debug,
    [string]$Source = "",
    [string]$UserBin = "$env:USERPROFILE\bin",
    [string]$CmdName = "apeireth"
)

$ErrorActionPreference = "Stop"

# 默认装 release 版 (主人常驻,体积小 5.27MB, 启动快 2x)
if (-not $Source) {
    $releasePath = ".openclaw\workspace\promethean\Apeireth-rust\target\release\apeireth-tui.exe"
    $debugPath = ".openclaw\workspace\promethean\Apeireth-rust\target\debug\apeireth-tui.exe"
    if ($Debug) {
        $Source = $debugPath
    } elseif (Test-Path $releasePath) {
        $Source = $releasePath
    } else {
        $Source = $debugPath
    }
}

function Info($msg) { Write-Host "[install] $msg" -ForegroundColor Cyan }
function Ok($msg)   { Write-Host "[install] $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "[install] $msg" -ForegroundColor Yellow }
function Err($msg)  { Write-Host "[install] ERROR: $msg" -ForegroundColor Red; exit 1 }

# 卸载
if ($Uninstall) {
    $dst = Join-Path $UserBin "$CmdName.exe"
    if (Test-Path $dst) {
        Remove-Item -Path $dst -Force
        Ok "卸载: $dst"
    } else {
        Warn "$dst 不存在, 无需卸载"
    }
    Write-Host ""
    Write-Host "PATH 里的 $UserBin 不会自动移除, 想移可以手动:" -ForegroundColor DarkGray
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', (([Environment]::GetEnvironmentVariable('Path','User') -split ';' | Where-Object { \$_ -ne '$UserBin' }) -join ';'), 'User')" -ForegroundColor DarkGray
    exit 0
}

# 安装
Info "Source: $Source"
Info "Target: $UserBin\$CmdName.exe"

if (-not (Test-Path $Source)) {
    Err "源文件不存在: $Source (请先 cargo build -p apeireth-tui --release)"
}

# 创建 ~/bin
if (-not (Test-Path $UserBin)) {
    New-Item -ItemType Directory -Path $UserBin -Force | Out-Null
    Ok "创建目录: $UserBin"
}

# 复制 exe
$dst = Join-Path $UserBin "$CmdName.exe"
Copy-Item -Path $Source -Destination $dst -Force
Ok "复制: $Source -> $dst"

# 加 ~/bin 到用户 PATH (不重复加)
$currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentUserPath -notlike "*$UserBin*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentUserPath;$UserBin", "User")
    Ok "已加 $UserBin 到用户 PATH"
} else {
    Ok "$UserBin 已在用户 PATH"
}

# 验证: 新 process 里跑 `apeireth --snapshot 0` 看是否成功
Info "验证 (在子进程里跑 $CmdName --snapshot 0)..."
$env:Path = "$UserBin;$env:Path"
$out = & $CmdName --snapshot 0 2>&1
$exit = $LASTEXITCODE
if ($exit -ne 0) {
    Err "验证失败 (exit=$exit): $out"
}
$len = ($out | Out-String).Length
Ok "验证通过, dump 长度 $len 字节"

Write-Host ""
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  ✅ 装好了!" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "  使用方法:" -ForegroundColor Cyan
Write-Host "    1. 重新打开一个 PowerShell 窗口 (让 PATH 生效)" -ForegroundColor White
Write-Host "    2. 敲:  apeireth" -ForegroundColor White
Write-Host "    3. 进 R19 TUI, 5 nav 切来切去, 按 q 退出" -ForegroundColor White
Write-Host ""
Write-Host "  调试模式:" -ForegroundColor Cyan
Write-Host "    apeireth --snapshot 0   # dump 舰桥页 ANSI 到 stdout" -ForegroundColor White
Write-Host "    apeireth --snapshot 1   # dump 对话页" -ForegroundColor White
Write-Host "    apeireth --snapshot 2   # dump 生长页" -ForegroundColor White
Write-Host "    apeireth --snapshot 3   # dump 历史页" -ForegroundColor White
Write-Host "    apeireth --snapshot 4   # dump 设置页" -ForegroundColor White
Write-Host ""
Write-Host "  卸载:" -ForegroundColor Cyan
Write-Host "    .\install.ps1 -Uninstall" -ForegroundColor White
Write-Host ""
