$ErrorActionPreference = 'Stop'
Set-Location '.openclaw\workspace\promethean\Apeireth-rust'
$content = Get-Content 'reports/agent-r122-6-debug-scan.log'
$total = $content.Count
$print = ($content | Select-String -Pattern 'print!\(').Count
$dbg = ($content | Select-String -Pattern 'dbg!\(').Count
$eprint = ($content | Select-String -Pattern 'eprintln!\(').Count
$todo = ($content | Select-String -Pattern 'todo!\(\)').Count

Write-Host "=== Debug 残留统计 ===" -ForegroundColor Cyan
Write-Host "Total lines: $total"
Write-Host "print!: $print"
Write-Host "dbg!: $dbg"
Write-Host "eprintln!: $eprint"
Write-Host "todo!(): $todo"
Write-Host ""

$byFile = $content | ForEach-Object {
    $parts = $_ -split ':', 2
    if ($parts.Count -eq 2) { $parts[0] } else { 'other' }
} | Group-Object | Sort-Object Count -Descending

Write-Host "=== 按文件分布 (Top 15) ===" -ForegroundColor Cyan
$byFile | Select-Object -First 15 | ForEach-Object {
    Write-Host ("{0,4}  {1}" -f $_.Count, $_.Name)
}

Write-Host ""
Write-Host "=== 按文件类型 (src / tests / examples / benches / build.rs) ===" -ForegroundColor Cyan
$byType = $byFile | ForEach-Object {
    $name = $_.Name
    if ($name -match '\\tests\\') { 'tests' }
    elseif ($name -match '\\examples\\') { 'examples' }
    elseif ($name -match '\\benches\\') { 'benches' }
    elseif ($name -match '\\build\.rs$') { 'build.rs' }
    elseif ($name -match '\\bin\\') { 'bin' }
    elseif ($name -match '\\src\\') { 'src' }
    else { 'other' }
} | Group-Object | Sort-Object Count -Descending
$byType | ForEach-Object { Write-Host ("{0,4}  {1}" -f $_.Count, $_.Name) }
