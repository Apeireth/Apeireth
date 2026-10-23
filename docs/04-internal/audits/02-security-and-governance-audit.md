# 02 · 安全与治理审计结论册

> **本册定位**：把「能力级别抬级取证 / 行为落差清账 / 治理链合练 / 安全硬化交付 / 沙箱 / 凭据 / 后端代码安全审计」类文档的结论合成一册；保留逐项结论、残余清单与 owner 决策位，并逐条复核「今天还成立吗」。
>
> **审计对象与日期**（各份来源日期按原样标注，不改写）：
>
> | # | 来源（原路径） | 日期 | 篇幅 |
> |---|---|---|---|
> | 1 | `docs/04-internal/backend-enhancement/implemented-to-wired-plan-2026-10-21.md` | 2026-10-21 | 388 行 |
> | 2 | `docs/04-internal/backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md` | 2026-10-16 | 142 行 |
> | 3 | `docs/04-internal/governance-chain-drill-2026-10-20.md` | 2026-10-20 | 138 行 |
> | 4 | `docs/04-internal/backend-enhancement/security-hardening-delivery-2026-10-20.md` | 2026-10-19 / 2026-10-20 | 60 行 |
> | 5 | `docs/04-internal/backend-enhancement/security-restart-closeout-2026-10-22.md` | 2026-10-21 / 2026-10-22 | 55 行 |
> | 6 | `docs/04-internal/backend-enhancement/egress-gate-wiring-design-2026-10-23.md` | 2026-10-23（**设计，未实施**） | 122 行 |
> | 7 | `docs/04-internal/secret-management-policy.md` | 2026-08-21 | 297 行 |
> | 8 | `reports/s1-sandbox-hardening-self-audit.md` | 未标日期（v1.0 时代，crate `apeireth-companion` 已在 `legacy/`） | 156 行 |
> | 9 | `reports/v2-rc1-backend-security-audit-2026-09-24.md` | 2026-09-24 | 443 行 |
>
> **审计方法**：只读取证（`git grep` 查生产装配路径真实调用 / 注册 → 检查测试证据 → 必要时单 crate 只读复跑）＋ 逐行读码判定，按四级口径（`IMPLEMENTED` / `PRODUCTION WIRED` / `DEFAULT ENABLED` / `HARDWARE VALIDATED`，见 [system-capabilities.md](../../01-architecture/system-capabilities.md) 头部）定级。
> **复核基线**：2026-10-23，仓内事实核验（工作树 HEAD `b4aabfc2`）。能力声明一律引用 [claims.yaml](../../status/claims.yaml)（含逐条 `level` 与 `verified_at`），架构契约引用 [contract.toml](../../architecture/contract.toml)，测试数字引用 [test-baseline.md](../../03-reference/test-baseline.md)（本册不抄写数字）。
>
> **结论摘要**
>
> 1. **抬级取证给出一个可复用判据**：`wiring.yaml` 的 `production_refs` 只是**路径启发式**（生成器按前缀判定），不能证明任何真实调用；「有没有生产引用」必须另做 `git grep` 取证。12 条 claim 无一条臆造或 0 装冒充，差距集中在**生成器口径倒挂 / CI 执行面缺口 / 验证面资产被套进四级口径**三类结构性问题。
> 2. **本批之后 12 条里已有 6 条抬到 `PRODUCTION WIRED`**（以 [claims.yaml](../../status/claims.yaml) 今日 `level` 为准），治理单一入口已在 CLI 装配根落地（`governance_chain::production(...)`）。
> 3. **治理链「零件齐、编排缺」的历史判断已被部分推翻**：2026-10-20 演练记录明说「编排未接线」，而今 CLI 已改用治理链装配；但**物理（密码学）多签仍挂 v2.1**，「逻辑票 ≠ 多签」的措辞纪律不变。
> 4. **安全硬化线（外部审计 8 项隐患）已全数修复入库**，但**残余清单没有消失**，只是逐条转入「owner 决策位 / 后续排期登记」——B2 回环注册面、B6 列级接线与盲索引残余、B8 签名与可复现构建。
> 5. **模型出口仍是治理盲区**（2026-10-23 设计）：工具调用受治理，**送给模型的内容不受治理**；出境脱敏闸本体已写好但**零挂载**，设计给出 5 步可独立合并、可回滚的实施序，默认关起步。
> 6. **2026-09-24 的 1 项 Critical + 13 项 High 中，绝大多数已修**（假加密正名、恒真门、路径逃逸、凭据静默覆盖、时序侧信道、SSE 缓冲无界等）；**WS 握手 token 仍从不校验**，属潜伏项。

---

## 1. IMPLEMENTED → PRODUCTION WIRED 抬级方案（12 条 claim 取证审计，2026-10-21）

**来源**：`docs/04-internal/backend-enhancement/implemented-to-wired-plan-2026-10-21.md` · **基线**：`HEAD d2b8c403`，工作树快照 2026-10-21 16:19–16:26（并发线未提交改动：10 个 M 文件 + 2 个未跟踪文件，原文 §0.4 列明）· **纪律**：只读代码与文档，未改任何文件。

### 结论（原文要点保留）

- **§0.2 关键结论（生成器口径倒挂）**：`wiring_report.py` 的 `PROD_PREFIXES` 只含 `crates/` 与 `frontend/companion-desktop/src-tauri/src/`，`is_production_source()` 仅按前缀 + 排除测试/文档后缀判定 ⇒ **IMPLEMENTED 条目的 `production_refs` 基本等于 sources 自身的路径命中，不能证明任何真实调用**；本审计因此全部另做 `git grep` 取证。ADR-0004 §7.4/§7.5：`PRODUCTION WIRED` 必须有生产引用、`IMPLEMENTED` 必须有 gaps 原因，缺证据即 FAIL，**禁止无生产引用却标 PRODUCTION WIRED**。
- **验证面**：`python scripts/docs_truth.py --verify` = claims=24 fail=0 stale=6 review_due=0，本 12 条全部 OK（6 条 STALE 均属 `bio.*` 批，与本 12 条无关）；12 条 `verified_at` 对 sources 的 STALE 检查 = 12/12 空。
- **分类计数**：(a) 已被生产使用、仅缺证据登记 = **3**（`sec.linux_containment` / `gov.chain_drill_ci` / `sec.governance_chain_drill`）；(b) 有默认关接线未登记开关 = **0**；(c) 纯库内/脚本面无生产调用 = **5**（`sec.storage_envelope_crypto` / `gov.chain_skeleton` / `gov.chain_hook_adapter` / `gov.reflection_queue` / `rel.release_asset_verify`）；(d) 前端桌面面按桌面装配根判定 = **4**（`sec.sql_readonly_gate` / `sec.provider_proxy_ssrf` / `sec.desktop_file_roots` / `sec.terminal_permission_surfaces`）。**总判**：12 条全部有真实代码/测试资产，**未发现臆造或 0 装冒充**（诚实纪律成立）；三类结构性问题 = ①生成器口径倒挂（4 条 (c) 与 2 条 (a) 受影响）；②CI 执行面缺口（桌面 CI 不跑 `cargo test`/`pnpm test`；发布流水线零引用发布资产校验脚本；治理链演练步骤远端首跑未验证）；③验证面/文档面资产被套进四级口径（建议按 ADR-0004 §7.7 的 `kind = "test-only"` 尾批口径裁决，**不硬抬 PRODUCTION WIRED**）。
- **逐条要点（12 条）**：

