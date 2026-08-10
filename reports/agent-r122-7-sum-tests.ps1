$lines = Select-String -Path "reports\agent-r122-7-workspace-test.log" -Pattern "test result: ok\. (\d+) passed" -AllMatches
$total = 0
$binaries = 0
foreach ($l in $lines) {
    foreach ($m in $l.Matches) {
        $n = 0
        if ([int]::TryParse($m.Groups[1].Value, [ref]$n)) {
            $total += $n
            $binaries += 1
        }
    }
}
Write-Output "TOTAL_PASSED=$total"
Write-Output "TEST_BINARIES=$binaries"
