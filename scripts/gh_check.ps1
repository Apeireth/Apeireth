#!/usr/bin/env pwsh
# GitHub Actions runs 检查器
# 用法: pwsh scripts/gh_check.ps1 [latest|failures|full]
# 需 -SkipCertificateCheck (本地证书问题)

param([string]$Mode = "latest")

$repo = "YintaTriss/apeireth-rust"
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