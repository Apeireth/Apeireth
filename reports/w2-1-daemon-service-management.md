# W2-① daemon 化与服务管理 (F 轴 P1) — 施工报告

> 状态: ✅ 交付 (Windows 面全链真机验证; Linux systemd 面**未实测**, 仅静态校验)
> 范围纪律: 零前端触碰 (唯一例外 `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs`, 只加不改既有语义); 不写 CHANGELOG; 不 commit。

## 1. 交付面

| # | 目标 | 落点 |
| --- | --- | --- |
| 1 | 服务化封装 | `packaging/service/apeireth-service.ps1` (+`.cmd` 入口): `install\|start\|stop\|status\|uninstall` (+内部 `run`), `-DryRun` 全动作可用; 登录自启 + 崩溃自拉起 (RestartCount/RestartInterval) + 单实例 (IgnoreNew); 日志落 `%LOCALAPPDATA%\Apeireth\logs\`; Linux: `packaging/service/linux/apeireth-gateway.service` (**未实测**, 静态校验钉住) |
| 2 | 常驻模式 | `apeireth gateway serve [--pid-file PATH]`: 常驻占位 (PID 文件 `key=value`, 二次启动显式报 `gateway already running (pid, port); refusing to start a second instance`) + 端口占用显式报"已在运行" + 优雅停机 (SIGTERM/控制事件 + 令牌门 `POST /v1/admin/shutdown` → 收尾审计 `gateway.shutdown` + 观测攒批全量落盘 + 占位释放); 路径全绝对锚 (工作目录无关) |
| 3 | 桌面壳协同 | `backend_supervisor.rs` 只增: 常驻探测面 (`run\gateway.pid` + `/health`) → 已有常驻 = 直连 (Ready/External, 不重复拉起, 停机语义照旧=外部不归桌面管); 无常驻 = 既有 spawn 路径零变化 |
| 4 | G10 协同 | 服务令牌面: install 铸 32 随机字节存系统凭据库 (目标 `apeireth.gateway-token`/用户 `apeireth`, **明文不落盘**) → `run` 注入服务 env `APEIRETH_GATEWAY_TOKEN` → 网关令牌门 Enforced; 桌面/TUI 同源读取 (探测到常驻=读服务令牌面); 无令牌 = 三态门警告帧语义照旧 |
| 5 | 运维文档 | `docs/02-guides/server-deployment.md` (安装/启停/日志/升级/卸载/故障排查/安全建议=令牌+防火墙) + `packaging/service/README.md` (用法 + 接线图) |

## 2. 新增测试 (26 可跑 + 2 真机探针 `#[ignore]`)

| 测试面 | 文件 | 钉住的行为 |
| --- | --- | --- |
| 常驻占位 (3) | `crates/adapters/gateway/src/resident.rs` | render/parse 往返、注释/未知键兼容、缺字段/坏值响亮报错 |
| 停机闸 (2) | `crates/adapters/gateway/src/shutdown.rs` | 幂等触发、等待者被唤醒 |
| 常驻服务行为 (4) | `crates/adapters/gateway/tests/resident_service.rs` | 二次启动显式拒绝+不篡改占位/陈旧与损坏占位接管/停机释放; 优雅停机收尾审计+攒批全量**落盘** (真文件断言); 停机端点对抗面 (缺令牌 401 auth_missing_key / 伪令牌 401 auth_invalid_key 不触发停机 / 正确令牌过门+幂等 first→repeat, 全程令牌不回显); 三态门口径照旧 (未设=放行+警告帧+审计, 逃生阀=静默留痕) |
| 服务脚本面 (5) | `crates/adapters/cli/tests/service_management.rs` | 脚本静态结构 (动作集/dry-run 闸/崩溃自拉起/单实例/日志锚/占位与停机接线); 密钥卫生 (令牌不落文件/不进任务定义, 只走服务 env); 真跑 dry-run 零副作用 (install/stop/uninstall 计划可见+状态目录不建+隔离任务名 status 如实报未安装+未安装 start 显式拒); 参数校验 (非法动作/端口 0/70000/空 bind 全显式拒且零副作用); Linux 单元静态校验 (段/键语法、唯一键、绝对路径、Restart=on-failure、KillSignal=SIGTERM、日志落位、WantedBy) |
| 真机 e2e (2) | `crates/adapters/cli/tests/resident_gateway_e2e.rs` | 真二进制二次启动显式拒 (`already running`+占位不动); 令牌门停机令 → 进程零码自退 + PID 文件释放 + `daemon-audit.jsonl` 落盘 `gateway.shutdown graceful=1` |
| 服务令牌面 (3+1) | `crates/foundation/credentials/src/service_token.rs` | 身份常量=跨工具同源锚 (`apeireth.gateway-token` = 用户.服务); 取值裁决 (缺失/空白=无令牌, 不泄露呈现); 只读服务令牌身份; 真机探针 (`--ignored`) |
| TUI 同源 (1) | `crates/adapters/tui/src/main.rs` | 显式值 > 服务令牌面 > 无 (三态门照旧), 空白逐层跳过 |
| 桌面协同 (6+1) | `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs` | 占位解析往返+坏内容读作无常驻; `APEIRETH_RUN_DIR` 显式锚优先; 探测面 (陈旧=无常驻 / /health 应答=命中); **已有常驻=直连不拉起侧车** (进程面无子句柄断言) + 外部停机语义照旧; 令牌三源收口 (spawn>env>服务面, 哨兵=不设防, 三者皆无=三态门照旧); 凭据库密文 UTF-16LE 编码契约; 真机探针 (`--ignored`) |

既有套件零回归: 根 workspace 全量 (含既有桌面生命周期 9 例) 见 §4; 桌面 `supervisor_lifecycle` 9/9 全绿 (探测在测试构造下默认关, 生命周期测试语义零变化)。

