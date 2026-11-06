# 落差清账·行为小修批（E-06 / E-07 / C-05）交付报告

> 日期：2026-10-16。范围：外部审计清单（Apeireth-宣称与实现落差清单.md）E-06、E-07 两节 + C-05 附带勘误。
> 战场：`frontend/companion-desktop/src-tauri/src/{provider_proxy.rs,backend_supervisor.rs,lib.rs}`、
> `frontend/companion-desktop/src-tauri/tests/supervisor_lifecycle.rs`、
> `crates/engine/runtime-assembly/tests/approval_history_survives_restart.rs`（新增）。
> 纪律：0 装 PASS / 中性表述 / 无脚本不发布数字 / 不写 CHANGELOG / 不 commit。
> 禁区零触碰：`frontend/companion-desktop/src/lib/**`、`docs/design/**`、`CHANGELOG.md`、`.github`。
> 取材口径：读码按 `路径:行` 复查；测试数字为本批实测（命令与日期随行）。

## 0. 三件一览

| 清单项 | 判定 | 改动 | 证据 |
|---|---|---|---|
| E-06 未配置即外呼 | 属实，已修 | 代发面加无凭据闸：无凭据外呼不出网，就地回 `reason=provider_not_configured` 提示帧 | `provider_proxy.rs` 5 测试（假传输面零调用断言） |
| E-07 后端首启 spawn 两次 | 属实，已修 | 模型/能力族重启判定移到 spawn 前解析：首启 spawn 持有到初始配置落定（上限 5s），一次 spawn 到 ready | `supervisor_lifecycle.rs` 起停事件序列断言 2 测试 + supervisor 单测 2 |
| C-05 审批默认不持久 | **审计误报**（勘误注记见 §3） | 不硬改：生产组合根本就组装 SqliteSessionStore，未动任何默认 | `approval_history_survives_restart.rs` 2 测试（跨"重启"存活） |

## 1. E-06 未配置即外呼（provider key 空 → 不发外呼）

**病灶（复核属实）**：桌面 webview 代发面 `provider_request`（`frontend/companion-desktop/src-tauri/src/provider_proxy.rs`）
对起手探测/模型列表类调用无条件代发；首启未填 key 时探测请求（GET `{base}/v1/models`，无凭据头）直达远端端点，
回 401，日志留下 `provider_request ok method=GET host=... status=401` 两行（`provider_proxy.rs` 代发日志即该格式）。

**修法**：代发前置一道无凭据闸（`unconfigured_suppression`）——
- 目标为**非环回**主机（`is_loopback_target`：localhost / 127.0.0.1 / ::1 之外）**且**
  请求**没有任何非空凭据头**（`has_credential`：authorization / proxy-authorization / x-api-key / api-key / x-auth-token / cookie，空值不算）→
  **不出网**，就地回合成「未配置提示帧」：
  `status=503`（刻意避开 401，绝不冒充服务端鉴权裁定）、`content-type: application/json`、
  body `{"error":{"code":"provider_not_configured","reason":"provider_not_configured","message":"provider key not configured; outbound call suppressed","solution":"在设置中填写提供商密钥后重试"},"host":...}`；
  `reason=provider_not_configured` 与网关能力面既有词汇（`crates/adapters/gateway/src/panels.rs:1226`）同口径；
- 日志面换新行：`provider_request skipped method=... host=... reason=provider_not_configured`（首启日志不再出现空 key 的 status=401 行）；
- **有 key 行为零变化**：任何带非空凭据头的请求按原路径原样代发（请求头/体/响应体逐字过境）；
- **环回面零变化**：本地网关（桌面 UI 自身的 /v1/models、token 门面）无凭据也照常代发。

**传输面可断言化**：代发改走 `RelayTransport` 抽象（生产实现 `ReqwestTransport` 收拢既有 `relay` 逻辑，超时/体积极限/脱敏不变），
测试用假传输面记录每次出网请求，「无凭据不外呼」以**零调用断言**钉住。

**测试（5，全部 `provider_proxy.rs` 内联）**：
1. `unconfigured_remote_probe_is_answered_without_egress` —— 无凭据模型列表探测：假传输面 **0 调用** + 显式 `reason=provider_not_configured`；
2. `empty_credential_headers_count_as_unconfigured` —— 空 `x-api-key`/空白 Authorization 等同未配置：0 调用；
3. `a_configured_call_relays_unchanged` —— 有 key：恰好 1 次出网，url/method/头/体逐字一致，响应原样过境（零回归）；
4. `loopback_targets_relay_without_credentials` —— 环回面（127.0.0.1 / [::1] / localhost）无凭据照常代发（零回归）；
5. `not_configured_frame_is_explicit_and_is_not_an_auth_verdict` —— 帧形钉住：503≠401、`error.reason`/`error.code` 显式、JSON 帧。

## 2. E-07 后端首启 spawn 两次（重启判定移到 spawn 前解析）

