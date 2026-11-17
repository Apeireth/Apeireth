<#
.SYNOPSIS
    Daily sentinel for the Apeireth build artifact slots (disk, memory, slots, strays).

.DESCRIPTION
    Read only report. The script never deletes a file and never kills a
    process. The only background job it starts is its own slot size scan,
    which is stopped when it exceeds the scan timeout.

    Sections:
      1. disk and memory of the drive that hosts the registered slots.
      2. size of every registered slot (docs/architecture/target-slots.json),
         measured with Get-ChildItem -Recurse -File | Measure-Object Length
         -Sum inside a job with a timeout.
      3. the authoritative audit of scripts/target_slot.py --audit. Its output
         is relayed verbatim, its exit code decides the exit code of this
         script. The stray judgement is NOT re implemented here.
      4. thresholds: free disk below -HardFreeMB is red, free disk below
         -WarnFreeMB is a yellow warning, stray total above -MaxStrayGB is red.

    Exit codes:
      0   everything is within the thresholds
      1   a red condition (disk, stray total or the audit itself)
      2   a tool or scheduled task failure

.NOTES
    Encoding: UTF-8 without BOM, LF line endings, ASCII only content.
    Windows PowerShell 5.1 decodes a UTF-8 without BOM file with the ANSI
    code page (cp936 on this machine). A non ASCII character can then swallow
    the next ASCII character and break the parse. Keep this file ASCII only.

    The Chinese text of the audit comes from the python tool at runtime, not
    from this file.
