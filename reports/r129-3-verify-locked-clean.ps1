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

$reportPath = "Apeireth-rust/reports/agent-r129-3-locked-sig-clean-2026-08-11.log"
"=== R129-3 LOCKED entry signature clean verify (mod names only, comments stripped) ===" | Out-File $reportPath -Encoding UTF8
"" | Out-File -Append $reportPath -Encoding UTF8

$overallPass = $true

foreach ($path in $modifiedLocked) {
  $name = Split-Path $path -Parent | Split-Path -Leaf
  "" | Out-File -Append $reportPath -Encoding UTF8
  "### $path" | Out-File -Append $reportPath -Encoding UTF8
  "### $path"

  # Get current mod names only (strip comments, trim)
  $curPubMod = Select-String -Path $path -Pattern "^pub mod " | ForEach-Object {
    $line = $_.Line
    # Remove comment
    if ($line -match '^(pub mod\s+\S+).*$') { $matches[1].Trim() }
  } | Sort-Object -Unique
  $curPubUse = Select-String -Path $path -Pattern "^pub use " | ForEach-Object {
    $line = $_.Line
    if ($line -match '^(pub use\s+[^{]+)\{?') { $matches[1].Trim() -replace '\s*\{$','' }
  } | Sort-Object -Unique

  # Get HEAD (abf12243) mod names only
  $headLib = git show "HEAD:./$path" 2>&1
  $headPubMod = $headLib | Select-String -Pattern "^pub mod " | ForEach-Object {
    $line = $_
    if ($line -match '^(pub mod\s+\S+).*$') { $matches[1].Trim() }
  } | Sort-Object -Unique
  $headPubUse = $headLib | Select-String -Pattern "^pub use " | ForEach-Object {
    $line = $_
    if ($line -match '^(pub use\s+[^{]+)\{?') { $matches[1].Trim() -replace '\s*\{$','' }
  } | Sort-Object -Unique

  $modRemoved = Compare-Object $headPubMod $curPubMod | Where-Object { $_.SideIndicator -eq "<=" }
  $modAdded = Compare-Object $headPubMod $curPubMod | Where-Object { $_.SideIndicator -eq "=>" }
  $useRemoved = Compare-Object $headPubUse $curPubUse | Where-Object { $_.SideIndicator -eq "<=" }
  $useAdded = Compare-Object $headPubUse $curPubUse | Where-Object { $_.SideIndicator -eq "=>" }

  "" | Out-File -Append $reportPath -Encoding UTF8
  "  HEAD pub mod: $($headPubMod.Count) | current: $($curPubMod.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  HEAD pub use: $($headPubUse.Count) | current: $($curPubUse.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  pub mod removed from HEAD (CRITICAL = B1 violation): $($modRemoved.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  pub mod added (new internal, allowed per 决策 #41 §2 + 决策 #47): $($modAdded.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  pub use removed from HEAD (CRITICAL = B1 violation): $($useRemoved.Count)" | Out-File -Append $reportPath -Encoding UTF8
  "  pub use added (new internal, allowed per 决策 #41 §2 + 决策 #47): $($useAdded.Count)" | Out-File -Append $reportPath -Encoding UTF8

  if ($modRemoved -and $modRemoved.Count -gt 0) {
    "  CRITICAL mod removals:" | Out-File -Append $reportPath -Encoding UTF8
    foreach ($d in $modRemoved) {
      "    $($d.InputObject)" | Out-File -Append $reportPath -Encoding UTF8
    }
    $overallPass = $false
  }
  if ($useRemoved -and $useRemoved.Count -gt 0) {
    "  CRITICAL use removals:" | Out-File -Append $reportPath -Encoding UTF8
    foreach ($d in $useRemoved) {
      "    $($d.InputObject)" | Out-File -Append $reportPath -Encoding UTF8
    }
    $overallPass = $false
  }

  Write-Output "$path : mod removed=$($modRemoved.Count) added=$($modAdded.Count), use removed=$($useRemoved.Count) added=$($useAdded.Count)"
}

"" | Out-File -Append $reportPath -Encoding UTF8
"=== OVERALL: B1 入口签名 (24 LOCKED baseline 0 改) ===" | Out-File -Append $reportPath -Encoding UTF8
"  Result: $(if($overallPass){'PASS (no original entries removed)'}else{'FAIL'})" | Out-File -Append $reportPath -Encoding UTF8
Write-Output "---"
Write-Output "B1 LOCKED entry signature verify: $(if($overallPass){'PASS'}else{'FAIL'})"
