# IMPLEMENTED → PRODUCTION WIRED 抬级方案（12 条 claim 取证审计）

> 审计日期：2026-10-21 · 审计者：claim-audit（只读取证）
> 审计基线：`HEAD d2b8c403`（git rev-parse HEAD）；工作树快照时间 2026-10-21 16:19–16:26（有并发线未提交改动，见 §0.4）
> 审计对象：`docs/status/claims.yaml` 中 `level: IMPLEMENTED` 的 12 条 claim
> 取证方法：读 claims/wiring/ADR-0004 四级口径 → `git grep` 查生产装配路径真实调用/注册 → 检查 tests 证据 → 必要时单 crate 只读复跑
> 纪律：本次审计**只读代码与文档**；本文件是唯一写入物；未改 `docs/status/**`、`scripts/**`、任何源码；未 git add/commit/push
> 工具链（本地复跑）：rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30) / node v24.16.0 / pnpm 11.21.0 / PowerShell 7.6.6

## 0. 口径与取证前提

### 0.1 四级口径（docs/01-architecture/system-capabilities.md 头部）

`IMPLEMENTED`（代码存在）→ `PRODUCTION WIRED`（接入 canonical 运行时主路径）→ `DEFAULT ENABLED`（无需 opt-in 即开启）→ `HARDWARE VALIDATED`（真机验证）。四级不等价；**不得跨级宣称**（AGENTS.md §1 双向诚实）。

### 0.2 wiring.yaml 的 production_refs 是路径启发式，不是调用点证据（关键结论）

- 生成器 `scripts/wiring_report.py`：`PROD_PREFIXES = ("crates/", "frontend/companion-desktop/src-tauri/src/")`，`is_production_source(rel)` 仅按前缀 + 排除 `/tests/`、`/test/`、`.md/.ps1/.mjs/.toml` 判定；`wired = {s ∈ sources if is_production_source(s)} ∪ env refs`。
- 生产根 `PROD_ROOTS = (crates/engine/runtime-assembly/src/canonical/production.rs, crates/adapters/cli/src/lib.rs, crates/adapters/cli/src/care_heartbeat.rs, crates/engine/runtime-assembly/src/canonical.rs, crates/engine/runtime/src/canonical/canonical_entry.rs)`。
- ⇒ **IMPLEMENTED 条目的 production_refs 基本等于 sources 自身的路径命中**（例如 gov 三条的 production_refs=[chain.rs]）。它**不能证明任何真实调用/注册**；本审计因此全部另做 `git grep` 调用点取证。
- 交叉校验规则（ADR-0004 §7.4/§7.5）：`PRODUCTION WIRED` 必须有生产引用、`IMPLEMENTED` 必须有 gaps 原因，缺证据即 FAIL；**禁止无生产引用却标 PRODUCTION WIRED**。
- `wiring.yaml` 与 `claims.yaml` 同步：claims 文件 SHA-256 = `1912958e52a5b422da5a0c066cb07963d251147b2eb07f23717f8f215b7ef758`，与 `wiring.yaml` 的 `claims_sha256` 完全一致（2026-10-21）。
- 验证面安全网：`python scripts/docs_truth.py --verify`（2026-10-21）= `claims=24 fail=0 stale=6 review_due=0`；**本审计的 12 条全部 OK**；6 条 STALE 均为 bio.*（`bio.nociception_pain` 等），来自并发线未提交改动（`crates/adapters/cli/src/lib.rs`、`.../canonical/production.rs`），与本 12 条无关。

### 0.3 12 条 claim 的基线定位

- claims 条目行号：`sec.linux_containment`:136、`sec.storage_envelope_crypto`:170、`gov.chain_skeleton`:188、`gov.chain_hook_adapter`:204、`gov.reflection_queue`:219、`gov.chain_drill_ci`:234、`sec.governance_chain_drill`:249、`rel.release_asset_verify`:265、`sec.sql_readonly_gate`:281、`sec.provider_proxy_ssrf`:297、`sec.desktop_file_roots`:311、`sec.terminal_permission_surfaces`:325（`docs/status/claims.yaml`）。
- 12 条 `verified_at` 对 sources 的 STALE 检查（`git diff --name-only <verified_at>..HEAD -- <sources>`，2026-10-21）= **12/12 空**（bdd64957 / 68fa5643 / 7153f5db / 5f52813d 四组）。

### 0.4 审计基线漂移（必须与本次结论一起读）

本次审计期间工作树存在**并发线未提交改动**（`git status --short`，2026-10-21 16:19–16:26）：

```
 M crates/adapters/cli/src/lib.rs                  (mtime 16:19:38)
 M crates/capabilities/tools/src/egress.rs
 M crates/capabilities/tools/src/lib.rs
 M crates/capabilities/tools/src/process/appcontainer.rs
 M crates/capabilities/tools/tests/content_feed.rs
 M crates/capabilities/tools/tests/content_feed_lifecycle.rs
 M crates/capabilities/tools/tests/fetch_execution.rs
 M crates/engine/runtime-assembly/src/canonical.rs
 M crates/engine/runtime-assembly/src/canonical/bio_production_wiring.rs
 M crates/engine/runtime-assembly/src/canonical/production.rs
 M crates/foundation/core/src/storage_atomic.rs
```
以及未跟踪文件 `crates/engine/runtime-assembly/src/canonical/bio_extra_probes.rs`、`crates/adapters/cli/tests/governance_chain_equivalence.rs`（16:20:04）。

**其中 `crates/adapters/cli/src/lib.rs` 已出现 R8-B2 治理单一入口实装**（工作树现状 :1559–1571：`PermissionPresetGovernanceHook::new(...)` → `GovernanceChain::new(Arc::new(preset_governance))` → `ChainGovernanceHook::new(...)` → `builder.with_governance(Arc::new(chain_governance))`，注释「R8-B2 治理单一入口 (ADR-0010 D3): 装配根收敛为 GovernanceChain::run 单点」）；未跟踪的 `cli/tests/governance_chain_equivalence.rs` 含 4 个四态等价测试（`chain_entry_equivalent_for_allow/deny/require_approval/not_configured`）。
⇒ **本文件「gov.chain_* 生产零调用」的结论只对 HEAD d2b8c403 成立**；工作树已有在途实装（未 commit、未登记 claims/wiring）。任何抬级动作必须以「先收口在途线、再登记」为序，否则登记表会再次落后于代码。

## ① 结论摘要（12 条分类计数）

| 分类 | 计数 | 条目 |
|---|---|---|
| (a) 已被生产使用，仅缺证据登记 | **3** | `sec.linux_containment`、`gov.chain_drill_ci`、`sec.governance_chain_drill` |
| (b) 有默认关接线未登记开关／缺默认关等价测试 | **0** | ——（无一条处在「已接默认关开关但未登记」形态；`gov.reflection_queue` 连开关都未接，归 (c)） |
| (c) 纯库内/脚本面无生产调用，需接线设计 | **5** | `sec.storage_envelope_crypto`、`gov.chain_skeleton`、`gov.chain_hook_adapter`、`gov.reflection_queue`、`rel.release_asset_verify` |
| (d) 前端桌面面，按桌面装配根判定生产口径 | **4** | `sec.sql_readonly_gate`(B1)、`sec.provider_proxy_ssrf`(B2)、`sec.desktop_file_roots`(B3)、`sec.terminal_permission_surfaces`(B4) |

**总判**：12 条全部有真实代码/测试资产，**未发现臆造或 0 装冒充**（诚实纪律成立）；但与 PRODUCTION WIRED 的差距集中在三类结构性问题——

