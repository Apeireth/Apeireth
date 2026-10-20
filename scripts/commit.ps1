#!/usr/bin/env pwsh
<#
.SYNOPSIS
  提交收口入口：显式 author 身份 + .githooks 检查 + staged secret/neutral 门禁。
.DESCRIPTION
  背景（2026-10-20 效率 P0）：仓库 .githooks 存在但 core.hooksPath 未设；
  全局启用会影响同工作树的并行 AI 线（pre-commit 要求显式 GIT_AUTHOR_*）。
  因此本脚本用"按次启用 hooks"（git -c core.hooksPath=.githooks commit），
  只影响本次提交，不动全局配置。

  流程：
  1) 校验/补齐 GIT_AUTHOR_* / GIT_COMMITTER_*（默认取 -AuthorName/-AuthorEmail 或 git config user.*）；
  2) 对 staged 改动跑 secret-scan（有脚本时）+ check-neutral-terms；
  3) git -c core.hooksPath=.githooks commit（commit-msg 校验 O-6 三段 + pre-commit 校验 author/LOCKED）。

.USAGE
  pwsh scripts/commit.ps1 -MessageFile $env:TEMP\msg.txt
  pwsh scripts/commit.ps1 -MessageFile msg.txt -AuthorName Mavis -AuthorEmail Mavis@apeireth.local
  pwsh scripts/commit.ps1 -MessageFile msg.txt -NoVerify        # 真例外时跳过 hooks（慎用）
  pwsh scripts/commit.ps1 -DryRun -MessageFile msg.txt          # 只打印将执行的动作

.NOTES
  0 装：不新增依赖；复用仓内现有门禁脚本；失败即退、不静默降级。
#>
param(
    [Parameter(Mandatory = $true)][string]$MessageFile,
    [string]$AuthorName = "",
    [string]$AuthorEmail = "",
    [switch]$NoVerify,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

if (-not (Test-Path $MessageFile)) { throw "message file not found: $MessageFile" }

if (-not $AuthorName) { $AuthorName = (git config user.name) }
if (-not $AuthorEmail) { $AuthorEmail = (git config user.email) }
if (-not $AuthorName -or -not $AuthorEmail) { throw "author identity missing: pass -AuthorName/-AuthorEmail" }

$env:GIT_AUTHOR_NAME = $AuthorName
$env:GIT_AUTHOR_EMAIL = $AuthorEmail
$env:GIT_COMMITTER_NAME = $AuthorName
$env:GIT_COMMITTER_EMAIL = $AuthorEmail
Write-Host "author: $AuthorName <$AuthorEmail>"

# 1) staged secret-scan（有则跑；无 staged 会自然跳过）
if (Test-Path "scripts/secret-scan.ps1") {
    Write-Host "== secret-scan (staged) ..."
    pwsh -NoProfile -File scripts/secret-scan.ps1 -Mode scan-staged
    if ($LASTEXITCODE -ne 0) { throw "secret-scan failed" }
}
# 2) 中性表述
if (Test-Path "scripts/check-neutral-terms.ps1") {
    Write-Host "== check-neutral-terms ..."
    pwsh -NoProfile -File scripts/check-neutral-terms.ps1
    if ($LASTEXITCODE -ne 0) { throw "check-neutral-terms failed" }
}

# 3) commit（按次启用 hooks）
$gitArgs = @('-c', 'core.hooksPath=.githooks', 'commit', '-F', $MessageFile)
if ($NoVerify) { $gitArgs += '--no-verify' }
if ($DryRun) {
    Write-Host ("DRY-RUN: git " + ($gitArgs -join ' '))
    exit 0
}
Write-Host "== git commit (hooks: $([bool](-not $NoVerify)))"
& git @gitArgs
exit $LASTEXITCODE
