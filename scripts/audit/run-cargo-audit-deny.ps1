# ============================================================================
# R20 阶段 6 — cargo audit + cargo deny 一键扫描 (Windows 同等)
# ============================================================================
# 用途: Windows PowerShell 版本, 与 .sh 同等行为
# 触发: 主人 2026-08-05 21:18 拍板"真派"
# 用法: pwsh -File scripts/audit/run-cargo-audit-deny.ps1
# ============================================================================

$ErrorActionPreference = 'Continue'  # 不让 audit/deny 非零退出中断流程

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot  = Resolve-Path (Join-Path $ScriptDir "..\..")
$ReportsDir = Join-Path $RepoRoot "reports"
$Date = (Get-Date -Format "yyyy-MM-dd")

if (-not (Test-Path $ReportsDir)) { New-Item -ItemType Directory -Path $ReportsDir | Out-Null }

Set-Location $RepoRoot

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  R20 阶段 6 - cargo audit + cargo deny (Windows)"
Write-Host "  Repo: $RepoRoot"
Write-Host "  Date: $Date"
Write-Host "============================================================"
Write-Host ""

# ---------- 1. cargo audit ----------
Write-Host ">>> [1/2] cargo audit" -ForegroundColor Yellow
cargo audit --json 2> "$RepoRoot\audit-audit.stderr.txt" | Out-File -FilePath "$RepoRoot\audit-report.json" -Encoding utf8
$AuditExit = $LASTEXITCODE
cargo audit 2>&1 | Out-File -FilePath "$ReportsDir\r20-cargo-audit-stdout-$Date.txt" -Encoding utf8
Write-Host "  audit exit=$AuditExit (0=clean, 1=vuln)"
Write-Host ""

# ---------- 2. cargo deny ----------
Write-Host ">>> [2/2] cargo deny check" -ForegroundColor Yellow
cargo deny check 2>&1 | Out-File -FilePath "$ReportsDir\r20-cargo-deny-stdout-$Date.txt" -Encoding utf8
$DenyExit = $LASTEXITCODE
Write-Host "  deny exit=$DenyExit (0=clean, 3=fail)"
Write-Host ""

# ---------- 汇总 ----------
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  扫描结果汇总"
Write-Host "============================================================"
$VulnCount = (Select-String -Path "$RepoRoot\audit-report.json" -Pattern '"count":\d+' | Select-Object -First 1) -replace '"count":', ''
Write-Host "  cargo audit: $VulnCount vulnerabilities, exit=$AuditExit"
Write-Host "  cargo deny:  exit=$DenyExit"
Write-Host "  报告:        reports/r20-cargo-audit-stdout-$Date.txt"
Write-Host "  JSON:        audit-report.json"
Write-Host "============================================================"

if ($AuditExit -ne 0) { exit $AuditExit }
if ($DenyExit  -ne 0) { exit $DenyExit }
exit 0
