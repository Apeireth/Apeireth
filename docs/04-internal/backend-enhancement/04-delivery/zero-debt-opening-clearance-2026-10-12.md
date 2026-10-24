# 零债开场清偿批（2026-10-12）— 清偿对照表 + 新债登记

> `Status: ✅ 收官（2026-10-12，W1 开工前"零债开场白"，纪律见 [CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md)）`
> 范围：纯清债（登记债逐件清），产品语义零变化、驾驶舱/前端观感零变化。
> 顺手重构超范围念头一律登记不实施（见 §5）。本文件即 §一.2 "见债必还" 的落账面。

## 1. 清偿对照表（债项 / 处置 / 证据）

| # | 债项（登记出处） | 处置 | 证据 |
|---|---|---|---|
| 1 | **src-tauri 55 条 clippy 告警**（`std::fs→fs_err`、MSRV 等，散布 backend_supervisor / workspace / keychain / supervisor_lifecycle；"历史上非绿门的口径补绿"） | ✅ **全清零**。① `std::fs::`/`std::path::Path::` 文件调用全部迁 `fs_err`（`backend_supervisor.rs`、`workspace.rs`、`tests/supervisor_lifecycle.rs`，`use fs_err as fs` 引入 + `Path::canonicalize`→`fs_err::canonicalize`）；② redundant closure ×2 → 直接函数引用；③ `var(..).ok()` 匹配 `Some` → 匹配 `Ok`；④ MSRV ×2：`Option::is_none_or`（stable 1.82）→ `map_or(true, ..)`——**不改 `rust-version = 1.77.2` 声明**（代码适配声明，保 MSRV 口径诚实，不改兼容面）；⑤ `CredWriteW` needless `&mut` → `&`（`keychain.rs`）；⑥ cargo fmt 归位 | `cd frontend/companion-desktop/src-tauri && cargo clippy --all-targets --locked -- -D warnings` exit 0（0 警告）；`cargo fmt --all -- --check` exit 0；`cargo test --locked -j 2` 全绿（58 单测 + 8 supervisor_lifecycle 真后端全过） |
| 2 | **ratatui 0.29→0.30 迁移欠账**（ROADMAP §4 P11 登记：消 paste(RUSTSEC-2024-0436)/lru 0.12(RUSTSEC-2026-0002/0253) 三条 transitive 面，三处同步 ignore 短接） | ✅ **根治落地**。`crates/adapters/tui` 升 ratatui 0.30.2 + crossterm 0.29（ratatui-crossterm 同版，全树 crossterm 单版本）；API 迁移面实测为 0.30 兼容子集，**0 处编译修**；依赖树复核：paste **出树**、lru 单一 0.18.5（修复线）、crossterm 单版本；三处同步 ignore 短接（deny.toml / audit.toml / cargo-audit.yml `--ignore`）随之清空，回归 0-ignore austere 模式；deny.toml bans 段 `lru` duplicate skip 随 dup 消失退役；ROADMAP P11 行标 ✅ done | `cargo test -p apeireth-tui -j 2`：**47 passed / 0 failed**（TestBackend 帧钉 = 观感零变化证据）；`cargo tree -i paste` 无此包；`cargo tree -i lru` → 仅 0.18.5；`cargo tree -i crossterm` → 仅 0.29.0；`cargo tree -d` 无 lru/paste/crossterm/ratatui dup |
| 3a | **`self_tuning_wire` 共享全局槽竞态**（已串行化部分，复核彻底性） | ✅ **复核完成 + 最后一处结构性洞闭合**。全局槽 = `orchestration::self_tuning::OVERRIDES`（进程级）；逐测试二进制复核（详见 §3），唯一真洞 = `tests/self_status_runtime_e2e.rs` 同二进制 3 用例裸写槽 + 精确值断言并行跑 → 补 `TUNING_SLOT_GUARD` 串行化（与库内 `TUNING_GLOBAL_GUARD` 同一隔离原则）；其余各处复核结论：无竞态或值不敏感，逐处留证 | §3 复核矩阵；`cargo test -p apeireth-runtime-assembly` 全绿 |
| 3b | **`file_lock_serializes_concurrent_writers` 负载 flake**（五案在册） | ✅ **治根**（锁实现确定性，非测试侧减敏）。定根：标记读取**瞬时失败**被推断为「持有者已死」→ `(None,None)` 误删**活跃锁** → 两写者同时进临界区 → 丢更新（Windows 负载下新建文件被安全软件短暂独占扫描即触发）。修复：`storage_atomic` 新增 `MarkerState` 四态裁决——接管**只认确证**（`Parsed(死PID)` 两次同值 / `Corrupt` 两次），`Unreadable`/`Absent` 一律保守等待重试，绝不推断陈旧；认领清理同规则。契约不变（互斥/死锁接管/超时 TimedOut/panic 释放全保留），新增 2 条对抗回归测试钉住新规则 | `cargo test -p apeireth-core --lib storage_atomic`：**10 passed / 0 failed**（含 `file_lock_never_takes_over_unreadable_marker` / `file_lock_takes_over_corrupt_marker_record` 两条新钉 + 原并发写压测）；语义说明见 `storage_atomic.rs` [`with_file_lock`] 语义段 |
| 4 | **grep 全库登记债字样**（欠账/挂账/TODO 债/技术债，排除档案/研究报告） | ✅ **逐条裁决**（§4 清单）。本批清 3 条（B-performance 锁债注记、absorption-ledger 余量备忘 ×2 核销），其余全部为「已带处置理由」的留账登记（人工/硬件/key/产品拍板类）或非登记债字样（会计隐喻/UI 文案），逐条记录不重写历史 | §4 逐条表；`scripts/check-neutral-terms.ps1` 零命中 |
| 5 | **孤儿/死面**（untracked 杂物、无用导出/无人调用的公开面） | ✅ **清确证者**：① `artifacts/launch-cockpit.ps1` 全库 0 引用 + 硬编码本机绝对路径 → **删除**；② `SelfTuningWire::observe`（自注"未接信号的接口预留入口"）与 `SelfTuningWire::history` 全库 0 调用 → **删除**（`TuningEvent` import 随清）；③ TUI crate `MotionMode::is_full`、`AppState::is_streaming` 全库 0 调用 → **删除**。保守面：`DEFAULT_LOCK_TIMEOUT`/`pid_alive` 等有内部实现/测试引用的公共契约面不动 | 逐名全库 grep 证据（word-boundary，含 legacy/reports 交叉排除误报）；删除后 clippy/fmt/47 测全绿 |