| id | 当时级别 / 分类 | 取证要点 | 抬级路径与风险 |
|---|---|---|---|
| `sec.linux_containment` | IMPLEMENTED / (a) | 生产默认工具路径已消费（`repo.rs:153`、`shell.rs:700`、`git_write.rs:938`、`worktree_apply.rs:464` 构造 `ProcessExecutor`）；默认配置 filesystem/search/repo = true；Linux 后端 = 进程组边界 + rlimit + `PR_SET_PDEATHSIG`；能力面自报 `ProcessTreeContainment=Partial`、`CpuLimit=Enforced` 等；测试真断言（树杀 / NO_NEW_PRIVS / NPROC partial） | 只需证据登记（notes + `verified_at` + 一次远端 Linux CI 首跑记录）；1–2 提交，风险低 |
| `sec.storage_envelope_crypto` | IMPLEMENTED / (c) | 库外零消费（`git grep` 无命中）；库内 API：`SealingLayer`/`FieldSink`、`export_sealed_backup`/`verify`/`restore`（阶段 1 全量校验后才落盘）、`KeyWrapFile`、`KekAnchor`（**无实现挂载**）；测试真断言（明文零泄漏 / AAD 绑分区 / 先验后写 / 无后门） | 3 波：波 1 CLI 密钥出口（1 提交，材料现成）→ 波 2 列级接线（需 owner 拍板 P0 字段 / KDF / 锚定 / 迁移，风险**高**）→ 波 3 并入备份出口；5–8 提交 |
| `gov.chain_skeleton` | IMPLEMENTED / (c) | HEAD 零调用；生产治理是 `GovernancePipeline` + preset hook；治理注入点唯一在 CLI 装配根；库内 214 passed；**工作树已有 R8-B2 chain 装配 + 未跟踪四态等价测试** | 收口在途 + 登记；2 提交，风险中（改变生产治理判定路径，回滚一行） |
| `gov.chain_hook_adapter` | IMPLEMENTED / (c) | 同上；库内测试含 allow / deny / require_approval / 无门 fail-closed / 转发审批决议 | 与 `gov.chain_skeleton` **同一 commit** 抬级，0–1 提交 |
| `gov.reflection_queue` | IMPLEMENTED / (c) | 零装配；库内 `ReflectionScheduler` 三型 + 「默认关等价测试」已存在；**无产品开关**（env registry 零命中）故不归 (b) | 依赖 R8-B2 先收口；R8-B3 接线需 owner 拍板（延迟反射 = 行为变化面）；2–3 提交，风险中高 |
| `gov.chain_drill_ci` | IMPLEMENTED / (a) | CI 步骤确在 workflow 内；本地等价命令 3 passed；**远端首跑未验证** | 属验证面，建议 `kind = "test-only"`，不硬抬；0–1 提交 |
| `sec.governance_chain_drill` | IMPLEMENTED / (a) | 演练文件 488 行、三路径 3/3；helpers 覆盖 2-of-3 具名票 + 持久化 verify + 篡改检查 | 同上，纯登记；0–1 提交 |
| `rel.release_asset_verify` | IMPLEMENTED / (c) | 脚本与文档齐备（tests=[] 如实）；**`.github/workflows/` 零命中** = 发布流水线不调用；结构缺口：`PROD_PREFIXES` 不含 `scripts/`、`.github/` ⇒ 即使接线也不能直接标 PRODUCTION WIRED，需架构线先裁决 schema | 接线 1 提交 + 证据 0–1 + 口径裁决 0–1；风险中（需下次 tag 实跑） |
| `sec.sql_readonly_gate`（B1） | IMPLEMENTED / (d) | Tauri 命令已注册 + 前端已调用；词法级只读闸真断言（含 WITH 内嵌 DML / EXPLAIN ANALYZE UPDATE / SELECT INTO / FOR UPDATE / 多语句）；PG/MySQL 只读事务帧序列断言 + 错误回滚 | 桌面面生产接线成立；缺 tests 登记 + CI 执行 + live E2E；2 提交 |
| `sec.provider_proxy_ssrf`（B2） | IMPLEMENTED / (d) | 目标类闸 + DNS 钉址 + 逐跳重验 + 头白名单真断言；**loopback 放行是产品设计**；残余 = 回环类级口子 + 跨主机重定向 rebinding 小窗 | 同 B1；2 提交 |
| `sec.desktop_file_roots`（B3） | IMPLEMENTED / (d) | 登记根派生 + canonicalize 祖先校验；junction / 链接逃逸实测拒绝；残余 TOCTOU 窗口未闭 | 同 B1；2 提交 |
| `sec.terminal_permission_surfaces`（B4） | IMPLEMENTED / (d) | 本体是**文档/UI 文案资产**（边界四条 + 审批族接线缝断言 + 三层权限面文档）；底层命令已注册 | 建议保持 IMPLEMENTED + 补测试执行证据；1–2 提交 |

- **建议首批（4 条）**：R8-B2 收口 + `gov.chain_skeleton`/`gov.chain_hook_adapter` 登记（收益/成本最高，代码已写好、四态等价测试在、回滚一行）；`sec.storage_envelope_crypto` 波 1（现成草稿，1 提交把「零出口」变「人类出口可用」）；`rel.release_asset_verify`（唯一缺口是流水线不调用）；B1/B2/B3 证据登记批。**不建议首批**：`sec.linux_containment`（需远端 Linux CI 首跑）、`gov.reflection_queue`（最重）、两个验证面 claim（等 §7.7 口径裁决）、B4（并入证据批即可）。
- **未验证项清单（11 条，保留）**：远端 CI 执行记录不可得；Linux 目标真机实测（Windows 本机跑不了 `#[cfg(target_os = "linux")]` 模块，`HARDWARE VALIDATED` 不成立）；桌面 live E2E 未实测；B2 残余可利用性未验证；B3 TOCTOU 窗口未验证；B6 加密面未验证项（列级改造 / 平台锚定 / KDF 强度 / P1 字段定级 / 口令无回显 / live E2E）；发布流水线改动后行为未实测、Pester 可用性未验证；wiring 前缀口径未裁决；在途并发改动的最终去向未验证；等价测试是否被 CI nextest 覆盖未实测；未跑全量 workspace 测试（故「其他 crate 未回归」不作结论）。

### 现状复核（2026-10-23）

| claim | 2026-10-21 判定 | 今日 `level`（[claims.yaml](../../status/claims.yaml)） | 复核结论 |
|---|---|---|---|
| `sec.linux_containment` | IMPLEMENTED / (a) | `IMPLEMENTED` | **仍成立**：未抬级，远端 Linux 首跑证据仍缺 |
| `sec.storage_envelope_crypto` | IMPLEMENTED / (c) | `PRODUCTION WIRED` | **已过时**：结论已落地（见 02 §4 B6、§5 波 3） |
| `gov.chain_skeleton` | IMPLEMENTED / (c) | `PRODUCTION WIRED` | **已过时**：R8-B2 已收口，CLI 装配根现为 `governance_chain::production(preset_governance)` |
| `gov.chain_hook_adapter` | IMPLEMENTED / (c) | `PRODUCTION WIRED` | **已过时**：与上一条同批抬级 |
| `gov.reflection_queue` | IMPLEMENTED / (c) | `IMPLEMENTED` | **仍成立**：延迟反射仍无产品开关 |
| `gov.chain_drill_ci` | IMPLEMENTED / (a) | `IMPLEMENTED` | **仍成立**（验证面，§7.7 口径裁决待 owner） |
| `sec.governance_chain_drill` | IMPLEMENTED / (a) | `IMPLEMENTED` | **仍成立**（验证面） |
| `rel.release_asset_verify` | IMPLEMENTED / (c) | `IMPLEMENTED` | **仍成立**：发布流水线仍零引用该脚本（口径裁决未落地） |
| `sec.sql_readonly_gate` | IMPLEMENTED / (d) | `PRODUCTION WIRED` | **已过时**：桌面面已抬级，且桌面 CI 现已跑 `cargo test --workspace --all-targets` |
| `sec.provider_proxy_ssrf` | IMPLEMENTED / (d) | `PRODUCTION WIRED` | **已过时**：已抬级；残余（回环注册面、rebinding）转入 02 §5 残余总账 |
| `sec.desktop_file_roots` | IMPLEMENTED / (d) | `PRODUCTION WIRED` | **已过时**：已抬级；TOCTOU 残余仍在册 |
| `sec.terminal_permission_surfaces` | IMPLEMENTED / (d) | `IMPLEMENTED` | **仍成立**：文档/UI 面按建议保持 `IMPLEMENTED` |

- **仍成立（判据本身）**：`production_refs` 是路径启发式、必须另做调用点取证 —— 对今天仍成立（`claims.yaml` 与 `wiring.yaml` 仍由生成器维护，生成物禁手改）。
- **部分已过时（CI 执行面缺口）**：桌面 CI 现已执行 `cargo test --workspace --all-targets`（`.github/workflows/companion-desktop-ci.yml`），B1–B3 的「测试资产不在 CI 执行面」已闭环；**`pnpm test` 仍未进该 workflow**（只跑 `pnpm check` / `check:assets`）⇒ B4 的建议**未闭环**。
- **未核实**：`rel.release_asset_verify` 的 wiring 前缀口径裁决、`gov.chain_drill_ci` 的远端首跑记录、11 条「未验证项」中的 live E2E / 利用性验证。

