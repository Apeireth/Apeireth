# 册 12 — 杂项交付记事（2 篇合册）

> **一句话定位**：许可声明产出、模型实测对比、SDK crate 决策、服务端形态交付与目录旧索引等零散交付记事。
>
> **来源篇数**：2｜**合册生成日**：2026-10-23
> **机械合订说明**：本册由脚本从原篇**机械摘取**（标题 / 元信息块 / 各节要点 / 归属路径），**不生成、不改写、不编造**；
> **本册未做逐条「现状复核」**——即不判断每条结论今天是否仍成立。要现行口径请查下方真源，原件全文见归档路径。
>
> **复核入口（现行真源）**：[claims.yaml](../docs/status/claims.yaml) ／ [test-baseline](../docs/03-reference/test-baseline.md) ／ [capabilities-matrix](../docs/03-reference/capabilities-matrix.md) ／ [00-backlog](../docs/04-internal/backend-enhancement/00-backlog.md)

## 目录

| # | 原标题 | 原日期 | 行数 |
|---|---|---|---|
| 1 | reports/ — 报告库（F-02 出库后口径） | 2026-10-18 | 80 |
| 2 | W2-① daemon 化与服务管理 (F 轴 P1) — 施工报告 | 原篇未标日期 | 77 |

---

### reports/ — 报告库（F-02 出库后口径） (2026-10-18)

**来源**：`reports/README.md`（80 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/README.md`（原件逐字留存）

**元信息**

# reports/ — 报告库（F-02 出库后口径）
> **F-02 大资产出库执行批（2026-10-18）· 批量日志迁出说明**
> v1.0 时代批量工作日志 + UUID/短哈希工作日志共 **878 件 / 34.7MB** 已打包迁 release 资产暂存区
> `release-assets/reports-archive-2026-07-08.tar`（随 Release 资产分发，`release-assets/` 不入 git 跟踪面）；
> **git 历史不动**，历史 blob 可按文件名找回。
> **711 处 docs 引用不改**——docs/ 与代码注释中出现的报告名属历史卷宗口径，**历史引用名保留**，

**各节要点（机械摘取，非全文）**

- **本目录现状（2026-10-18 起）**
  - - **已迁出（878 件）**：文件名带 2026-07/08 日期的 v1.0 时代批量日志（`agent-*`、`decision-*`、
  - `rNN-*`、`round*`、`1.0-release-*`、`achievement-*`、`dispatch-*` 等）+ UUID/短哈希命名工作日志。
  - - **留库（195 件 tracked）**：近期证据/验收报告（2026-09~10）、`benchmark-baseline.md` /
  - `benchmark-reproduction.md`、`diagnostics-review/`、`pet-real-smoke-2026-10-11/`、`audit-2026-08-19/`、
- **reports/ — R12 收尾报告（历史口径，R12 收尾时点登记）**
  - > **范围**: R12 接手期间的所有报告 (12 份)
  - > **创建时间**: R12 收尾任务 T16 (technical_writer) + R12 评审
  - > **当前**: 12 个报告 (~283KB)
- **📚 报告清单**
- **收尾总结 (2)**
  - | 报告 | 大小 | 任务 | 内容 |
  - | `r12-finalize-2026-07-30.md` | 21.6KB | T16 | R12 收尾总结 |
  - | `r12-future-changes-2026-07-30.md` | 26.2KB | T14 | R12 重大变更预备 |
- **MVP 启动 (1)**
  - | 报告 | 大小 | 任务 | 内容 |
  - | `r13-mvp-kickoff-2026-07-30.md` | 9.5KB | T9 | R13 MVP Phase 0 + 1.1 启动 |
- **附录 N 评审 (5)**
  - | 报告 | 大小 | 任务 | 内容 |
  - | `apeireth-omnibus-appendix-n-r12-handoff-arch-check.md` | 32.0KB | T4-M3 | 架构评审 |
  - | `apeireth-omnibus-appendix-n-r12-handoff-sec-check.md` | 10.7KB | T4-M2.5-SEC | 安全评审 |
  - | `apeireth-omnibus-appendix-n-r12-handoff-perf-check.md` | 15.2KB | T4-M2.5-PERF | 性能评审 |
- **收尾接续 (4)**
  - | 报告 | 大小 | 任务 | 内容 |
  - | `r12-baseline-verification-2026-07-30.md` | 25.8KB | T1 | §5.B 6/6 PASS |
  - | `r12-working-changes-audit-2026-07-30.md` | 24.5KB | T2 | working changes 35 files +1759/-316 |
  - | `r12-t6-commit-audit-v2-2026-07-30.md` | 32.6KB | T7 | T6-A/B/C 接续审计 v2 |
- **用途**
  - 1. **接手者** (R13 / R14 团队) — 历史回溯
  - 2. **R14 Rust 重写** — `r12-future-changes-2026-07-30.md` 是 8 类大变动预备
  - 3. **R13 MVP 接力** — `r13-mvp-kickoff-2026-07-30.md` 继续
- **维护原则**
  - - ❌ 不修改 (R12 已归档)
  - - ✅ 可以 git commit 新增报告
  - _主哲学 anchor: 主 00:56 任何人都能接手 + 主 17:43 实事求是._

**归属链接**：docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.ts

### W2-① daemon 化与服务管理 (F 轴 P1) — 施工报告 (原篇未标日期)

**来源**：`reports/w2-1-daemon-service-management.md`（77 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/w2-1-daemon-service-management.md`（原件逐字留存）

