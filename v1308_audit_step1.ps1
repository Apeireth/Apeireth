# V1308 audit step 1: 修真期间新增 package 列表
$ErrorActionPreference = "Stop"
Set-Location ".openclaw\workspace\promethean"

$now = Get-Content "Apeireth-rust/Cargo.lock" | Select-String -Pattern '^name = ' | ForEach-Object { $_.ToString().Replace('name = "', '').Replace('"', '') }
$prev = git show HEAD:Apeireth-rust/Cargo.lock | Select-String -Pattern '^name = ' | ForEach-Object { $_.ToString().Replace('name = "', '').Replace('"', '') }

Write-Host "HEAD Cargo.lock package count: $($prev.Count)"
Write-Host "Now Cargo.lock package count:  $($now.Count)"
Write-Host "Delta:  $($now.Count - $prev.Count)"

$added = $now | Where-Object { $prev -notcontains $_ } | Sort-Object
Write-Host ""
Write-Host "=== ADDED PACKAGES (now not in HEAD) ==="
Write-Host "Total added: $($added.Count)"
$added | ForEach-Object { Write-Host "  $_" } | Out-Null

# Categorize
$workspaceAdded = $added | Where-Object { $_ -like "apeireth-*" }
$transitiveAdded = $added | Where-Object { $_ -notlike "apeireth-*" }
Write-Host ""
Write-Host "=== CATEGORIZE ==="
Write-Host "Workspace crates added: $($workspaceAdded.Count) ($($workspaceAdded -join ', '))"
Write-Host "Transitive deps added:  $($transitiveAdded.Count)"

# Output to JSON
$result = @{
    head_count = $prev.Count
    now_count = $now.Count
    delta = $now.Count - $prev.Count
    added_total = $added.Count
    added_workspace = $workspaceAdded
    added_transitive = $transitiveAdded
} | ConvertTo-Json -Depth 5
Set-Content -Path "v1308_audit_step1.json" -Value $result -Encoding utf8
Write-Host ""
Write-Host "Written: v1308_audit_step1.json"