---

## 2. 落差清账·行为小修批（E-06 / E-07 / C-05，2026-10-16）

**来源**：`docs/04-internal/backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md` · **战场**：桌面侧 `provider_proxy.rs` / `backend_supervisor.rs` / `lib.rs` + `supervisor_lifecycle.rs`；新增 `crates/engine/runtime-assembly/tests/approval_history_survives_restart.rs` · **纪律**：0 装 PASS / 中性表述 / 无脚本不发布数字 / 不写 CHANGELOG / 不 commit。

### 结论（原文要点保留）

| 清单项 | 判定 | 改动 | 证据 |
|---|---|---|---|
| **E-06 未配置即外呼** | 属实，已修 | 代发面加无凭据闸：无凭据外呼不出网，就地回 `reason=provider_not_configured` 提示帧 | `provider_proxy.rs` 5 测试（假传输面零调用断言） |
| **E-07 后端首启 spawn 两次** | 属实，已修 | 模型/能力族重启判定移到 spawn 前解析：首启 spawn 持有到初始配置落定（上限 5s），一次 spawn 到 ready | `supervisor_lifecycle.rs` 起停事件序列断言 2 测试 + supervisor 单测 2 |
| **C-05 审批默认不持久** | **审计误报**（勘误注记） | 不硬改：生产组合根本就组装 `SqliteSessionStore`，未动任何默认 | `approval_history_survives_restart.rs` 2 测试 |

- **E-06 修法**：目标为**非环回**主机且请求**无任何非空凭据头**（authorization / proxy-authorization / x-api-key / api-key / x-auth-token / cookie，空值不算）→ **不出网**，就地回 `status=503`（刻意避开 401，**绝不冒充服务端鉴权裁定**）+ JSON 帧 `{"error":{"code":"provider_not_configured","reason":"provider_not_configured", ...}}`；日志换新行 `provider_request skipped ... reason=provider_not_configured`；**有 key 行为零变化**；**环回面零变化**（本地网关 / 模型门面无凭据也照常代发）。传输面抽象为 `RelayTransport`（生产实现收拢既有逻辑，超时/体积极限/脱敏不变），测试用假传输面以**零调用断言**钉住。5 测试：无凭据模型列表探测 0 调用、空凭据等同未配置、有 key 恰好 1 次出网且逐字一致、环回照常代发、帧形钉住（503≠401 + 显式 reason/code）。
- **E-07 修法**：`BackendSupervisor` 增配置解析面 `config_resolution`（watch），`apply_backend_config` 在**落值之后**翻真；新入口 `start_after_initial_config(window)`（`INITIAL_CONFIG_WINDOW = 5s` 只是上限，信号一到立即继续，超时则按原语义照常 spawn）；两条起跑线竞态按「同配置良性竞态」收口；起停事件序列面 `LifecycleEvent`（Spawn/Ready/ReadyFailed/SpawnFailed/RestartRequired/Stopped）供逐事件断言。目标口径 = 首启事件序列恰 `[Spawn, Ready]`、`restart_count == 0`、单一 spawn 上 `/v1/models` 已含注入模型。4 测试：首启恰一次（真进程）、窗口先到期后到配置仍恰重启一次（事件序列恰 `[Spawn, Ready, RestartRequired, Stopped, Spawn, Ready]`）、空生命周期面、首次 apply 翻真解析信号。
- **C-05 勘误注记（拟抄回清单 C-05 节）**：生产组合根 `crates/adapters/cli/src/lib.rs` 的 `build_canonical_runtime_with_sessions_from_env()` 显式组装 `SqliteSessionStore::open(session_db_path())`（`APEIRETH_SESSION_DB` 可覆写，默认 `.apeireth/sessions.sqlite3`）；CLI 与 `apeireth gateway serve` 同一条装配线；桌面端后端即 `gateway serve` 子进程并注入绝对 `APEIRETH_SESSION_DB`；清单所指的 `InMemorySessionStore` 是内核 RuntimeBuilder 的**惰性缺省**，生产路径不经过它 ⇒ **审计误报**，未改任何默认值。证据：待批记录 / 冻结指纹 / 审批族标注 / `active_approval_id` / `ApprovalRequired` 事件在「重启」后原样存活；决议后再次重启终态 + 人工理由 + `ApprovalRequired → ApprovalResolved` 事件链两段齐全。
- **门禁与报备**：本批新增 11 例、0 新增外部依赖；全量 / clippy / fmt / 前端检查 / secret-scan / 中性表述扫描全绿（数字口径以 [test-baseline.md](../../03-reference/test-baseline.md) 为准）；`pnpm test` 1/76 套件红属**既有断言漂移**，登记 D14 标红不代改；fmt 门禁在 HEAD 即有 2 处**非本批文件**的存量格式债，按「最小改动 + 报备 + 理由」做纯机械归位。

### 现状复核（2026-10-23）

- **仍成立（E-06 / E-07 / C-05 的结论与修法）**：三处改动的语义在今日代码面仍成立（无凭据不外呼、重启判定前移、生产会话库为持久化面）；本批未逐行复验测试体。
- **已过时（门禁数字）**：原文门禁数字按 2026-10-16 口径，**不是现行口径**（现行见 [test-baseline.md](../../03-reference/test-baseline.md)）。
- **仍成立（两条纪律）**：①「503 ≠ 401」——不冒充服务端鉴权裁定；②「审计误报也要留勘误注记」（原清单在附件目录无法就地写入，故以报告为准抄回）。
- **未核实**：D14（`session-preset-persistence.mjs` 治理同源断言漂移）是否仍红，本批未跑前端测试。

---

## 3. 治理链端到端合练记录（B7，2026-10-20）

**来源**：`docs/04-internal/governance-chain-drill-2026-10-20.md` · **口径（0 装，严守）**：演练对象是**现役 M-of-N 逻辑票链**（字符串票，`onion.rs` 自述 0 装占位、非真密码学校验）；**逻辑票 ≠ 密码学多签**，物理不可伪造的多签留 v2.1。本记录是「链可走通」的**执行证据**，**不是**「生产已接线」的证据。

### 结论（原文要点保留）

- **链路图（六段）**：① V1+V2+V3 AND 门（`ActionGuard::check_action` + `check_ha`）→ ② 审议意见记名（Ballot → synthesize → `passes_strategy`；hold 形态 evaluate）→ ③ M-of-N 逻辑票（`verify_multisig`：2-of-3、去重真实人类、拒未登记者/畸形票；三路径 Accepted / Insufficient / 超时不成案）→ ④ 审批闭合（`ApprovalAuditPair` 成对提交 + `scan_approval_pairs` + `may_resolve_approval`）→ ⑤ 审计落盘（`AuditHashChain` append → JSON 落盘 → 读回 verify → 篡改一字检出 `RecordHashMismatch`）→ ⑥ 反思期（`SelfDisableAudit` 记录反思查询 + 白名单 + 违规报 HA）。**串联顺序与「hold/超时 ⇒ 不批准」的组合规则是本次演练定义的胶水：库里没有单一编排函数把六步串成链（如实报：编排未接线）**；各环节判定用的都是现役真实实现。
- **三路径执行记录**：通过路径（7 席具名票 5 赞成/1 反对/1 弃权 → 弃权不计票；超多数 + 加权多数双策略通过；2-of-3 Accepted；成对闭合零悬挂；反思期白名单放行 + 元问题拦下；11 条审计记录落盘 + 篡改检出）；拒绝路径（两种否决形态分别 hold；票数不足；防凑票对抗：同人×2 去重、未登记/畸形不计数；offline 恒拒；`InvalidHA`；拒绝闭合 fail-closed；**跨用户审批一律拒**）；超时路径（`HOLD_DELIBERATION_TIMEOUT_MS` 边界阈值前 1ms 不触发、到阈值必触发 `DeliberationTimeout`；票未达法定数；`expired → ApprovalOutcome::Unavailable` fail-closed 不授权；越权收尾同拒）。
- **销账对齐（只读对账，不改 backlog）**：D2 四行（物理多签 v2.1 / UpgradeCycle 留白 / 风险阈值校准 / Kani 真接线）**一项未销**；D8 三条中「审议→多签→反思期串联零执行记录」**已销**，「`multi_sign` 全库调用点均为 `None`」与「反思期审计默认不运行」**未销**。
- **缺口 6 项（0 装，没走通的环节如实记）**：①编排未接线；②逐动作签名收集未接线（`gate.rs` 自述签名收集由调用方接线，但该调用方在生产面不存在，`verify_multisig` 今天零生产调用点）；③物理（密码学）多签缺位（签名是 `"signer:digest"` 字符串票，防凑票靠去重登记表，无不可伪造性）；④反思期无常驻调度（测试内是逐路径手动跑）；⑤审计持久化面缺位（`AuditHashChain` 是内存链，本次「落盘」是测试胶水；现役只有 `MemoryApprovalAuditSink`，生产级持久 sink 未接线）；⑥审议计时无自动器（阈值常量 + 调用方注入，库内无计时器）。
- **合规自查**：LOCKED 5 项 0 触碰；战场外文件 0 触碰；零第三方产品名；fs_err 为既有 dev 依赖；未 commit。

