# 验收脚本 — 主人离场期间 Mavis 每 5 分钟跑一次
# 主人 8/10 02:55 派

$ErrorActionPreference = 'Continue'
Set-Location '.openclaw\workspace\promethean\Apeireth-rust'
$env:Path = '.cargo\bin;' + $env:Path

$report = @()
$fail = $false

function Ok($msg) { $script:report += "  OK  $msg" }
function Bad($msg) { $script:report += "  FAIL $msg"; $script:fail = $true }

# 1. workspace.version
$report += "[workspace.version]"
try {
  $c = Get-Content 'Cargo.toml' -Raw
  if ($c -match 'version\s*=\s*"(\d+\.\d+\.\d+)"') { Ok "version = $($Matches[1])" }
  else { Bad "version not found" }
} catch { Bad $_.Exception.Message }

# 2. R11 baseline 3 值 (在 tests/integration_r_measure.rs:42-44 LOCKED)
$report += "[R11 baseline 3 值]"
try {
  $f = Get-Content 'tests\integration_r_measure.rs' -Raw
  $allFound = $true
  foreach ($pair in @(@('R11_V1141_BASELINE: f64 = 0.8682','V1141'), @('R11_V1131_BASELINE: f64 = 0.8532','V1131'), @('R11_V1136_BASELINE: f64 = 0.9063','V1136'))) {
    if ($f -notmatch [regex]::Escape($pair[0])) { $allFound = $false; Bad "missing $($pair[1])" }
  }
  if ($allFound) { Ok "0.8682 / 0.8532 / 0.9063 LOCKED in tests/integration_r_measure.rs" }
} catch { Bad $_.Exception.Message }

# 3. 9 器官 + 5 LOCKED crate mtime 没动 (从今晚 02:55 起算, R119 已形式解锁但今晚 0 触碰仍是硬指标)
$report += "[9 器官 + LOCKED mtime (since 02:55)]"
try {
  $overnightStart = Get-Date '2026-08-10 02:55:00'
  $changed = @()
  foreach ($crate in @('apeireth-cognition','apeireth-core','apeireth-sovereignty','apeireth-formal','apeireth-asi','apeireth-onion','apeireth-naming-v05')) {
    $path = "crates\$crate"
    if (-not (Test-Path $path)) { continue }
    $latest = (Get-ChildItem $path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object LastWriteTime -Maximum -ErrorAction SilentlyContinue).Maximum
    if ($latest -gt $overnightStart) { $changed += "$crate ($($latest.ToString('HH:mm:ss')))" }
  }
  if ($changed.Count -eq 0) { Ok "all 7 LOCKED untouched since 02:55" }
  else { Bad "CHANGED since 02:55: $($changed -join ', ')" }
} catch { Bad $_.Exception.Message }

# 4. cargo metadata workspace 解析
$report += "[cargo metadata]"
try {
  $out = cargo metadata --no-deps --format-version=1 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { Bad "exit $LASTEXITCODE" }
  else {
    # 数 path+file:// 出现次数 (cargo metadata 每个 package 一个 id)
    $n = ([regex]::Matches($out, 'path\+file://')).Count
    Ok "$n packages in metadata"
  }
} catch { Bad $_.Exception.Message }

# 5. reports 写了几个 agent 报告
$report += "[agent 报告]"
try {
  $reports = Get-ChildItem 'reports\agent-*.md' -ErrorAction SilentlyContinue
  Ok "$($reports.Count) reports on disk"
  foreach ($r in $reports | Sort-Object LastWriteTime -Descending | Select-Object -First 5) {
    $report += "    $($r.Name) ($($r.LastWriteTime.ToString('HH:mm:ss')))"
  }
} catch { Bad $_.Exception.Message }

$report | ForEach-Object { Write-Host $_ }
if ($fail) { Write-Host ""; Write-Host "❌ BASELINE FAIL" -ForegroundColor Red; exit 1 }
else { Write-Host ""; Write-Host "✅ BASELINE OK" -ForegroundColor Green }
