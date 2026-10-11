# 服务器部署与运维 (常驻服务形态)

> 面向: 要把侧车网关跑成 **7×24 常驻服务** 的运维者 (服务器跑核心, 手机/桌面/TUI 指挥)。
> 对应工程面: W2-①「daemon 化与服务管理」(F 轴 P1)。
> 脚本与接线图: [`packaging/service/README.md`](../../packaging/service/README.md)。

## 1. 形态概览

常驻形态 = `apeireth gateway serve` 的 daemon 友好化 + 一层服务管理封装:

| 面 | 内容 |
| --- | --- |
| 服务管理 | Windows: `apeireth-service install\|start\|stop\|status\|uninstall` (计划任务); Linux: systemd 单元 (**未实测**) |
| 单实例 | PID 文件 (`--pid-file`): 二次启动显式报「已在运行」, 不双开 |
| 优雅停机 | SIGTERM/控制事件 → 收尾审计 (`gateway.shutdown`) + 观测攒批全量落盘 → 退出 |
| 崩溃自拉起 | Windows: 计划任务 `RestartCount/RestartInterval`; Linux: `Restart=on-failure` |
| 令牌面 (G10) | install 生成持久令牌存系统凭据库 (明文不落盘); 服务 env 注入; 桌面/TUI 同源读取 |
| 桌面协同 | 桌面启动探测常驻 (端口/健康检查) → 已有常驻 = 直连, 无常驻 = 拉起侧车 |

## 2. Windows: 安装

前置: `apeireth.exe` (canonical CLI) 已就位; PowerShell 5.1+。

```powershell
# 安装 (注册登录自启 + 生成持久令牌 + 建日志/数据目录)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install

# 先看计划不动系统
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install -DryRun

# 常用参数
#   -Port 8080 -Bind 127.0.0.1     监听面 (非回环暴露见 §6 安全建议)
#   -BinaryPath <apeireth.exe>     可执行文件 (缺省按 脚本旁/仓库 target/PATH 解析)
#   -RotateToken                   换新令牌 (缺省复用, 连接端不断线)
#   -TaskName <名>                 任务名隔离 (测试/多实例)
```

安装做了四件事:

1. **计划任务** `ApeirethGateway`: 登录自启 + 错过触发补跑 + 崩溃自拉起
   (动作非零退出视为失败, 每分钟重试最多 3 次) + 单实例 (`MultipleInstances IgnoreNew`);
2. **持久令牌**: 32 随机字节 (64 hex) 存系统凭据库 (目标 `apeireth.gateway-token`,
   用户 `apeireth`, 本地机器持久) —— 明文不落盘, 只在服务进程 env 出现;
3. **目录锚**: `%LOCALAPPDATA%\Apeireth\logs` (日志) / `\run` (PID 文件) / `\data`
   (会话库/认知库/审计档案) —— 全绝对路径, 与启动 CWD 无关;
4. **运行动作** `run` (计划任务内部): 现读凭据库令牌 → 注入
   `APEIRETH_GATEWAY_TOKEN` → `apeireth gateway serve --bind ... --port ... --pid-file ...`,
   输出滚动写入 `logs\gateway-service.log`。

## 3. 启停与状态

```powershell
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 start    # 启动 (已在跑则报 already running)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 status   # 只读状态
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 stop     # 优雅停机
```

**stop 的三级递降** (每次只走必要层级):

1. **优雅停机**: 带服务令牌 `POST /v1/admin/shutdown` (令牌门下) → 网关收尾
   (写 `gateway.shutdown` 审计 + 观测攒批全量落盘 + 释放 PID 文件) → 退出;
2. 优雅停机不成就 `Stop-ScheduledTask` 停任务;
3. 仍不退则终止网关进程树 (兜底强杀, 收尾不保证 —— 会留下"未优雅"的空窗)。

status 输出: 任务安装态/上次结果、常驻占位 (pid/port/bind/起始时刻)、健康检查、
**令牌在否** (只报 present/absent, 永不回显值)、日志/数据目录。

## 4. 日志与数据

| 内容 | 落位 |
| --- | --- |
| 服务运行日志 (网关 stdout/stderr 滚动) | `%LOCALAPPDATA%\Apeireth\logs\gateway-service.log` |
| 审计档案 (JSONL, 含停机收尾 `gateway.shutdown`) | `%LOCALAPPDATA%\Apeireth\data\daemon-audit.jsonl` |
| 会话/认知库 | `%LOCALAPPDATA%\Apeireth\data\sessions.sqlite3` / `cognitive.sqlite3` |
| 常驻占位 (PID 文件) | `%LOCALAPPDATA%\Apeireth\run\gateway.pid` |

`gateway.pid` 是 `key=value` 行 (pid/port/bind/started_at_ms), 网关优雅退出时自动删除;
崩溃残留的占位在下次启动按"端口不应答 = 陈旧"自动接管, 不会卡死启动。

## 5. 升级与卸载

```powershell
# 升级: stop → 覆盖 apeireth.exe → start (令牌/配置/数据不动; install 复用既有令牌)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 stop
# ... 替换 apeireth.exe ...
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 start

# 卸载 (保留日志与数据; -KeepToken 可保留令牌)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 uninstall
```