#>
param(
    [int]$WarnFreeMB = 40000,
    [int]$HardFreeMB = 15000,
    [int]$MaxStrayGB = 20,
    [switch]$Quiet,
    [switch]$RegisterTask
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$RegistryPath = Join-Path $RepoRoot 'docs/architecture/target-slots.json'
$SlotToolPath = Join-Path $RepoRoot 'scripts/target_slot.py'
$ScanTimeoutSec = 90

$exitCode = 0
$redReasons = @()
$yellowReasons = @()

function Write-Line {
    param([string]$Text)
    if (-not $Quiet) { Write-Output $Text }
}

function Write-Always {
    param([string]$Text)
    Write-Output $Text
}

function Write-Note {
    param([string]$Text)
    if (-not $Quiet) { [Console]::Error.WriteLine($Text) }
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

function Measure-SlotSize {
    # Size of a directory tree in bytes. The scan runs in a job so that a
    # slow or locked tree can not block the report; on timeout the job is
    # stopped and nothing is reported for that slot.
    param([string]$Path, [int]$TimeoutSec = 90)
    if (-not (Test-Path -LiteralPath $Path)) { return [int64]0 }
    $job = Start-Job -ScriptBlock {
        param($target)
        $sum = (Get-ChildItem -LiteralPath $target -Recurse -File -Force -ErrorAction SilentlyContinue |
            Measure-Object -Property Length -Sum).Sum
        if ($null -eq $sum) { return [int64]0 }
        return [int64]$sum
    } -ArgumentList $Path
    $null = Wait-Job -Job $job -Timeout $TimeoutSec
    if ($job.State -eq 'Completed') {
        $result = Receive-Job -Job $job
        Remove-Job -Job $job -Force
        if ($null -eq $result) { return [int64]0 }
        return [int64]$result
    }
    Stop-Job -Job $job
    Remove-Job -Job $job -Force
    return $null
}

# --- 1. scheduled task registration -----------------------------------------
if ($RegisterTask) {
    $taskName = 'Apeireth-DiskWatch'
    # A scheduled task must not depend on a store alias, so pwsh.exe is only
    # used when it is a real installation. The Windows PowerShell 5.1 host is
    # the fallback (this script runs on both).
    $shell = Get-Command pwsh.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $shell) {
        $candidate = $shell.Source
        if ([string]::IsNullOrEmpty($candidate)) { $candidate = $shell.Path }
        if ($candidate -like '*\WindowsApps\*') { $shell = $null }
    }
    if ($null -eq $shell) {
        $shell = Get-Command powershell.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    }
    if ($null -eq $shell) {
        Write-Err 'disk-watch.ps1: neither pwsh.exe nor powershell.exe was found, the scheduled task was NOT created.'
        $exitCode = 2
    } else {
        $shellPath = $shell.Source
        if ([string]::IsNullOrEmpty($shellPath)) { $shellPath = $shell.Path }
        $action = ('"{0}" -NoProfile -ExecutionPolicy Bypass -File "{1}" -Quiet' -f $shellPath, $PSCommandPath)
        Write-Line ("disk-watch.ps1: registering the daily task '{0}'" -f $taskName)
        $taskOutput = & schtasks.exe /create /tn $taskName /tr $action /sc daily /st 03:00 /f 2>&1
        $taskExit = $LASTEXITCODE
        foreach ($item in $taskOutput) {
            if ($item -is [System.Management.Automation.ErrorRecord]) {
                [Console]::Error.WriteLine($item.ToString())
            } else {
                Write-Output ([string]$item)
            }
        }
        if ($taskExit -eq 0) {
            Write-Always ("disk-watch.ps1: scheduled task '{0}' created (daily 03:00)." -f $taskName)
        } else {
            Write-Err ("disk-watch.ps1: schtasks failed with exit {0}, the task '{1}' was NOT created." -f $taskExit, $taskName)
            $exitCode = 2
        }
    }
}

# --- 2. registry ------------------------------------------------------------
$slotEntries = @()
$defaultSlot = 'base'
$reportDrive = $null
if (Test-Path -LiteralPath $RegistryPath) {
    try {
        $registry = Get-Content -LiteralPath $RegistryPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($registry.PSObject.Properties.Name -contains 'default_slot') { $defaultSlot = [string]$registry.default_slot }
        foreach ($prop in $registry.slots.PSObject.Properties) {
            $slotEntries += [pscustomobject]@{ Name = $prop.Name; Path = [string]$prop.Value.path }
        }
    } catch {
        Write-Err ("disk-watch.ps1: cannot read the slot registry ({0}): {1}" -f $RegistryPath, $_.Exception.Message)
        $exitCode = 2
    }
} else {
    Write-Err ("disk-watch.ps1: slot registry not found: {0}" -f $RegistryPath)
    $exitCode = 2
}

# --- 3. disk and memory -----------------------------------------------------
$driveId = $null
foreach ($entry in $slotEntries) {
    if ($entry.Name -eq $defaultSlot) {
        try { $driveId = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($entry.Path)).Substring(0, 2) } catch { $driveId = $null }
        break
    }
}
if ([string]::IsNullOrEmpty($driveId)) {
    try { $driveId = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($RepoRoot)).Substring(0, 2) } catch { $driveId = 'C:' }
}

