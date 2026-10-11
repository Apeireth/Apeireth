# packaging/service — 常驻服务管理面 (W2-① daemon 化)

把侧车网关从「桌面壳拉起的子进程」升级为**可独立常驻的服务**:
Windows 计划任务封装 (`apeireth-service`) + Linux systemd 单元 (未实测)。
运维手册见 [`docs/02-guides/server-deployment.md`](../../docs/02-guides/server-deployment.md)。

## 文件

| 文件 | 平台 | 作用 |
| --- | --- | --- |
| `apeireth-service.cmd` | Windows | 控制台入口 (转发到 ps1) |
| `apeireth-service.ps1` | Windows | `install \| start \| stop \| status \| uninstall` (+ 内部 `run`) |
| `linux/apeireth-gateway.service` | Linux | systemd 单元 (**未实测**, 仅静态校验) |

## 用法 (Windows)

```powershell
# 安装: 注册登录自启计划任务 + 生成持久令牌 (凭据库) + 建日志/数据目录
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install

# 只看计划、不动系统 (任何动作都可安全 dry-run 复跑)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 install -DryRun

pwsh -NoProfile -File packaging/service/apeireth-service.ps1 start     # 启动
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 status    # 状态 (只读)
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 stop      # 优雅停机
pwsh -NoProfile -File packaging/service/apeireth-service.ps1 uninstall # 卸载 (保留日志与数据)
```

常用参数: `-Port 8080` `-Bind 127.0.0.1` `-BinaryPath <apeireth.exe>`、
`-TaskName <名>` (测试/多实例隔离)、`-RotateToken` (换令牌)、
`-KeepToken` (卸载时保留令牌)、`-RunDir/-LogDir/-DataDir` (状态目录重锚)。

## 接线图 (常驻形态)

```
                Windows 计划任务 "ApeirethGateway"
                登录自启 · 崩溃自拉起(RestartCount) · 单实例(IgnoreNew)
                                   │
                                   │  powershell apeireth-service.ps1 run
                                   ▼
 系统凭据库 ──读令牌──► run 动作 ──注入 env APEIRETH_GATEWAY_TOKEN──► apeireth gateway serve
 apeireth.gateway-token            │                                   --bind 127.0.0.1 --port 8080
 (用户 apeireth, 明文不落盘)        │                                   --pid-file %LOCALAPPDATA%\Apeireth\run\gateway.pid
                                   │                                          │
                                   │                                          ├─ /health           健康检查 (探测面)
                                   │                                          ├─ /v1/...           既有网关面 (语义不变)
                                   │                                          └─ /v1/admin/shutdown 令牌门 · 优雅停机
                                   ▼
                        %LOCALAPPDATA%\Apeireth\logs\gateway-service.log   (运行日志)
                        %LOCALAPPDATA%\Apeireth\data\daemon-audit.jsonl    (审计档案, 停机收尾落盘)

 桌面壳启动探测: run\gateway.pid → /health 应答?
   ├─ 应答 = 已有常驻 ── 直连 (不重复拉起) + 读凭据库同源令牌 (探测到常驻=读服务令牌面)
   └─ 不应答 = 无常驻 ── 按现行为拉起侧车 (自铸令牌, 每 spawn 一枚)

 TUI: --token / env 显式给值优先; 缺省回落服务令牌面 (同源); 无令牌 = 三态门警告帧语义照旧。
```

## 令牌面 (G10)

- `install` 生成 32 随机字节 (64 hex) 存入系统凭据库 (目标 `apeireth.gateway-token`,
  用户 `apeireth`, 本地机器持久) —— **明文不落盘**; 服务运行期只以 env
  (`APEIRETH_GATEWAY_TOKEN`) 注入网关进程;
- `run` 动作启动时现读凭据库 → env → 网关令牌门 (`GatewayTokenGate::Enforced`) 强制执行
  (W2-② 起覆盖**全 API 面**, 仅 `/health` 探活豁免);
- 桌面壳 / TUI 从**同一凭据条目**取同源令牌 (keyring 默认映射: 目标 = 用户.服务);
- 无令牌 = 三态门口径照旧 (放行 + 警告帧 + 审计), 不新造第四种口径;
- `uninstall` 删除令牌 (`-KeepToken` 保留); `install -RotateToken` 换新。

## 远程面 (W2-②)

- 绑定地址: `-Bind` 显式给值 > env `APEIRETH_LISTEN` > `127.0.0.1` (缺省仅本机);
- 非回环绑定**强制令牌** (fail-closed): 令牌族两面皆空时网关拒启动; 逃生阀
  `APEIRETH_GATEWAY_TOKEN=none` 明示可裸奔但留审计;
- TLS 面为接口桩 (`APEIRETH_TLS_CERT`/`APEIRETH_TLS_KEY` 显式未接线, 设值只得显式警告,
  流量仍明文)。

## Linux 面 (未实测)

`linux/apeireth-gateway.service` 仅做静态字段/语法校验 (测试面钉住), 未在真机
执行过。与 Windows 面的已知差异: 令牌走 `/etc/apeireth/gateway.env`
(root:root 0600) 的 env 文件 (明文落盘姿态不同), 停机走 `systemctl stop` → SIGTERM。
上机前逐项核对路径/用户, 详见运维手册「Linux (未实测)」章。

## 自检

- 脚本单元面 (dry-run / 参数校验 / 静态结构 / 密钥卫生) 与 systemd 单元静态校验:
  `cargo test -p apeireth-cli --test service_management`;
- 常驻占位 / 优雅停机收尾 / 停机端点令牌门: `cargo test -p apeireth-gateway --test resident_service`;
- 真机二次启动拒绝 + 停机收尾 e2e: `cargo test -p apeireth-cli --test resident_gateway_e2e`。
