#!/usr/bin/env pwsh
# GitHub Actions runs 检查器
# 用法: pwsh scripts/gh_check.ps1 [latest|failures|full] [-Remote origin]
# 需 -SkipCertificateCheck (本地证书问题)
#
# 仓库名**不硬编码**: 从 git remote 的 URL 推导 owner/repo（2026-11-04 批）。
# 此前硬编码的仓库名指向的是另一个仓 ⇒ 查错仓库, 静默返回无关 run。

param(
    [string]$Mode = "latest",
    [string]$Remote = "origin"
)

# ---- 仓库名推导: git remote URL -> owner/repo ----
# 兼容三种 remote 形态:
#   git@github.com:OWNER/REPO.git
#   https://github.com/OWNER/REPO(.git)
#   ssh://git@github.com/OWNER/REPO(.git)
$remoteUrl = (git remote get-url $Remote 2>$null | Select-Object -First 1)
if (-not $remoteUrl) {
    Write-Host "Error: 取不到 remote '$Remote' 的 URL（先 git remote add 或用 -Remote 指定）" -ForegroundColor Red
    exit 2
}
$clean = $remoteUrl `
    -replace '^git@[^:]+:', '' `
    -replace '^ssh://(?:git@)?[^/]+/', '' `
    -replace '^https?://[^/]+/', ''
$clean = ($clean.TrimEnd('/')) -replace '\.git$', ''
$repoParts = $clean -split '/'
if ($repoParts.Count -lt 2 -or [string]::IsNullOrWhiteSpace($repoParts[-2])) {
    Write-Host "Error: 无法从 remote URL 解析 owner/repo: $remoteUrl" -ForegroundColor Red
    exit 2
}
$repo = "$($repoParts[-2])/$($repoParts[-1])"

$base = "https://api.github.com/repos/$repo/actions/runs"

$pages = switch ($Mode) {
    "latest"     { 1 }
    "failures"   { 10 }
    "full"       { 10 }
    default      { 1 }
}

$allRuns = @()
for ($p = 1; $p -le $pages; $p++) {
    try {
        $url = "$base?per_page=100&page=$p"
        $response = Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 30 -Headers @{"User-Agent"="PowerShell"} -SkipCertificateCheck
        if ($response.workflow_runs.Count -eq 0) { break }
        $allRuns += $response.workflow_runs
    } catch {
        Write-Host "Error page ${p}: $_"
        break
    }
}

Write-Host "Repo: $repo  (derived from remote '$Remote')"
Write-Host "Fetched $($allRuns.Count) runs"
Write-Host ""

$allRuns | Group-Object conclusion | Select-Object Count, Name | Format-Table -AutoSize
Write-Host ""

if ($Mode -eq "failures") {
    $failures = $allRuns | Where-Object { $_.conclusion -eq "failure" }
    Write-Host "Failures (latest $($failures.Count)):"
    $failures | Select-Object name, head_sha, created_at | Format-Table -AutoSize
} else {
    $allRuns | Select-Object name, conclusion, head_sha, created_at | Format-Table -AutoSize
}