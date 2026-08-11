$logPath = "Apeireth-rust\reports\agent-r154-3-cargo-test-2026-08-11.log"
$lines = Select-String -Path $logPath -Pattern "^test result:" | ForEach-Object { $_.ToString() }
$total_passed = 0
$total_failed = 0
$total_ignored = 0
$count = 0
foreach ($line in $lines) {
  if ($line -match "ok\.\s+(\d+)\s+passed;\s+(\d+)\s+failed;\s+(\d+)\s+ignored") {
    $total_passed += [int]$Matches[1]
    $total_failed += [int]$Matches[2]
    $total_ignored += [int]$Matches[3]
    $count++
  }
}
Write-Output "Test suites: $count"
Write-Output "Total passed: $total_passed"
Write-Output "Total failed: $total_failed"
Write-Output "Total ignored: $total_ignored"
$anyFail = $lines | Where-Object { $_ -match "FAILED" -or $_ -notmatch "ok\." }
if ($anyFail) { Write-Output "Non-ok results:"; $anyFail | ForEach-Object { Write-Output $_ } } else { Write-Output "All test result lines are ok. (0 fail)" }
