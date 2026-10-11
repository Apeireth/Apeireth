# 星槎 · 便携随身包布局脚本（I 想法 K / 星港批次推进）
#
# 契约（I-starport-builtins.md「星槎」）：
#   1. 便携判定是原生侧真件（portable_mode.rs：exe 同级 portable.flag 文件
#      或 data/ 目录立便携）——本脚本只负责造出该布局，判定不在此重复；
#   2. 0 装：源目录缺可执行文件 / 输出目录危险（源目录内、根目录）一律拒绝，
#      不猜不将就；
#   3. 密钥不随身（keychain 语义不变，便携包不打包任何凭据）；
#   4. -DryRun 只打印计划不动盘。
#
# 用法：pwsh -File make-portable.ps1 -SourceDir <已构建产物目录> [-OutDir <输出>] [-DryRun]

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDir,
    [string]$OutDir = "",
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Fail([string]$Message) {
    throw "make-portable: $Message"
}

# ---- 0 装校验 ----
if (-not (Test-Path -LiteralPath $SourceDir -PathType Container)) {
    Fail "源目录不存在或非目录：$SourceDir"
}
$sourceFull = (Resolve-Path -LiteralPath $SourceDir).Path
$exe = Get-ChildItem -LiteralPath $sourceFull -Filter '*.exe' -File |
    Where-Object { $_.Name -notmatch '(?i)^(vcruntime|msvcp|ucrtbase|python|unins)' } |
    Select-Object -First 1
if (-not $exe) {
    Fail "源目录未找到可执行文件（先完成构建）：$sourceFull"
}

if ($OutDir -eq "") {
    $OutDir = Join-Path (Split-Path -Parent $sourceFull) 'apeireth-portable'
}
$outFull = [System.IO.Path]::GetFullPath($OutDir)
$sourcePrefix = $sourceFull.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
if ($outFull.StartsWith($sourcePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    Fail "输出目录不许落在源目录内：$outFull"
}
if ($outFull -match '^[A-Za-z]:\\?$' -or $outFull -eq '/') {
    Fail "输出目录不许是文件系统根：$outFull"
}

Write-Host "源目录: $sourceFull"
Write-Host "入口可执行: $($exe.Name)"
Write-Host "输出目录: $outFull"
Write-Host "计划动作:"
Write-Host "  1) 复制构建产物 -> 输出目录"
Write-Host "  2) 造 portable.flag（便携判定标记，portable_mode.rs 认它）"
Write-Host "  3) 造 data\（数据随身目录）"
Write-Host "  4) 写 PORTABLE-README.txt（含密钥不随身声明）"

if ($DryRun) {
    Write-Host "[DryRun] 未动盘。"
    exit 0
}

# ---- 布局 ----
if (Test-Path -LiteralPath $outFull) {
    Remove-Item -LiteralPath $outFull -Recurse -Force
}
New-Item -ItemType Directory -Path $outFull | Out-Null
Copy-Item -Path (Join-Path $sourceFull '*') -Destination $outFull -Recurse -Force
New-Item -ItemType File -Path (Join-Path $outFull 'portable.flag') | Out-Null
New-Item -ItemType Directory -Path (Join-Path $outFull 'data') | Out-Null

@(
    'Apeireth 便携随身包',
    '',
    '- 运行: 双击 ' + $exe.Name + '（免安装；本目录即基地，data\ 为随身数据）。',
    '- 便携判定: 本目录的 portable.flag 由原生侧识别，删除即回常规安装模式。',
    '- 密钥不随身: 本包不含任何凭据；首次在新机器使用提供商需重新录入。',
    '- 现场纪律: 陌生机器上高危面默认收紧（见安全纵深文档）。'
) | Set-Content -LiteralPath (Join-Path $outFull 'PORTABLE-README.txt') -Encoding UTF8

Write-Host "完成: $outFull"