1. **生成器口径倒挂**：wiring 的 production_refs 是路径启发式，真正缺的是「调用点/注册点」证据；4 条 (c) 与 2 条 (a) 都受此影响。
2. **CI 执行面缺口**：`companion-desktop-ci.yml` 不跑 `cargo test`/`pnpm test`（B1–B4 的测试资产全部不在 CI 执行面）；`publish-release.yml` 零引用 `scripts/verify-release-assets.ps1`（B8 脚本「零 workflow 调用」）；`rust.yml` 的治理链演练步骤（:81-82）远端首跑未验证。
3. **验证面/文档面资产被套进四级口径**：`gov.chain_drill_ci`、`sec.governance_chain_drill`、`sec.terminal_permission_surfaces` 本质是测试/演练/文案资产，天然无法「接入 canonical 运行时主路径」；建议按 ADR-0004 §7.7 的 `kind = "test-only"` 尾批口径裁决，**不硬抬 PRODUCTION WIRED**。

## ② 逐条表与逐条取证

> 「证据」列均为 2026-10-21 本机实测（命令可复跑）；「未验证」= 本审计无法在当前环境取得该证据，不臆造。

### 汇总表

| # | id | HEAD 级 | 分类 | 抬级目标 | 预计提交 | 归属线 | 风险 |
|---|---|---|---|---|---|---|---|
| 1 | sec.linux_containment | IMPLEMENTED | (a) | PRODUCTION WIRED（默认工具路径间接已达，补证据） | 1–2 | 安全线（owner tools） | 低 |
| 2 | sec.storage_envelope_crypto | IMPLEMENTED | (c) | PRODUCTION WIRED（分 2 波：密钥出口 → 列级接线） | 5–8 | 安全线（owner storage） | 高 |
| 3 | gov.chain_skeleton | IMPLEMENTED | (c) | PRODUCTION WIRED（收口在途 R8-B2） | 2 | 安全线 | 中 |
| 4 | gov.chain_hook_adapter | IMPLEMENTED | (c) | PRODUCTION WIRED（同 R8-B2，登记合并） | 0–1 | 安全线 | 中 |
| 5 | gov.reflection_queue | IMPLEMENTED | (c) | 先 PRODUCTION WIRED（默认关），后评估 DEFAULT ENABLED | 2–3 | 安全线 | 中高 |
| 6 | gov.chain_drill_ci | IMPLEMENTED | (a) | 验证面：不升四级，补远端首跑证据 | 0–1 | 安全线（CI 证据归效率线执行） | 低 |
| 7 | sec.governance_chain_drill | IMPLEMENTED | (a) | 验证面：不升四级，补证据/口径裁决 | 0–1 | 安全线 | 低 |
| 8 | rel.release_asset_verify | IMPLEMENTED | (c) | PRODUCTION WIRED（发布流水线调用；需 wiring 前缀口径裁决） | 2–3 | 安全线（B8）/release owner | 中 |
| 9 | sec.sql_readonly_gate (B1) | IMPLEMENTED | (d) | 桌面面 PRODUCTION WIRED（条件式：登记+CI+live 验证） | 2 | 安全线（owner companion-desktop） | 低中 |
| 10 | sec.provider_proxy_ssrf (B2) | IMPLEMENTED | (d) | 同 B1 | 2 | 安全线（owner companion-desktop） | 低中 |
| 11 | sec.desktop_file_roots (B3) | IMPLEMENTED | (d) | 同 B1 | 2 | 安全线（owner companion-desktop） | 低中 |
| 12 | sec.terminal_permission_surfaces (B4) | IMPLEMENTED | (d) | 文档/UI 面：保持 IMPLEMENTED + 补测试执行证据 | 1–2 | 安全线（owner companion-desktop） | 低 |

> 归属线依据：B1–B8 与 R8（治理管线）均属**安全线**（`docs/04-internal/backend-enhancement/security-hardening-delivery-2026-10-20.md` 标题与 ADR-0010 §D7：R8-A..D 全归安全线）；wiring/docs_truth 登记执行归**效率线**（ADR-0004 §7.3–§7.5）；生成器 schema/前缀口径归**架构线**（ADR-0004 Owners）。预计提交合计约 20–27（分 3 波，不含 owner 决策等待）。

---

### 1. sec.linux_containment — (a) 已被生产使用，仅缺证据登记

**claim**：Linux 进程遏制边界（partial，如实）；sources=`crates/capabilities/tools/src/process/linux.rs`、`platform.rs`；tests=`crates/capabilities/tools/tests/process_executor.rs`；default/switch=null（无产品开关）；owner=tools。

**取证（2026-10-21）**
- `git grep -n "ProcessExecutor" -- crates ':!crates/capabilities/tools/tests'` → 生产消费点：`crates/capabilities/tools/src/repo.rs:153`（`impl` invoke 内 `tokio::task::spawn_blocking(move || { let executor = ProcessExecutor::new(); executor.execute(&request) })`）、`crates/capabilities/tools/src/shell.rs:700`、`crates/capabilities/tools/src/git_write.rs:938`、`crates/capabilities/tools/src/worktree_apply.rs:464`。
- 生产装配：`crates/engine/runtime-assembly/src/canonical/production.rs:923` `let provider = RepoModule::new(root.clone());`（外层 `if config.repo` 且 root 存在）；默认配置 `production.rs:425 impl Default for ProductionModulesConfig`，`:435-437 filesystem/search/repo = true`；CLI 生产装配经 `..CognitiveModuleConfig::default()` 补齐（`crates/adapters/cli/src/lib.rs:1995` 构造、`:2125` 默认补齐）⇒ **repo 工具默认注册**，其执行必走 `ProcessExecutor`。
- OS 后端选择：`crates/capabilities/tools/src/process/mod.rs:46-54`（`cfg(target_os="linux") pub(crate) mod linux`; `cfg(not(windows/linux/macos)) pub(crate) mod platform`）⇒ **Linux 目标构建**下 shell/repo/git_write/worktree 四族全部走 linux.rs 后端；`platform.rs` 仅奇异平台 fallback（头注自称 "intentionally weak"，能力全 Unsupported）。
- linux.rs 能力面：进程组边界（child 为组 leader，超时杀全组）+ `setrlimit(RLIMIT_AS/CPU/FSIZE/NPROC)` + `PR_SET_PDEATHSIG` + 需降权时 `PR_SET_NO_NEW_PRIVS`，全部 pre_exec fail-closed；`capabilities()`：`ProcessTreeContainment=Partial、MemoryLimit=Partial、ProcessCountLimit=Partial、CpuLimit=Enforced、FileSizeLimit=Enforced、PrivilegeReduction=Partial、FilesystemIsolation/NetworkIsolation=Unsupported、FailClosedPreExecutionContainment=Enforced`。
- tests 真断言（非空壳）：`crates/capabilities/tools/tests/process_executor.rs`（657 行）Linux `#[cfg(target_os = "linux")] mod linux_tests`（:530 起）含 `timeout_terminates_the_whole_process_group_tree`（断言孙进程被杀）、`privilege_reduction_requirement_sets_no_new_privs`（断言 stdout=="NO_NEW_PRIVS:1\n"）、`process_count_limit_is_reported_partial_not_enforced`（断言 "Linux backend advertises RLIMIT_NPROC as PARTIAL only"）；Windows 侧 Job Object 6 条、macOS 侧 2 条。
- CI：`.github/workflows/rust.yml:42` OS matrix 含 `ubuntu-latest`；`:77-78` 显式步骤 `cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture` ⇒ Linux 测试在 CI 有执行面（**远端是否真跑过：未验证**，本机 Windows 无法跑 linux 模块）。
- 数字：claims 的 verify_cmd `cargo test -p apeireth-tools-canonical --test process_executor` 本地**未复跑**（Windows 上 cfg 屏蔽 Linux mod，复跑只能证明跨平台部分），标「未验证」。