### 现状复核（2026-10-23）

- **已过时（缺口 ① 编排未接线）**：治理链已成为生产装配面 —— CLI 装配根现走 `governance_chain::production(preset_governance)`；[contract.toml](../../architecture/contract.toml) 的 status_note 亦记 R8-A/B1/C/D 交付、R8-B2 收口。
- **部分已过时（缺口 ⑤ 审计持久化面缺位）**：已出现持久审计面（`crates/adapters/gateway/src/chain_audit.rs`、安全线 S2 批的 `FileChainAuditSink` 持久审计 —— jsonl 哈希链 / 拒篡改 / 重启 verify / savepoint 回滚），见 02 §5。
- **仍成立（缺口 ③ 物理多签）**：真 Ed25519 多签仍留 v2.1（`core/src/onion.rs` 多签仍为 hex 占位）。
- **仍成立（缺口 ④ 反思期无常驻调度 / ⑥ 审议计时无自动器）**：`SelfDisableAudit` 仍需调用方构造并周期执行；`HOLD_DELIBERATION_TIMEOUT_MS` 仍只是阈值常量。
- **部分已过时（缺口 ② 签名收集零生产调用点）**：随治理链接管生产面，签名收集的宿主已存在；物理多签本体仍缺。
- **仍成立（措辞纪律）**：「逻辑票 ≠ 密码学多签」的全篇一致措辞必须保留，任何对外描述不得把 M-of-N 逻辑票说成密码学多签。
- **未核实**：三路径测试与 D2/D8 销账行本批未复跑。

---

## 4. 安全加固线交付报告（B1–B8，2026-10-19 / 2026-10-20）

**来源**：`docs/04-internal/backend-enhancement/security-hardening-delivery-2026-10-20.md` · **来源说明**：owner 提供的外部安全审计（8 项风险）· **处置**：8 项全部修复入库（B1–B4 于 2026-10-19，B5–B8 于 2026-10-20）；与生物机制线并行施工，同文件 hunk 级分账，零裹挟 · **全 commit 链（10）**：8 修复 + 交付记录 + 索引收工。

### 结论（原文要点保留）

| # | 审计风险 | 批次 | 修法 | 残余（如实） |
|---|---|---|---|---|
| 1 | SQL 只读限制可被 WITH DML 绕过（高危） | B1 | 词法级只读闸（fail-closed）+ PG/MySQL 服务端只读事务双保险 | 保守启发式可能误拒冷门合法 SQL（fail-closed 方向） |
| 2 | 桌面终端权限高于「shell 已关闭」印象 | B4 | UI 权限边界明示 + 三层权限面文档 + 文案测试钉子 | 超时树杀待接（既有债，文案如实标注） |
| 3 | Provider 中继允许任意主机（SSRF 面） | B2 | 目标类闸 + DNS 钉址 + 重定向逐跳闸 + 头白名单 | 回环类级口子待「本地端点注册面」；公网 allowlist 待续 |
| 4 | 文件操作不限定工作区 | B3 | 登记根派生 + canonicalize 祖先校验（junction 实测拒） | TOCTOU 窗口未闭（已缓解）；live E2E 未实测 |
| 5 | Gateway TLS 未接线 + none 逃生阀 | B5 | PEM 真加载 fail-closed + TLS listener + 三态状态面；live 握手 E2E 真测 | TLS 1.2/1.3 矩阵、ECC、证书轮换未实测；none 逃生阀保留（显式自担 + 审计） |
| 6 | 会话/记忆静态未加密 + 备份恢复不完整 | B6 | 分区 DEK + AES-256-GCM 信封（AAD 绑分区）+ KEK 三槽 + SealedBackup 宁拒不损 | 列级改造未接线（原语已交）；平台锚定未接线；P1 字段定级待拍板 |
| 7 | 治理链「组件已写、整条链未演练」 | B7 | M-of-N 逻辑票链三路径演练 + 对抗用例 + 演练记录落档 | 缺口 6 项（见 02 §3 §缺口） |
| 8 | 安装包来源验证不足 | B8 | 发布资产校验脚本（哈希三口径 + 签名如实报）+ 流程文档 | 签名证书 / 可复现构建 / 平台校验 / 分通道 = **排期 4 项 owner 决策** |

- **门禁真账（各批收口实测）**：桌面 crate（B1–B4）151 passed / 0 failed、clippy 0、fmt 0、前端套件全绿；gateway（B5）304 passed（含 live TLS 握手 E2E）；storage（B6）184 passed（三不变量钉死）；governance（B7）197 passed；scripts（B8）7 用例真二进制实测全符预期；LOCKED 5 项 0 触碰；secret-scan 全过；0 新外部包（依赖边均为锁内既有包）。全 workspace 三件套见文档末尾「收口补记」（passed 总数不抄写，以单一真源为准）。
- **工程方法真账（多 AI 并行）**：8 批次 10 个施工者接力（3 次子代理静默中断，按宪章 §13 #3 主代理自验接手/重派，无弃尸）；与生物机制线并行期同文件混改 → **hunk 级分账提交**（选择性暂存），`Cargo.lock` 依赖边同理分账；负载 flake 登记未消（admin_config 热更新 / supervisor_lifecycle 端口竞争，隔离复跑即绿）。
- **后续排期登记（owner 决策位 6 项）**：①B2「本地端点注册面」+ 公网 allowlist；②B6 P1 敏感字段定级 + KDF 参数基准校准；③B6 列级改造逐表接入 + 平台锚定；④B7 治理缺口 6 项（与 D2 挂账合并排期）；⑤B8 排期 4 项；⑥B5 未实测面补测。

### 现状复核（2026-10-23）

- **仍成立（8 项修法本体）**：B1–B8 的修法描述与当时一致；B1/B2/B3 的 claim 已在 [claims.yaml](../../status/claims.yaml) 抬为 `PRODUCTION WIRED`。
- **部分已过时（残余清单）**：①B2 回环类级口子 → 已在 S3 二段（本地端点注册制）与 B2-CFG（用户自定义服务器白名单配置面）收口；②B6 列级改造 → 已交示范与六档定级，但**逐列接入仍未完成**；③B5 未实测面 → 已补 TLS 协议矩阵 / 椭圆曲线 / 冷启动轮换（S3 批）；④B7 缺口 6 项 → 部分收口（编排已接、审计持久化面已补）；⑤B8 4 项 → 仍挂 owner 决策（见 02 §5）。
- **仍成立（残余中的两条硬残余）**：B3 的 **TOCTOU 窗口未闭**；B5 的 **none 逃生阀保留**（显式自担 + 留审计）。
- **未核实**：各批门禁数字（按 2026-10-19/20 口径）与依赖边分账细节本批未复查。

---

## 5. 安全线重启批收口报告（S1→S3，2026-10-21 / 2026-10-22）

**来源**：`docs/04-internal/backend-enhancement/security-restart-closeout-2026-10-22.md`

### 结论（原文要点保留）

**入库清单（8 commits，均主代理亲验）**

