Set-Location "Apeireth-rust"

# 24 LOCKED crates per R131-5 1:28 baseline
$lockedCrates = @(
  @{ Name = "supervisor"; Path = "crates/apeireth-supervisor/src/lib.rs"; BaselineMtime = "2026-08-06 08:06:43" },
  @{ Name = "agent"; Path = "crates/apeireth-agent/src/lib.rs"; BaselineMtime = "2026-08-10 21:48:02" },
  @{ Name = "council"; Path = "crates/apeireth-council/src/lib.rs"; BaselineMtime = "2026-08-10 03:31:20" },
  @{ Name = "bus"; Path = "crates/apeireth-bus/src/lib.rs"; BaselineMtime = "2026-08-10 15:54:20" },
  @{ Name = "protocol"; Path = "crates/apeireth-protocol/src/lib.rs"; BaselineMtime = "2026-08-10 00:33:07" },
  @{ Name = "mcp"; Path = "crates/apeireth-mcp/src/lib.rs"; BaselineMtime = "2026-08-10 17:53:13" },
  @{ Name = "tool-registry"; Path = "crates/apeireth-tool-registry/src/lib.rs"; BaselineMtime = "2026-08-10 03:10:31" },
  @{ Name = "tool-runtime"; Path = "crates/apeireth-tool-runtime/src/lib.rs"; BaselineMtime = "2026-08-10 21:50:59" },
  @{ Name = "graph"; Path = "crates/apeireth-graph/src/lib.rs"; BaselineMtime = "2026-08-10 21:52:15" },
  @{ Name = "pipeline"; Path = "crates/apeireth-pipeline/src/lib.rs"; BaselineMtime = "2026-08-10 21:22:20" },
  @{ Name = "tool-approval"; Path = "crates/apeireth-tool-approval/src/lib.rs"; BaselineMtime = "2026-08-10 16:18:12" },
  @{ Name = "extension"; Path = "crates/apeireth-extension/src/lib.rs"; BaselineMtime = "2026-08-06 08:06:43" },
  @{ Name = "evolution"; Path = "crates/apeireth-evolution/src/lib.rs"; BaselineMtime = "2026-08-10 21:45:12" },
  @{ Name = "api"; Path = "crates/apeireth-api/src/lib.rs"; BaselineMtime = "2026-08-10 22:22:38" },
  @{ Name = "core"; Path = "crates/apeireth-core/src/lib.rs"; BaselineMtime = "2026-08-09 20:48:47" },
  @{ Name = "memory"; Path = "crates/apeireth-memory/src/lib.rs"; BaselineMtime = "2026-08-10 03:43:14" },
  @{ Name = "asi"; Path = "crates/apeireth-asi/src/lib.rs"; BaselineMtime = "2026-08-10 16:18:12" },
  @{ Name = "tools"; Path = "crates/apeireth-tools/src/lib.rs"; BaselineMtime = "2026-08-09 02:01:52" },
  @{ Name = "cli"; Path = "crates/apeireth-cli/src/lib.rs"; BaselineMtime = "2026-08-10 21:29:44" },
  @{ Name = "bench"; Path = "crates/apeireth-bench/src/lib.rs"; BaselineMtime = "2026-08-10 03:32:18" },
  @{ Name = "cognition"; Path = "crates/apeireth-cognition/src/lib.rs"; BaselineMtime = "2026-08-06 08:06:43" },
  @{ Name = "action"; Path = "crates/apeireth-action/src/lib.rs"; BaselineMtime = "2026-08-06 08:06:43" },
  @{ Name = "life-force"; Path = "crates/apeireth-life-force/src/lib.rs"; BaselineMtime = "2026-08-06 20:02:17" },
  @{ Name = "constraint"; Path = "crates/apeireth-constraint/src/lib.rs"; BaselineMtime = "2026-08-06 08:06:43" }
)

$reportPath = "Apeireth-rust\reports\agent-r154-3-24-locked-sig-verify-2026-08-11.log"
$results = @()
$results += "=== R154-3 24 LOCKED 入口签名 verify (R131-5 1:28 baseline 严守, 决策 #74 B1 V1.0 release 0 改) ==="
$results += ""
$results += "master HEAD = $(git rev-parse HEAD)"
$results += "Verify time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$results += ""
$results += "Verification approach: Compare pub mod / pub use NAMES (extracted by parsing 'pub mod <name>;' / 'pub use <name>::')"
$results += "between HEAD and current working dir. Entry signature = set of public module names."
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