**病灶（复核属实）**：启动即 spawn（`lib.rs` setup 自动起）→ ready → Settings-UI 的初始配置推送（模型列表/能力族，
**不可热更**，见 `backend_supervisor.rs` `restart_relevant_pairs` 注释）到 `apply_backend_config` → 与 spawn 期快照比对
→ `backend.restart required (models/capabilities/other-family change is not hot-applicable)` → stop → 再 spawn：
`spawn→ready→restart required→stopped→spawn`，浪费一次起停（审计日志逐行对应）。

**修法**：把「模型/能力族变更是否需要重启」的判定**移到 spawn 前解析**——
- `BackendSupervisor` 增配置解析面 `config_resolution`（watch）：`apply_backend_config` 在**落值之后**翻真（=「配置面已开口」）；
- 新入口 `start_after_initial_config(window)`：首启 spawn 持有等待该信号（`INITIAL_CONFIG_WINDOW = 5s` 只是上限；
  信号一到立即继续），spawn 因此读到**已解析**的配置快照；窗口超时则按原语义照常 spawn；
- `lib.rs` 自动起改走新入口；后续配置推送语义**零变化**（可热更仍热更、不可热更仍恰重启一次）；
- 两条起跑线竞态（自动起 vs 配置推送自起）按「同配置良性竞态」收口（`start_resolving_config`）：输家看到 Starting/Ready 即视为达成，不报错、不二次 spawn；
- 起停事件序列面 `LifecycleEvent`（Spawn/Ready/ReadyFailed/SpawnFailed/RestartRequired/Stopped，`lifecycle_events()` 有序读出）供测试逐事件断言。

**目标达成口径**：首启事件序列恰为 `[Spawn, Ready]`（一次 spawn 到 ready），`restart_count == 0`，
且单一 spawn 上 `/v1/models` 已含注入模型（配置真到达了那一个进程）。

**测试（4）**：
6. `first_boot_spawns_once_to_ready_with_the_initial_config_resolved`（`supervisor_lifecycle.rs`，真进程）——
   自动起入口 + 初始配置并发推送，断言事件序列恰 `[Spawn, Ready]`、无 RestartRequired/Stopped、`restart_count == 0`、
   注入模型在该进程的 /v1/models 可见；
7. `launch_entry_after_spawn_keeps_the_existing_restart_semantics`（`supervisor_lifecycle.rs`，真进程）——
   窗口先到期（零等待）后到配置：仍恰重启一次，事件序列恰 `[Spawn, Ready, RestartRequired, Stopped, Spawn, Ready]`
   （既有 supervisor_lifecycle「晚到不可热更变更 = 恰重启一次」语义零回归）；
8. `a_fresh_supervisor_has_an_empty_lifecycle_face`（supervisor 单测）；
9. `apply_backend_config_resolves_the_launch_config_question`（supervisor 单测）—— 首次 apply 翻真解析信号、
   无变更 apply 不 spawn 不产生事件。
   既有 `supervisor_lifecycle.rs` 全部用例（spawn/ready/stop、provider_env/capability_env 恰重启一次、
   双 start 拒绝、令牌门、app-data 锚定）原样回归。

## 3. C-05 审批默认不持久 —— 审计误报，勘误注记（不硬改）

**读码实况**：生产组合根 `crates/adapters/cli/src/lib.rs:827`
`build_canonical_runtime_with_sessions_from_env()` 于 `:837` 调 `production_session_store()`（`:977`）显式组装
**`SqliteSessionStore::open(session_db_path())`**（`:987`，`APEIRETH_SESSION_DB` 覆写，默认 `.apeireth/sessions.sqlite3`）。
CLI 与 `apeireth gateway serve` 同一条装配线（`:907` 注释自述「The CLI is the composition root. Gateway reuses this function」）；
桌面端后端即 `gateway serve` 子进程（`backend_supervisor.rs` `spawn_args`），且 spawn 时把 `APEIRETH_SESSION_DB`
注入为 app-data 绝对路径（`spawn_backend` 锚定段，防相对路径启动失败）。清单所指
`InMemorySessionStore`（`crates/engine/runtime/src/canonical/runtime.rs:710`）是内核 RuntimeBuilder 的**惰性缺省**，
`session.rs:814-817` 文档自述「process-local only，重启持久化请组装 SqliteSessionStore」，生产路径不经过它。
→ **结论：审计误报（原条自带 ⚠未二次核验）**；按「不硬改」纪律未改任何默认值。

**证据测试（2，`crates/engine/runtime-assembly/tests/approval_history_survives_restart.rs` 新增）**：
10. `pending_approval_history_survives_a_session_store_reopen` —— 文件型生产会话库关闭重开（= 进程重启）后：
    待批记录、冻结指纹、审批族标注、`active_approval_id`、`ApprovalRequired` 事件原样存活；