| Commit | 批次 | 内容 |
|---|---|---|
| `4e380a69` | S1·R8-B2 | CLI 装配根切换单一入口（preset 入链 Gate；四态行为等价真账；helper 模块尺寸棘轮净 0） |
| `e812fda9` | S2·R8-B3 | FileChainAuditSink 持久审计（jsonl 哈希链 / 拒篡改 / 重启 verify / savepoint 回滚）+ ChainApprovalAudit（IM 审批对经链落账）+ production() 响亮降级；ADR-0010 §6/§9 + CHANGELOG 同步 |
| `d6b6420b` | S3·B5 | TLS 补测：协议矩阵 1.2/13 真握手、椭圆曲线 P-256 真链、冷启动轮换 + 旧信任拒；生产证书 5 步清单 |
| `f72a6f13` | S3·B6 | 加密字段六档定级（content_hash 改判加密、vector_json 保明、Unrated fail-closed）+ KDF 210k→600k + sqlite_sink 列级接入示范 |
| `e4b09767` | S3·B2 | SSRF 收口二段：LocalEndpointRegistry（回环注册制）+ ProviderAllowlist（公网限已配置基址）+ 重定向逐跳钉址 + GatedResolver |
| `f4b0f335` | S 收口 | 本页 + CHANGELOG 批初版（S1–S3 六 commit 台账） |
| `12e77138` | S3·B6-BLIND | 查重盲索引：分区 DEK 派生 HMAC 键（绑分区 id、Drop 零化）+ 索引面自建表与内容面分离（查询不触密文）+ 5 集成/7 单测；残余 4 项如实登记 |
| `44082dea` | S3·B2-CFG | 用户自定义服务器白名单配置面：既有信封惯例持久（坏档拒开、同锁原子）+ 策略合并单入口 + 解析层补洞 + UI「工具与安全」管理面 |

- **S3 拍板记录（owner「按你的建议来」，2026-10-21）**：**B6 P1 定级** = 向量列保明（检索基元 + 产品宪法「摆在明面」）、聚合元数据保明、content_hash / dedup 指纹列改判加密（字典反推面）、KDF 600k；残余 = 相似度推理（明向量 + 密文长度）、盲索引待选型、Unrated 边界列待逐列核、600k 开销未实测。**B8 排期**：P1 代码签名证书接入 = **owner 资源决策位**（采购/签发面）；P2 可复现构建证明 = 登记排期；P3 其他平台校验链实测 = 登记排期；P4 清单与安装包分通道 = 登记排期。**B7 治理缺口 6 项** = 与 D2 挂账合并排期，归治理线正题（以演练记录 §5 为准）。
- **环境体检**：`%TEMP%` 拒写 = 沙箱按可执行文件位置收口（target 下二进制禁写用户 `%TEMP%`）→ 测试跑前 `TEMP` 指向 `target\_tmp`（建议效率线固化进快反馈门禁）；页面文件峰值不足（建议固定 32–48GB 或错峰编译）；service dry-run 计划任务 cmdlet 受限子进程拒访问（登记）。
- **残余总账（owner 决策位 / 后续批）**：**2026-10-22 owner 裁决四项** = ①代码签名证书 = **挂起**（正式版发布前再评估是否采购）；②清单与安装包分通道 = 登记排期（正式版发布前实施）；③用户自定义服务器白名单配置面 = **立项实施 → 已交付**；④查重盲索引 = **拍板实施 → 已交付**（残余如实登记）。**B2 残余** = 向导基址注册时机归前端线；**审批仍端点级非意图级**（放行主机 = 全部可中继，UI 提示语已警示）；名字目标类闸判词经**连接期**浮出（比 URL 级闸晚一步）；桌面 live E2E 未实测；store 手改可塞非法条目（不产生审批 fail-closed、表内显无效行，加载期逐条拒开留后续）。**B5 残余** = 生产证书真验（持真链时按清单）、热重载 / 自动证书 / 双向认证需求另立。**B6 残余** = 真读写路径逐列接线（归口存储 owner）、平台锚定；**盲索引残余 4 项**：持盲索引键者可离线字典枚举（本构造不防）、索引条目无行级认证（篡改 = 命中失效/漏检，删除与重放可致伪命中）、键随分区密钥同命运（密钥丢失 = 索引面同失）、原值口径由调用方统一（同值不同序列化 = 不同键）。**快反馈门禁环境红**（nextest 段 OOM / `%TEMP%`）→ 环境修好前以各 crate scoped 实测为准。

### 现状复核（2026-10-23）

- **仍成立（残余总账）**：B2 端点级审批（非意图级）、B5 生产证书真验、B6 逐列接线 + 盲索引 4 项残余、B8-P2/P3 排期、B7 缺口 6 项归治理线 —— 均仍在册（owner 决策位未销）。
- **仍成立（owner 裁决）**：代码签名证书 = **挂起**（未采购）；分通道 = 登记排期。
- **待核实（B8-P4 的前半）**：`rel.release_asset_verify` 仍是 `IMPLEMENTED`、发布流水线**仍未调用**校验脚本（02 §1 复核）⇒ 原文「SHA256SUMS 已随发布流水线接线」与当前 claims 状态存在出入，**以 [claims.yaml](../../status/claims.yaml) 为准**。
- **未核实**：8 个 commit 的具体内容本批未逐个 diff 复核（仅按仓内现状与 claims 状态推断）。

---

## 6. 模型出网闸（Egress Gate）接线设计（2026-10-23，**设计未实施**）

**来源**：`docs/04-internal/backend-enhancement/egress-gate-wiring-design-2026-10-23.md` · **上位结论**：先给模型出网装闸 → 才能做「能力面开、通道面关」（ADR-0013 §2.2 + 顺序约束）。

### 结论（原文要点保留）

- **问题：治理管手脚，不管嘴**（三个已核实事实）：① `crates/engine/runtime/src/canonical/provider.rs` 对 `guard|governance|policy|approval|consent` 的引用数 = **0**；② `execute.rs` 把 `NormalizedRequest` 装配完**直接发出**；③ 治理链六阶段**没有任何一段检查「这段内容能不能出境」**。⇒ 工具调用受治理，**送给模型的内容不受治理**，而 ADR-0013 的红线恰好落在这条无人看管的通道上。出境脱敏闸本体（`redaction_bridge.rs`：HMAC 占位、同会话恒同、跨会话换别名、回程可还原、历史回扫、零真值账本）**已写得很完整，缺的是挂在模型出口上**。
- **设计：一个端口 + 一个卡点 + 三档语义**：端口 `trait EgressGate { fn vet(&self, request) -> EgressVerdict }` + `enum EgressVerdict { Pass, Redacted(NormalizedRequest), Deny { reason } }` 放 runtime；实现 `RedactionEgressGate` 放 assembly（bridge + 策略 + 账本）；CLI 装配根注入。卡点 = `ProviderRouter::complete_inner`（所有 provider 调用汇于此，覆盖全部模型出口，不必在十几个 adapter 各挂一遍）。语义：**Pass**（原样发出）/ **Redacted**（发占位 + 回程还原）/ **Deny**（拒绝即帧，不发请求，返回可读原因）。**fail-closed**：闸未接线 / 判定失败 / 策略读取失败 ⇒ 一律 Deny。
- **出域账本**：append-only，坏行读作不存在（与既有审计账本同纪律）；**不回真值**（占位与对象类型即可），账本本身不得成为泄漏面；与既有零真值账本对齐，不另造一套。
- **五条边界**（外部顾问判据）：①原始记忆→派生摘要（摘要/折入路径）；②私人对象→工作对象（对象级判定）；③用户自己→第三方（出网控制）；④内部推断→外部行动（工具执行审批）；⑤**模型上下文→provider**（本设计，当前零挂载）。本闸先覆盖第 5 条，其余按 ADR-0014 落地清单补齐。
- **失败语义与可观测性**：拒绝原因必须可读（示例文案）；拒绝要进自述面与审计链、不静默；闸的健康状态进自述面（已接线 / 判定次数 / 最近拒绝原因类型），沿用既有 probe 模式。
- **实施顺序（每步可独立合并、可回滚）**：①端口 + 单测（默认实现 = `Pass`，零行为变化）→ ②实现 + 装配（env `APEIRETH_EGRESS_GATE`，默认关，默认关时逐字节零变化）→ ③判定策略（对象级 + 用户授权面，按对象 × 用途、可撤销）→ ④账本 + 自述面 → ⑤默认开评估（**不抢跑**）。
- **测试计划**：端口三分支各一例；卡点层拒绝时**不发请求**（mock provider 零调用）；脱敏层占位出境 + 回程还原一致 + 跨会话占位不同；fail-closed（闸 panic / 策略读失败 → Deny）；账本坏行读作不存在 + 账本不含真值（grep 断言）。
- **不做**：不承诺 provider 侧行为（「模型侧残留已抹除」不写进能力声明）；不做「一律脱敏」；不改 provider adapter（只动咽喉一处）；不默认开。
- **与其他待办的关系**：与设备审批送达面无关，可并行；**顺序约束**：本设计第 1–2 步落地前**不得**把默认工具面开大；claims 登记用阶梯口径，`default` 单独标（默认关 = `"OFF"`）。