## 2. 门禁（宪章第四节）全过

| 门禁 | 命令 | 结果 |
|---|---|---|
| 工作区测试 | `cargo test --workspace --locked -j 2` | ✅ 全绿（exit 0）；驾驶舱 47 测另于收尾态复跑 47/47（覆盖死面删除后的最终态） |
| clippy root | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告 |
| clippy src-tauri | `cargo clippy --all-targets --locked -- -D warnings`（src-tauri 独立 workspace） | ✅ 0 警告（55 → 0） |
| fmt | `cargo fmt --all -- --check`（root + src-tauri 双跑） | ✅ 干净 |
| 前端 | `pnpm check` / `pnpm test`（frontend/companion-desktop） | ✅ 0 errors 0 warnings / 50 of 50 suites |
| 中性表述 | `pwsh -NoProfile -File scripts/check-neutral-terms.ps1` | ✅ 零命中（OK, 38 changed files） |
| secret-scan | `pwsh -NoProfile -File scripts/secret-scan.ps1` | ✅ No secrets found |

## 3. `self_tuning_wire` 全局槽竞态——彻底性复核矩阵

全局槽写者 = `install_effective_values`（`SelfTuningWire::new` 构造即写 + 每条记录 `persist_and_apply`）与 `clear_effective_overrides`。逐测试二进制（同进程才有竞态面）：