## 3. 真机验证证据 (2026-10-12)

1. **凭据库写读往返** (服务安装面同款 P/Invoke marshaling): `wrote=True read=True roundtrip=selftest-token-0123456789abcdef deleted=True` (自测目标, 用后即删);
2. **跨工具同源** (服务安装面写入 → 各读取面读出, 逐字节一致断言, 探针用后即删):
   - 桌面壳读取面 (CredReadW, `SERVICE_TOKEN_TARGET`): `SERVICE TOKEN FACE READ OK (30 bytes)` + `SERVICE_TOKEN_PROBE_EXPECTED` 强校验通过;
   - TUI/凭据层读取面 (keyring 默认目标映射 `Entry::new("gateway-token","apeireth")` → 目标 `apeireth.gateway-token`): `SERVICE TOKEN FACE READ OK (30 bytes)` + 同一强校验通过;
   - 探针条目 `apeireth.gateway-token` 已删除 (`deleted=True`), 机器无残留;
3. **dry-run 执行面** (真跑 pwsh): install/stop/uninstall 打印 `[dry-run]` 计划、状态目录零创建; status (只读) 如实报 `installed: False` / `gateway: stopped` / `service token: absent|present`; 未安装 start 显式 `not installed` 非零退出; 非法动作/端口/空 bind 参数校验全非零退出;
4. **真机 e2e** (真 `apeireth` 二进制): 二次启动拒绝文案含 `already running`/`refusing to start a second instance`/活占位 pid; 令牌门停机令 200 → 进程 exit 0 → `daemon-audit.jsonl` 含 `gateway.shutdown`/`graceful=1` → `gateway.pid` 释放。

## 4. 门禁

| 门 | 结果 |
| --- | --- |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ exit 0 (零警告) |
| `cargo test --workspace --locked -j 2 --no-fail-fast` | ✅ 既有套件零回归。两次全量实测: ① exit 0 全绿; ② **177 套件 / 5036 passed / 22 ignored** 仅 1 例负载 flake —— `apeireth-tools-canonical` 的 `appcontainer::probe_result_matches_capability_report` (既有真机 AppContainer 探针, 非本批代码, 隔离复跑绿, 按宪章「负载 flake 隔离复跑定性」处置) |
| `cargo fmt --all -- --check` | ⚠ 本批文件干净 (`rustfmt --check` 逐文件 exit 0); 仓内 11 个**他线在途**文件非 rustfmt 干净 (git_write 分级线, 见通报) |
| 桌面 workspace `cargo test` | ✅ 全绿 (backend_supervisor 41 例含 1 探针 ignored + supervisor_lifecycle 9/9 + 其余) |
| 桌面 workspace `cargo clippy --all-targets -- -D warnings` | ⚠ 红 1 处**他线在途** `src/pg_client.rs:205` (`manual_range_contains`, 矿脉外部驱动线未跟踪文件, 禁区零触碰); 归因复核 `-A clippy::manual_range_contains` → exit 0 (本批面干净) |
| `pnpm check` / `pnpm test` (frontend/companion-desktop) | ✅ 0 错 0 警 / 71/71 套件 |
| `scripts/check-neutral-terms.ps1` | ✅ 136 文件零命中 |
| `scripts/secret-scan.ps1 -Mode scan-all` | ✅ 无密钥 |

## 5. 诚实边界 (0 装)

1. **Linux 面未实测**: systemd 单元仅静态校验 (字段/语法测试钉住); 令牌面在 Linux 走 `/etc/apeireth/gateway.env` (root 0600, 明文落盘姿态与 Windows 凭据库不同), 上机前需逐项核对路径/用户/密钥管理;
2. **Windows 任务形态**: 用户级计划任务 (登录自启) —— 无人值守**开机**即起 (未登录) 需改任务主体 (SYSTEM/受限账户) 并把令牌面改为该账户可读, 文档 §6 已注明;
3. **桌面非 Windows 令牌面读取暂无接口** (桌面打包面为 Windows): 无令牌回落三态门照旧, 不假装取到;
4. **占位 vs 端口**: 单实例拒绝以「占位存活 (端口应答)」判定 (跨平台同口径), `pid` 字段为诊断元数据, 不做进程表查询;
5. **stop 兜底强杀** (`taskkill /T /F`) 不保证收尾落盘 —— 仅在优雅停机超时后走, 文档已注明"未优雅空窗";
6. **他线在途红点** (非本批, 已通报父代理): `file_write_knob::docs_state_the_graded_git_write_boundary` 曾红 (git 写分级线文档/测试不一致, 本批施工中被该线自行修复, 以最终全量绿为准) / `pg_client.rs` clippy / 11 文件 fmt —— 三项均在他线范围, 本批按禁区纪律零触碰。

## 6. 复跑命令

```powershell
cargo test --workspace --locked -j 2 --no-fail-fast        # 根 workspace 全量
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings
cargo test -p apeireth-gateway --test resident_service      # 常驻/停机/令牌门
cargo test -p apeireth-cli --test service_management        # 脚本面 (含真跑 dry-run)
cargo test -p apeireth-cli --test resident_gateway_e2e      # 真机二次启动拒绝 + 停机收尾
cargo test -p apeireth-credentials                          # 服务令牌面
cargo test -p apeireth-tui                                  # TUI 同源回落
cd frontend/companion-desktop/src-tauri && cargo test       # 桌面 (含 supervisor_lifecycle)
cd frontend/companion-desktop && pnpm check && pnpm test
pwsh -NoProfile -File scripts/check-neutral-terms.ps1
pwsh -NoProfile -File scripts/secret-scan.ps1 -Mode scan-all
```
