# Windows Scoop 安装指南

> **平台**: Windows 10 (1809+) / Windows 11 / Windows Server 2019+
> **包格式**: Scoop Manifest (JSON)
> **服务管理**: NSSM (Non-Sucking Service Manager, manifest 估补) / Task Scheduler
> **D-06 决策**: 主人 2026-08-05 20:53 拍 A, 8 包齐发

---

## 0. 系统要求

| 组件 | 最低版本 | 推荐版本 | 备注 |
|---|---|---|---|
| Windows | 10 (1809) | 11 (23H2) | Build 17763+ |
| PowerShell | 5.1 | 7.4+ | `pwsh` |
| Scoop | 0.4+ | 0.5+ | `scoop --version` |
| .NET (可选) | 6 | 8 | Manifest depends |
| 磁盘 | 200 MB | 500 MB | |
| 内存 | 2 GB | 4 GB | |

**架构**: `x64` (x86_64) / `arm64` 估补 (R20 阶段 4)

**安装位置** (Scoop 默认):
- `%USERPROFILE%\scoop\apps\apeireth\current\apeireth.exe` (用户级, 无需 admin)
- `%USERPROFILE%\.apeireth\` (数据)

---

## 1. 一行安装 (推荐)

**PowerShell 7+**:

```powershell
# 1.1 装 scoop (如未装)
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm get.scoop.sh | iex

# 1.2 加 bucket (manifest 在 apeireth/scoop-bucket 仓库)
scoop bucket add apeireth https://github.com/apeireth/scoop-bucket

# 1.3 装
scoop install apeireth

# 1.4 配置 APEIRETH_HOME
[Environment]::SetEnvironmentVariable('APEIRETH_HOME', "$env:USERPROFILE\.apeireth", 'User')
$env:APEIRETH_HOME = "$env:USERPROFILE\.apeireth"
New-Item -ItemType Directory -Path $env:APEIRETH_HOME -Force | Out-Null
```

或用仓库内脚本:

```powershell
.\scripts\install\install-scoop.ps1
```

---

## 2. Manifest 依赖 (per packaging/scoop/apeireth.json)

**安装时** (`depends`): 空 (无强制依赖)

**建议装** (`suggest`):

- `extras/postgresql` — 数据库
- `main/redis` — 缓存

**runtime**: Visual C++ Redistributable (2015-2022), manifest 检查:

```powershell
# 检查 VC++ Redist
Get-ChildItem 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64' -ErrorAction SilentlyContinue
# 期望: 找到 Install = 1
```

缺则装:

```powershell
scoop install extras/vcredist2022
```

---

## 3. 服务管理 (NSSM / Task Scheduler)

Scoop Manifest 本身不注册服务, 需手动 (估补: packaging/scoop/apeireth.json 加 `installer.script` 自动 NSSM 注册):

**NSSM 方式** (推荐):

```powershell
# 3.1 装 NSSM
scoop install main/nssm

# 3.2 注册服务
nssm install Apeireth "$env:USERPROFILE\scoop\apps\apeireth\current\apeireth.exe" "serve"
nssm set Apeireth AppDirectory "$env:USERPROFILE\scoop\apps\apeireth\current"
nssm set Apeireth DisplayName "Apeireth OS - AI Growth Platform"
nssm set Apeireth Start SERVICE_AUTO_START
nssm set Apeireth AppStdout "$env:USERPROFILE\.apeireth\logs\service.log"
nssm set Apeireth AppStderr "$env:USERPROFILE\.apeireth\logs\service.err"

# 3.3 启
nssm start Apeireth
```

**Task Scheduler 方式** (无 NSSM):

```powershell
$action = New-ScheduledTaskAction -Execute "$env:USERPROFILE\scoop\apps\apeireth\current\apeireth.exe" -Argument "serve"
$trigger = New-ScheduledTaskTrigger -AtLogOn
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName "Apeireth" -Action $action -Trigger $trigger -Settings $settings -User "$env:USERNAME" -RunLevel Highest
```

---

## 4. 健康检查

```powershell
# 4.1 二进制
scoop which apeireth
# 期望: C:\Users\<user>\scoop\apps\apeireth\current\apeireth.exe

# 4.2 版本
apeireth --version
# 期望: apeireth 1.0.0

