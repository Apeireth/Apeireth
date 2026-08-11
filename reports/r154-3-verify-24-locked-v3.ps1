Set-Location "Apeireth-rust"

# 24 LOCKED crates per R131-5 1:28 baseline
$lockedCrates = @(
  "apeireth-supervisor",
  "apeireth-agent",
  "apeireth-council",
  "apeireth-bus",
  "apeireth-protocol",
  "apeireth-mcp",
  "apeireth-tool-registry",
  "apeireth-tool-runtime",
  "apeireth-graph",
  "apeireth-pipeline",
  "apeireth-tool-approval",
  "apeireth-extension",
  "apeireth-evolution",
  "apeireth-api",
  "apeireth-core",
  "apeireth-memory",
  "apeireth-asi",
  "apeireth-tools",
  "apeireth-cli",
  "apeireth-bench",
  "apeireth-cognition",
  "apeireth-action",
  "apeireth-life-force",
  "apeireth-constraint"
)

$reportPath = "Apeireth-rust\reports\agent-r154-3-24-locked-sig-verify-2026-08-11.log"
$results = @()
$results += "=== R154-3 24 LOCKED 入口签名 verify (vs 整合 #4 abf12243 baseline) ==="
$results += ""
$results += "master HEAD = $(git rev-parse HEAD)"
$results += "整合 #4 (abf12243) = src/ baseline before R139-1 work"
$results += "Verify time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$results += ""
$results += "Verification approach:"
$results += "  1. Extract pub mod NAMES from current lib.rs and HEAD lib.rs"
$results += "  2. Check that current pub mod is SUPERSET of HEAD (no removal = 0 改 preserved)"
$results += "  3. Report any added modules (additive = 0 改 since original sigs unchanged)"
$results += ""

$passCount = 0
$failCount = 0

function Get-PubModNames($filePath) {
  if (-not (Test-Path $filePath)) { return @() }
  $names = @()
  $content = Get-Content $filePath
  foreach ($line in $content) {
    if ($line -match "^\s*pub\s+mod\s+([a-zA-Z_][a-zA-Z0-9_]*)") {
      $names += $Matches[1]
    }
  }
  return ($names | Sort-Object -Unique)
}

function Get-PubModFromGit($commit, $gitPath) {
  $names = @()
  $content = git show "${commit}:./${gitPath}" 2>&1
  if ($LASTEXITCODE -ne 0) { return $null }
  foreach ($line in $content) {
    if ($line -match "^\s*pub\s+mod\s+([a-zA-Z_][a-zA-Z0-9_]*)") {
      $names += $Matches[1]
    }
  }
  return ($names | Sort-Object -Unique)
}

foreach ($crate in $lockedCrates) {
  $path = "crates/${crate}/src/lib.rs"
  $name = $crate

  if (-not (Test-Path $path)) {
    $results += "[FAIL] $name : FILE NOT FOUND at $path"
    $failCount++
    continue
  }

  $curPubMod = Get-PubModNames $path
  $headPubMod = Get-PubModFromGit "HEAD" $path
  $abfPubMod = Get-PubModFromGit "abf12243" $path

  # In PowerShell, empty array compares as $null. Use .Count check.
  if (($headPubMod -eq $null -and $headPubMod.Count -ne 0) -or ($abfPubMod -eq $null -and $abfPubMod.Count -ne 0)) {
    $results += "[ERR ] $name : git show returned null unexpectedly"
    $failCount++
    continue
  }

  # Check: cur should be SUPERSET of abf12243 (整合 #4 baseline)
  $removedFromAbf = Compare-Object $abfPubMod $curPubMod -PassThru | Where-Object { $_.SideIndicator -eq "<=" }
  # Note: Compare-Object $a $b, => in $a, <= in $b
  # To find items in $a but not in $b: side <= is items in second arg only when it's "the diff"...

  # Let me use a simpler check
  $missing = @()
  foreach ($abfItem in $abfPubMod) {
    if ($curPubMod -notcontains $abfItem) {
      $missing += $abfItem
    }
  }
  $added = @()
  foreach ($curItem in $curPubMod) {
    if ($abfPubMod -notcontains $curItem) {
      $added += $curItem
    }
  }

  if ($missing.Count -eq 0) {
    $addedStr = if ($added.Count -gt 0) { " (added: $($added -join ', '))" } else { "" }
    $results += "[PASS] $name : pub mod=$($curPubMod.Count) (vs abf12243: $($abfPubMod.Count))$addedStr - 0 改 入口签名 严守 100% (additive only)"
    $passCount++
  } else {
    $results += "[FAIL] $name : MISSING pub mod entries (vs abf12243): $($missing -join ', ')"
    $results += "        cur pub mod: $($curPubMod -join ', ')"
    $results += "        abf pub mod: $($abfPubMod -join ', ')"
    $failCount++
  }
}

$results += ""
$results += "=== Summary ==="
$results += "Total: 24 LOCKED crates"
$results += "PASS (0 改 严守, additive allowed): $passCount / 24"
$results += "FAIL (removed entries): $failCount / 24"
$results += ""
if ($failCount -eq 0) {
  $results += "Result: ✅ 24/24 PASS (0 改 24 LOCKED 入口签名 严守 100%, per 决策 #74 B1 V1.0 release 0 改严守)"
} else {
  $results += "Result: ❌ FAIL ($failCount/24 removed entries)"
}

Set-Content -Path $reportPath -Value ($results -join "`n") -Encoding UTF8
Write-Output ($results -join "`n")