| 测试二进制 | 槽写者 | 槽派生值断言 | 复核结论 |
|---|---|---|---|
| `apeireth-runtime-assembly`（lib） | 仅 `self_tuning_wire::tests` 一对（装调整值/清槽） | 同对用例 | ✅ 已由 `TUNING_GLOBAL_GUARD` 串行（既有关锁，复核无漏） |
| `self_status_runtime_e2e`（integration） | 3 用例裸写（wire 构造×3 + 显式 install×1） | `tuning["values"]` 精确值、preset | ⚠️→✅ **本批闭合**：原靠"写者都写基线"的值巧合免疫（wire 构造即写全局槽，加一个信号即破），现补 `TUNING_SLOT_GUARD` 串行 3 用例 |
| `capability_switch_chains`（integration） | 1 用例内 2 个 wire 构造 | 无（全部断言名册布尔） | ✅ 无竞态面（唯一写者用例内自洽） |
| `apeireth-orchestration`（lib） | 仅 `effective_prefers_override_then_env_baseline` 一例 | 同例 | ✅ 单用例独占，无并行面 |
| `apeireth-organ`（lib） | 仅 `clear_effective_overrides`（tone 旋钮用例），全二进制**无 install** | `apply_emotion` 系断言读 `effective()` | ✅ 无竞态（槽恒空，clear 为幂等空操作）；宿主 env 污染型敏感面另列 §5 N6 |
| `apeireth-tools`（lib） | 无（`self_status` 测试用注入值，纯函数） | 注入值 | ✅ 不触槽 |

## 4. grep 全库登记债字样——逐条裁决

检索词：欠账 / 挂账 / TODO 债 / 技术债。排除：`docs/archive/`、`legacy/`、`reports/`、`research/`、`artifacts/`（档案/研究报告/审计转储）、CHANGELOG（历史发版记录，纪律"不写 CHANGELOG"）。

| 处置 | 命中 | 理由 |
|---|---|---|
| ✅ 本批清 | `docs/04-internal/backend-enhancement/B-performance.md` 锁竞争注记 | file_lock flake 已定根，注记更新为实况 |
| ✅ 本批核销（留痕） | `docs/04-internal/absorption-ledger.md` 余量备忘 ×2（reflexion 换统一文件锁 / stored_doc 消费方接线） | 两项均已落地（reflexion.rs 已挂 `with_file_lock` 三测在案；stored-doc-consumer-migration.md 6 文件族清单），旧行留痕标 ✅ |
| 📌 留账（理由在册） | `docs/04-internal/live-verification-ledger.md` §2 挂账族（真机点击流 #2 / live E2E 需有效 key / embedding 端点等） | 客观条件卡（需主人人工/凭证/端点），不可单方核销——台账即"未测真话集" |
| 📌 留账（理由在册） | `docs/04-internal/stage123-gap-matrix.md` 挂账行（物理多签推迟 v2.1、UpgradeCycle 三处留白、风险阈值未校准、Kani 真接线） | 产品能力缺口 + 拍板推迟/需实测校准，各行已带处置理由；非本批"纯清债"可清 |
| 📌 留账（理由在册） | `docs/design/frontend-handoff.md` §6 前端欠账 6 项、`docs/design/01-DESIGN-SYSTEM.md` 仍断面、`frontend/design-preview/peer-gap-and-frontend-plan.md` v0 诚实边界 | 前端产品/审美/契约预留工作；红线"驾驶舱/前端观感零变化"禁本批动手 |
| 📌 留账（理由在册） | `docs/02-guides/core-mechanisms-explained.md`、`frontend/companion-desktop/README.md`、`docs/first-run-click-through-checklist.md` 点击流欠账 | = 台账挂账 #2 的镜像指针（需主人人工），保留 |
| ✅ 无需动（非登记债） | `crates/foundation/credentials/src/hook.rs`「挂账」= 审批台账会计隐喻；`essence.css`「挂账说明文字」= UI 文案描述；`scripts/doc-caliber-baseline.txt` = 机器生成基线头；`capability-switch-power-audit.md`「挂账」行 = 已整改落账的审计留痕 | 术语/历史留痕，非工程登记债 |
| ✅ 无需动（带日期历史档案） | `team-work-doc.md`（v1 快照带 banner）、`HANDOFF-NOTES`/`handoff-*`/`engineering-*`/`claims-evidence-matrix`/`v1-vs-v2-capability-gap-audit`/`w3-legacy-gap-archaeology`/`pm-review`/`design-intent`/`backlog.md` P3 观察项/`stored-doc-consumer-migration.md` | 宪章 §三.2「带日期的历史档案不动」；各条均自带处置/理由 |