### 现状复核（2026-10-23）

- **仍成立（核心事实）**：模型出口仍无治理引用 —— `crates/engine/runtime/src/canonical/provider.rs` 对治理相关标识的引用计数本批复核仍为 **0**；「治理管手脚不管嘴」**未被推翻**。
- **仍成立（设计纪律）**：fail-closed、拒绝即帧、账本不回真值、默认关起步、只动咽喉一处、不承诺 provider 侧行为 —— 六条纪律可直接作为实施验收判据。
- **部分已过时**：[claims.yaml](../../status/claims.yaml) 已有 `sec.egress_gate` 且为 `PRODUCTION WIRED`，但那是**工具出网**面；**模型出网**面在本设计落地前仍无对应 claim（设计为「待实施」，尚未登记 claim）。
- **未核实**：设计稿提到的 ADR-0013 §2.2 / ADR-0014 §1.2 与 `redaction_bridge.rs` 的具体行号本批未逐条核对。

---

## 7. 凭证管理规范（R215 防御层，2026-08-21）

**来源**：`docs/04-internal/secret-management-policy.md`

### 结论（原文要点保留）

- **背景（R215 教训）**：2026-08-21 在任务**收尾阶段**才发现 3 个 `reports/` 文件里写了完整真 key（95 字符），且 token 在对话上下文与多个子代理 prompt 中明文传递；修复 = 历史重写 + 物理删 blob + `.gitignore` 加固；**仍需主人 revoke + rotate**。核心失误：把「0 装 PASS 哲学」错误应用为「测试 = 真验」→「真验 = 把真 key 写进 report」；一开始没做全仓 secret 扫描。
- **哲学锚穿透**：S-2（「真接 ≠ 真接 = 把真凭证入库」）、O-1（凭证管理是安全第一线）、O-2（泄漏扫描有公开 pattern 表与 config schema 可参考）、O-5（**默认假设仓库已经泄露**，直到扫描证明清白）、O-4（凭证存放路径 + ignore + hook + CI gate 必须可被新成员一目了然）。
- **存放规范**：主人本地凭证不入库（唯一合法存放点在用户主目录下的 `apikey-ultra.txt` 等；另一份 token 文件因不在 workspace，`.gitignore` 无需管）；子代理读取后**立刻清环境变量**；**绝不**把凭证写入任何文件 / 报告 / commit message / 文档；子代理 prompt 必含凭证边界段（不读、不打印、假设任何入库内容默认公开、发现疑似泄露立刻停止并上报）。
- **四层防御**：①预 commit hook（扫暂存区，高置信命中即 block）；②CI gate（scan-all + scan-history，防御 force-push 偷渡，失败 red 阻断合入）；③平台级 Secret Scanning（需主人在仓库设置开启）；④`.gitleaks.toml` 配置（本地 allowlist 路径 + 20+ 高置信规则）。
- **报告 / 测试 / commit / 文档规范**：真 key 一律 REDACTED 形态占位；测试用**明显假的**占位串（注意长度边界与误报，需在 allowlist 登记）；commit message 绝不放 key 引用；已泄露的 commit message 用 `git filter-repo --replace-message` 修。
- **历史修复流程（7 步）**：立即 revoke + rotate（<5 分钟）→ 备份 `.git` → 写 `filter-rules.txt`（该文件**不能写真 key**）→ 历史重写工具重写全部分支与标签 → reflog expire + gc 物理删除 → 验证 0 命中 → 复跑测试与 clippy → 清理备份与规则文件。
- **backlog 状态（原文）**：装扫描器 binary = ❌ 阻塞（release-assets 网络）；平台 Secret Scanning = ❌ 需主人设置；预 commit hook 安装脚本 = ✅ 已写；CI workflow 步骤 = ✅ 已加；本文 = ✅；PowerShell 扫描器 = ✅；`.gitleaks.toml` = ✅；子代理 prompt 模板 = ❌ 后续批；季度扫描 = ❌ 后续批。
- **0 装 PASS 标注**：不装扫描器 binary；用 PowerShell 扫描器兜底；不集成平台级扫描（需主人开）；`.gitleaks.toml` 已写好待用；`.gitignore` 已加固；不在 main loop 实时跑扫描器（CI gate 已够）。
- **Lessons Learned**：做对 = 最后阶段全仓 grep 发现泄露 + 历史与 blob 完整清理 + REDACTED 占位可追溯；做错 = 没在开局就全仓扫描、没设 hook、没加 CI gate、没写规范、没先试兜底方案。

### 现状复核（2026-10-23）

- **仍成立（纪律面）**：REDACTED 形态、子代理凭证边界段、CI 双模式扫描、`.gitignore` 加固、「默认假设已泄露」的扫描姿态，全部仍然有效。
- **部分已过时（工具面）**：原文以「PowerShell 扫描器为 fallback、扫描器 binary 未装」为口径；今日仓内已有 `.gitleaks.toml` 与 `scripts/secret-scan.ps1` 并在门禁中被调用，但**「扫描器 binary 是否已装、平台级 Secret Scanning 是否已开」本批未核实**（需主人侧确认）。
- **仍成立（backlog 中未销项）**：子代理 prompt 凭证边界模板与季度扫描 —— 未核实是否已落地。
- **未核实**：`.gitignore` 中凭证相关行是否仍齐全（见 03 §2 的现状复核）。

---

## 8. S1 最小权限执行自审（v1.0 时代，日期未标注）

**来源**：`reports/s1-sandbox-hardening-self-audit.md`

### 结论（原文要点保留）

- **交付物（1637 行 + 47 行）**：受限 token 模块（521 行，Win32：CreateRestrictedToken + TokenIntegrityLevel + DACL）；目录 ACL 模块（419 行，工具沙箱根目录 read-only DACL + 守卫 Drop 自动还原）；AppContainer 档（117 行，**trait 口，0 装 PASS**）；沙箱配置扩展（+350 行，S1 字段 + `prepare_child` 总入口）；集成测试 204 行（13 例）；模块声明与字面量补全等。
- **三件套分层**：①受限 token（跨平台为 no-op，needs_hardening=true 时返 Err）；②只读 DACL；③AppContainer 后端（**仅 trait 口，0 装 PASS**）。
- **SandboxConfig 扩展 4 字段**（向后兼容旧 JSON）：`integrity_level`（Untrusted/Low/Medium）、`deny_only_sids`（BuiltinAdministrators/World/AuthenticatedUser/Interactive）、`directory_acl_roots`（与环境变量协作）、`use_app_container`（高危档，默认 false）；新增 `has_privilege_hardening()`（与 `has_limits()` 解耦：资源 vs 权限）。
- **整合入口**：`prepare_child(cfg) -> PreparedChild{ token, dir_acl }`，持有至 worker 退出，Drop 自动还原 ACL + 关闭 token 句柄；**待续 wire-up**：真接到 worker spawn 需 `CreateProcessAsUserW`（与标准库 `Command::spawn` 不兼容）⇒ 属后续 wire-up；作业对象仍是主防线，S1 为纵深加固。
- **测试**：lib 单测分模块统计 + 集成 13 例全绿（JSON 解析兼容 / 枚举往返 / **0 装 PASS 测试**：全部后端 `available=false` + 状态标「未接」/ 跨平台 Err 断言 / 模板断言）+ **4 个 Windows 真接测试**。
- **0 装 PASS 诚实标注**：三类后端未接（`available=false` + 「未接」标注）；受限 token 与目录 ACL 为 Windows 真接；跨平台 `prepare_child` 在 needs_hardening=true 时**如实返 Err**。
- **已知约束 / 未实现 4 项**：token 未接 worker spawn（需 `CreateProcessAsUserW`）；AppContainer 档 0 装（需 profile 创建 + SID 派生 + Capability 清单）；跨平台 S1 0 装（Linux 路线 = `PR_SET_NO_NEW_PRIVS` + seccomp + namespaces）；目录 ACL 还原以「原 DACL 重建」为粒度（**非原子**，调用间窗口极小但非零；生产环境应走事务化设计）。