**元信息**

```
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

**各节要点（机械摘取，非全文）**

- **1. 交付面**
  - | # | 目标 | 落点 |
  - | --- | --- | --- |
  - | 1 | 服务化封装 | `packaging/service/apeireth-service.ps1` (+`.cmd` 入口): `install\|start\|stop\|status\|uninstall` (+内部 `run`), `-DryRun` 全动作可用; 登录自启 + 崩溃自拉起 (RestartCount/RestartInterval) + 单实例 (IgnoreNew); 日志落 `%LOCALAPPDATA%\Apeireth\logs\`; Linux: `packaging/service/linux/apeireth-gateway.service` (
  - | 2 | 常驻模式 | `apeireth gateway serve [--pid-file PATH]`: 常驻占位 (PID 文件 `key=value`, 二次启动显式报 `gateway already running (pid, port); refusing to start a second instance`) + 端口占用显式报"已在运行" + 优雅停机 (SIGTERM/控制事件 + 令牌门 `POST /v1/admin/shutdown` → 收尾审计 `gateway.shutdown` + 观测攒批全量落盘 + 占位释放); 路径全绝对锚 (工作目录无关) |
- **2. 新增测试 (26 可跑 + 2 真机探针 `#[ignore]`)**
  - | 测试面 | 文件 | 钉住的行为 |
  - | --- | --- | --- |
  - | 常驻占位 (3) | `crates/adapters/gateway/src/resident.rs` | render/parse 往返、注释/未知键兼容、缺字段/坏值响亮报错 |
  - | 停机闸 (2) | `crates/adapters/gateway/src/shutdown.rs` | 幂等触发、等待者被唤醒 |
- **3. 真机验证证据 (2026-10-12)**
  - 1. **凭据库写读往返** (服务安装面同款 P/Invoke marshaling): `wrote=True read=True roundtrip=selftest-token-0123456789abcdef deleted=True` (自测目标, 用后即删);
  - 2. **跨工具同源** (服务安装面写入 → 各读取面读出, 逐字节一致断言, 探针用后即删):
  - - 桌面壳读取面 (CredReadW, `SERVICE_TOKEN_TARGET`): `SERVICE TOKEN FACE READ OK (30 bytes)` + `SERVICE_TOKEN_PROBE_EXPECTED` 强校验通过;
  - - TUI/凭据层读取面 (keyring 默认目标映射 `Entry::new("gateway-token","apeireth")` → 目标 `apeireth.gateway-token`): `SERVICE TOKEN FACE READ OK (30 bytes)` + 同一强校验通过;
- **4. 门禁**
  - | 门 | 结果 |
  - | --- | --- |
  - | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ exit 0 (零警告) |
  - | `cargo test --workspace --locked -j 2 --no-fail-fast` | ✅ 既有套件零回归。两次全量实测: ① exit 0 全绿; ② **177 套件 / 5036 passed / 22 ignored** 仅 1 例负载 flake —— `apeireth-tools-canonical` 的 `appcontainer::probe_result_matches_capability_report` (既有真机 AppContainer 探针, 非本批代码, 隔离复跑绿, 按宪章「负载 flake 隔离复跑定性」处置) |
- **5. 诚实边界 (0 装)**
  - 1. **Linux 面未实测**: systemd 单元仅静态校验 (字段/语法测试钉住); 令牌面在 Linux 走 `/etc/apeireth/gateway.env` (root 0600, 明文落盘姿态与 Windows 凭据库不同), 上机前需逐项核对路径/用户/密钥管理;
  - 2. **Windows 任务形态**: 用户级计划任务 (登录自启) —— 无人值守**开机**即起 (未登录) 需改任务主体 (SYSTEM/受限账户) 并把令牌面改为该账户可读, 文档 §6 已注明;
  - 3. **桌面非 Windows 令牌面读取暂无接口** (桌面打包面为 Windows): 无令牌回落三态门照旧, 不假装取到;
  - 4. **占位 vs 端口**: 单实例拒绝以「占位存活 (端口应答)」判定 (跨平台同口径), `pid` 字段为诊断元数据, 不做进程表查询;
- **6. 复跑命令**

**归属链接**：frontend/companion-desktop/src-tauri/src/backend_supervisor.rs、packaging/service/apeireth-service.ps1、../docs/02-guides/server-deployment.md、packaging/service/README.md、crates/adapters/gateway/src/resident.rs、crates/adapters/gateway/src/shutdown.rs、crates/adapters/gateway/tests/resident_service.rs、crates/adapters/cli/tests/service_management.rs、crates/adapters/cli/tests/resident_gateway_e2e.rs、crates/foundation/credentials/src/service_token.rs、crates/adapters/tui/src/main.rs、scripts/check-neutral-terms.ps1

