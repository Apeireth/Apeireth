# 热替换已安装的 Apeireth Companion（原地换 exe，不打安装包、不走安装器）。
# Program Files 写权限需要管理员；本机 UAC 为「静默提权」（ConsentPromptBehaviorAdmin=0），
# 由外层 Start-Process -Verb RunAs 调起即可，无弹窗。
# 用法（提权运行；先停掉旧实例，否则运行中的 exe 被文件锁挡住）：
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\deploy-companion-hotswap.ps1
#
# 路径**不硬编码**（2026-11-04 批）：此前 Src 写死某个开发者机上的
# C:\Users\<用户名>\... 路径 ⇒ 换机即失效。现 Src 默认由仓内构建产物相对推导
# （frontend/companion-desktop/src-tauri/target/release/），DstDir 默认仍是产品
# 安装目录（Program Files\Apeireth Companion，非用户专属路径），两者均可参数覆盖。
param(
  [string]$Src = '',
  [string]$DstDir = 'C:\Program Files\Apeireth Companion'
)
$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not $Src) {
  $Src = Join-Path $RepoRoot 'frontend\companion-desktop\src-tauri\target\release\companion-desktop.exe'
}
Write-Output "src: $Src"
Write-Output "dst: $DstDir"
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dst = Join-Path $DstDir 'companion-desktop.exe'
if (!(Test-Path $Src)) { throw "新构建不存在: $Src" }
Copy-Item $dst "$dst.bak-$stamp" -Force
Copy-Item $Src $dst -Force
$item = Get-Item $dst
Write-Output ("swapped: {0} ({1} bytes, {2})" -f $item.FullName, $item.Length, $item.LastWriteTime)