**分类 (a)**：不是「生产零调用」——生产默认工具路径（repo→ProcessExecutor）已消费该实现；缺的是**调用点证据登记**与 Linux 目标实测记录。抬 PRODUCTION WIRED 只需证据层动作，不需改代码。

**抬级步骤（文件:函数级）**
1. `docs/status/claims.yaml:136` 条目的 `notes` 增生产调用点证据（`production.rs:923 RepoModule` + `production.rs:437 repo 默认 true` + `repo.rs:137-160 invoke→ProcessExecutor` + `process/mod.rs:52 linux 模块`），`verified_at` bump 到当前 HEAD（登记后重跑 `python scripts/wiring_report.py` 再生 wiring.yaml）。
2. 取一次**远端 CI Linux 首跑**证据：`rust.yml:77-78` 步骤的 run URL/结论写入 notes（无远端访问时如实标「未验证」）。
3. （可选）在 `crates/capabilities/tools/tests/process_executor.rs` 补 1 条「默认 repo 配置下 ProcessExecutor 被构造」的接线断点测试——非必需，避免为证据改代码。

**预计提交**：1–2（纯文档/登记；效率线执行，tools owner 复核）· **风险**：低（只动登记；唯一风险是登记早于远端首跑）· **归属线**：安全线（owner tools）。

---

### 2. sec.storage_envelope_crypto — (c) 纯库内无生产调用，需接线设计

**claim**：会话/记忆静态信封加密 + 备份恢复（B6 库级）；sources=`crates/engine/storage/src/crypto` + 两个 tests 文件；tests 同上；owner=storage；notes 自述「会话/记忆读写路径生产接线未在本批（cli/runtime 未见消费）」——**与实测一致**。

**取证（2026-10-21）**
- `git grep -n "SealedStore\|sealed_backup\|EnvelopeCrypto\|PartitionDek\|KeyWrap\|sealed::\|crypto::" -- crates ':!crates/engine/storage'` → **无输出（exit 1）**；`crates/engine/storage/src/` 内 crypto 目录外、tests 外 grep `SealingLayer|SealedField|PartitionKey|SealedBackup|KekAnchor` → 零命中 ⇒ **库外零消费、库内非测试零装配**。
- storage 依赖方（`git grep -l apeireth-storage -- crates/*/Cargo.toml` 侧确认）：cli、memory、runtime-assembly 依赖 storage，但均未消费 crypto 面。
- 库内 API（可用作接线面）：`crates/engine/storage/src/crypto/layer.rs` `trait FieldSink:20`、`SealingLayer<S>:34`（`new(sink,key: PartitionKey):41`、`put_sealed:46`、`get_sealed:52`）；`crypto/backup.rs` `export_sealed_backup(bundle_dir,&KeyWrapFile,&PartitionKey,&[BackupEntry],now_ms):113`、`verify_sealed_backup:180`、`restore_sealed_backup(bundle_dir,to_root,&UnlockKey):199`（阶段 1 全量校验后才落盘）；`crypto/wrap.rs` `KeyWrapFile`（口令/恢复件/锚定三槽）；`crypto/anchor.rs` `KekAnchor` trait（头注：现无任何实现挂载）。
- tests 真断言：`cargo test -p apeireth-storage --test crypto_invariants --test sealed_backup_restore --locked -j 2`（2026-10-21 本机复跑）→ **exit 0；crypto_invariants 7 passed；sealed_backup_restore 10 passed**（含 `sealed_serialization_never_contains_the_plaintext_on_any_face`、`cross_partition_misuse_is_refused_by_the_partition_aad`、`tampered_ciphertext_tag_or_nonce_is_refused`、`a_single_flipped_byte_refuses_the_whole_restore_with_zero_disk_writes`、`cross_partition_bundle_cannot_be_restored_under_another_partition` 等），关键安全语义（明文零泄漏、AAD 绑分区、先验后写、无后门）有真断言。
- 现成抬级素材：B6 施工报告 `artifacts/b6-crypto-report-2026-10-20.md` 与**已写好的 CLI 出口草稿** `artifacts/b6-crypto-cli-wiring/{README.md,crypto_keys.rs}`（未跟踪 artifacts）——三动词 `wrap|recovery-export|unlock`（分区 DEK 双槽包裹+恢复件导出/补发/解锁自检），附带 `cli/src/lib.rs` 加 `pub mod crypto_keys;`、`cli/src/main.rs` help+dispatch 两处最小补丁与搬入必跑命令。

**分类 (c)**：加密原语与证明齐备（IMPLEMENTED 如实），但**会话/记忆读写路径零接线**、备份出口未并入 G5 引擎、KekAnchor 无实现。

**抬级步骤（文件:函数级）**
- **波 1（密钥出口，1 commit）**：原样搬入 `artifacts/b6-crypto-cli-wiring/crypto_keys.rs` → `crates/adapters/cli/src/crypto_keys.rs`，按 README 打两处最小补丁（`crates/adapters/cli/src/lib.rs` 模块表加 `pub mod crypto_keys;`；`crates/adapters/cli/src/main.rs` help 三行 + dispatch `"crypto" => apeireth_cli::crypto_keys::run_crypto(&args[1..])`）。跑 `cargo test -p apeireth-cli --locked -j 2` + clippy `-D warnings` + fmt。**只解「密钥面人类出口」，不改任何存储路径**（0 装诚实保留：口令回显/无回显输入未接等）。
- **波 2（B6-2 列级接线，3–5 commits，需 owner 拍板后）**：用 `SealingLayer`/`FieldSink` 接写入点——`crates/engine/runtime-assembly/src/sqlite_session.rs:119`/`:220`（`INSERT INTO sessions (id, data)` 会话 JSON 整包）、`crates/engine/memory/src/continuity_link.rs:108`、`session_lifecycle.rs:353`、`session_note.rs:81`/`:639`、`append_only.rs:114`（payload 列）、`context_ledger.rs:133`、`agent_trace.rs:305`、`commitments.rs:453`。前置决策（owner 拍板位）：P0 字段清单（B6 报告 §3）、KDF 迭代参数（仓内 210k 为保守常量非强度结论）、KekAnchor 选型（口令 vs 平台凭据库/桌面 keychain）、既有明文库迁移策略。按「分区/表族」分 commit。
- **波 3（B6-3 密封备份并入既有备份出口，1–2 commits）**：`crates/adapters/cli/src/backup.rs:256 run_backup`/`:217 backup_export_config` 与 `crates/capabilities/tools/src/backup.rs:1010 BackupExportTool`（能力 id `tool.backup_export`:70，默认关 `APEIRETH_ENABLE_BACKUP_EXPORT`，env-registry:356）对话合并；不另起炉灶。
- **登记**：完成波 2/3 后 `docs/status/claims.yaml:170` 改 level=PRODUCTION WIRED + notes + verified_at，重跑 docs_truth/wiring_report。HARDWARE VALIDATED 需真盘恢复演练（密钥丢失路径）→ 另立证据。

**预计提交**：5–8（分 2–3 波）· **风险**：**高**——静态加密迁移涉及既有明文数据、密钥丢失即不可恢复（`KEY_LOSS_BOUNDARY`）、每字段 AEAD 性能、跨分区 AAD 语义；且 owner 决策未拍板（P0/P1 字段、KDF 强度、锚定选型）→ 建议先立项拍板再动波 2。· **归属线**：安全线（owner storage；CLI 出口归装配线，波 3 与 G5 备份线交叉需协调）。

---

### 3. gov.chain_skeleton — (c) 纯库内，生产零调用（HEAD）

