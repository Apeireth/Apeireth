# scripts/target-sweep.ps1 —— target/ 手术式清理 v2（2026-10-16 验收整改 R1/R2）
# 口径：删 incremental/doc/**stale pdb** 与 src-tauri/target 同档；保留 deps/build/.fingerprint
# Target dir source: RESOLVED at runtime ($env:CARGO_TARGET_DIR => cargo metadata => fallback <RepoRoot>\target)
# （pdb 可由 cargo 随时重建、零重编——台账 live-verification-ledger #40 已验证；RUST_BACKTRACE 符号化时触发重链）
param([switch]$DryRun, [switch]$Force, [switch]$All)
$RepoRoot = Split-Path $PSScriptRoot -Parent
$helper = Join-Path $PSScriptRoot 'target_dir.py'

# --- Resolve the REAL cargo target dir (no hardcoded <repo>\target) -------------
# NOTE (2026-11-17): this block is intentionally ASCII-only.
#   These .ps1 files are UTF-8 WITHOUT BOM. Windows PowerShell 5.1 decodes them
#   with the ANSI code page (cp936) => the mojibake can swallow the next ASCII
#   char (e.g. `{` right after a full-width colon) and even merge lines
#   => parse errors. The real logic lives in scripts/target_dir.py (Python,
#   always UTF-8); this block is only a thin PowerShell adapter:
#     1) $env:CARGO_TARGET_DIR  (a relative value is anchored at RepoRoot)
#     2) python scripts/target_dir.py  (it runs cargo metadata internally)
#     3) fallback <RepoRoot>\target, WITH an explicit notice (never silent).
#   A missing or broken Python must NOT crash the script => degrade to fallback.
function Resolve-CargoTargetDir {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$HelperPath
    )
    # 1) env var first (cargo itself looks at it first too)
    if ($env:CARGO_TARGET_DIR) {
        $raw = $env:CARGO_TARGET_DIR
        if (-not [System.IO.Path]::IsPathRooted($raw)) { $raw = Join-Path $RepoRoot $raw }
        return [System.IO.Path]::GetFullPath($raw)
    }
    # 2) shared helper (inside: env => cargo metadata => fallback)
    if (Test-Path $HelperPath) {
        $python = if ($env:PYTHON) { $env:PYTHON } else { 'python' }
        $prevEap = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'   # native stderr throws under PS 5.1 with EAP=Stop
        $merged = @()
        $code = -1
        $failure = ''
        try {
            $merged = & $python $HelperPath --repo-root $RepoRoot 2>&1
            $code = $LASTEXITCODE
        }
        catch {
            $failure = $_.Exception.Message
        }
        finally {
            $ErrorActionPreference = $prevEap
        }
        # stdout entries are String (the path); stderr entries are ErrorRecord (notices)
        $outLines = @($merged | Where-Object { $_ -is [string] } | ForEach-Object { "$_".Trim() } | Where-Object { $_ -ne '' })
        $errLines = @($merged | Where-Object { $_ -is [System.Management.Automation.ErrorRecord] } | ForEach-Object { "$_".TrimEnd() } | Where-Object { $_ -ne '' })
        foreach ($errLine in $errLines) { Write-Host $errLine }   # helper notices => never silent
        if ($code -eq 0 -and $outLines.Count -ge 1) {
            $resolved = $outLines[0]
            try { $resolved = [System.IO.Path]::GetFullPath($resolved) } catch { }
            return $resolved
        }
        if ($failure -ne '') {
            Write-Host ("[target-dir] WARN: cannot run python scripts/target_dir.py : {0}" -f $failure)
        }
        else {
            Write-Host ("[target-dir] WARN: python scripts/target_dir.py exit code {0} => using fallback" -f $code)
        }
    }
    # 3) fallback (with an explicit notice => never silent)
    $fallback = Join-Path $RepoRoot 'target'
    Write-Host ("[target-dir] WARN: cannot resolve the real cargo target dir => fallback to {0} (<RepoRoot>\target)" -f $fallback)
    return [System.IO.Path]::GetFullPath($fallback)
}

# 根 cargo workspace 的产物目录
$root = Resolve-CargoTargetDir -RepoRoot $RepoRoot -HelperPath $helper
# `frontend\companion-desktop\src-tauri` 是**另一个独立 cargo 项目** ⇒ 它有自己的产物目录
$tauriRoot = Join-Path $RepoRoot 'frontend\companion-desktop\src-tauri'
$tauri = Resolve-CargoTargetDir -RepoRoot $tauriRoot -HelperPath $helper
function Get-Size($p) { if (Test-Path $p) { (Get-ChildItem $p -Recurse -File -Force -EA SilentlyContinue | Measure-Object Length -Sum).Sum } else { 0 } }
# ⚠️ 两个 cargo 项目**可能解析到同一个目录**（例如 CARGO_TARGET_DIR 指向同一处）
#    ⇒ ⇒ 先去重再求和/再删 ⇒ 否则 50GB 阈值被双计数虚高、删除也会重复跑一遍
$bases = @($root)
if ($tauri -ne $root) { $bases += $tauri }
$total = 0
foreach ($b in $bases) { $total += (Get-Size $b) }
"target/ + src-tauri/target 当前: $([math]::Round($total/1GB,1)) GB"
if ($total -lt 50GB -and -not $Force -and -not $All) { "低于 50GB 阈值（-Force 强制 / -All 全档）"; exit 0 }
foreach ($base in $bases) {
  if (-not (Test-Path $base)) { continue }
  $dirs = @("$base\debug\incremental", "$base\release\incremental", "$base\doc", "$base\x86_64-pc-windows-msvc\release\incremental")
  foreach ($t in $dirs) { $s = Get-Size $t; if ($s -gt 0) { if ($DryRun) { "[dry-run] 将删: $t ($([math]::Round($s/1GB,1)) GB)" } else { Remove-Item $t -Recurse -Force -EA SilentlyContinue; "已删: $t ($([math]::Round($s/1GB,1)) GB)" } } }
  # R1：stale pdb 档（除正在使用的 build 目录 pdb 也一并可再生）
  $pdbs = Get-ChildItem $base -Recurse -Filter *.pdb -File -Force -EA SilentlyContinue
  if ($pdbs) { $s = ($pdbs | Measure-Object Length -Sum).Sum; if ($DryRun) { "[dry-run] 将删 pdb: $($pdbs.Count) 个 ($([math]::Round($s/1GB,1)) GB) 于 $base" } else { $pdbs | Remove-Item -Force -EA SilentlyContinue; "已删 pdb: $($pdbs.Count) 个 ($([math]::Round($s/1GB,1)) GB) 于 $base" } }
}
$after = 0
foreach ($b in $bases) { $after += (Get-Size $b) }
"现在: $([math]::Round($after/1GB,1)) GB（回收 $([math]::Round(($total-$after)/1GB,1)) GB）"