卸载会: 停机 → 注销计划任务 → 删除服务令牌 → 清占位文件。**日志与数据不删**
(需要彻底清除时人工删除 `%LOCALAPPDATA%\Apeireth\` 对应目录)。

## 6. 安全建议

1. **令牌必配**: install 默认生成持久令牌, 敏感端点 (`/v1/admin/config`、
   `/v1/approvals/resolve`、`/v1/admin/shutdown`) 即强制 `Authorization: Bearer <token>`。
   无令牌时网关三态门口径照旧 (放行 + 警告帧 + 审计) —— 但常驻暴露面请勿裸奔。
2. **防火墙**: 默认绑定 `127.0.0.1` (仅本机)。确需 LAN/远程访问时:
   非回环绑定 (`-Bind 0.0.0.0`) + 主机防火墙只放行可信网段 + 强制令牌;
   更细的 TLS/每客户端授权属 F 轴 P3 (远程访问) 范围, 当前版本不提供。
3. **令牌轮换**: `install -RotateToken`; 轮换后重连端 (桌面/TUI) 自动取新值
   (同源读取), 常驻服务需 `stop` + `start` 让新令牌进服务 env。
4. **令牌不外流**: 令牌只存系统凭据库与服务 env —— 不落明文文件、不进任务定义、
   不进日志/帧/审计 (`apeireth-service status` 也只报在否)。丢了令牌 = `install -RotateToken`。
5. **账户面**: 计划任务以当前用户身份运行 (登录自启)。无人值守开机即起、或以
   受限服务账户运行, 需自行改任务主体并把令牌面改为该账户可读 (见 §8 已知边界)。

## 7. 故障排查

| 症状 | 排查 |
| --- | --- |
| `status` 显示 installed 但 gateway: stopped | 看 `logs\gateway-service.log` 尾部; 计划任务 `last result` 非 0 = 网关退出过 (崩溃自拉起最多 3 次/分钟间隔) |
| 启动报 `gateway already running (pid N, port P)` | 有常驻实例活着 (PID 文件占位)。要换实例先 `stop`; 占位陈旧 (进程已死) 会被下次启动自动接管 |
| 启动报 `gateway already running: <addr> is already in use` | 端口被占 (另一个网关或别的进程)。换 `-Port` 或先停掉占用者 |
| 桌面壳直连了旧常驻 | 桌面按 `%LOCALAPPDATA%\Apeireth\run\gateway.pid` + `/health` 探测; 常驻已停而占位残留时桌面回落拉起侧车; 端口变了请重启桌面壳 |
| 敏感端点 401 `auth_missing_key` / `auth_invalid_key` | 客户端没带/带错令牌。桌面/TUI 走同源令牌面自动带; 手工 curl 需 `Authorization: Bearer <token>` (从凭据库取, 不落日志) |
| 无令牌警告帧刷屏 (`gateway_auth_unprotected`) | 服务没带令牌跑 (令牌面缺席)。`status` 看 token 在否, `install -RotateToken` 后 `stop`+`start` |
| 停机后仍有残留 | 非优雅路径 (强杀) 可能留下未删的 PID 文件: 不影响下次启动 (陈旧自动接管), 介意可手删 |

## 8. Linux 面 (未实测)

`packaging/service/linux/apeireth-gateway.service` 仅做**静态校验** (字段/语法由
`apeireth-cli` 测试面钉住), 未在真机执行 —— 上机前逐项核对路径与用户。要点:

- 安装/启停: `systemctl enable --now apeireth-gateway` / `systemctl stop apeireth-gateway`;
- 崩溃自拉起: `Restart=on-failure` + `RestartSec=5s`; 停机: `KillSignal=SIGTERM`
  → 网关优雅停机 (收尾审计 + 落盘), 30 秒兜底;
- 令牌面与 Windows 的**已知差异**: 走 `/etc/apeireth/gateway.env`
  (root:root 0600) 的 env 文件 —— 明文落盘姿态不同 (Windows 面为凭据库
  明文不落盘), 上机时按目标发行版的密钥管理设施自行收紧;
- 与 Windows 面共用同一网关语义: `--pid-file` 单实例占位、`/v1/admin/shutdown`
  优雅停机、桌面/TUI 令牌面回落 env (`APEIRETH_GATEWAY_TOKEN`)。

## 9. 接线与契约索引

- 脚本用法 + 接线图: [`packaging/service/README.md`](../../packaging/service/README.md);
- 常驻占位/优雅停机/停机端点行为测试: `apeireth-gateway` 的 `resident_service` 测试面;
- 脚本 dry-run/参数校验 + systemd 单元静态校验: `apeireth-cli` 的 `service_management` 测试面;
- 真机二次启动拒绝 + 停机收尾 (审计+落盘) e2e: `apeireth-cli` 的 `resident_gateway_e2e` 测试面;
- 桌面探测协同 + 令牌三源收口: 桌面壳 `backend_supervisor` 测试面;
- 令牌门口径 (三态): `docs/04-internal/backend-enhancement/E-threat-model.md` G10 条目。