**claim**：R8-A 治理链六段顺序执行骨架；sources=`crates/foundation/governance/src/chain.rs`、`lib.rs`；tests=`chain.rs`（库内）；owner=governance。

**取证（2026-10-21）**
- **HEAD 零调用**：`git grep -n "GovernanceChain\|ChainGovernanceHook\|ReflectionScheduler" -- crates ':!crates/foundation/governance'` → **无输出（exit 1）**；早前含 `chain::` 的 grep 仅命中 `crates/engine/guard` 的无关 `BehaviorChain`。
- 生产治理现状（HEAD）：`crates/adapters/cli/src/lib.rs:1021 PermissionGovernanceHook::new_shared(policy.clone())`、`:1559 PermissionPresetGovernanceHook::new(...)`、`:1563 builder.with_governance(Arc::new(preset_governance))`——生产管治面是 `GovernancePipeline` + preset hook，**不是** `GovernanceChain`；`crates/adapters/gateway/src` 内无任何 `GovernanceHook`/`with_governance` 引用（`git grep` exit 1）⇒ **runtime 治理注入点唯一在 CLI 装配根**（`build_canonical_runtime_with_parts`，cli/src/lib.rs:1549–1571 一带）。
- 可运行证据：`cargo test -p apeireth-governance --lib --locked -j 2`（2026-10-21）→ exit 0，**214 passed / 0 failed**（与 ADR-0010 R8-C 交付记录一致）；chain.rs 库内约 20 条测试（六段顺序、deny 短路、audit fail-closed、注入端口 fail-closed 等）。
- 设计依据：ADR-0010（Accepted 2026-10-20）D1/D2/D3/D7；`docs/architecture/r8-b-entry-convergence-design.md` 推荐方案 B（ChainGovernanceHook 为 runtime 唯一治理面；B1 适配器已交付 → B2 CLI 装配根切换 → B3 持久 AuditSink + IM 审计对）。
- **在途实装（工作树，未提交）**：见 §0.4，`cli/src/lib.rs:1559–1571` 已切换为 chain 装配，另有未跟踪等价测试 `crates/adapters/cli/tests/governance_chain_equivalence.rs`（4 个四态测试）。

**分类 (c)**（HEAD）；工作树在途状态 = 「已写好待收口」。

**抬级步骤（文件:函数级）**
1. **收口在途 R8-B2（1 commit，安全线）**：把工作树 `crates/adapters/cli/src/lib.rs:1559-1571` 的 chain 装配 + `crates/adapters/cli/tests/governance_chain_equivalence.rs` 四态测试一并提交（若并行线先提交则改为复核其 commit）；提交前在同一 commit 内跑 `cargo test -p apeireth-cli --locked -j 2` + clippy + fmt。回滚 = 单处 `with_governance` 参数换回 `preset_governance`（一行级）。
2. **登记（1 commit，效率线执行/安全线复核）**：`docs/status/claims.yaml:188` 改 `level: PRODUCTION WIRED`、notes 写「装配根 = cli/src/lib.rs build_canonical_runtime_with_parts，R8-B2 commit <sha>」、`verified_at` bump；重跑 `python scripts/wiring_report.py` 再生 wiring.yaml（生成物禁手改）。
3. 若 owner 要求治理行为等价性证明入 CI：确认 `governance_chain_equivalence.rs` 被 `cargo nextest run --workspace --profile ci`（rust.yml）覆盖（cli 集成测试在工作区 nextest 内，**待实测确认**）。

**预计提交**：2 · **风险**：中（改变生产治理判定路径；等价测试覆盖 Allow/Deny/RequireApproval/NotConfigured 四态且链适配器只加审计不改判定；回滚一行；另需与并行线协调同文件先后，避免 hunk 冲突）· **归属线**：安全线。

---

### 4. gov.chain_hook_adapter — (c) 纯库内，生产零调用（HEAD）

**claim**：R8-B1 `ChainGovernanceHook` 适配器；sources=`chain.rs`；tests=`chain.rs`；owner=governance。

**取证（2026-10-21）**
- 同 #3：HEAD 生产零调用（grep exit 1）；适配器库内测试齐全（`chain_hook_allowed_path_maps_to_allow_and_audits`、`chain_hook_deny_carries_the_phase_label`、`chain_hook_require_approval_stays_distinct`、`chain_hook_without_gate_fails_closed`、`chain_hook_forwards_approval_resolved_to_gate` 等）；`cargo test -p apeireth-governance --lib` → 214 passed。
- 在途：工作树 `cli/src/lib.rs:1568-1571` 已出现 `ChainGovernanceHook::new(Arc::new(governance_chain))` + `builder.with_governance(...)`（未提交）。

**分类 (c)**（HEAD）；抬级与 #3 **同一 commit**（R8-B2 收口即同时成为生产引用），登记可合并为同一 commit 或紧随 1 个登记 commit。

**抬级步骤**：同 #3 步骤 1（同一 R8-B2 收口 commit）+ `claims.yaml:204` 登记（level=PRODUCTION WIRED、notes 指向装配行号、verified_at bump、wiring 再生）。

**预计提交**：0–1（与 #3 合并计）· **风险**：中（同 #3）· **归属线**：安全线。

---

### 5. gov.reflection_queue — (c) 纯库内，默认未装配（零行为变化）

**claim**：R8-C 进程内延迟反思队列；sources=`chain.rs`；tests=`chain.rs`；owner=governance。

**取证（2026-10-21）**
- `git grep -n "ImmediateReflectionScheduler\|DelayedReflectionScheduler\|ReflectionScheduler" -- crates ':!crates/foundation/governance'` → 无输出（exit 1）⇒ 生产零装配。
- 库内实现与测试：`chain.rs:306 trait ReflectionScheduler`、`:339 ImmediateReflectionScheduler`、`:370 DelayedReflectionScheduler`（`new(clock: Arc<dyn ChainClock>, delay_ms: i64):396`、`pending_len:411`、`take_due_now:419`）、`ChainConfig:507`、`GovernanceChain::with_reflection_scheduler:568`/`with_clock:575`；测试含 `delayed_scheduler_releases_each_due_reflection_once`、`chain_audits_due_reflections_before_deciding`、`default_scheduler_never_reports_due_reflections`（默认关等价测试已存在）、`chain_fails_closed_when_due_reflection_audit_fails`。
- **无产品开关**：`docs/architecture/env-registry.toml` grep `GOVERNANCE|CHAIN|REFLECT` 零命中 ⇒ 不存在「默认关开关未登记」形态，**故不归 (b)**。

**分类 (c)**；落地路径依赖 R8-B2 先收口（无生产链则队列无宿主）。

**抬级步骤（文件:函数级）**
1. 前置：#3 的 R8-B2 收口（chain 成为唯一治理面）。
2. **R8-B3 接线（2–3 commits，安全线，需 owner 拍板）**：在 `crates/adapters/cli/src/lib.rs:1187 build_production_governance_parts_from_env` / `:1549 build_canonical_runtime_with_parts` 构造 `GovernanceChain` 时注入 `with_reflection_scheduler(...)`（默认 `ImmediateReflectionScheduler` 保持零行为变化）与 `with_clock(SystemChainClock)`；延迟模式须新增**默认关**开关（env 名待 owner 拍板，例如 `APEIRETH_GOVERNANCE_REFLECTION_DELAY_MS`；先登记 `docs/architecture/env-registry.toml` + 默认关零变化测试，符合 ADR-0004 §7.2）；持久 AuditSink 介质（是否并入 IM 审计对/审计哈希链）按 ADR-0010 §9 未决项拍板。
3. 登记：`claims.yaml:219` level 视结果定：若仅默认 Immediate 装配 → 仍属「零行为变化接线」，可主张 PRODUCTION WIRED（治理面已挂，反思器为默认实现）；若延迟反射真启用需另测 → 先 PRODUCTION WIRED（默认关），DEFAULT ENABLED 需 owner 明确授权（默认关是口径红线，不轻改）。

