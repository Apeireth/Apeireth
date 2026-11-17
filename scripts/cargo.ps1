<#
.SYNOPSIS
    Slot aware and disk guarded cargo launcher for the Apeireth repository.

.DESCRIPTION
    This script is a thin shell. Every decision is delegated to the
    repository tools:
      scripts/disk_guard.py               - disk pre check (fail closed)
      docs/architecture/target-slots.json - registry of the allowed slots
    The script never deletes a file and never kills a process.

    Target directory priority:
      1. CARGO_TARGET_DIR already set by the caller -> used verbatim
         (the caller wins, the slot is only the default).
      2. otherwise the registered slot root of -Slot.
      3. with -Line <slug>, a per line sub directory of that slot root
         (keeps parallel lines from fighting over the same build files).

    Environment hygiene for the cargo child process:
      RUSTC_WRAPPER             -> always emptied (sccache permission trap)
      CARGO_BUILD_JOBS         -> 2 when the caller did not set it
      CARGO_INCREMENTAL        -> 0 when the caller did not set it
      CARGO_PROFILE_TEST_DEBUG -> 0 when the caller did not set it
      PYTHONUTF8               -> 1 when the caller did not set it
      PYTHONIOENCODING         -> utf-8 when the caller did not set it
    A value that the caller already set always wins, except RUSTC_WRAPPER.
    A caller value that is empty or blank counts as "not set" (cargo ignores
    an empty value, an empty CARGO_BUILD_JOBS would silently give cargo all
    cores), so the repository default is applied in that case.

    Exit codes:
      0   cargo succeeded
      n   cargo exit code, passed through unchanged
      2   usage or configuration error (unknown slot, bad -Line, missing
          registry, cargo not found)
      3   disk pre check failed, cargo was not started (fail closed)

    The disk pre check never modifies the disk. When the free space is
    already below the threshold this script aborts itself and only asks for
    the report of the guard; the ensure mode of the guard (which deletes
    test binaries) is called only when there is enough space, so it cannot
    reclaim anything. Reclaiming stays an explicit operator action:
      python scripts/clean_test_binaries.py --target <target dir>

.NOTES
    Encoding: UTF-8 without BOM, LF line endings, ASCII only content.
    Windows PowerShell 5.1 decodes a UTF-8 without BOM file with the ANSI
    code page (cp936 on this machine). A non ASCII character can then swallow
    the next ASCII character and break the parse. Keep this file ASCII only.

    Argument binding (measured on this machine, see the delivery report):
    PowerShell binds script parameters, so trailing cargo arguments are not a
    lossless channel:
      * without PositionalBinding = false the first trailing token would land
        in -Slot or -Line, therefore it is switched off here and every
        trailing token is collected in -CargoArgs;
      * a value behind an unknown short option ("-p apeireth-core") is dropped
        by the binder on Windows PowerShell 5.1 and rejected as an ambiguous
        parameter name on PowerShell 7;
      * a bare "--" token is rejected by the binder.
    The reliable form is one quoted command string:
      pwsh -NoProfile -File scripts/cargo.ps1 -Slot base -CargoArgs "test -p apeireth-core -- --nocapture"
    A single -CargoArgs value that contains white space is split into separate
    arguments; quotes inside that string are honoured.
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$Slot = 'base',
    [string]$Line = '',
    [int]$EnsureDiskMB = 20000,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArgs
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$RegistryPath = Join-Path $RepoRoot 'docs/architecture/target-slots.json'
$DiskGuardPath = Join-Path $RepoRoot 'scripts/disk_guard.py'

function Write-Line {
    param([string]$Text)
    Write-Output $Text
}

function Write-Err {
    param([string]$Text)
    [Console]::Error.WriteLine($Text)
}

function Get-PythonCommand {
    foreach ($name in @('python', 'python3')) {
        $cmd = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -ne $cmd) { return $cmd.Source }
    }
    return $null
}

function Get-FreeMBOnDrive {
    param([string]$Path)
    try {
        $full = [System.IO.Path]::GetFullPath($Path)
        $root = [System.IO.Path]::GetPathRoot($full)
        if ([string]::IsNullOrEmpty($root)) { return [int64](-1) }
        $drive = New-Object System.IO.DriveInfo($root)
        return [int64]($drive.AvailableFreeSpace / 1MB)
    } catch {
        return [int64](-1)
    }
}

function Split-CommandString {
    # Splits a single command string into arguments. White space separates,
    # double or single quotes keep a value together and are removed.
    param([string]$Text)
    $tokens = New-Object 'System.Collections.Generic.List[string]'
    $current = New-Object System.Text.StringBuilder
    $inToken = $false
    $quote = [char]0
    foreach ($ch in $Text.ToCharArray()) {
        if ($quote -ne [char]0) {
            if ($ch -eq $quote) { $quote = [char]0 } else { [void]$current.Append($ch) }
            continue
        }
        if ($ch -eq '"' -or $ch -eq "'") { $quote = $ch; $inToken = $true; continue }
        if ([char]::IsWhiteSpace($ch)) {
            if ($inToken) {
                [void]$tokens.Add($current.ToString())
                [void]$current.Clear()
                $inToken = $false
            }
            continue
        }
        $inToken = $true
        [void]$current.Append($ch)
    }
    if ($inToken) { [void]$tokens.Add($current.ToString()) }
    return $tokens.ToArray()
}