## 5. 新债登记（清不动的，写理由）+ 顺手重构登记（不实施）

| # | 登记项 | 理由 / 落点 |
|---|---|---|
| N1 | 台账 §2 挂账族（真机点击流 / live E2E 换 key 补跑 / embedding 端点验证） | 需主人人工、有效凭证、可用端点——客观条件在他人侧，保持挂账（`live-verification-ledger.md` §2 原行） |
| N2 | stage123-gap-matrix 挂账行（物理多签 v2.1 / UpgradeCycle 留白 / 阈值校准 / Kani 真接线） | 产品能力缺口 = 要么拍板推迟要么需阶段实测校准，超出"纯清债"红线（产品语义零变化）；各行理由在册 |
| N3 | frontend-handoff §6 前端欠账 6 项 + 设计系统仍断面 | 前端产品/审美工作，红线禁观感变化；本批不碰前端实现 |
| N4 | reflexion 文件并发测试偶发 flake（台账 #57 顺记） | **疑与 file_lock 负载 flake 同根**（同一 `with_file_lock`，病根=接管误判）。本批定根后理论上随之消失，但**未复现复跑定性前不宣称已消**——观察项，若再现按新病灶立案 |
| N5 | deny.toml `[bans]` 段 v1 时代遗留 duplicate skip（crossterm / ratatui / compact_str / strum 等条目理由文本可能与现状不符） | 非本批改动造成（lru 条已随本批迁移退役）；全量对账属 deps 治理批范围，登记待复核 |
| N6 | `TunableParam::effective()` 读宿主 env 的测试敏感面（如 organ `apply_emotion` 系测试硬编码饱和度推导常量，宿主若设 `APEIRETH_TUNE_TONE_SATURATION` 会红） | 非竞态（进程内无写者），属宿主环境污染类敏感面；测试侧显式装基线值的减敏改法是测试语义微调，登记待测试治理批统一做 |
| R1（登记不实施） | `self_tuning_wire.rs` 落盘用 `std::fs::OpenOptions` → fs_err | root workspace lints 默认关 clippy 全集（`[workspace.lints.clippy]` 显式 allow），不进门禁；clippy.toml 的 fs_err 政策在 root 为"声明待迁移"既有口径（clippy.toml 头注自述），改它是全仓 fs_err 迁移批（clippy.toml 自注 R18 T10 遗留），超本批范围 |
| R2（登记不实施） | reflexion `StoreBusy` 注释口径顺手扩写 / `DEFAULT_LOCK_TIMEOUT` 等契约常量收敛 | 注释口径已做最小对齐（跨文件最小改动）；其余属"顺手"级，不实施 |

## 6. 收尾备注

- 本批与并行施工线共树（在途改动：`kani.yml` / `river_topology.rs` / `harness_panic_freedom.rs` / `main.ts` 桌宠窗），全程叠加不覆盖、edit 前先读后改；`Cargo.lock` 为共享收敛点，以 `--locked` 门禁复验一致性。
- 负载 flake 定性口径（宪章 §四.1）：本批 `file_lock_serializes_concurrent_writers` 以单跑 + 整批复跑双绿收尾；病根已从"概率性误删活跃锁"降为"确证才接管"，负载敏感源消除。