**预计提交**：2–3 · **风险**：**中高**——延迟反思改变审计/决策时序（行为变化面），fail-closed 语义须专门测试；持久审计介质未决（owner 决策）。· **归属线**：安全线。

---

### 6. gov.chain_drill_ci — (a) CI 已接线，仅缺远端首跑证据

**claim**：R8-D 治理链六段演练入 CI（rust-tests 显式步骤，尚未首跑）；sources=`.github/workflows/rust.yml`；tests=`crates/foundation/governance/tests/governance_chain_drill.rs`；owner=governance。

**取证（2026-10-21）**
- 步骤确实在 HEAD 工作流内：`.github/workflows/rust.yml:81-82`「Governance chain drill (R8-D)」→ `cargo test -p apeireth-governance --test governance_chain_drill --locked -- --nocapture`。
- 本地等价命令复跑：`cargo test -p apeireth-governance --test governance_chain_drill --locked -j 2` → exit 0，`running 3 tests`，`test result: ok. 3 passed; 0 failed`（`chain_pass_path_ratifies_and_persists_audit`、`chain_reject_path_pins_named_dissent_and_insufficient_votes`、`chain_timeout_path_holds_and_fails_closed`）。
- **远端 CI 首跑：未验证**（本机无法访问 GitHub Actions 运行记录；claims notes 自述 0 次，与之一致）。
- 漂移检查：`git diff --name-only 7153f5db..HEAD -- .github/workflows/rust.yml crates/foundation/governance/tests/governance_chain_drill.rs` = 空。

**分类 (a)**：CI 步骤即「生产使用」（验证资产的生产面就是 CI）；缺的是首跑运行记录，属**证据登记**，不是代码抬级。且该 claim 属验证面（test-only），**不建议硬抬 PRODUCTION WIRED**——建议按 ADR-0004 §7.7 尾批口径裁决 `kind = "test-only"`（测试/演练资产不计入四级能力口径），claims 里保持 IMPLEMENTED 并注明。

**抬级步骤**：把一次远端 rust-tests run 的 URL/结论（R8-D 步骤绿）记入 `docs/status/claims.yaml:234` notes 并 bump verified_at（0–1 commit，效率线执行）；如远端不可得，notes 明确写「本地实测 3 passed（2026-10-21）+ 远端未验证」。

**预计提交**：0–1 · **风险**：低（纯证据）· **归属线**：安全线（governance owner，CI 执行归效率线）。

---

### 7. sec.governance_chain_drill — (a) 验证面资产，仅缺证据登记/口径裁决

**claim**：M-of-N 逻辑票链三路径端到端合练（B7 验证证据）；sources=tests 文件 + `docs/04-internal/governance-chain-drill-2026-10-20.md`；tests=同文件；owner=governance；自述「验证面资产，不是新能力；M-of-N ≠ 密码学多签」。

**取证（2026-10-21）**
- tests 真断言（488 行演练文件）：三路径测试通过率 3/3（本机复跑，见 #6）；helpers 覆盖 `vote/proposal/permission_onion/ha_2of3`（2-of-3 M-of-N）、`step`、`persist_verify_and_tamper_check`（持久化+verify+篡改检查）；演练记录文档存在（`Test-Path` = True）。
- 该文件同时被 #6 的 CI 步骤执行 ⇒ 已在 CI 执行面，缺远端运行记录；另：它作为「验证证据」天然无 runtime 主路径可接。

**分类 (a)**：同 #6，属验证面资产；**无法到达也无法证明 PRODUCTION WIRED**。

**抬级步骤**：建议 1 commit 纯登记（`claims.yaml:249` notes 写本地 3/3 与文档路径、verified_at bump；远端记录同 #6），并在 notes 中明确「验证面：不作为四级能力抬级对象，按 ADR-0004 §7.7 test-only 口径待裁」。

**预计提交**：0–1 · **风险**：低（纯登记；风险是口径未裁决前被误读为能力级）· **归属线**：安全线。

---

### 8. rel.release_asset_verify — (c) 脚本零 workflow 调用（发布流水线未接线）

**claim**：发布资产校验脚本（SHA-256 + 签名状态如实报，B8）；sources=`scripts/verify-release-assets.ps1` + 两篇文档；**tests: []**（空，如实）；owner=release。

**取证（2026-10-21）**
- `git grep -n "verify-release-assets" -- .github docs scripts` → 仅命中 `docs/status/claims.yaml:271/276`、`docs/status/wiring.yaml:278`、`docs/packaging/release-asset-verification.md:28/:46/:73/:82`、`docs/04-internal/backend-enhancement/B8-release-signing-2026-10-19.md:21` 与脚本自身注释；**`.github/workflows/` 零命中** ⇒ 发布流水线零调用。
- `.github/workflows/publish-release.yml`：Windows job 用 `packaging/desktop/build-desktop-nsis.ps1` 构建 NSIS（:20/:56-58）→ 上传 artifact（:62-65）→ release job 下载（:77-81）→ `softprops/action-gh-release` 发布 `release-assets/*.exe` + `*.sha256`（:84/:92）→ release 说明自述 "Integrity: the matching `.sha256` asset carries the SHA-256 checksum"（:103）。**无** `scripts/generate-release-manifest.ps1` 调用、**无** `scripts/verify-release-assets.ps1` 步骤。
- 脚本能力（已实测交付，非本次复跑）：参数 `[string[]]$ArtifactPath`（位置必填）、`-ManifestPath`、`-ExpectedSha256`、`-RequireSignature`；支持 `SHA256SUMS` 清单或 `.sha256` sidecar；exit 0=全部一致（RequireSignature 时签名须有效）/1=哈希不一致/清单缺项/参数不合法/签名非有效；Authenticode 状态如实报（未签名不算失败）；只校验不改被检文件。文档 `docs/packaging/release-asset-verification.md:73` 记「SHA-256 清单校验 已交付并实测（2026-10-20，Windows；正反两路 7 用例）」。
- **结构性口径缺口**：wiring 生成器的 `PROD_PREFIXES` 只含 `crates/` 与 `frontend/companion-desktop/src-tauri/src/`，**不含 `scripts/`、`.github/`** ⇒ 该 claim 的 production_refs 恒为 []，按 ADR-0004 §7.5「禁止无生产引用却标 PRODUCTION WIRED」，**即使接进发布流水线也不能直接标 PRODUCTION WIRED**——需架构线先裁决 schema（扩前缀 / 新 evidence 形态），否则只能作为「CI 接线 + 文档证据」登记。

**分类 (c)**：脚本与文档齐备（IMPLEMENTED 如实，tests=[] 也如实）；缺口是发布流水线未调用。

**抬级步骤（文件:函数级）**
1. **接线（1 commit，release owner 复核）**：改 `.github/workflows/publish-release.yml` 的 Windows build job——`build-desktop-nsis` 之后加 `pwsh -NoProfile -File scripts/generate-release-manifest.ps1 -DistDir target/desktop-nsis`（生成 `SHA256SUMS` + `releaseon`，脚本参数已核实：`DistDir/Version/CommitSha/ReleaseTag/StageMetadata`），再对每个 `release-assets/*.exe` 跑 `pwsh -NoProfile -File scripts/verify-release-assets.ps1 -ArtifactPath <exe> -ManifestPath target/desktop-nsis/SHA256SUMS`，exit 0 才继续发布；把 `SHA256SUMS` 一并纳入 release files。发布说明改为「已由流水线自动校验」。
2. **测试证据（0–1 commit）**：发布 job 内该步骤本身即集成测试；claims `tests` 字段可登记 `.github/workflows/publish-release.yml`（如 schema 允许）或维持 [] 并在 notes 写「CI 自动校验步骤 + 手工 7 用例」。Pester 单测需先实测 runner 可用性（未验证）。
3. **口径裁决（架构线，0 commit 或 1 个 ADR 修订）**：决定 `scripts/`/`.github/` 是否纳入生产引用（或为 release 类 claim 引入 `evidence` 形态）；裁决前该 claim 建议保持 IMPLEMENTED，notes 记「CI 接线完成，待口径」。

