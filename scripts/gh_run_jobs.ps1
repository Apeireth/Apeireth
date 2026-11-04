param([long]$RunId = 0, [string]$Remote = "origin")
# Privacy: 0 hardcode GitHub PAT in scripts. Read from $env:GITHUB_TOKEN
$token = $env:GITHUB_TOKEN
$h = @{"User-Agent"="PowerShell";"Authorization"="token $token"}

if ($RunId -eq 0) {
    Write-Host "Usage: pwsh scripts/gh_run_jobs.ps1 <run_id> [-Remote origin]"
    exit
}

# 仓库名从 git remote 推导（不硬编码）——推导方式同 scripts/gh_check.ps1
$remoteUrl = (git remote get-url $Remote 2>$null | Select-Object -First 1)
if (-not $remoteUrl) {
    Write-Host "Error: 取不到 remote '$Remote' 的 URL" -ForegroundColor Red
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

try {
    $jobs = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/actions/runs/$RunId/jobs" -Method Get -TimeoutSec 30 -Headers $h -SkipCertificateCheck
    Write-Host "=== Run $RunId (repo $repo) ==="
    $jobs.jobs | ForEach-Object {
        Write-Host ""
        Write-Host "Job: $($_.name)"
        Write-Host "  conclusion: $($_.conclusion)"
        $_.steps | ForEach-Object {
            $marker = if ($_.conclusion -eq "failure") { "FAIL" } elseif ($_.conclusion -eq "success") { "OK" } else { "..." }
            Write-Host "  [$marker] Step $($_.number): $($_.name) ($($_.conclusion))"
        }
    }
} catch {
    Write-Host "Err: $_"
}