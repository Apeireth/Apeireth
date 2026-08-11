Set-Location "Apeireth-rust/reports/"
$files = Get-ChildItem -Name "agent-r129-3-*" | Where-Object { $_.EndsWith(".md") }
foreach ($f in $files) {
  Write-Output "File: [$f] len=$($f.Length)"
  if ($f -like "agent-r129-3-?*-8-step-verify-2026-08-11.md" -and $f -notlike "agent-r129-3-8-step-verify-2026-08-11.md") {
    Write-Output "  This is the duplicate (with non-ascii chars in name)"
    $head = Get-Content $f -TotalCount 5
    Write-Output "  First 5 lines:"
    $head | ForEach-Object { Write-Output "    | $_" }
  }
}