**预计提交**：2–3 · **风险**：中（改发版流水线，需下次 tag 实跑验证；签名仍是排期待办，不得假装已签名）· **归属线**：安全线（B8，release owner 执行）；wiring 口径归架构线。

---

### 9. sec.sql_readonly_gate（B1）— (d) 桌面生产面已接线，缺证据登记/CI 执行

**claim**：桌面三数据库客户端只读闸加固（词法级写类拒绝 + 远程只读事务纵深）；sources=`sqlite_workbench.rs`/`pg_client.rs`/`mysql_client.rs`（src-tauri）；**tests: []**；owner=companion-desktop。

**取证（2026-10-21）**
- **Tauri 命令注册**：`frontend/companion-desktop/src-tauri/src/lib.rs:551` `.invoke_handler(tauri::generate_handler![...])` 清单含 `sqlite_workbench::query_sqlite_file`、`pg_client::query_postgres`、`mysql_client::query_mysql`。
- **TS 侧真实调用**：`frontend/companion-desktop/src/lib/starport/apps/db-app.ts:128/133/137` `tauriInvoke('query_postgres'/'query_mysql'/'query_sqlite_file')`。
- **测试真断言**：`sqlite_workbench.rs` 4 个 `#[test]`（`readonly_gate_allows_read_forms`、`readonly_gate_rejects_writes_and_multi`——含 INSERT/UPDATE/DELETE/DROP/CREATE/ATTACH/VACUUM/多语句/WITH 内嵌 DML/EXPLAIN ANALYZE UPDATE/SELECT INTO/FOR UPDATE、`value_mapping_hides_blob_payload`、`real_file_roundtrip_and_truncation`）；`pg_client.rs` 5 个（含 `:952-965` 帧序列断言 `BEGIN READ ONLY`→查询→`COMMIT`、`:1049 error_path_rolls_back_read_only_transaction`）；`mysql_client.rs` 3 个（含 `START TRANSACTION READ ONLY` 帧断言与错误回滚）。
- 实现入口：`query_sqlite_file:336` 内 `:337` 先 `ensure_readonly_sql:300`；`pg_client.rs:489`、`mysql_client.rs:498` 同样先闸。
- **本机复跑**：`cd frontend/companion-desktop/src-tauri && cargo test --locked -j 2`（2026-10-21）→ exit 0：lib **140 passed / 3 ignored**（143 running）+ `supervisor_lifecycle` **11 passed** = **151 passed**，与交付记录「151 passed」一致（3 ignored 为环境相关既有忽略）。
- **CI 缺口**：`.github/workflows/companion-desktop-ci.yml` tauri-shell job（:34 ubuntu-latest）只跑 `cargo check --workspace --all-targets`（:68-70），**不跑 cargo test**；svelte-ui job 只跑 `pnpm check`（:100）/`pnpm check:assets`（:107），**不跑 pnpm test** ⇒ B1–B4 的测试资产均不在 CI 执行面。

**分类 (d)**：桌面生产装配根（Tauri `invoke_handler`）已注册命令、UI 已调用、NSIS 安装包出货（`publish-release.yml` 打桌面安装包）⇒ **桌面面生产接线成立**；缺口 = claims tests 为空 + CI 不执行 + 无 live E2E。按 wiring 口径（`frontend/companion-desktop/src-tauri/src/` 在生产前缀内）可主张 PRODUCTION WIRED（桌面面），但需先补登记与执行证据。

**抬级步骤（文件:函数级）**
1. 登记（1 commit，效率线执行/owner 复核）：`docs/status/claims.yaml:281` 的 `tests` 填实际测试位置（`frontend/companion-desktop/src-tauri/src/sqlite_workbench.rs`、`pg_client.rs`、`mysql_client.rs` 三个 in-file 测试文件，或列测试名），verified_at bump。
2. CI 执行（1 commit，companion-desktop line）：`companion-desktop-ci.yml` tauri-shell 加 `cargo test --workspace --locked`（Windows 专属测试需实测 ubuntu 运行可行性；更稳的替代 = 新增 `windows-latest` job 跑桌面 crate 测试）。
3. 抬级口径：满足「tests 登记 + CI 绿 + 一次桌面 live 手工验证记录（三数据库客户端真实连接一次）」后，level→PRODUCTION WIRED 并在 notes 注明「桌面面生产口径（非 canonical runtime）」。

**预计提交**：2 · **风险**：低–中（CI 改造成本与 ubuntu 上 tauri 测试可行性；保守启发式误拒合法 SQL 属 fail-closed 方向，已如实登记）· **归属线**：安全线（owner companion-desktop，CI 归效率线）。

---

### 10. sec.provider_proxy_ssrf（B2）— (d) 桌面生产面已接线，缺证据登记/CI 执行

**claim**：Provider 中继 SSRF 面收紧（目标类闸 + DNS 钉址 + 重定向逐跳闸 + 头白名单）；sources=`provider_proxy.rs`；**tests: []**；owner=companion-desktop。

**取证（2026-10-21）**
- 命令注册：`lib.rs:551` 清单含 `provider_proxy::provider_request`；TS 调用：`frontend/companion-desktop/src/lib/starport/provider-transport.ts:50` `invoke('provider_request')`。
- 测试真断言：`provider_proxy.rs` 12 个 `#[test]`——`ip_literal_variants_normalize_into_their_address_class:1064`、`refuses_non_public_target_literals`（私网/链路本地/组播/未指定/广播/共享 100.64/基准 198.18/文档 192.0.2/协议 2001:: 全拒，且断言 transport 调用数为 0）、`public_target_literals_relay`、`every_resolved_address_must_pass_the_gate`（混合 DNS fail-closed）、`localhost_name_is_the_local_face`、`redirect_targets_re_pass_the_gate`、`rejects_non_http_url_schemes`、`rejects_unsupported_methods`、`enforces_response_body_cap`、头白名单/凭据擦除等。
- 实现：`gate_target:293`（解析并钉住地址，后续连接不二次解析）、`gate_address:176`、`gate_resolved_addrs:263`、`gate_redirect_target:326`（逐跳闸）、`is_loopback_target:421`、`redirect_policy:537`（`MAX_REDIRECT_HOPS` 有界）、`relay_via:678`；**loopback 放行是产品设计**（本地 gateway/本地模型 base URL 为 loopback，127.0.0.0/8 与 ::1 放行）——残余「回环类级口子（任意本地 HTTP 服务可作目标，待端点注册面收紧）」与「跨主机重定向 rebinding 小窗」已在代码注释/交付记录如实登记。
- 本机复跑（同 #9）：桌面 crate 151 passed（含 12 条 provider_proxy 测试，位于 lib 套件内）。

**分类 (d)**：同 #9（命令注册 + UI 调用 + 安装包出货）；缺口 = tests 登记、CI 执行、live E2E（真实网络/重定向）。

**抬级步骤**：同 #9——①`claims.yaml:297` tests 填 `provider_proxy.rs`（in-file，12 测试）；②companion-desktop CI 加 cargo test；③live 手工验证（真实 provider 中继一次 + 本地 loopback 端点一次）；另建议把「端点注册面收口」列为 B2 后续立项（安全线，notes 已记残余）。