### 现状复核（2026-10-23）

- **已过时（代码位置）**：本审计对象是 v1.0 时代的 `apeireth-companion` crate（受限 token / 目录 ACL / AppContainer 三件套），该 crate 已在 `legacy/` 归档区，**不在现行工作区**。今日对应能力面是 `crates/capabilities/tools/src/process/` 的进程遏制实现（Windows 作业对象 + CREATE_SUSPENDED + 降权 + rlimit + `PR_SET_PDEATHSIG`，见 02 §1 `sec.linux_containment` 取证），二者**不是同一实现**，不可互相替代引用。
- **仍成立（三条工程判断）**：①纵深加固与主防线分工（沙箱主防线 + 权限纵深）；②「能力面 = trait 口 + `available=false` + 状态标『未接』」的 0 装表达法；③ACL 还原非原子的窗口须如实登记。
- **未核实（未实现项的诚实登记）**：受限 token 未接 worker spawn、AppContainer 档 0 装、跨平台权限硬化 0 装 —— 本批**未在 active crate 中找到**受限 token / 目录 ACL 模块，故按「未核实」登记而非断言。
- **未核实**：原文测试计数（51 全绿、4 Windows 真接）与具体行号属 v1 时代事实，本批未复跑。

---

## 9. v2.0.0-rc.1 后端代码安全审计（2026-09-24）

**来源**：`reports/v2-rc1-backend-security-audit-2026-09-24.md` · **范围**：`crates/**` 全部 crate 的 src（约 18 万行 Rust）；未触碰前端 / 研究区 / 归档区 / 构建产物 · **方法**：高危模式全仓扫描（unsafe / Command / SQL 拼接 / unwrap / panic / 密钥字面量）→ 安全关键文件逐行精读 → 并行分组深审 → 每条 Critical/High 指控回读原始代码取证；**全程只读** · **严重度**：Critical = RCE / L0 HA 绕过；High = 权限提升 / 自停守绕过 / 密钥泄露 / 静默数据破坏；Medium = 信息泄露 / DoS；Low = 最佳实践违反。

### 结论（原文要点保留）

- **执行摘要**：**未发现 Critical 级远程代码执行**。核心风险四类：①「假安全」形式的诚信型缺陷（直击项目自身「0 装」红线）；②本地网关零认证 + 宽松 CORS；③凭据全链路泄露窗口；④未接线 API 缺少纵深防御（接线之日即成高危）。另注：生产代码 unwrap/panic 密度实际很低（330 处，多为互斥 poison expect；表面统计的 2827 个 unwrap 中 3328 个位于测试模块，是统计假象）。
- **Critical 1 项 —— C1 `p2p_mesh.rs` 宣称 Noise_XX / 洋葱加密，实为 hex 明文 + 长度假校验**：`encrypted_payload_hex` 与加密无关（hex 解码即明文）；`checksum_sha256` 只含负载长度（不同内容的同长度负载校验值完全相同）；`packet_id` 可预测、可枚举、同长度即碰撞；`process_roaming_delta` 的「Merkle root 校验」只是 `len() >= 8`；模块**无任何「这是占位」的诚实声明**（与同仓另一模块的诚实声明形成刺眼对照）。现状：无生产调用方，属「接线即泄密」的潜伏 Critical。建议二选一：接真加密，**或**立刻改名 + 删宣称 + 加显式占位声明（不允许维持现状）。
- **High 13 项（逐条保留 + 本批复核状态）**：

| # | 问题 | 位置要点 | 原文建议 | 2026-10-23 复核 |
|---|---|---|---|---|
| H1 | 网关零认证 + 宽松 CORS：任意网页可驱动 agent、**代替人类批准**挂起工具调用、热改 provider 基址致密钥外泄、读全部会话与记忆 | 主路由 + 面板路由；宽松 CORS 层 | 加回环 token / Origin 白名单；`approvals/resolve` 与 `admin/config` 至少要本地口令；`--bind 0.0.0.0` 时强制认证或拒绝启动 | **已修**：现有网关令牌门 + 令牌族 + 本地来源白名单 + 「不设防」显式声明与审计 |
| H2 | `gate.rs` 的 V2/V3 权限门是**恒真空操作**（`x \|\| true`），「三者 AND」退化为只有 V1；固化测试替空操作背书 | 权限门 + 集成测试 | 删两处恒真；`check_ha` 在线分支改 fail-closed；修正固化测试 | **已修**：代码注释明写「H2 修复（2026-09-24 审计）」，V2 解析风险→洋葱层、V3 要求已登记真实人类 |
| H3 | guard 在 intent 缺失时整体 fail-open（低分 benign），高危效果被静默放行（write/delete → 允许）；审批恢复路径传空上下文放大 | 意图对齐层 + hook + 恢复路径 | intent 缺失对写/删/执行/网络发送一律 RequireApproval；冻结轮次上下文持久化并回传 | **已修**：写/删/执行/网络发送/凭据类升为高风险不匹配（0.75）→ RequireApproval，纯读维持 0.45 |
| H4 | 补丁应用器无任何根目录包含校验 —— 绝对路径 / `..` 任意文件写与删 | 三处 `root_dir.join(path)`，无 canonicalize / starts_with | 逐 action 校验（规范根 + canonicalize + `starts_with(root)`）；临时名改 `create_new` | **已修**：现实现已分别 canonicalize root 与目标并要求 `starts_with(root)` |
| H5 | 凭据加密后端 `set`/`delete` 的 `load_all().unwrap_or_default()` 在数据损坏时**静默覆盖全部凭据** | 加密文件后端 | 区分「文件不存在」与「存在但打不开」；加锁 | **已修**：该文件已无 `unwrap_or_default` 写法 |
| H6 | WebSocket 协议层 Auth 帧 token **从不校验**，任何 token 均可开会话；帧字段无长度/字符集上限 | 会话准入 + 帧定义 | 准入增加期望 token 注入点（恒定时间比较）；帧字段加上限 | **仍成立**：`admit_auth` 至今只比对协议版本，未见期望 token 比较（属**潜伏项**，传输面未接线） |
| H7 | IM webhook 校验失败错误消息**泄露期望 token**；非常量时间比较；4 步校验只实现 1 步；未解密事件体直接放行 | IM 适配模块 | 错误只含 mismatch 不含期望值；恒定时间比较；stub 期显式 NotImplemented | **已修**：现实现只报 `mismatch`，且为恒定时间比较 |
| H8 | 元问题禁令两套强弱不一致的实现，公开 trait 默认方法是弱版（可绕过） | 哲学守门 trait 默认 + const fn | 删默认实现或委托 const fn，保持单一真源；补等价负向测试 | **已修**：默认实现已直接委托 const fn，100+ 负向测试覆盖两条路径 |
| H9 | 嵌入模块的 API key 以明文 `String` 存长生命周期结构体且 `derive(Debug)`，违反同 crate 自订不变量 | 嵌入 provider | 改持 `Option<Secret>` 或手写脱敏 Debug；补对称防泄露测试 | **已修**：现持 `Option<Secret>`，注释明写「Secret handling (H9)」 |
| H10 | 抓取工具的 HTML 无障碍树提取：畸形页面触发 panic + 快照递归无界栈溢出（进程级 DoS） | 抓取工具 | 空栈改条件处理；渲染改迭代或加深度上限；后处理放阻塞线程池 | **已修**：现附回归测试「闭标签多于开标签不 panic」「深层嵌套截断而非 abort」 |
| H11 | provider 流式解析缓冲无大小上限、无空闲超时 → 端点驱动的内存耗尽 | 流式解析 | buffer 超阈值即返回错误；可选加空闲超时 | **已修**：现设 1 MiB 缓冲上限并拒绝继续缓冲 |
| H12 | 加密文件后端无写锁、无 fsync → 并发写损坏、崩溃静默丢写 | 记忆加密后端 | 加锁包住 index + 开文件 + 写；写完 sync；多进程加文件锁 | **已修**：现持写锁 + `sync_data()` |
| H13 | 两条流写入路径对同一批表语义分裂：**跨 session 数据泄漏** + tombstone 失效 + 反向不可见 | 记忆存储双路径 | 统一行布局；或表加前缀；空值兜底收紧 | **已修**：写路径现同时写 `session_id` / `tombstoned_at` 列 |