# 4.3 /health
(Invoke-WebRequest -Uri "http://localhost:8080/health" -UseBasicParsing).Content
# 期望: {"status":"ok","version":"1.0.0"}

# 4.4 服务状态 (NSSM)
nssm status Apeireth
# 期望: SERVICE_RUNNING
```

---

## 5. 配置文件

**默认路径**:

- `~/scoop/apps/apeireth/current/apeireth.exe` — 二进制
- `%APEIRETH_HOME%\config.toml` — 主配置
- `%APEIRETH_HOME%\data\` — 数据
- `%APEIRETH_HOME%\logs\` — 日志

**配置示例**:

```powershell
# 5.1 创建配置
$configDir = "$env:APEIRETH_HOME"
@"
[server]
bind = "127.0.0.1:8080"
workers = 4

[storage]
db_url = "postgresql://apeireth:secret@localhost:5432/apeireth"
"@ | Out-File -Encoding utf8 "$configDir\config.toml"

# 5.2 重启服务
nssm restart Apeireth
```

**`checkver` 自动更新检测** (per manifest):

```powershell
scoop status apeireth
# 期望 (如过时): apeireth 1.0.0 --> 1.0.1 (update available)
```

---

## 6. 升级

```powershell
# 6.1 停服务
nssm stop Apeireth  # 或 Task Scheduler: Stop-ScheduledTask -TaskName "Apeireth"

# 6.2 拉新 manifest + 二进制
scoop update
scoop update apeireth

# 6.3 启
nssm start Apeireth
```

**回滚** (如新版有 bug):

```powershell
scoop uninstall apeireth
scoop install "https://github.com/apeireth/scoop-bucket/raw/main/bucket/apeireth-1.0.0.json"
```

---

## 7. 卸载

```powershell
# 7.1 简版
scoop uninstall apeireth

# 7.2 完整 (含服务 + 数据)
nssm stop Apeireth
nssm remove Apeireth confirm
Unregister-ScheduledTask -TaskName "Apeireth" -Confirm:$false
scoop uninstall apeireth
Remove-Item -Recurse -Force "$env:USERPROFILE\.apeireth"
[Environment]::SetEnvironmentVariable('APEIRETH_HOME', $null, 'User')
```

通用卸载:

```powershell
# PowerShell 版本估补 (目前 uninstall-all.sh 仅 bash)
scoop uninstall apeireth
```

---

## 8. 故障排查

| 症状 | 排查 |
|---|---|
| `scoop install` 报 `failed to extract` | 临时目录满, `scoop cache rm *` + 重试 |
| 启动报 `VCRUNTIME140.dll not found` | 装 VC++ Redist: `scoop install extras/vcredist2022` |
| 端口 8080 占用 | `netstat -ano \| findstr :8080`, taskkill /PID <pid> /F |
| `apeireth` 不在 PATH | 注销重登 (Scoop 配 PATH 在 user env), 或: `scoop reset` |
| NSSM 报 `Access is denied` | 用 admin PowerShell |
| 防火墙挡 8080 | New-NetFirewallRule -DisplayName "Apeireth" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow |
| Windows Defender 拦 exe | 加白名单: Add-MpPreference -ExclusionPath "$env:USERPROFILE\scoop\apps\apeireth" |

---

## 9. WSL2 旁路 (如果走 WSL)

很多 Windows 开发者用 WSL2 跑 Linux 工具, 装 Linux 版本更顺:

```bash
# WSL2 内
wsl
curl -fsSL -O https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-1.0.0-x86_64-linux.tar.gz
sudo ./scripts/install/install-tarball.sh
```

或用 Linuxbrew (per `macos-brew-install.md` §9):

```bash
brew tap apeireth/tap
brew install apeireth/tap/apeireth
```

---

## 10. 参考

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4`](../stage4/v09021-rust-translation-blueprint-2026-08-05.md)
- packaging: [`packaging/scoop/`](../../packaging/scoop/)
- 兄弟文档: [`macos-brew-install.md`](macos-brew-install.md) / [`package-comparison.md`](package-comparison.md)
- Scoop 文档: <https://github.com/ScoopInstaller/Scoop/wiki>
- NSSM: <https://nssm.cc/>