function ConvertTo-NativeArgument {
    # Quotes one argument for the Windows command line of the child process.
    param([string]$Value)
    if ($null -eq $Value) { return '""' }
    if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') { return $Value }
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.Append('"')
    $slashes = 0
    foreach ($ch in $Value.ToCharArray()) {
        if ($ch -eq '\') { $slashes++; continue }
        if ($ch -eq '"') {
            [void]$sb.Append('\' * ($slashes * 2 + 1))
            [void]$sb.Append('"')
            $slashes = 0
            continue
        }
        if ($slashes -gt 0) {
            [void]$sb.Append('\' * $slashes)
            $slashes = 0
        }
        [void]$sb.Append($ch)
    }
    if ($slashes -gt 0) { [void]$sb.Append('\' * ($slashes * 2)) }
    [void]$sb.Append('"')
    return $sb.ToString()
}

# --- 1. slot registry and slot validation ---------------------------------
if (-not (Test-Path -LiteralPath $RegistryPath)) {
    Write-Err ("cargo.ps1: slot registry not found: {0}" -f $RegistryPath)
    exit 2
}

try {
    $registry = Get-Content -LiteralPath $RegistryPath -Raw -Encoding UTF8 | ConvertFrom-Json
} catch {
    Write-Err ("cargo.ps1: cannot read the slot registry ({0}): {1}" -f $RegistryPath, $_.Exception.Message)
    exit 2
}

$slotNames = @()
$slotPath = $null
foreach ($prop in $registry.slots.PSObject.Properties) {
    $slotNames += $prop.Name
    if ($prop.Name -eq $Slot) { $slotPath = [string]$prop.Value.path }
}

if ([string]::IsNullOrEmpty($slotPath)) {
    Write-Err ("cargo.ps1: unknown slot '{0}'. Registered slots: {1}" -f $Slot, ($slotNames -join ', '))
    Write-Err 'cargo.ps1: usage: pwsh -NoProfile -File scripts/cargo.ps1 -Slot base -CargoArgs "build --release"'
    exit 2
}

if ($Line -ne '') {
    if (($Line -notmatch '^[A-Za-z0-9._-]+$') -or ($Line -eq '.') -or ($Line -eq '..')) {
        Write-Err ("cargo.ps1: invalid -Line '{0}'. Allowed: letters, digits, dot, dash, underscore." -f $Line)
        exit 2
    }
}

# --- 2. target directory ----------------------------------------------------
$targetSource = 'slot'
$callerTargetDir = $env:CARGO_TARGET_DIR
if (-not [string]::IsNullOrEmpty($callerTargetDir)) {
    $targetDir = $callerTargetDir
    $targetSource = 'caller'
    Write-Line ("cargo.ps1: CARGO_TARGET_DIR is set by the caller, keeping caller setting: {0}" -f $targetDir)
    if ($Line -ne '') {
        Write-Line ("cargo.ps1: -Line '{0}' is ignored because the caller fixed CARGO_TARGET_DIR." -f $Line)
    }
} else {
    $targetDir = $slotPath
    if ($Line -ne '') { $targetDir = Join-Path $slotPath $Line }
}

# --- 3. cargo arguments -----------------------------------------------------
$argList = @()
if ($null -ne $CargoArgs) { $argList = @($CargoArgs) }
if ($argList.Count -eq 1 -and ($argList[0] -match '\s')) {
    $split = @()
    foreach ($token in (Split-CommandString $argList[0])) {
        if ($null -ne $token) { $split += $token }
    }
    $argList = $split
}
if ($argList.Count -eq 0) {
    Write-Err 'cargo.ps1: no cargo arguments given.'
    Write-Err 'cargo.ps1: usage: pwsh -NoProfile -File scripts/cargo.ps1 -Slot base -CargoArgs "build --release"'
    exit 2
}

$freeMB = Get-FreeMBOnDrive -Path $targetDir

# --- 4. disk pre check (fail closed, before cargo) ---------------------------
# The pre check must not modify the disk. The ensure mode of the guard
# (scripts/disk_guard.py --ensure) deletes test binaries when the disk is
# short, so the short case is decided here first and the ensure mode is only
# called when there is enough space. In that case the guard cannot reclaim
# anything and the pre check stays read only.
$python = Get-PythonCommand
$guardOk = $true

# Threshold of the repository guard: max(caller value, absolute hard line).
# The hard line can be moved with APEIRETH_DISK_HARD_FREE_MB, 0 switches it off.
$hardLineMB = 10000
$hardLineRaw = [System.Environment]::GetEnvironmentVariable('APEIRETH_DISK_HARD_FREE_MB')
if (-not [string]::IsNullOrWhiteSpace($hardLineRaw)) {
    $parsedHardLine = 0
    if ([int]::TryParse($hardLineRaw, [ref]$parsedHardLine)) { $hardLineMB = $parsedHardLine }
}
$effectiveEnsureMB = $EnsureDiskMB
if ($hardLineMB -gt 0 -and $hardLineMB -gt $effectiveEnsureMB) { $effectiveEnsureMB = $hardLineMB }

if ($null -eq $python) {
    Write-Line 'cargo.ps1: python was not found on PATH, disk pre check falls back to the .NET free space check.'
    if (($freeMB -ge 0) -and ($effectiveEnsureMB -gt 0) -and ($freeMB -lt $effectiveEnsureMB)) {
        $guardOk = $false
        Write-Err ("cargo.ps1: disk pre check failed: free {0} MB is below the required {1} MB." -f $freeMB, $effectiveEnsureMB)
    }
} else {
    $diskIsShort = ($freeMB -ge 0) -and ($effectiveEnsureMB -gt 0) -and ($freeMB -lt $effectiveEnsureMB)
    if ($diskIsShort) {
        # Read only report mode, the ensure mode would delete test binaries.
        Write-Line ("cargo.ps1: disk pre check: python {0} {1} (read only report)" -f $python, $DiskGuardPath)
        $reportOutput = & $python $DiskGuardPath 2>&1
        foreach ($item in $reportOutput) {
            if ($item -is [System.Management.Automation.ErrorRecord]) {
                [Console]::Error.WriteLine($item.ToString())
            } else {
                Write-Output ([string]$item)
            }
        }
        $guardOk = $false
        Write-Err ("cargo.ps1: disk pre check failed: free {0} MB is below the required {1} MB." -f $freeMB, $effectiveEnsureMB)
    } else {
        Write-Line ("cargo.ps1: disk pre check: python {0} {1} --ensure {2}" -f $python, $DiskGuardPath, $EnsureDiskMB)
        $guardOutput = & $python $DiskGuardPath --ensure $EnsureDiskMB 2>&1
        $guardExit = $LASTEXITCODE
        foreach ($item in $guardOutput) {
            if ($item -is [System.Management.Automation.ErrorRecord]) {
                [Console]::Error.WriteLine($item.ToString())
            } else {
                Write-Output ([string]$item)
            }
        }
        if ($guardExit -ne 0) {
            $guardOk = $false
            Write-Err ("cargo.ps1: disk pre check failed (scripts/disk_guard.py exit {0})." -f $guardExit)
        }
    }
}

if (-not $guardOk) {
    Write-Err 'cargo.ps1: aborting before cargo (fail closed, AGENTS.md 5.2).'
    Write-Err ("cargo.ps1: reclaim space with: python scripts/clean_test_binaries.py --target {0}" -f $targetDir)
    exit 3
}

# --- 5. environment hygiene for the cargo child process ---------------------
$currentEnv = [System.Environment]::GetEnvironmentVariables('Process')
$envOverrides = @{}
$envOverrides['CARGO_TARGET_DIR'] = $targetDir
$envOverrides['RUSTC_WRAPPER'] = ''
foreach ($pair in @(
        @('CARGO_BUILD_JOBS', '2'),
        @('CARGO_INCREMENTAL', '0'),
        @('CARGO_PROFILE_TEST_DEBUG', '0'),
        @('PYTHONUTF8', '1'),
        @('PYTHONIOENCODING', 'utf-8'))) {
    $name = $pair[0]
    $present = $currentEnv.Contains($name)
    $value = ''
    if ($present) { $value = [string]$currentEnv[$name] }
    if ($present -and -not [string]::IsNullOrWhiteSpace($value)) {
        # A real caller value wins.
        $envOverrides[$name] = $value
    } else {
        # Not set, or set to an empty value that cargo would ignore anyway.
        $envOverrides[$name] = $pair[1]
    }
}

# --- 6. one summary line, then cargo ----------------------------------------
Write-Line ("cargo-plan slot={0} target={1} source={2} free_mb={3} cmd=cargo {4}" -f `
        $Slot, $targetDir, $targetSource, $freeMB, ($argList -join ' '))

$envText = @()
foreach ($name in ($envOverrides.Keys | Sort-Object)) {
    $envText += ("{0}={1}" -f $name, $envOverrides[$name])
}
Write-Line ("cargo-env {0}" -f ($envText -join ' '))

$cargoCmd = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $cargoCmd) {
    Write-Err 'cargo.ps1: cargo was not found on PATH.'
    exit 2
}
$cargoExe = $cargoCmd.Source
if ([string]::IsNullOrEmpty($cargoExe)) { $cargoExe = $cargoCmd.Path }

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $cargoExe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = (Get-Location).Path
$quoted = @()
foreach ($item in $argList) { $quoted += ConvertTo-NativeArgument $item }
$psi.Arguments = ($quoted -join ' ')
foreach ($name in $envOverrides.Keys) {
    $psi.EnvironmentVariables[$name] = [string]$envOverrides[$name]
}

$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi
if (-not $proc.Start()) {
    Write-Err 'cargo.ps1: the cargo process could not be started.'
    exit 2
}
$proc.WaitForExit()
$code = $proc.ExitCode
Write-Line ("cargo-exit={0}" -f $code)
exit $code