- **Medium 28 项 / Low 约 30 项（分组概要，保留要点）**：**凭据与密钥** = 文件凭据店「先写后 chmod」窗口 + 无锁 load-modify-save + 非原子写；master key 与密文同目录；加密后端读放大与重复 id 语义（更新被静默忽略、返回首条匹配）；审计日志存 API key **明文前缀**（字段文档自称哈希，实现只截前 16 字符）；秘密容器系统性 `derive(Debug/Serialize)`；跨主机重定向转发自定义凭据头。**存储与持久化** = 机器标识占位 UUID 被当真（含跨机碰撞与密钥盐耦合）；连接池写线程 panic 即永久报废；会话存储 `list()` 无属主过滤 + last-writer-wins；迁移跨进程并发硬失败（应 `BEGIN IMMEDIATE`）；去重 LRU 回滚产生幽灵条目；持久化向量索引全量物化 + 静默丢行；未设 busy_timeout；内存与存储两套 schema 所有权碰撞。**网络与工具边界** = 出网 allowlist 模式不校验解析后 IP + IPv6 内嵌盲区；**护栏前置守门零生产调用**（shell 执行前无高危命令内容过滤）；响应缓存无上限；沙箱 spawn 泄漏 stdin 句柄；SDK 沙箱白名单三缺陷（前缀匹配 / 特权端口未实施 / 禁 root 仅字面量）。**HTTP 与管理配置** = 基址校验允许明文 http；密钥掩码按字节切片可 panic；自停审计无截断 + 查询无上限；限流按能力而非按会话 + poison panic；runtime 工程债（单轮工具调用无数量上限、子循环缺 action_id、畸形参数 JSON 被静默吞为 null、会话锁 map 只增不减、审批记忆按（会话,能力）粒度）。**Low 组（示例）** = 帧字段无上限 / 无 `deny_unknown_fields`；流桥缓冲无界；对等表无老化；硬编码口令且非常量时间；状态机可静默卡死；全角门误判中文标点；单人多签分支不校验签名者身份；文件系统检查后使用竞态；进程限制在 debug 构建 panic / release 回绕；子进程 kill 后不 wait；AppContainer ACE 永驻不回收；只读判定用子串匹配导致规则永不触发；锁 poison 级联 panic；「证据可自证」的证据守卫不可作可信来源；SDK C ABI 所有权契约不一致；每次调用新建 HTTP client；外部标签破坏跨语言契约；NaN 通过范围校验；429 硬编码 60s 忽略服务端 Retry-After。
- **明确未发现问题的方面（结案清单 16 项，保留）**：进程执行边界时序正确（挂起→入作业对象→恢复、杀树有实测、无逃逸路径、引号规则与系统解析一致、能力报告 fail-closed）；出网主路径设计正确（解析后校验 + 钉扎 + 逐跳重验 + 降级拒绝 + userinfo 拒绝 + 不走环境代理 + 响应体上限 + 总超时）；文件系统包含检查顺序正确；guard 决策只升级不降级、错误 fail-closed；凭据链路主干（脱敏、空值视为缺失、无 catch-all、Debug 不泄密）；agent loop 终止性（回环预算 + 审批恢复不重置 + side-call 预算 CAS）；能力注册（重复拒、歧义 fail-closed、ID 双路径校验）；**SQL 全部参数化**（所有拼串点逐一核验为编译期常量，无注入）；迁移事务（storage 版 IMMEDIATE + ROLLBACK + 幂等）；append-only 完整性（6 张流表 + episodes + 5 张审计表触发器）；TLS/网络（无跳过证书校验、无降级、跨主机重定向删 Authorization）；无 ReDoS；FFI 绑定（空指针检查、非 UTF-8 fail-soft、CString 配对、无重入）；生产 unwrap 密度低；provider 路由（永久失败不级联 + 瞬态按序 fallback + 时钟回退兜底）；夜间巡检落盘**不含 episode 正文**。
- **修复优先级路线图**：P0（接线前必须修，约 1.5 人日）= C1 正名或接真加密、补丁根校验、WS token 校验、SDK 沙箱白名单三缺陷；P1（一周内）= 网关认证/Origin 白名单 + 端点保护 + bind 告警、凭据损坏/不存在区分 + 文件锁、IM 错误去秘 + 恒定时间比较、intent 缺失 fail-closed + 删恒真门、嵌入 Secret 化 / 抓取 panic / 流式缓冲上限、加密后端写锁 + fsync、流表双路径统一；P2（一个迭代内）= 凭据原子写与 chmod 顺序、master key 独立封存、禁重定向、机器标识占位过滤与缓存、写线程 catch_unwind、锁范围 + LRU 驱逐、allowlist IP 校验、基址 https-only + 掩码防 panic、迁移 IMMEDIATE、busy_timeout、审计真哈希、秘密容器 Debug 脱敏、护栏接线或明确职责归属文档；P3 = 其余 Medium/Low + 命名强于实现的项正名 + 两个覆盖不足区域的补审。
- **修复轮终态（2026-09-24 追加）**：按 crate 分区并行修复，全部未触碰前端、未 commit；全量测试 exit 0 / 0 failed、`cargo check` 通过、clippy `-D warnings` exit 0；新增回归测试约 90 个（并发写防护、路径逃逸、敌意源、超长响应截断、字符边界、审批恢复上下文保真、配额回收、SSML 转义等）；变更规模 114 个 crate 源/测试文件，+7928/-1041 行。追加轮：把生产代码中约 62 处锁 poison expect 统一为 `unwrap_or_else(|poisoned| poisoned.into_inner())`。**关键设计决策备查**：H2 的三门分工重写；H1 的本地来源白名单 + 可选令牌门（未设置时放过，浏览器向量由 CORS 白名单收敛）；C1 的字段与函数正名（`MeshPacket` / `payload_hex` / `payload_len_hex` / `sender_public_key_hex` + `TODO(v2.1)` + peer 表上限 256）；H12/H13 的写锁与行布局统一；H3 的不匹配升级。**已知测试环境备注**：确定性沙箱 profile 名跨运行状态污染导致间歇性失败（隔离运行 3 连跑均绿；长期方案 = 按 run 随机 profile 名，需连带 ACE 回收设计）。

### 现状复核（2026-10-23）

- **已修（本批复核确认）**：C1（函数已正名为 `wrap_mesh_packet`，原导出名已不存在）、H1、H2、H3、H4、H5、H7、H8、H9、H10、H11、H12、H13 —— **13/14 条**。
- **仍成立**：**H6（WS 握手 token 从不校验）** —— `admit_auth` 至今只比对协议版本；属「接线即高危」的潜伏项，与 02 §1 中 `sec.terminal_permission_surfaces` 的文档面结论同源待办。
- **仍成立（结案清单）**：16 项「明确未发现问题」的结案判定在本批复核中抽样确认仍成立（SQL 全部参数化、出网主路径设计、凭据 Debug 脱敏、agent loop 终止性等）。
- **部分已过时（覆盖范围与规模）**：审计针对 rc.1 时的 17/18 crate；今日工作区口径见 [AGENTS.md](../../../AGENTS.md) §3（30 members）；规模与 crate 数按历史口径读。
- **仍成立（修复轮纪律）**：「未触碰前端 / 未 commit / 全量 0 failed / clippy 0 / 新增回归测试钉住」的验收形态是本仓既有交付纪律。
- **未核实**：Medium 28 项与 Low 约 30 项的逐条现状本批**未逐条复核**（仅抽样若干，见上表标注）。

---

## 附：本册与其它册的交叉点

能力声明真源 [claims.yaml](../../status/claims.yaml)（02 §1 的 12 条逐条 `level` 与 `verified_at`）；架构契约真源 [contract.toml](../../architecture/contract.toml)（治理波次 R8 与依赖边棘轮）；架构与能力侧结论见 [01-architecture-and-capability-audit.md](01-architecture-and-capability-audit.md)；口径与文本可信度侧结论见 [03-caliber-and-text-hygiene-audit.md](03-caliber-and-text-hygiene-audit.md)。
