# scripts/target-sweep.ps1 —— target/ 手术式清理 v2（2026-10-16 验收整改 R1/R2）
# 口径：删 incremental/doc/**stale pdb** 与 src-tauri/target 同档；保留 deps/build/.fingerprint
# （pdb 可由 cargo 随时重建、零重编——台账 live-verification-ledger #40 已验证；RUST_BACKTRACE 符号化时触发重链）
param([switch]$DryRun, [switch]$Force, [switch]$All)
$root = Split-Path $PSScriptRoot -Parent | Join-Path -ChildPath 'target'
$tauri = Split-Path $PSScriptRoot -Parent | Join-Path -ChildPath 'frontend\companion-desktop\src-tauri\target'
function Get-Size($p) { if (Test-Path $p) { (Get-ChildItem $p -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum).Sum } else { 0 } }
$total = (Get-Size $root) + (Get-Size $tauri)
"target/ + src-tauri/target 当前: $([math]::Round($total/1GB,1)) GB"
if ($total -lt 50GB -and -not $Force -and -not $All) { "低于 50GB 阈值（-Force 强制 / -All 全档）"; exit 0 }
foreach ($base in @($root, $tauri)) {
  if (-not (Test-Path $base)) { continue }
  $dirs = @("$base\debug\incremental", "$base\release\incremental", "$base\doc", "$base\x86_64-pc-windows-msvc\release\incremental")
  foreach ($t in $dirs) { $s = Get-Size $t; if ($s -gt 0) { if ($DryRun) { "[dry-run] 将删: $t ($([math]::Round($s/1GB,1)) GB)" } else { Remove-Item $t -Recurse -Force -EA SilentlyContinue; "已删: $t ($([math]::Round($s/1GB,1)) GB)" } } }
  # R1：stale pdb 档（除正在使用的 build 目录 pdb 也一并可再生）
  $pdbs = Get-ChildItem $base -Recurse -Filter *.pdb -File -Force -EA SilentlyContinue
  if ($pdbs) { $s = ($pdbs | Measure-Object Length -Sum).Sum; if ($DryRun) { "[dry-run] 将删 pdb: $($pdbs.Count) 个 ($([math]::Round($s/1GB,1)) GB) 于 $base" } else { $pdbs | Remove-Item -Force -EA SilentlyContinue; "已删 pdb: $($pdbs.Count) 个 ($([math]::Round($s/1GB,1)) GB) 于 $base" } }
}
$after = (Get-Size $root) + (Get-Size $tauri)
"现在: $([math]::Round($after/1GB,1)) GB（回收 $([math]::Round(($total-$after)/1GB,1)) GB）"