11. `resolved_approval_history_survives_another_reopen` —— 决议后再次重启：终态 + 人工理由 +
    `ApprovalRequired → ApprovalResolved` 事件链两段齐全，`list()` 面可见。

**勘误注记（拟抄入清单 C-05 节）**：
> 勘误注记（2026-10-16，作者侧读码复核 + 实测证据）：本条为审计误报。生产组合根
> `crates/adapters/cli/src/lib.rs:827-993` 显式组装 `SqliteSessionStore`（`apeireth gateway serve` 与 CLI 同线，
> 桌面侧车为其子进程并注入绝对 `APEIRETH_SESSION_DB`）；`InMemorySessionStore` 仅为内核 builder 惰性缺省，
> 生产不经过。审批历史跨「重启」存活已由
> `crates/engine/runtime-assembly/tests/approval_history_survives_restart.rs` 钉住。未改任何默认值。
>
> （附注：清单文件位于会话附件目录、由工具面按只读件托管，本批无法就地写入；本注记以本报告为准，
> 可整段抄回清单 C-05 节。）

## 4. 测试汇总

- 本批新增 **11 例**（E-06 5 + E-07 4 + C-05 2），零新增外部依赖。
- 全量与门禁数字见 §5（脚本实测，随命令与日期）。

## 5. 门禁（宪章 §四 + AGENTS.md 校验命令，`-j 2`；实测 2026-10-16，rustc 1.97.1）

| 门禁 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ exit 0；**192 套件（测试二进制目标）/ 5392 passed / 0 failed / 18 ignored**，全 log 留档 `artifacts/batch-2026-10-16-workspace-test.log`（数字可复算） |
| clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ exit 0，零警告 |
| fmt | `cargo fmt --all -- --check`（根 + 桌面独立 workspace 双跑） | ✅ 双 0 diff |
| 前端检查 | `pnpm check`（svelte-check） | ✅ 0 errors / 0 warnings |
| 前端测试 | `pnpm test` | ⚠ **75/76 套件绿**；1 套件红为**既有**断言漂移（`session-preset-persistence.mjs`「治理同源」），非本批文件，已登记 D14 标红不代改（§6） |
| 中性表述扫描 | `scripts/check-neutral-terms.ps1` | ✅ 零命中 |
| secret-scan | `scripts/secret-scan.ps1`（scan-staged + scan-all） | ✅ 双 OK |
| 桌面独立 workspace | `cargo test -j 2` + `cargo clippy --all-targets -j 2 -- -D warnings` | ✅ **122 unit（本批 +7）+ 11 supervisor_lifecycle（本批 +2）/ 0 failed / 3 ignored**；clippy 零警告 |

**数字口径注**：`docs/03-reference/test-baseline.md`（单一真源，本批不代改）复测注记同日
191 套件 / 5390 passed / 18 ignored（落差清账·文本可信度批复测）+ 本批新增 1 套件 2 例 = 上表 192 / 5392 / 18，
链路自洽。审计附录「199 套件 / 5360 passed / 22 ignored」为审计者自计口径（其「套件」定义与测试二进制目标计数
不同）；按 AGENTS.md「不一致如实报本批实测值 + 日期，不代改真源」处理。
**既有套件零回归**：全量 0 failed；`supervisor_lifecycle` 既有 9 用例（spawn/ready/stop、provider_env/
capability_env 恰重启一次、双 start 拒绝、令牌门、app-data 锚定）原样全绿。

## 6. 跨范围与在途（宪章 §二.4 报备）

- 本批文件仅 §0 战场清单；工作树内其余改动（packaging/、scripts/、docs/ 口径族、`frontend/companion-desktop/src-tauri/Cargo.toml`
  构建缓存段等）为并行在途线，本批零触碰、零代改。
- **跨范围最小改动（报备）**：`cargo fmt --all -- --check` 在 HEAD 即有 2 个**非本批文件**的存量格式债
  （`crates/adapters/gateway/tests/user_domain_isolation.rs:389` 一处、`frontend/companion-desktop/src-tauri/src/workspace_list.rs` 四处），
  阻塞门禁 item 3「fmt 干净」。按宪章 §六.3「验收必经但不在战场内的面 = 最小改动 + 报备 + 理由」做**纯机械** rustfmt
  归位（零语义改动），其余不动。
- **标红不代改（登记 D14）**：`pnpm test` 1/76 套件红——`frontend/companion-desktop/tests/session-preset-persistence.mjs`
  「治理同源」断言字面落后于会话写闸装配重构（断言 `Arc::clone(&session_store)` 字面，现装配为 `sessions.store()`），
  治理语义意图仍成立；属批准档持久化线的治理断言演进，已登记 `00-backlog.md` D14，非本批代改。
- `docs/03-reference/test-baseline.md` 为测试数字单一真源，本批不代改；数字以本报告 §5 实测 + 日期如实报。
- README 族冻结照旧；本批无 README 变更。未 commit（按任务纪律）。
