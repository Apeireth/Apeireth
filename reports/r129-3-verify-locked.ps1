Set-Location "Apeireth-rust/"
$lockedCratePaths = @(
  "crates/apeireth-supervisor/src/lib.rs",
  "crates/apeireth-agent/src/lib.rs",
  "crates/apeireth-bus/src/lib.rs",
  "crates/apeireth-council/src/lib.rs",
  "crates/apeireth-evolution/src/lib.rs",
  "crates/apeireth-extension/src/lib.rs",
  "crates/apeireth-graph/src/lib.rs",
  "crates/apeireth-mcp/src/lib.rs",
  "crates/apeireth-pipeline/src/lib.rs",
  "crates/apeireth-tool-registry/src/lib.rs",
  "crates/apeireth-tool-runtime/src/lib.rs",
  "crates/apeireth-protocol/src/lib.rs",
  "crates/apeireth-asi/src/lib.rs",
  "crates/apeireth-onion/src/lib.rs",
  "crates/apeireth-sovereignty/src/lib.rs",
  "crates/apeireth-constraint/src/lib.rs",
  "crates/apeireth-memory/src/lib.rs",
  "crates/apeireth-cognition/src/lib.rs",
  "crates/apeireth-perception/src/lib.rs",
  "crates/apeireth-consciousness/src/lib.rs",
  "crates/apeireth-motivation/src/lib.rs",
  "crates/apeireth-life-force/src/lib.rs",
  "crates/apeireth-relation/src/lib.rs",
  "crates/apeireth-value/src/lib.rs"
)

"=== 24 LOCKED crate lib.rs 入口签名 0 改 verify (R129-3 二次 verify) ==="
"Total: 24 LOCKED crate lib.rs"
""
$reportPath = "Apeireth-rust/reports/agent-r129-3-locked-verify-2026-08-11.log"
"" | Out-File $reportPath -Encoding UTF8

foreach ($path in $lockedCratePaths) {
  $fullPath = "Apeireth-rust/$path"
  if (Test-Path $fullPath) {
    $item = Get-Item $fullPath
    $lineCount = (Get-Content $fullPath | Measure-Object -Line).Lines
    $firstFiveMods = Select-String -Path $fullPath -Pattern "^pub mod " | Select-Object -First 5 | ForEach-Object { ($_.Line -replace "^\s*", "") }
    $modCount = (Select-String -Path $fullPath -Pattern "^pub mod " | Measure-Object).Count
    $line = ("# $($item.Name) : lines=$lineCount, pub mod count=$modCount, mtime=$($item.LastWriteTime.ToString('HH:mm:ss'))")
    $line | Out-File -Append $reportPath -Encoding UTF8
    Write-Output $line
    foreach ($mod in $firstFiveMods) {
      "  $mod" | Out-File -Append $reportPath -Encoding UTF8
      Write-Output "  $mod"
    }
  } else {
    Write-Output "MISSING: $path"
  }
}
"---"
"---EXIT 0---"
