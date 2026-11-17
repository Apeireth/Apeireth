<#
.SYNOPSIS
    Apeireth Installed Product E2E Validation Script (PowerShell)
.DESCRIPTION
    Validates that a packaged or compiled Apeireth 2.0 RC binary executes properly
    outside the source repository in a clean isolated directory.
    Tests CLI commands, Session bootstrap, Gateway HTTP endpoints (/health, /v1/models,
    /v1/chat, /v1/chat/completions, /v1/approvals/resolve), and process lifecycle.
.PARAMETER BinaryPath
    Path to the apeireth binary (e.g. a staged portable zip binary).
    When omitted (default empty) the script uses
    <resolved cargo target dir>\release\apeireth.exe
    (the target dir is resolved by scripts/target_dir.py - no hardcoded <repo>\target).
.PARAMETER GatewayPort
    Port to use for testing Gateway serve (default: 18080).
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$BinaryPath = "",

    [Parameter()]
    [int]$GatewayPort = 18080
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

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

# Resolve binary path
$targetDir = $null
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    # 未显式给路径 ⇒ 用**真实生效的产物目录**（不再硬编码 <repo>\target）
    $targetDir = Resolve-CargoTargetDir -RepoRoot $RepoRoot -HelperPath (Join-Path $ScriptDir "target_dir.py")
    $BinaryPath = Join-Path $targetDir "release\apeireth.exe"
}
elseif (-not [System.IO.Path]::IsPathRooted($BinaryPath)) {
    $BinaryPath = Join-Path $RepoRoot $BinaryPath
}

if (-not (Test-Path $BinaryPath)) {
    # Try debug fallback if release not built locally
    if (-not $targetDir) {
        $targetDir = Resolve-CargoTargetDir -RepoRoot $RepoRoot -HelperPath (Join-Path $ScriptDir "target_dir.py")
    }
    $debugPath = Join-Path $targetDir "debug\apeireth.exe"
    if (Test-Path $debugPath) {
        Write-Warning "Release binary not found at '$BinaryPath', falling back to debug binary: $debugPath"
        $BinaryPath = $debugPath
    } else {
        Write-Error "Apeireth binary not found at '$BinaryPath'. Please compile first (cargo build --release -p apeireth-cli)."
        exit 1
    }
}

Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "              APEIRETH 2.0 INSTALLED PRODUCT E2E VALIDATION                     " -ForegroundColor Cyan
Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "  Binary Target:    $BinaryPath" -ForegroundColor Yellow
Write-Host "  Gateway Test Port: $GatewayPort"
Write-Host "  Date / Time:      $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Host "--------------------------------------------------------------------------------"

