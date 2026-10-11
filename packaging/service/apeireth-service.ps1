#Requires -Version 5.1
<#
.SYNOPSIS
    Apeireth 常驻服务管理面 (Windows): install | start | stop | status | uninstall.

.DESCRIPTION
    把侧车网关注册为「登录自启 + 崩溃自拉起」的常驻服务 (Windows 计划任务),
    并给出启停/状态/卸载的运维入口。设计口径:

    * 常驻形态 = `apeireth gateway serve --bind ... --port ... --pid-file ...`,
      PID 文件即单实例占位 (二次启动显式报"已在运行", 不双开);
    * 令牌面 (G10): install 生成一枚 32 字节随机令牌 (64 hex), 存入系统凭据库
      (凭据目标 apeireth.gateway-token / 用户 apeireth, 本地机器持久)。
      **明文不落盘** —— 令牌只存在于凭据库与服务进程环境; 桌面壳 / TUI
      连常驻服务时从同一凭据条目取同源令牌。无令牌时后端三态门口径照旧
      (放行 + 警告帧 + 审计, 见 gateway 令牌门文档)。
    * 日志落 %LOCALAPPDATA%\Apeireth\logs\ (gateway-service.log);
      数据落 %LOCALAPPDATA%\Apeireth\data\ (会话库 / 认知库 / 审计档案)。
    * 优雅停机: stop 先走令牌门下的 `POST /v1/admin/shutdown`
      (收尾审计 + 落盘后退出), 不成就退回任务停止 / 进程树终止。

.PARAMETER Action
    install | start | stop | status | uninstall | run (run 为计划任务内部动作)。

.PARAMETER DryRun
    只打印计划、不动系统。任何动作都可安全 dry-run 复跑。

.PARAMETER Port
    网关监听端口 (缺省 8080)。取值 1..65535。

.PARAMETER Bind
    网关监听地址。缺省取 env `APEIRETH_LISTEN`, 再缺省 127.0.0.1 (仅本机)。
    非回环暴露由网关远程绑定闸强制令牌 (fail-closed: 无令牌拒启动)。

.PARAMETER BinaryPath
    `apeireth` 可执行文件路径。缺省按 脚本旁 / 仓库 target / PATH 顺序解析。

.PARAMETER TaskName
    计划任务名 (缺省 ApeirethGateway; 测试可用独立名字隔离)。

.PARAMETER RotateToken
    install 时强制换新令牌 (缺省复用既有令牌, 连接端不断线)。

.PARAMETER KeepToken
    uninstall 时保留令牌 (缺省随卸载删除)。

.EXAMPLE
    pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install
.EXAMPLE
    pwsh -NoProfile -File packaging/service/apeireth-service.ps1 stop
.EXAMPLE
    pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install -DryRun
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet('install', 'start', 'stop', 'status', 'uninstall', 'run')]
    [string]$Action,

    [switch]$DryRun,

    [ValidateRange(1, 65535)]
    [int]$Port = 8080,

    # W2-② 绑定旋钮: -Bind 显式给值优先, 缺省取 env APEIRETH_LISTEN, 再缺省仅本机。
    [string]$Bind = $(if ([string]::IsNullOrWhiteSpace($env:APEIRETH_LISTEN)) { '127.0.0.1' } else { $env:APEIRETH_LISTEN.Trim() }),

    [string]$BinaryPath,

    [string]$TaskName = 'ApeirethGateway',

    [string]$RunDir,
    [string]$LogDir,
    [string]$DataDir,

    [switch]$RotateToken,
    [switch]$KeepToken
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# 路径面 (工作目录无关: 一律绝对锚, 不看进程 CWD)
# ---------------------------------------------------------------------------

if (-not $RunDir) {
    if ($env:APEIRETH_RUN_DIR) { $RunDir = $env:APEIRETH_RUN_DIR }
    else { $RunDir = Join-Path $env:LOCALAPPDATA 'Apeireth\run' }
}
if (-not $LogDir) { $LogDir = Join-Path $env:LOCALAPPDATA 'Apeireth\logs' }
if (-not $DataDir) { $DataDir = Join-Path $env:LOCALAPPDATA 'Apeireth\data' }

