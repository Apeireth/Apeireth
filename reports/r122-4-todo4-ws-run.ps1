$results = @()
for ($i = 1; $i -le 5; $i++) {
    Write-Host "=== Workspace Run $i ==="
    $output = cargo test --workspace --no-fail-fast 2>&1
    $last = $output | Select-Object -Last 5
    $last | ForEach-Object { Write-Host $_ }
    # find "test result: ok" or "test result: FAILED"
    $results_line = $output | Select-String -Pattern "test result:" | Select-Object -Last 1
    Write-Host "RESULT: $results_line"
    $results += [PSCustomObject]@{ Run = $i; Result = $results_line.ToString().Trim() }
    Write-Host ""
}
Write-Host "=== Summary ==="
$results | Format-Table -AutoSize -Wrap
