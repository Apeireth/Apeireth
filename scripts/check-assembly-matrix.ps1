# B2 workspace 级装配层编译验证矩阵 (docs/release-plan.md §三.4 / suites.toml)
# 用法: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-assembly-matrix.ps1
# 日志: logs/assembly-matrix.log (验收证据)
$ErrorActionPreference = "Continue"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$LogDir = Join-Path $RepoRoot "logs"
$Log = Join-Path $LogDir "assembly-matrix.log"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
Set-Content $Log "assembly matrix run: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"

$cases = @(
    @{ Name = "1-base-only (档1 本体)"; Args = @("check", "-p", "apeireth-cli", "--no-default-features", "--features", "base"); Expect = "pass" },
    @{ Name = "2-pack-local-intel (档2 本地智能包)"; Args = @("check", "-p", "apeireth-cli", "--no-default-features", "--features", "base,local-intel"); Expect = "pass" },
    @{ Name = "3-pack-gui (档2 GUI包)"; Args = @("check", "-p", "apeireth-cli", "--no-default-features", "--features", "base,gui"); Expect = "pass" },
    @{ Name = "4-all-packs-suites (三档全开)"; Args = @("check", "-p", "apeireth-cli", "--all-features"); Expect = "pass" },
    @{ Name = "5-memory-no-default (54ed4c7d: semantic cfg 门控后转 PASS)"; Args = @("check", "-p", "apeireth-memory", "--no-default-features"); Expect = "pass" },
    @{ Name = "6-memory-onnx (本地智能包 crate 级)"; Args = @("check", "-p", "apeireth-memory", "--features", "onnx"); Expect = "pass" },
    @{ Name = "7-memory-no-default-with-tests (测试代码 cfg 门控校验)"; Args = @("check", "-p", "apeireth-memory", "--no-default-features", "--tests"); Expect = "pass" }
)

$summary = @()
foreach ($c in $cases) {
    $cmd = "cargo " + ($c.Args -join " ")
    Add-Content $Log ""
    Add-Content $Log "==== [$($c.Name)] $cmd (expect: $($c.Expect)) ===="
    & cargo @($c.Args) 2>&1 | ForEach-Object { Add-Content $Log $_ }
    $code = $LASTEXITCODE
    $verdict = if ($c.Expect -eq "known-debt") { "known-debt (exit=$code)" }
               elseif ($code -eq 0) { "PASS" } else { "FAIL" }
    Add-Content $Log "==== [$($c.Name)] exit=$code => $verdict ===="
    $summary += "{0} => {1}" -f $c.Name, $verdict
}

Add-Content $Log ""
Add-Content $Log "==== SUMMARY ===="
foreach ($s in $summary) { Add-Content $Log $s; Write-Host $s }