$PidFile = Join-Path $RunDir 'gateway.pid'
$ServiceLog = Join-Path $LogDir 'gateway-service.log'

# ---------------------------------------------------------------------------
# 令牌面 (系统凭据库; 明文不落盘)
# ---------------------------------------------------------------------------

# 凭据条目身份与桌面壳 / TUI 读取面一致 (keyring 默认映射: 目标 = 用户.服务)。
$TokenTargetName = 'apeireth.gateway-token'
$TokenUserName = 'apeireth'

# 凭据库 P/Invoke (通用凭据类型; 本地机器持久, 跨进程可见)。
if (-not ('Apeireth.ServiceCredentialStore' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace Apeireth {
    public static class ServiceCredentialStore {
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        public struct CREDENTIAL {
            public int Flags;
            public int Type;
            public string TargetName;
            public string Comment;
            public long LastWritten;
            public int CredentialBlobSize;
            public IntPtr CredentialBlob;
            public int Persist;
            public int AttributeCount;
            public IntPtr Attributes;
            public string TargetAlias;
            public string UserName;
        }

        [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern bool CredWriteW(ref CREDENTIAL credential, int flags);

        [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern bool CredReadW(string target, int type, int flags, out IntPtr credential);

        [DllImport("advapi32.dll", SetLastError = true)]
        public static extern void CredFree(IntPtr buffer);

        [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern bool CredDeleteW(string target, int type, int flags);
    }
}
'@
}

$CredTypeGeneric = 1
$CredPersistLocalMachine = 2

function New-ServiceToken {
    # 32 随机字节 → 64 hex (与桌面壳自铸令牌同形)。
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $bytes = New-Object byte[] 32
        $rng.GetBytes($bytes)
    } finally {
        $rng.Dispose()
    }
    return (-join ($bytes | ForEach-Object { $_.ToString('x2') }))
}

function Write-ServiceToken([string]$Token) {
    $blob = [System.Text.Encoding]::Unicode.GetBytes($Token)
    $blobPtr = [System.Runtime.InteropServices.Marshal]::AllocHGlobal($blob.Length)
    try {
        [System.Runtime.InteropServices.Marshal]::Copy($blob, 0, $blobPtr, $blob.Length)
        $credential = [Apeireth.ServiceCredentialStore+CREDENTIAL]::new()
        $credential.Flags = 0
        $credential.Type = $CredTypeGeneric
        $credential.TargetName = $TokenTargetName
        $credential.Comment = 'Apeireth resident gateway access token (service env face)'
        $credential.LastWritten = 0
        $credential.CredentialBlobSize = $blob.Length
        $credential.CredentialBlob = $blobPtr
        $credential.Persist = $CredPersistLocalMachine
        $credential.AttributeCount = 0
        $credential.Attributes = [System.IntPtr]::Zero
        $credential.TargetAlias = ''
        $credential.UserName = $TokenUserName
        $ok = [Apeireth.ServiceCredentialStore]::CredWriteW([ref]$credential, 0)
        if (-not $ok) {
            throw "CredWriteW failed (target=$TokenTargetName)"
        }
    } finally {
        [System.Runtime.InteropServices.Marshal]::FreeHGlobal($blobPtr)
    }
}

function Read-ServiceToken {
    # 读不到 = $null (未安装 / 已删除), 调用方按"无令牌"口径处理。
    $ptr = [System.IntPtr]::Zero
    $ok = [Apeireth.ServiceCredentialStore]::CredReadW($TokenTargetName, $CredTypeGeneric, 0, [ref]$ptr)
    if (-not $ok) { return $null }
    try {
        $credential = [System.Runtime.InteropServices.Marshal]::PtrToStructure(
            $ptr, [type][Apeireth.ServiceCredentialStore+CREDENTIAL])
        if ($credential.CredentialBlobSize -le 0) { return $null }
        $blob = New-Object byte[] $credential.CredentialBlobSize
        [System.Runtime.InteropServices.Marshal]::Copy(
            $credential.CredentialBlob, $blob, 0, $credential.CredentialBlobSize)
        $token = [System.Text.Encoding]::Unicode.GetString($blob).TrimEnd([char]0)
        if ([string]::IsNullOrWhiteSpace($token)) { return $null }
        return $token
    } finally {
        [Apeireth.ServiceCredentialStore]::CredFree($ptr)
    }
}

function Remove-ServiceToken {
    $ok = [Apeireth.ServiceCredentialStore]::CredDeleteW($TokenTargetName, $CredTypeGeneric, 0)
    if (-not $ok) {
        $code = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
        if ($code -ne 1168) { throw "CredDeleteW failed (target=$TokenTargetName, win32=$code)" }
    }
}

# ---------------------------------------------------------------------------
# 执行面: 每一步都过 dry-run 闸 (计划只打印, 不动系统)
# ---------------------------------------------------------------------------

function Invoke-Step {
    param([string]$Description, [scriptblock]$Body)
    if ($DryRun) {
        Write-Output "[dry-run] $Description"
        return
    }
    & $Body
}

function Resolve-GatewayBinary {
    if ($BinaryPath) {
        if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
            throw "gateway binary not found: $BinaryPath"
        }
        return (Resolve-Path -LiteralPath $BinaryPath).Path
    }
    $candidates = @()
    $repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $candidates += (Join-Path $repoRoot 'target\release\apeireth.exe')
    $candidates += (Join-Path $repoRoot 'target\debug\apeireth.exe')
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $command = Get-Command 'apeireth.exe' -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $command = Get-Command 'apeireth' -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw 'gateway binary not found; pass -BinaryPath explicitly'
}

# ---------------------------------------------------------------------------
# 常驻占位面 (PID 文件: key=value 行, 与网关写入口径一致)
# ---------------------------------------------------------------------------

function Get-ResidentRecord {
    if (-not (Test-Path -LiteralPath $PidFile)) { return $null }
    $record = @{ pid = $null; port = $null; bind = $null; started_at_ms = $null }
    foreach ($line in Get-Content -LiteralPath $PidFile) {
        $trimmed = $line.Trim()
        if ($trimmed -eq '' -or $trimmed.StartsWith('#')) { continue }
        $pair = $trimmed.Split('=', 2)
        if ($pair.Count -ne 2) { continue }
        $key = $pair[0].Trim()
        $value = $pair[1].Trim()
        if ($record.ContainsKey($key)) { $record[$key] = $value }
    }
    if (-not $record['port']) { return $null }
    return $record
}

function Test-GatewayLive([int]$ProbePort) {
    # 与桌面壳探测同口径: 端口应答即认为常驻在跑。
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $task = $client.ConnectAsync('127.0.0.1', $ProbePort)
        if ($task.Wait(800)) { return $client.Connected }
        return $false
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

function Test-TaskInstalled {
    return [bool](Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue)
}

# ---------------------------------------------------------------------------
# stop: 优雅停机 → 任务停止 → 进程树终止 (三级递降)
# ---------------------------------------------------------------------------

function Invoke-GracefulStop([hashtable]$Record) {
    $token = Read-ServiceToken
    $url = "http://127.0.0.1:$($Record['port'])/v1/admin/shutdown"
    try {
        if ($token) {
            Invoke-RestMethod -Method Post -Uri $url -TimeoutSec 5 `
                -Headers @{ Authorization = "Bearer $token" } | Out-Null
        } else {
            # 无令牌 = 后端三态门口径照旧 (放行 + 警告帧 + 审计), 照旧发停机令。
            Invoke-RestMethod -Method Post -Uri $url -TimeoutSec 5 | Out-Null
        }
        Write-Output "graceful stop requested at $url"
    } catch {
        Write-Output "graceful stop request failed: $($_.Exception.Message)"
        return $false
    }
    $deadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $deadline) {
        if (-not (Test-GatewayLive ([int]$Record['port']))) {
            Write-Output 'gateway stopped gracefully (port closed)'
            return $true
        }
        Start-Sleep -Milliseconds 250
    }
    Write-Output 'gateway still answering after graceful stop deadline'
    return $false
}

function Stop-ResidentGateway {
    $record = Get-ResidentRecord
    if ($record -and (Test-GatewayLive ([int]$record['port']))) {
        if ($DryRun) {
            Write-Output "[dry-run] request graceful stop at port $($record['port']) (/v1/admin/shutdown with the service token)"
            Write-Output "[dry-run] fall back to stopping task '$TaskName' if the gateway keeps answering"
            Write-Output "[dry-run] fall back to terminating the gateway process tree"
            return
        }
        if (Invoke-GracefulStop $record) { return }
    }
    if (Test-TaskInstalled) {
        Invoke-Step "stop scheduled task '$TaskName'" { Stop-ScheduledTask -TaskName $TaskName }
    }
    if ($record -and $record['pid']) {
        Invoke-Step "terminate gateway process tree (pid $($record['pid']))" {
            & taskkill.exe /PID $record['pid'] /T /F 2>&1 | Out-Null
        }
    }
    Invoke-Step "remove resident slot file $PidFile" {
        if (Test-Path -LiteralPath $PidFile) { Remove-Item -LiteralPath $PidFile -Force }
    }
}

# ---------------------------------------------------------------------------
# 动作面
# ---------------------------------------------------------------------------

function Install-Service {
    $binary = Resolve-GatewayBinary
    Write-Output "gateway binary: $binary"
    Invoke-Step "create log directory $LogDir" {
        New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    }
    Invoke-Step "create run directory $RunDir" {
        New-Item -ItemType Directory -Force -Path $RunDir | Out-Null
    }
    Invoke-Step "create data directory $DataDir" {
        New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
    }

    $existing = Read-ServiceToken
    if ($existing -and -not $RotateToken) {
        Write-Output "service token: present (reused; use -RotateToken to replace)"
        Invoke-Step 'keep existing service token in the credential store' { }
    } else {
        Invoke-Step 'mint a 32-byte service token and store it in the credential store (plaintext never touches disk)' {
            Write-ServiceToken (New-ServiceToken)
        }
    }

    $taskAction = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument (
        '-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + $PSCommandPath +
        '" run -BinaryPath "' + $binary + '" -Port ' + $Port + ' -Bind "' + $Bind +
        '" -TaskName "' + $TaskName + '" -RunDir "' + $RunDir + '" -LogDir "' + $LogDir +
        '" -DataDir "' + $DataDir + '"')
    # 登录自启 (用户级计划任务: 登录即起) + 错过触发补跑。
    $taskTrigger = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
    # 崩溃自拉起: 动作非零退出视为失败, 每分钟重试, 最多 3 次; 不限时长。
    $taskSettings = New-ScheduledTaskSettingsSet `
        -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) `
        -ExecutionTimeLimit ([TimeSpan]::Zero) -StartWhenAvailable `
        -MultipleInstances IgnoreNew
    Invoke-Step "register scheduled task '$TaskName' (logon autostart + restart on failure, single instance)" {
        Register-ScheduledTask -TaskName $TaskName -Action $taskAction -Trigger $taskTrigger `
            -Settings $taskSettings -Force | Out-Null
    }

    Write-Output "installed service '$TaskName' (port $Port, bind $Bind)"
    Write-Output "logs: $ServiceLog"
    Write-Output "data: $DataDir"
    Write-Output 'next: apeireth-service start'
}

function Start-ServiceTask {
    if (-not (Test-TaskInstalled)) {
        throw "service '$TaskName' is not installed; run install first"
    }
    $record = Get-ResidentRecord
    if ($record -and (Test-GatewayLive ([int]$record['port']))) {
        Write-Output "gateway already running (pid $($record['pid']), port $($record['port']))"
        return
    }
    Invoke-Step "start scheduled task '$TaskName'" {
        Start-ScheduledTask -TaskName $TaskName
    }
    Write-Output "start requested for service '$TaskName'"
}

function Show-Status {
    $installed = Test-TaskInstalled
    Write-Output "service '$TaskName' installed: $installed"
    if ($installed) {
        $info = Get-ScheduledTaskInfo -TaskName $TaskName
        Write-Output "task state: $((Get-ScheduledTask -TaskName $TaskName).State)"
        Write-Output "last result: $($info.LastTaskResult)"
    }
    $record = Get-ResidentRecord
    if ($record) {
        Write-Output "resident slot: pid=$($record['pid']) port=$($record['port']) bind=$($record['bind']) started_at_ms=$($record['started_at_ms'])"
    } else {
        Write-Output 'resident slot: none'
    }
    if ($record -and (Test-GatewayLive ([int]$record['port']))) {
        $health = "http://127.0.0.1:$($record['port'])/health"
        try {
            Invoke-RestMethod -Uri $health -TimeoutSec 3 | Out-Null
            Write-Output "health: ok ($health)"
        } catch {
            Write-Output "health: port answers but /health failed: $($_.Exception.Message)"
        }
        Write-Output 'gateway: running'
    } else {
        Write-Output 'gateway: stopped'
    }
    if (Read-ServiceToken) { Write-Output 'service token: present' } else { Write-Output 'service token: absent' }
    Write-Output "logs: $ServiceLog"
    Write-Output "data: $DataDir"
}

function Uninstall-Service {
    Stop-ResidentGateway
    if (Test-TaskInstalled) {
        Invoke-Step "unregister scheduled task '$TaskName'" {
            Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        }
    } else {
        Write-Output "service '$TaskName' not installed; nothing to unregister"
    }
    if ($KeepToken) {
        Invoke-Step 'keep service token (-KeepToken)' { }
    } else {
        Invoke-Step 'remove service token from the credential store' { Remove-ServiceToken }
    }
    Write-Output "uninstalled service '$TaskName'"
    Write-Output "kept logs: $LogDir"
    Write-Output "kept data: $DataDir"
}

function Invoke-ServiceRun {
    # 计划任务内部动作: 解令牌 → 注入服务环境 → 前台跑网关 (任务生命周期 =
    # 网关生命周期, 非零退出触发任务级重启)。
    New-Item -ItemType Directory -Force -Path $LogDir, $RunDir, $DataDir | Out-Null
    $token = Read-ServiceToken
    if ($token) {
        $env:APEIRETH_GATEWAY_TOKEN = $token
    } else {
        Write-Output 'service token absent; gateway token gate runs in its default warned mode (three-state semantics)'
    }
    $env:APEIRETH_DATA_DIR = $DataDir
    $env:APEIRETH_SESSION_DB = Join-Path $DataDir 'sessions.sqlite3'
    $env:APEIRETH_COGNITIVE_DB = Join-Path $DataDir 'cognitive.sqlite3'
    $env:APEIRETH_RUN_DIR = $RunDir
    $binary = Resolve-GatewayBinary
    Write-Output "service run: binary=$binary bind=$Bind port=$Port pid-file=$PidFile"
    & $binary gateway serve --bind $Bind --port $Port --pid-file $PidFile 2>&1 |
        Tee-Object -FilePath $ServiceLog | Out-Host
    exit $LASTEXITCODE
}

if ([string]::IsNullOrWhiteSpace($Bind)) { throw 'bind address must not be empty' }

switch ($Action) {
    'install' { Install-Service }
    'start' { Start-ServiceTask }
    'stop' { Stop-ResidentGateway }
    'status' { Show-Status }
    'uninstall' { Uninstall-Service }
    'run' {
        if ($DryRun) { throw 'run is the scheduled-task action and cannot be dry-run' }
        Invoke-ServiceRun
    }
}
