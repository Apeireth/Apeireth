# 热替换已安装的 Apeireth Companion（原地换 exe，不打安装包、不走安装器）。
# Program Files 写权限需要管理员；本机 UAC 为「静默提权」（ConsentPromptBehaviorAdmin=0），
# 由外层 Start-Process -Verb RunAs 调起即可，无弹窗。
# 用法（提权运行；先停掉旧实例，否则运行中的 exe 被文件锁挡住）：
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\deploy-companion-hotswap.ps1
param(
  [string]$Src = 'C:\Users\31683\Apeireth-rust\frontend\companion-desktop\src-tauri\target\release\companion-desktop.exe',
  [string]$DstDir = 'C:\Program Files\Apeireth Companion'
)
$ErrorActionPreference = 'Stop'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dst = Join-Path $DstDir 'companion-desktop.exe'
if (!(Test-Path $Src)) { throw "新构建不存在: $Src" }
Copy-Item $dst "$dst.bak-$stamp" -Force
Copy-Item $Src $dst -Force
$item = Get-Item $dst
Write-Output ("swapped: {0} ({1} bytes, {2})" -f $item.FullName, $item.Length, $item.LastWriteTime)
