param([string]$Sha, [int]$WaitSec = 60, [string]$Remote = "origin")
if (-not $Sha) {
    $Sha = git rev-parse HEAD 2>$null
}
# Privacy: 0 hardcode GitHub PAT in scripts. Read from $env:GITHUB_TOKEN
$token = $env:GITHUB_TOKEN
$h = @{"User-Agent"="PowerShell";"Authorization"="token $token"}

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

Write-Host "Waiting $WaitSec seconds for CI to pick up commit $Sha (repo $repo)..."
Start-Sleep -Seconds $WaitSec

try {
    $runs = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/actions/runs?sha=$Sha" -Method Get -TimeoutSec 30 -Headers $h -SkipCertificateCheck
    Write-Host "=== Runs for $($Sha.Substring(0,7)) ==="
    $runs.workflow_runs | Group-Object conclusion | Select-Object Count, Name | Format-Table -AutoSize
    Write-Host ""
    $runs.workflow_runs | ForEach-Object {
        $marker = if ($_.conclusion -eq "failure") { "FAIL" } elseif ($_.conclusion -eq "success") { "OK" } elseif ($_.conclusion -eq $null) { "RUN" } else { "? $($_.conclusion)" }
        Write-Host "  [$marker] $($_.name) (id=$($_.id))"
    }
} catch {
    Write-Host "Err: $_"
}