function Get-PubUseNames($filePath) {
  if (-not (Test-Path $filePath)) { return @() }
  $names = @()
  $content = Get-Content $filePath
  foreach ($line in $content) {
    # pub use xxx::... -> extract first segment
    if ($line -match "^\s*pub\s+use\s+([a-zA-Z_][a-zA-Z0-9_]*)::") {
      $names += $Matches[1]
    } elseif ($line -match "^\s*pub\s+use\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;") {
      # pub use crate::Type; (single type)
      $names += $Matches[1] + " (single)"
    }
  }
  return ($names | Sort-Object -Unique)
}

function Get-GitPubModNames($gitPath) {
  $names = @()
  $content = git show "HEAD:./$gitPath" 2>&1
  if ($LASTEXITCODE -ne 0) { return $null }
  foreach ($line in $content) {
    if ($line -match "^\s*pub\s+mod\s+([a-zA-Z_][a-zA-Z0-9_]*)") {
      $names += $Matches[1]
    }
  }
  return ($names | Sort-Object -Unique)
}

function Get-GitPubUseNames($gitPath) {
  $names = @()
  $content = git show "HEAD:./$gitPath" 2>&1
  if ($LASTEXITCODE -ne 0) { return $null }
  foreach ($line in $content) {
    if ($line -match "^\s*pub\s+use\s+([a-zA-Z_][a-zA-Z0-9_]*)::") {
      $names += $Matches[1]
    } elseif ($line -match "^\s*pub\s+use\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;") {
      $names += $Matches[1] + " (single)"
    }
  }
  return ($names | Sort-Object -Unique)
}

foreach ($crate in $lockedCrates) {
  $path = $crate.Path
  $name = $crate.Name
  $baseline = $crate.BaselineMtime

  if (-not (Test-Path $path)) {
    $results += "[FAIL] $name : FILE NOT FOUND at $path"
    $failCount++
    continue
  }

  $curPubMod = Get-PubModNames $path
  $curPubUse = Get-PubUseNames $path
  $headPubMod = Get-GitPubModNames $path
  $headPubUse = Get-GitPubUseNames $path

  if ($null -eq $headPubMod -or $null -eq $headPubUse) {
    $results += "[ERR ] $name : git show failed"
    $failCount++
    continue
  }

  $modDiff = Compare-Object $curPubMod $headPubMod
  $useDiff = Compare-Object $curPubUse $headPubUse

  $modChanged = $modDiff -and $modDiff.Count -gt 0
  $useChanged = $useDiff -and $useDiff.Count -gt 0

  if (-not $modChanged -and -not $useChanged) {
    $results += "[PASS] $name : pub mod=$($curPubMod.Count) pub use_targets=$($curPubUse.Count), 0 改 entry sig (cur == HEAD)"
    $passCount++
  } else {
    $results += "[FAIL] $name : SIG CHANGED!"
    $results += "        cur pub mod: $($curPubMod -join ', ')"
    $results += "        HEAD pub mod: $($headPubMod -join ', ')"
    if ($modChanged) {
      $results += "        pub mod diff (cur vs HEAD):"
      foreach ($d in $modDiff) { $results += "          $($d.SideIndicator) $($d.InputObject)" }
    }
    if ($useChanged) {
      $results += "        pub use diff (cur vs HEAD):"
      foreach ($d in $useDiff) { $results += "          $($d.SideIndicator) $($d.InputObject)" }
    }
    $failCount++
  }
}

$results += ""
$results += "=== Summary ==="
$results += "Total: 24 LOCKED crates"
$results += "PASS: $passCount / 24"
$results += "FAIL: $failCount / 24"
$results += ""
$results += "Result: $(if($failCount -eq 0){'? 24/24 PASS (0 改 入口签名 严守 100%)'}else{'? FAIL'})"

Set-Content -Path $reportPath -Value ($results -join "`n") -Encoding UTF8
Write-Output ($results -join "`n")
