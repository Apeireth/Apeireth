$ErrorActionPreference = 'Stop'
$log = '.openclaw\workspace\promethean\Apeireth-rust\reports\agent-r122-6-doc.log'
$content = Get-Content $log

# Total raw warnings + sum
$warnLines = $content | Where-Object { $_ -match '^warning:' -and $_ -notmatch 'generated \d+ warning' }
$sum = 0
$content | ForEach-Object {
    if ($_ -match 'generated (\d+) warning') {
        $script:sum += [int]$matches[1]
    }
}

Write-Host "=== Doc Generation Summary ===" -ForegroundColor Cyan
Write-Host "Total raw warning lines: $($warnLines.Count)"
Write-Host "Sum of 'generated N warnings' counts: $sum"
Write-Host ""

# Per-crate generated warnings
$summaryLines = @()
$content | ForEach-Object {
    if ($_ -match '^warning:\s+\`([^\`]+)\`\s+\(lib doc\)\s+generated\s+(\d+)\s+warning') {
        $script:summaryLines += [PSCustomObject]@{ Crate = $matches[1]; Count = [int]$matches[2] }
    }
}
Write-Host "=== Per-crate 'generated N warnings' (Top 20) ===" -ForegroundColor Cyan
$summaryLines | Sort-Object Count -Descending | Select-Object -First 20 | Format-Table -AutoSize
Write-Host ""

# Errors
$errLines = $content | Where-Object { $_ -match '^error' }
Write-Host "=== Errors ===" -ForegroundColor Cyan
Write-Host "Total error lines: $($errLines.Count)"
$errLines | ForEach-Object { Write-Host $_ }
Write-Host ""

# Warning kinds
$warnKinds = @()
$content | ForEach-Object {
    if ($_ -match '^warning:\s+') {
        $line = $_ -replace '^warning:\s+', ''
        $kind = ($line -split ':')[0].Trim()
        if ($kind -and $kind -notmatch '^`') {
            $script:warnKinds += $kind
        }
    }
}
Write-Host "=== Top 15 warning kinds ===" -ForegroundColor Cyan
$warnKinds | Group-Object | Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object {
    Write-Host ("{0,5}  {1}" -f $_.Count, $_.Name)
}
Write-Host ""

# Top files
$fileWarn = @()
$content | ForEach-Object {
    if ($_ -match '^(.+\.rs):\d+:\d+:\s*warning:') {
        $script:fileWarn += $matches[1]
    }
}
Write-Host "=== Top 15 files with most warnings ===" -ForegroundColor Cyan
$fileWarn | Group-Object | Sort-Object Count -Descending | Select-Object -First 15 | ForEach-Object {
    Write-Host ("{0,5}  {1}" -f $_.Count, $_.Name)
}
