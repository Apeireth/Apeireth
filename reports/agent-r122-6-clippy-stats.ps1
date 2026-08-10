$ErrorActionPreference = 'Stop'
$log = '.openclaw\workspace\promethean\Apeireth-rust\reports\agent-r122-6-clippy.log'
$content = Get-Content $log

Write-Host "=== Sample of summary lines (last 5) ===" -ForegroundColor Cyan
$content | Where-Object { $_ -match '^warning:' -and $_ -match 'generated' } | Select-Object -Last 5 | ForEach-Object { Write-Host $_ }
Write-Host ""

Write-Host "=== Per-target warning distribution (from 'generated N warnings' lines) ===" -ForegroundColor Cyan
$summaryLines = @()
$content | ForEach-Object {
    if ($_ -match '^warning:\s+(.+?)\s+generated\s+(\d+)\s+warning') {
        $script:summaryLines += [PSCustomObject]@{ Target = $matches[1]; Count = [int]$matches[2] }
    }
}
$byTarget = $summaryLines | Group-Object Target | ForEach-Object {
    [PSCustomObject]@{
        Target = $_.Name
        Total  = ($_.Group | Measure-Object Count -Sum).Sum
        Runs   = $_.Count
    }
} | Sort-Object Total -Descending
$byTarget | Select-Object -First 30 | Format-Table -AutoSize
Write-Host ""

Write-Host "=== Top 15 warning kinds (after stripping path prefix) ===" -ForegroundColor Cyan
$warnKinds = @()
$content | ForEach-Object {
    if ($_ -match '^warning:\s+') {
        # Remove path prefix (e.g. "crates\foo\src\bar.rs:123:45: warning: ...")
        $line = $_ -replace '^warning:\s+', ''
        # Get the warning kind (first segment, e.g. "unused variable" or "calls to `to_string`")
        $kind = ($line -split ':')[0].Trim()
        if ($kind -and $kind -notmatch '^`') {
            $script:warnKinds += $kind
        }
    }
}
$warnKinds | Group-Object | Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object {
    Write-Host ("{0,5}  {1}" -f $_.Count, $_.Name)
}
Write-Host ""

Write-Host "=== Top 15 files with most warnings ===" -ForegroundColor Cyan
$fileWarn = @()
$content | ForEach-Object {
    if ($_ -match '^(.+\.rs):\d+:\d+:\s*warning:') {
        $script:fileWarn += $matches[1]
    }
}
$fileWarn | Group-Object | Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object {
    Write-Host ("{0,5}  {1}" -f $_.Count, $_.Name)
}
Write-Host ""

Write-Host "=== Future-incompat (deprecation, not fix in R122-6) ===" -ForegroundColor Cyan
$content | Where-Object { $_ -match 'future version of Rust|future-incompat' } | ForEach-Object { Write-Host $_ }
