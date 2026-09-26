# ============================================================================
# run-benchmarks.ps1 — 基准复现一键脚本 (无重跑脚本不发布纪律的脚本侧)
# ----------------------------------------------------------------------------
# 用法:
#   pwsh -NoProfile -File scripts/run-benchmarks.ps1                      # 全跑, 输出 markdown 表
#   pwsh -NoProfile -File scripts/run-benchmarks.ps1 -Bench hybrid-search # 单个基准 (独立命令)
#   pwsh -NoProfile -File scripts/run-benchmarks.ps1 -Bench list          # 列出全部基准 key
#   pwsh -NoProfile -File scripts/run-benchmarks.ps1 -Raw -OutFile reports\bench-raw.md
# 参数:
#   -Bench <key|all|list>  基准选择 (默认 all; key 见 -Bench list)
#   -Quick                 冒烟模式 (样本量 0.1 倍, 只验可跑不取正式数)
#   -Raw                   附逐样本原始值 (RAW 行)
#   -OutFile <path>        将完整输出 (环境 + 表 + 原始输出) 另存到文件
# 口径: `cargo run --release --locked -p apeireth-bench-harness`, 即产品发布
#       同款优化档 (workspace release: opt-level=3 / fat LTO) 下的实测值。
# 退出码: 0 = 成功; 非 0 = 基准或构建失败。
# ============================================================================

param(
    [string]$Bench = "all",
    [switch]$Quick,
    [switch]$Raw,
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot

try {
    if ($Quick) { $env:BENCH_QUICK = "1" }
    if ($Raw) { $env:BENCH_RAW = "1" }

    # ---- 环境信息 (随结果一同输出, 保证报告可追溯) ----
    $cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name
    $ramGb = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
    $os = (Get-CimInstance Win32_OperatingSystem).Caption
    $rustcV = (rustc --version) 2>&1
    $gitRev = (git rev-parse --short HEAD) 2>&1
    $stamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss K"

    $header = @()
    $header += "# 基准复现输出 (scripts/run-benchmarks.ps1)"
    $header += ""
    $header += "- 时间: $stamp"
    $header += "- 主机: $env:COMPUTERNAME / OS: $os"
    $header += "- CPU: $cpu / RAM: $ramGb GB"
    $header += "- 工具链: $rustcV"
    $header += "- 仓库版本: $gitRev"
    $header += "- 命令: ``pwsh -NoProfile -File scripts/run-benchmarks.ps1 -Bench $Bench$(if ($Quick) { ' -Quick' })$(if ($Raw) { ' -Raw' })``"
    $header += "- 优化档: ``cargo run --release --locked`` (workspace release: opt-level=3, lto=fat)"
    $header += ""

    # ---- 跑基准 (stdout = 基准输出; 构建进度走 stderr 不混入) ----
    $benchOut = & cargo run --release --locked -p apeireth-bench-harness -- $Bench
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Output "FAIL  bench harness exit=$code"
        exit $code
    }

    # ---- ROW 行 -> markdown 表 ----
    $rows = @($benchOut | Where-Object { $_ -is [string] -and $_.StartsWith("ROW|") })
    $table = @()
    if ($rows.Count -gt 0) {
        $table += "| 基准 | 操作口径 | 样本 n | 单位 | min | mean | stddev | P50 | P99 | max | 目标 | 判定 |"
        $table += "| :--- | :--- | ---: | :--- | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :--- |"
        foreach ($row in $rows) {
            $f = $row.Substring(4) -split "\|"
            # f: key,title,op,n,unit,min,mean,stddev,p50,p99,max,target,verdict,note
            $table += "| **$($f[1])** | $($f[2]) | $($f[3]) | $($f[4]) | $($f[5]) | $($f[6]) | $($f[7]) | $($f[8]) | $($f[9]) | $($f[10]) | $($f[11]) | $($f[12]) |"
        }
    }

    $result = @()
    $result += $header
    $result += $table
    $result += ""
    $result += "## 原始输出 (逐基准统计块)"
    $result += ""
    $result += '```text'
    $result += $benchOut
    $result += '```'

    $text = $result -join "`n"
    Write-Output $text

    if ($OutFile -ne "") {
        $dir = Split-Path -Parent $OutFile
        if ($dir -ne "" -and -not (Test-Path $dir)) {
            New-Item -ItemType Directory -Force -Path $dir | Out-Null
        }
        Set-Content -Path $OutFile -Value $text -Encoding UTF8
        Write-Output ""
        Write-Output "SAVED $OutFile"
    }
    exit 0
}
finally {
    Pop-Location
}