**预计提交**：2 · **风险**：低–中（回环口子与 rebinding 小窗是已知残余，不阻塞登记但不得宣称已收口）· **归属线**：安全线。

---

### 11. sec.desktop_file_roots（B3）— (d) 桌面生产面已接线，缺证据登记/CI 执行

**claim**：桌面文件操作收口登记根（登记根派生 + canonicalize 祖先校验）；sources=`workspace_list.rs`；**tests: []**；owner=companion-desktop。

**取证（2026-10-21）**
- 命令注册：`lib.rs:551` 清单含 `workspace_list::list_workspace_dir`、`create_dir_entry`、`rename_entry`、`delete_entry`（另有 `get_workspace_dir`/`set_workspace_dir` 等非本 claim）。
- TS 调用：`folder-tiles-app.ts:224 list_workspace_dir`、`:243 create_dir_entry`、`:186 rename_entry`、`:199 delete_entry`。
- 测试真断言：`workspace_list.rs` 9 个 `#[test]`（:320-511）——`unregistered_root_fails_closed`、`listing_rejects_empty_and_non_dir`、`entry_name_traversal_components_rejected`、`outside_root_rejected_for_every_op`、`link_escape_base_rejected_in_root_link_allowed`（junction/链接逃逸实测拒绝）、`sort_*`、`write_ops_lifecycle_and_zero_fab` 等。
- 函数锚点：`resolve_registered_root:61`、`contain_within_root:79`、`resolve_base_in_root:94`、`validate_entry_name:156`、`*_in_root` 系列（:107/:171/:183/:205）。
- 本机复跑（同 #9）：151 passed（含 workspace_list 9 条）⇒ 与交付记录一致。
- 残余如实：TOCTOU 窗口未闭（std 无句柄级原子绑定，缓解为操作规范）；live E2E 未实测。

**分类 (d)**：同 #9。

**抬级步骤**：同 #9——①`claims.yaml:311` tests 填 `workspace_list.rs`（in-file 9 测试）；②CI 加 cargo test；③live 手工验证（junction/`..` 逃逸真实拒绝一次）；残余 TOCTOU 不阻塞登记但须在 notes 保留。

**预计提交**：2 · **风险**：低–中（TOCTOU 残余如实，不宣称已闭）· **归属线**：安全线。

---

### 12. sec.terminal_permission_surfaces（B4）— (d) 文档/UI 文案面，底层命令已接线

**claim**：终端权限边界明示 + 三层权限面文档；sources=`frontend/companion-desktop/src/lib/starport/apps/terminal-app.ts` + `docs/02-guides/permission-surfaces.md`；tests=`frontend/companion-desktop/tests/starport-face-apps.mjs`；owner=companion-desktop。

**取证（2026-10-21）**
- 底层命令注册：`lib.rs:551` 清单含 `terminal_exec::run_workspace_command`；TS 调用 `terminal-app.ts:98`。
- 文案钉子真实存在：`frontend/companion-desktop/tests/starport-face-apps.mjs:79-82` 断言四条边界文案（以当前 Windows 用户权限执行命令 / 不受智能体审批链保护 / 不是操作系统沙箱 / `enable_shell=false` 关闭的是智能体工具面），:75-77 断言 `run_workspace_command`/截断/审批族接线缝；文档 `docs/02-guides/permission-surfaces.md` 存在且为三层权限面单一说明（引 `terminal_exec.rs` 契约为执行语义真源，本文只言边界）。
- 本机复跑：`cd frontend/companion-desktop && pnpm test`（2026-10-21）→ exit 0，**82 of 82 suites passed**（`tests/run-all.mjs:100` SUITES 含 `starport-face-apps.mjs`），与交付记录一致。
- **CI 缺口**：「pnpm test」不在 CI：`companion-desktop-ci.yml` svelte-ui job 只跑 `pnpm check`/`pnpm check:assets`（:100/:107）。

**分类 (d)**：claim 本体是**文档/UI 文案资产**，其「生产接线」= 文案已呈现在桌面终端面板（底层命令已注册）+ 测试钉子存在；但它**不是 runtime 能力**，无法也不应抬 PRODUCTION WIRED——建议按 ADR-0004 §7.7 的 test-only/derived kind 思路裁决其登记口径（保持 IMPLEMENTED + notes 注明文档面），或由架构线为「文档面 claim」定义登记形态。

**抬级步骤**
1. 测试执行入 CI（1 commit，companion-desktop line）：`companion-desktop-ci.yml` svelte-ui job 在 `pnpm check` 后加 `pnpm test`（node-only 测试，风险低）。
2. 口径登记（0–1 commit，效率线 + 架构线口径）：`claims.yaml:325` notes 写「文档/UI 文案面；底层命令 terminal_exec::run_workspace_command 已注册（lib.rs:551）+ UI 调用（terminal-app.ts:98）；pnpm test 82/82（2026-10-21 本机复跑）」；level 保持 IMPLEMENTED，或按 §7.7 口径改登记为验证/文档面。

**预计提交**：1–2 · **风险**：低（底层执行语义未改：terminal 是用户直操面，不受审批链；claim 不改变审批语义）· **归属线**：安全线（owner companion-desktop，CI 归效率线）。

## ③ 建议首批 3–4 条及理由

**建议首批 = 4 条：`gov.chain_skeleton` + `gov.chain_hook_adapter`（同一工程批）+ `sec.storage_envelope_crypto`（波 1 密钥出口）+ `rel.release_asset_verify`。** 另建议紧随一批处理 B1/B2/B3 证据登记。

| 顺序 | 条目 | 理由（为什么先做） | 依赖/前置 |
|---|---|---|---|
| 1 | `gov.chain_skeleton` + `gov.chain_hook_adapter`（一个 R8-B2 收口批） | **在途代码已写好**（工作树 `cli/src/lib.rs:1559-1571` + 未跟踪 4 态等价测试），一次收口让 2 条 claim 同时获得生产引用（收益/成本最高）；行为等价有四态测试、回滚一行，风险可控；且登记越晚，claims 与代码漂移越大（当前 docs_truth 已因工作树漂移报 6 条 STALE） | 需 lead 与并行线协调同文件提交先后；收口 commit 内跑 cli 测试三件套 |
| 2 | `sec.storage_envelope_crypto`（波 1） | **现成草稿**（`artifacts/b6-crypto-cli-wiring/`）可 1 commit 落位 `apeireth crypto wrap|recovery-export|unlock`，把「零出口」变「人类出口可用」；且为波 2 列级接线立项提供真实入口与用户反馈面；不触碰加密写入路径，风险可控 | 不依赖他人；波 2 需 owner 先拍板 P0 字段/KDF/锚定/迁移 |
| 3 | `rel.release_asset_verify` | 脚本已交付实测（7/7 手工用例）、文档已定流程，**唯一缺口是发布流水线不调用**；1 commit 即可把「发布必跑校验」落地，把 3 条 (c) 中最低成本的一条变生产接线；影响面限于发版路径 | release owner 复核 + 下次 tag 实跑；wiring 前缀口径需架构线裁决后才能标 PRODUCTION WIRED |
| 4 | B1/B2/B3 证据登记批（3 条合并 1–2 commits） | 三条桌面安全能力的**测试已有真断言且本机 151 passed 可复现**（2026-10-21 复跑），只缺 claims tests 登记 + CI 执行；同一份 CI 改动可一次覆盖三条；是 (d) 类里成本最低、风险最小的批 | companion-desktop line 同意 CI 增加 cargo test（需实测 ubuntu/windows 运行） |

