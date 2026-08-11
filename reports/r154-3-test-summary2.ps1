$logPath = "Apeireth-rust\reports\agent-r154-3-cargo-test-2026-08-11.log"
$out = "Apeireth-rust\reports\agent-r154-3-cargo-test-summary.txt"
$lines = Select-String -Path $logPath -Pattern "^test result:" | ForEach-Object { $_.ToString() }
$total_passed = 0
$total_failed = 0
$total_ignored = 0
$count = 0
$nonOk = @()
foreach ($line in $lines) {
  if ($line -match "ok\.\s+(\d+)\s+passed;\s+(\d+)\s+failed;\s+(\d+)\s+ignored") {
    $total_passed += [int]$Matches[1]
    $total_failed += [int]$Matches[2]
    $total_ignored += [int]$Matches[3]
    $count++
  } else {
    $nonOk += $line
  }
}
$report = "Test suites: $count`n"
$report += "Total passed: $total_passed`n"
$report += "Total failed: $total_failed`n"
$report += "Total ignored: $total_ignored`n"
$report += "Non-ok lines: $($nonOk.Count)`n"
if ($nonOk.Count -gt 0) { $report += ($nonOk -join "`n") + "`n" }
Set-Content -Path $out -Value $report -Encoding UTF8
Write-Output $report