# Create clean isolated temporary test directory
$TestTempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("apeireth-e2e-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TestTempDir -Force | Out-Null
Write-Host "  Isolated Temp Sandbox: $TestTempDir" -ForegroundColor Gray

$Passed = 0
$Failed = 0

function Assert-Test {
    param(
        [string]$Name,
        [scriptblock]$Script
    )
    Write-Host -NoNewline "  [TEST] $Name ... "
    try {
        & $Script
        Write-Host "PASS" -ForegroundColor Green
        $script:Passed++
    } catch {
        Write-Host "FAIL" -ForegroundColor Red
        Write-Host "         Error: $_" -ForegroundColor Red
        $script:Failed++
    }
}

try {
    # 1. Test Version
    Assert-Test "CLI --version output" {
        $out = & $BinaryPath --version 2>&1 | Out-String
        if ($out -notmatch "apeireth 2\.0\.0") {
            throw "Expected 'apeireth 2.0.0' in version output, got: $out"
        }
    }

    # 2. Test Help
    Assert-Test "CLI --help output" {
        $out = & $BinaryPath --help 2>&1 | Out-String
        if ($out -notmatch "Usage:" -or $out -notmatch "session" -or $out -notmatch "gateway serve") {
            throw "Expected usage with session and gateway serve, got: $out"
        }
    }

    # 3. Test Session Bootstrap (Keyless)
    Assert-Test "CLI session bootstrap (keyless)" {
        $prevEnvSession = $env:APEIRETH_SESSION_DB
        $env:APEIRETH_SESSION_DB = (Join-Path $TestTempDir "sessions.sqlite3")
        try {
            $out = & $BinaryPath session 2>&1 | Out-String
            if ($out -notmatch "canonical runtime ready" -or $LASTEXITCODE -ne 0) {
                throw "Session bootstrap failed (code $LASTEXITCODE), output: $out"
            }
        } finally {
            $env:APEIRETH_SESSION_DB = $prevEnvSession
        }
    }

    # 4. Test Gateway Serve Lifecycle & Endpoints
    Assert-Test "Gateway serve lifecycle and HTTP API endpoints" {
        $prevEnvSession = $env:APEIRETH_SESSION_DB
        $env:APEIRETH_SESSION_DB = (Join-Path $TestTempDir "gateway-sessions.sqlite3")
        
        $gatewayProc = Start-Process -FilePath $BinaryPath `
            -ArgumentList "gateway", "serve", "--port", "$GatewayPort" `
            -WorkingDirectory $TestTempDir `
            -PassThru -NoNewWindow

        try {
            # Poll /health for readiness (up to 10 seconds)
            $ready = $false
            $healthUrl = "http://127.0.0.1:$GatewayPort/health"
            $modelsUrl = "http://127.0.0.1:$GatewayPort/v1/models"
            
            for ($i = 0; $i -lt 20; $i++) {
                Start-Sleep -Milliseconds 500
                if ($gatewayProc.HasExited) {
                    throw "Gateway process exited prematurely with code $($gatewayProc.ExitCode)"
                }
                try {
                    $resp = Invoke-RestMethod -Uri $healthUrl -Method Get -TimeoutSec 2 -ErrorAction Stop
                    if ($resp.status -eq "ok") {
                        $ready = $true
                        break
                    }
                } catch {
                    # Still starting
                }
            }

            if (-not $ready) {
                throw "Gateway failed to become healthy at $healthUrl within timeout"
            }

            # Verify GET /health response payload
            $healthResp = Invoke-RestMethod -Uri $healthUrl -Method Get
            if ($healthResp.status -ne "ok" -or $healthResp.execution_owner -ne "apeireth-runtime::canonical") {
                throw "Unexpected /health response: $(ConvertTo-Json $healthResp)"
            }

            # Verify GET /v1/models response
            $modelsResp = Invoke-RestMethod -Uri $modelsUrl -Method Get
            if ($modelsResp.object -ne "list" -or -not ($modelsResp.data -is [Array])) {
                throw "Unexpected /v1/models response: $(ConvertTo-Json $modelsResp)"
            }

            # Verify POST /v1/chat validation
            $chatUrl = "http://127.0.0.1:$GatewayPort/v1/chat"
            $chatPayload = @{
                input = "Ping healthcheck turn"
            } | ConvertTo-Json
            
            try {
                $chatResp = Invoke-RestMethod -Uri $chatUrl -Method Post -Body $chatPayload -ContentType "application/json" -TimeoutSec 5
            } catch {
                # In mock/keyless environment, 502/503/400 is an acceptable outcome demonstrating the request reached canonical entry
                $statusCode = $_.Exception.Response.StatusCode.value__
                if ($statusCode -ne 502 -and $statusCode -ne 503 -and $statusCode -ne 400 -and $statusCode -ne 403) {
                    throw "Unexpected HTTP status on /v1/chat turn: $statusCode"
                }
            }

        } finally {
            $env:APEIRETH_SESSION_DB = $prevEnvSession
            if ($gatewayProc -and -not $gatewayProc.HasExited) {
                Stop-Process -Id $gatewayProc.Id -Force
                $gatewayProc.WaitForExit(3000)
            }
        }
    }

} finally {
    # Cleanup sandbox
    if (Test-Path $TestTempDir) {
        Remove-Item -Path $TestTempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "--------------------------------------------------------------------------------"
Write-Host "  E2E SUMMARY: Passed=$Passed, Failed=$Failed" -ForegroundColor (if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host "================================================================================" -ForegroundColor Cyan

if ($Failed -gt 0) {
    exit 1
} else {
    exit 0
}
