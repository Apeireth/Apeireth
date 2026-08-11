Set-Location "Apeireth-rust/"

# 6 modified LOCKED lib.rs files (per git status M)
$modifiedLocked = @(
  "crates/apeireth-agent/src/lib.rs",
  "crates/apeireth-evolution/src/lib.rs",
  "crates/apeireth-graph/src/lib.rs",
  "crates/apeireth-pipeline/src/lib.rs",
  "crates/apeireth-sovereignty/src/lib.rs",
  "crates/apeireth-tool-runtime/src/lib.rs"
)

$reportPath = "Apeireth-rust/reports/agent-r129-3-locked-sig-diff-2026-08-11.log"
"=== R129-3 LOCKED entry signature diff (HEAD vs current) ===" | Out-File $reportPath -Encoding UTF8
"" | Out-File -Append $reportPath -Encoding UTF8

foreach ($path in $modifiedLocked) {
  $relPath = $path -replace "crates/", "crates/"
  $name = Split-Path $path -Parent | Split-Path -Leaf
  "" | Out-File -Append $reportPath -Encoding UTF8
  "### $path" | Out-File -Append $reportPath -Encoding UTF8
  "### $path"

  # Get current pub mod + pub use + pub fn signatures
  $curPubMod = Select-String -Path $path -Pattern "^pub mod " | ForEach-Object { $_.Line.Trim() } | Sort-Object
  $curPubUse = Select-String -Path $path -Pattern "^pub use " | ForEach-Object { $_.Line.Trim() } | Sort-Object

  # Get HEAD (abf12243) pub mod + pub use + pub fn signatures
  $headLib = git show "HEAD:./$path" 2>&1
  $headPubMod = $headLib | Select-String -Pattern "^pub mod " | ForEach-Object { $_.Line.Trim() } | Sort-Object
  $headPubUse = $headLib | Select-String -Pattern "^pub use " | ForEach-Object { $_.Line.Trim() } | Sort-Object

  "  Current pub mod count: $($curPubMod.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  HEAD pub mod count: $($headPubMod.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  Current pub use count: $($curPubUse.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  HEAD pub use count: $($headPubUse.Count)" | Out-File -Append $reportPath -Encoding UTF8

  $modDiff = Compare-Object $curPubMod $headPubMod
  $useDiff = Compare-Object $curPubUse $headPubUse

  if ($modDiff -and $modDiff.Count -gt 0) {
    "  pub mod diff (entry signature changed):" | Out-File -Append $reportPath -Encoding UTF8
    foreach ($d in $modDiff) {
      "    $($d.SideIndicator) $($d.InputObject)" | Out-File -Append $reportPath -Encoding UTF8
    }
  } else {
    "  pub mod: IDENTICAL (entry signature 0 changed)" | Out-File -Append $reportPath -Encoding UTF8
  }

  if ($useDiff -and $useDiff.Count -gt 0) {
    "  pub use diff (entry signature changed):" | Out-File -Append $reportPath -Encoding UTF8
    foreach ($d in $useDiff) {
      "    $($d.SideIndicator) $($d.InputObject)" | Out-File -Append $reportPath -Encoding UTF8
    }
  } else {
    "  pub use: IDENTICAL (entry signature 0 changed)" | Out-File -Append $reportPath -Encoding UTF8
  }

  Write-Output "  pub mod: $(if($modDiff -and $modDiff.Count -gt 0){'CHANGED'}else{'IDENTICAL'}), pub use: $(if($useDiff -and $useDiff.Count -gt 0){'CHANGED'}else{'IDENTICAL'})"
}
