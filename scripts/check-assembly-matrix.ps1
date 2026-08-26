# Root workspace assembly/documentation validation (suites.toml)
# 用法: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-assembly-matrix.ps1
# 日志: logs/assembly-matrix.log (验收证据)
$ErrorActionPreference = "Continue"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$LogDir = Join-Path $RepoRoot "logs"
$Log = Join-Path $LogDir "assembly-matrix.log"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
Set-Content $Log "assembly matrix run: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"

$cases = @(
    @{ Name = "1-cli-default"; Args = @("check", "-p", "apeireth-cli", "--locked"); Expect = "pass" },
    @{ Name = "2-cli-no-default-base"; Args = @("check", "-p", "apeireth-cli", "--no-default-features", "--features", "base", "--locked"); Expect = "pass" },
    @{ Name = "3-memory-no-default"; Args = @("check", "-p", "apeireth-memory", "--no-default-features", "--locked"); Expect = "pass" },
    @{ Name = "4-workspace"; Args = @("check", "--workspace", "--all-targets", "--locked"); Expect = "pass" }
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