$diskTotalMB = [int64]0
$diskFreeMB = [int64]0
$diskUsedPct = [double]0
try {
    $logical = Get-CimInstance -ClassName Win32_LogicalDisk -Filter ("DeviceID='{0}'" -f $driveId) -ErrorAction Stop
    if ($null -ne $logical) {
        $diskTotalMB = [int64]($logical.Size / 1MB)
        $diskFreeMB = [int64]($logical.FreeSpace / 1MB)
    }
} catch {
    Write-Err ("disk-watch.ps1: CIM query for drive {0} failed: {1}" -f $driveId, $_.Exception.Message)
}
if ($diskTotalMB -le 0) {
    try {
        $drive = New-Object System.IO.DriveInfo($driveId + '\')
        $diskTotalMB = [int64]($drive.TotalSize / 1MB)
        $diskFreeMB = [int64]($drive.AvailableFreeSpace / 1MB)
    } catch {
        Write-Err ("disk-watch.ps1: free space of drive {0} is unknown: {1}" -f $driveId, $_.Exception.Message)
    }
}
$diskUsedMB = $diskTotalMB - $diskFreeMB
if ($diskTotalMB -gt 0) { $diskUsedPct = [math]::Round((100.0 * $diskUsedMB) / $diskTotalMB, 1) }
Write-Line ("disk: drive={0} total_mb={1} used_mb={2} free_mb={3} used_pct={4}" -f `
        $driveId, $diskTotalMB, $diskUsedMB, $diskFreeMB, $diskUsedPct)

$memFreeMB = [int64]0
$memTotalMB = [int64]0
try {
    $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
    if ($null -ne $os) {
        $memFreeMB = [int64]($os.FreePhysicalMemory / 1024)
        $memTotalMB = [int64]($os.TotalVisibleMemorySize / 1024)
    }
} catch {
    Write-Err ("disk-watch.ps1: memory information is unavailable: {0}" -f $_.Exception.Message)
}
Write-Line ("memory: free_mb={0} total_mb={1}" -f $memFreeMB, $memTotalMB)

# --- 4. registered slots and their size -------------------------------------
foreach ($entry in $slotEntries) {
    $size = Measure-SlotSize -Path $entry.Path -TimeoutSec $ScanTimeoutSec
    if ($null -eq $size) {
        Write-Line ("slot: {0} path={1} size=timeout after {2}s" -f $entry.Name, $entry.Path, $ScanTimeoutSec)
    } else {
        Write-Line ("slot: {0} path={1} size_mb={2}" -f $entry.Name, $entry.Path, [int64]($size / 1MB))
    }
}

# --- 5. the authoritative audit, relayed verbatim ---------------------------
$python = Get-PythonCommand
$auditExit = 0
$strayGB = $null
if ($null -eq $python) {
    Write-Err 'disk-watch.ps1: python was not found on PATH, the slot audit could not run.'
    $exitCode = 2
} else {
    $auditOutput = & $python $SlotToolPath --audit 2>&1
    $auditExit = $LASTEXITCODE
    foreach ($item in $auditOutput) {
        if ($item -is [System.Management.Automation.ErrorRecord]) {
            Write-Note ($item.ToString())
        } else {
            Write-Line ([string]$item)
        }
    }
    try {
        $jsonText = (& $python $SlotToolPath --audit --json 2>$null | Out-String)
        $auditData = $jsonText | ConvertFrom-Json
        if ($null -ne $auditData.stray_total_gb) { $strayGB = [double]$auditData.stray_total_gb }
    } catch {
        Write-Err ("disk-watch.ps1: the stray total could not be read from the audit json: {0}" -f $_.Exception.Message)
    }
}

# --- 6. thresholds ----------------------------------------------------------
if ($diskFreeMB -lt $HardFreeMB) {
    $redReasons += ("free disk {0} MB is below the hard line {1} MB" -f $diskFreeMB, $HardFreeMB)
} elseif ($diskFreeMB -lt $WarnFreeMB) {
    $yellowReasons += ("free disk {0} MB is below the warning line {1} MB" -f $diskFreeMB, $WarnFreeMB)
}

if ($null -ne $strayGB -and $strayGB -gt $MaxStrayGB) {
    $redReasons += ("stray total {0} GB is above the limit {1} GB" -f $strayGB, $MaxStrayGB)
}

if ($auditExit -ne 0) {
    $redReasons += ("the slot audit failed with exit {0}" -f $auditExit)
}

if ($redReasons.Count -gt 0 -and $exitCode -eq 0) { $exitCode = 1 }

$result = 'OK'
if ($redReasons.Count -gt 0) { $result = 'RED' } elseif ($yellowReasons.Count -gt 0) { $result = 'WARN' }

$reasonText = 'none'
if ($redReasons.Count -gt 0) { $reasonText = ($redReasons -join '; ') }
elseif ($yellowReasons.Count -gt 0) { $reasonText = ($yellowReasons -join '; ') }

Write-Always ("disk-watch result={0} free_mb={1} hard_mb={2} warn_mb={3} stray_gb={4} max_stray_gb={5} audit_exit={6} reason={7}" -f `
        $result, $diskFreeMB, $HardFreeMB, $WarnFreeMB, $strayGB, $MaxStrayGB, $auditExit, $reasonText)

foreach ($reason in $yellowReasons) { Write-Note ("disk-watch WARNING: {0}" -f $reason) }
foreach ($reason in $redReasons) { Write-Note ("disk-watch RED: {0}" -f $reason) }

exit $exitCode
