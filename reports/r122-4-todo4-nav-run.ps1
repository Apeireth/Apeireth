$results = @()
for ($i = 1; $i -le 5; $i++) {
    Write-Host "=== Run $i ==="
    $output = cargo test -p apeireth-tui --test nav_settings_test --no-fail-fast 2>&1
    $last = $output | Select-Object -Last 5
    $last | ForEach-Object { Write-Host $_ }
    # check pass/fail
    $passed = ($output | Select-String -Pattern "test result: ok\. (\d+) passed" | Select-Object -First 1).Matches[0].Groups[1].Value
    $failed = ($output | Select-String -Pattern "test result:.*?(\d+) failed" | Select-Object -First 1).Matches[0].Groups[1].Value
    Write-Host "RESULT: passed=$passed failed=$failed"
    $results += [PSCustomObject]@{ Run = $i; Passed = $passed; Failed = $failed }
    Write-Host ""
}
Write-Host "=== Summary ==="
$results | Format-Table -AutoSize