**不建议首批**：`sec.linux_containment`（需远端 Linux CI 首跑证据，本机不可得；登记可等 rust-tests 下次绿后一并做）；`gov.reflection_queue`（依赖 R8-B3 + owner 决策，改动行为面，最重）；`gov.chain_drill_ci` / `sec.governance_chain_drill`（验证面，等 §7.7 口径裁决后 0–1 commit 顺手登记）；`sec.terminal_permission_surfaces`（文档面，与 B1–B3 证据批合并即可）。

## ④ 未验证项清单（本审计未取得证据，不臆造）

1. **远端 CI 执行记录**：GitHub Actions 运行状态在本机不可得——`rust.yml` 的 process_executor 步骤（:77-78）在 ubuntu-latest 的历史/最近结果、治理链演练步骤（:81-82）首跑、`companion-desktop-ci.yml` 历史结果，全部**未验证**；本地只复跑了本地等价命令（见各条）。
2. **Linux 目标真机/真 CI 的进程遏制实测**：本机 Windows 无法执行 `process_executor.rs` 的 `#[cfg(target_os = "linux")] mod linux_tests`（被 cfg 屏蔽）；Linux 侧 3 条断言（树杀/NO_NEW_PRIVS/NPROC partial）只在源码层核实，**未在本机运行**；`HARDWARE VALIDATED` 不成立（ubuntu-latest 亦非真机）。
3. **桌面 live E2E**：B1–B4 的 live 场景（真实三数据库连接、真实 provider 网络/重定向、真实 junction 逃逸、真实终端面板）**未实测**（claims notes 自述一致）；本次只跑桌面 crate 单测（151 passed）与 `pnpm test`（82/82）。
4. **B2 残余可利用性**：回环类级口子（任意本地 HTTP 服务可作中继目标）与跨主机重定向 rebinding 小窗**未做利用验证**。
5. **B3 TOCTOU 残余**：未做窗口期验证。
6. **B6 加密面未验证项**（B6 报告 §7 自述）：列级改造未接线、KekAnchor 平台锚定实现未接线、KDF 迭代强度未实测（210k 为保守常量非强度结论）、P1 字段定级（memory_vectors 等）待拍板、口令无回显未接、live E2E 未实测。
7. **发布流水线改动后行为**：`publish-release.yml` 加校验步骤后的真实发版行为**未实测**（需下一次 tag）；Pester 在 runner 的可用性**未验证**。
8. **wiring 前缀口径**：`scripts/`、`.github/` 是否纳入生产引用未裁决（本文件据 `scripts/wiring_report.py` 的 `PROD_PREFIXES` 与 ADR-0004 §7.5 推断，**架构线未裁决**）；release 类 claim 的 production_refs 现状恒空。
9. **并行线在途改动最终去向**：工作树 10 个 M 文件 + 2 个未跟踪文件（§0.4）归属与提交时点**未验证**；本文件结论以 2026-10-21 16:26 快照为准，若并行线先行提交，`gov.chain_*` 结论需按新 HEAD 重核。
10. **`governance_chain_equivalence.rs` 是否被 CI nextest 覆盖**：cli 集成测试在工作区 nextest 内属推断，**未实测**其进入 `cargo nextest run --workspace` 选择集。
11. **单 crate 复跑边界**：本次只跑 4 组只读单 crate 命令（gov lib / gov drill / storage 两个测试目标 / 桌面 crate test / pnpm test），未跑全量 `cargo test --workspace`（按纪律不做全量）；因此「其他 crate 未回归」不作结论。

---

## 附录 A：本次取证命令与结果（可复核）

| # | 命令（2026-10-21，仓库根） | 结果摘要 |
|---|---|---|
| 1 | `git grep -n "GovernanceChain\|ChainGovernanceHook\|ReflectionScheduler" -- crates ':!crates/foundation/governance'` | HEAD：无输出 exit 1（生产零调用） |
| 2 | `git grep -n "with_governance" -- crates` | HEAD 生产 src 无命中；工作树仅 `crates/adapters/cli/src/lib.rs:1571`；gateway src 零命中（exit 1） |
| 3 | `git grep -n "SealedStore\|sealed_backup\|EnvelopeCrypto\|PartitionDek\|KeyWrap\|sealed::\|crypto::" -- crates ':!crates/engine/storage'` | 无输出 exit 1（库外零消费） |
| 4 | `git grep -n "ProcessExecutor" -- crates ':!crates/capabilities/tools/tests'` | repo.rs:153 / shell.rs:700 / git_write.rs:938 / worktree_apply.rs:464 |
| 5 | `Select-String .../production.rs -Pattern "repo: true\|filesystem: true\|search: true"` | :435-437 默认 true；:923 RepoModule 注册 |
| 6 | `git grep -n "verify-release-assets" -- .github docs scripts` | .github 零命中；仅 docs + claims/wiring + 脚本注释 |
| 7 | `Select-String .../publish-release.yml -Pattern "sha256\|manifest"` | :92 `release-assets/*.sha256`、:103 integrity 说明；无生成/校验步骤 |
| 8 | `Select-String .../companion-desktop-ci.yml`（jobs/steps） | tauri-shell :34 ubuntu / :68-70 仅 cargo check；svelte-ui :100/:107 仅 pnpm check、check:assets；无 cargo test / pnpm test |
| 9 | `cargo test -p apeireth-governance --lib --locked -j 2` | exit 0；**214 passed / 0 failed** |
| 10 | `cargo test -p apeireth-governance --test governance_chain_drill --locked -j 2` | exit 0；**3 passed / 0 failed** |
| 11 | `cargo test -p apeireth-storage --test crypto_invariants --test sealed_backup_restore --locked -j 2` | exit 0；**7 passed + 10 passed** |
| 12 | `cd frontend/companion-desktop && pnpm test` | exit 0；**82 of 82 suites passed** |
| 13 | `cd frontend/companion-desktop/src-tauri && cargo test --locked -j 2` | exit 0；lib **140 passed/3 ignored** + supervisor_lifecycle **11 passed** = **151 passed**（与交付记录一致） |
| 14 | `python scripts/docs_truth.py --verify` | `claims=24 fail=0 stale=6 review_due=0`；本 12 条全部 OK；6 STALE 均为 bio.*（并发未提交改动） |
| 15 | `git diff --name-only <verified_at>..HEAD -- <12 条 sources>` | 12/12 空（无 STALE） |
| 16 | `git status --short` / `git diff --stat` | 10 M + 2 ??（并发线在途，§0.4）；`claims.yaml` 未改（mtime 15:56:35） |
| 17 | `git grep -n "ENABLE_SHELL_ENV" -- crates/adapters/cli` | `const ENABLE_SHELL_ENV: &str = "APEIRETH_ENABLE_SHELL"`（cli/src/lib.rs:82） |

## 附录 B：建议批次路线（不含等待 owner 决策的时间）

- **Wave 1（低风险、现成资产）**：R8-B2 收口 + claims 登记（2 条）· storage CLI 密钥出口（1 条）· release 发布流水线校验接线（1 条）→ 4 条。
- **Wave 2（证据/CI 批）**：B1/B2/B3 登记 + companion-desktop CI 执行（3 条）· B4 `pnpm test` 入 CI + 登记（1 条）· linux_containment 登记 + Linux CI 证据（1 条）；
- **Wave 3（设计/拍板批）**：storage B6-2/B6-3（owner 拍板 P0 字段/KDF/锚定/迁移后）· R8-B3 反思队列默认关接线 · §7.7 验证面口径裁决（gov.chain_drill_ci / sec.governance_chain_drill）· release wiring 前缀 schema 裁决。

> 本文件只是**方案**：所有代码/状态变更须由归属线在各自提交内完成，并在收口时重跑 `python scripts/docs_truth.py --verify` 与 `python scripts/wiring_report.py`（生成物禁手改）。
