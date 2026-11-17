# ============================================================================
# measure-session-startup.ps1 — `apeireth session` 启动到 ready 全进程实测脚本
# ----------------------------------------------------------------------------
# 口径: 全进程墙钟 = 进程创建 → 组装到 ready (打印 `canonical runtime ready`) →
#       退出。含 tokio 运行时创建与进程退出 (比 cold-start 基准口径宽, 如实标注)。
# 形态: 真冷 (每轮全新数据目录) 与 热开 (同一数据目录重复启动) 两组各 N 轮。
# 前置: `cargo build --release --locked -p apeireth-cli` 已产出
#       <resolved target dir>\release\apeireth.exe
#       (the target dir is resolved by scripts/target_dir.py - no hardcoded <repo>\target).
# 用法: pwsh -NoProfile -File scripts/measure-session-startup.ps1 [-Runs 12]
# 输出: 每组 min/mean/P50/max + 逐轮原始毫秒值; 退出码 0 = 全部启动成功。
# 纪律: 数字可复核 (本脚本 + 日期 + 工具链, 见输出抬头)。
# ============================================================================

param(
    [int]$Runs = 12
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

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

Push-Location $RepoRoot

try {
    $targetDir = Resolve-CargoTargetDir -RepoRoot $RepoRoot -HelperPath (Join-Path $PSScriptRoot 'target_dir.py')
    $exe = Join-Path $targetDir "release\apeireth.exe"
    if (-not (Test-Path $exe)) {
        Write-Output "FAIL  缺 $exe —— 先跑 cargo build --release --locked -p apeireth-cli"
        exit 2
    }

    $stamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss K"
    $rustcV = (rustc --version) 2>&1
    $gitRev = (git rev-parse --short HEAD) 2>&1
    Write-Output "# apeireth session 启动实测 (scripts/measure-session-startup.ps1)"
    Write-Output "- 时间: $stamp / 工具链: $rustcV / 仓库版本: $gitRev / 轮数: $Runs"
    Write-Output "- 口径: 全进程墙钟 (进程创建 → ready → 退出, 含 tokio 运行时创建)"
    Write-Output ""

    $base = Join-Path $env:TEMP ("apeireth-session-startup-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $base | Out-Null

    function Measure-Session([string]$dir) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $env:APEIRETH_DATA_DIR = $dir
        $env:APEIRETH_SESSION_DB = Join-Path $dir "sessions.sqlite3"
        $env:APEIRETH_COGNITIVE_DB = Join-Path $dir "cognitive.sqlite3"
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $out = & $exe session 2>&1 | Out-String
        $sw.Stop()
        if ($LASTEXITCODE -ne 0) {
            throw "apeireth session 启动失败 exit=$LASTEXITCODE out=$out"
        }
        if ($out -notmatch "canonical runtime ready") {
            throw "启动输出未见 ready 行: $out"
        }
        return $sw.Elapsed.TotalMilliseconds
    }

    function Stats([double[]]$values, [string]$label, [string]$note) {
        $sorted = $values | Sort-Object
        $n = $sorted.Count
        $p50 = $sorted[[math]::Floor(($n - 1) * 0.5)]
        $mean = ($sorted | Measure-Object -Average).Average
        Write-Output ("{0}: n={1} min={2:N2} mean={3:N2} P50={4:N2} max={5:N2} ms ({6})" -f `
            $label, $n, ($sorted | Measure-Object -Minimum).Minimum, $mean, $p50, ($sorted | Measure-Object -Maximum).Maximum, $note)
        Write-Output ("  raw: " + (($sorted | ForEach-Object { "{0:N2}" -f $_ }) -join " "))
    }

    $cold = @()
    for ($i = 0; $i -lt $Runs; $i++) {
        $cold += Measure-Session (Join-Path $base "cold-$i")
    }
    Stats @($cold) "真冷 (每轮全新数据目录)" "A组"

    $warmDir = Join-Path $base "warm"
    $warm = @()
    for ($i = 0; $i -lt $Runs; $i++) {
        $warm += Measure-Session $warmDir
    }
    Stats @($warm) "热开 (同库重复启动)" "B组"

    Remove-Item -Recurse -Force $base -ErrorAction SilentlyContinue
    exit 0
}
finally {
    Pop-Location
}
