#!/usr/bin/env pwsh
<#
.SYNOPSIS
  逐 crate 跑 `cargo fmt -p <crate> --check`，汇总退出码（Windows 上可用的 fmt 门禁入口）。
.DESCRIPTION
  为什么需要它：`cargo fmt --all -- --check` 在 Windows 上**不可用** ——
  cargo-fmt 把每个目标文件当作 argv 传给 rustfmt ⇒ 超过 Windows 命令行长度上限
  ⇒ rustfmt 报「文件名或扩展名太长。 (os error 206)」并打印自己的 usage（看起来像"格式坏了"，其实是调用方式坏了）。
  逐 crate 调用（`cargo fmt -p <crate> --check`）不撞该上限 ⇒ 覆盖同一批文件。

  覆盖口径：crate 清单来自 `cargo metadata --no-deps`（= 根 `Cargo.toml` `[workspace] members`，
  本仓 30 个 member），而 cargo fmt 默认枚举包的**所有** target（lib / bins / tests / benches / examples）
  ⇒ 与 `cargo fmt --all` 的目标集合一致。CI（Linux）仍以 `cargo fmt --all -- --check` 为准
  （见 `.github/workflows/rustfmt.yml`）；本脚本是 **Windows 侧替代入口**，不取代 CI 门禁。

  0 装边界（诚实真账）：
  - 本脚本**只判格式**（rustfmt --check），不跑 clippy / test / build；
  - 结果以**本机 rustfmt 版本**为准；跨 rustfmt 版本可能有差异 ⇒ CI 的 Linux 口径才是发布口径；
  - 本脚本**不改任何文件**（只用 `--check`）⇒ 不会顺手把工作区改写。
.EXAMPLE
  pwsh -NoProfile -File scripts/fmt-check.ps1
.EXAMPLE
  pwsh -NoProfile -File scripts/fmt-check.ps1 -Only apeireth-tools-canonical
.NOTES
  退出码：0 = 全部 crate 0 diff；1 = 有 crate 报 diff（逐个列出 + 打印 diff）；2 = 环境/参数问题。
#>
param(
    [string[]]$Only = @(),
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

function Write-Info {
    param([string]$Text)
    Write-Host $Text
}

# ── 1. 取 crate 清单（唯一来源 = cargo metadata，不手抄第二份）────────────────
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Info '[fmt-check] 环境问题：PATH 上找不到 cargo ⇒ exit 2'
    exit 2
}

$metaRaw = (& cargo metadata --no-deps --format-version 1 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) {
    Write-Info "[fmt-check] 环境问题：`cargo metadata --no-deps` 退出码 $LASTEXITCODE ⇒ exit 2"
    Write-Info $metaRaw
    exit 2
}

try {
    $meta = $metaRaw | ConvertFrom-Json
} catch {
    Write-Info "[fmt-check] 环境问题：cargo metadata 输出不是合法 JSON ⇒ exit 2"
    Write-Info $_
    exit 2
}

$crates = @($meta.packages | ForEach-Object { $_.name } | Sort-Object)
if ($crates.Count -eq 0) {
    Write-Info '[fmt-check] 环境问题：cargo metadata 没给出任何 package ⇒ exit 2'
    exit 2
}

# ── 2. 可选：只查指定 crate（精确名，不匹配 ⇒ 参数问题）──────────────────────
if ($Only.Count -gt 0) {
    $selected = @()
    foreach ($name in $Only) {
        $hit = @($crates | Where-Object { $_ -eq $name })
        if ($hit.Count -eq 0) {
            Write-Info "[fmt-check] 参数问题：workspace 里没有 crate '$name' ⇒ exit 2"
            exit 2
        }
        $selected += $hit
    }
    $crates = @($selected | Sort-Object -Unique)
}

# ── 3. 逐 crate 检查（Windows 可用的 fmt 调用方式）───────────────────────────
$failed = @()
foreach ($crate in $crates) {
    $output = & cargo fmt -p $crate --check 2>&1
    $code = $LASTEXITCODE
    if ($code -eq 0) {
        if (-not $Quiet) { Write-Info "[ OK ] $crate" }
        continue
    }
    $failed += $crate
    Write-Info "[FAIL] $crate (cargo fmt exit=$code)"
    foreach ($line in $output) { Write-Info "    $line" }
}

# ── 4. 汇总（退出码可判 ⇒ 适合当门禁）──────────────────────────────────────
if ($failed.Count -gt 0) {
    Write-Info ("[fmt-check] ❌ {0}/{1} 个 crate 有格式差异：{2}" -f $failed.Count, $crates.Count, ($failed -join ', '))
    Write-Info '[fmt-check] 修法：`cargo fmt -p <crate>`（逐 crate 改写，Windows 可用）'
    exit 1
}

Write-Info ("[fmt-check] ✅ {0} 个 crate 全部 0 diff（`cargo fmt -p <crate> --check` 逐 crate 口径）" -f $crates.Count)
exit 0
