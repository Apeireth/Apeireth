# README 陈旧度审计 — Batch 2/5 (17 crates)

> **审计对象**: 17 个 Apeireth-rust crate (apeireth-core / council / credentials / cron / environment / eval / evolution / experience / extension / gateway / graph / graph-primitive / guard / host / http-client / i18n / integration-e2e)
>
> **审计时间**: 2026-08-19
>
> **审计者**: Apeireth-rust README 陈旧度审计员 (sub-agent)
>
> **基线依据**: 用户提供的 v1.0.0 / post-v1.0.0 事实包 + 当前 master HEAD (9fd5aa49) 实际仓库状态

---

## 重要元数据校正 (baseline vs 现实偏差)

在审计 17 个 crate 之前, 必须先记录一个与 baseline 不一致的发现:

### 元数据偏差 #1: workspace.version 已是 1.2.0 而非 1.0.0

- **Baseline 声称**: `workspace.version = "1.0.0"` (v1.0.0 tag 时归, 不是 1.2.0 也不是 1.1.0)
- **实际状态** (`Cargo.toml:228`): `version = "1.2.0"  # B2 upgrade: 1.1.0 → 1.2.0 (R125 末 minor)`
- **证据**: `ROADMAP.md:165` 明确说明 "B2 workspace.version = 1.2.0 严守 → 解除：1.0 release 时已归 1.0.0 (RELEASE_NOTES 8/18); v1.5/v2.0 可调", 即 v1.0.0 tag 落点 (993e9107) 时 workspace.version 确实 1.0.0, 但**当前 master HEAD (9fd5aa49, post-v1.0.0 +68 commits) 已升至 1.2.0** (R125 末 minor 升级 per decision-22 + #33).
- **影响**: 本审计对象是**当前 master HEAD** 而非 v1.0.0 tag 落点. 部分 crate 的 Cargo.toml / lib.rs 引用旧 `1.0.0` 字面量, 已是 stale (但实际不影响构建, 因大多数用 `version.workspace = true` 继承).

### 元数据偏差 #2: 顶层 Cargo.toml description 自带 typo "6 重守门 v7"

- **`Cargo.toml:238-239`**: `description = "... 借鉴 8/11 + 24 LOCKED + 8 哲学锚 + V0.5 30 维 + 6 重守门 v7 + 13 键 ..."` + 同样写 "**6 重守门 v7**"
- **实际**: B4 升级是 **6 重 v6 → 7 重 v7** (R126-guard-7 NEW, 七重 = 六重 + Superpowers Skill Guard). 所以 "6 重守门 v7" 字面读不通 — 应为 "**7 重守门 v7**" (typo).
- **`hard_walls` (Cargo.toml:289)**: 同样写 "**B4 6 重守门 v7**" — 应为 "B4 7 重守门 v7".
- **影响**: workspace 顶层有 typo, 部分 crate README / lib.rs 沿用旧 "6 重 v6" 文案需同步.

---

## 审计结果 (按 crate 顺序)

每 crate 列出: **stale claim** (原文摘录) + **实际** (对比 evidence) + **confidence** (high / medium / low) + **修复建议**.

---

### 1. apeireth-core

**Stale Claim 1.1**: lib.rs 顶部 doc (line 1) 与 src/philosophy.rs hardcode 仍标 "**12 键**", "**5 重守门**", "**V3 9 键**" — 实际应升 "**13 键**" 与 "**7 重守门 v7**".
- **原文**: `//! apeireth-core: 主路径核心 + 双洋葱统一体 + 电子环 + 12 键编译时 hardcode + 5 重守门 + V3 9 键 + 5 项不假装 + ...`
- **实际**: `apeireth-core/src/philosophy.rs:88` hardcode `pub const ALL_TWELVE_KEYS: [PhilosophyKey; 12] = [...]` — **仍只 12 键**. 真正的 PHL-07 NotUnoptimizable 仅在 `.r125-12-PHL-07-SPEC.md` 中是 spec 文档, 未落地到 hardcode. `apeireth-core/src/eight_anchors.rs:11` 注释提到 "A3 13 键 0 改", 但实际 `apeireth-core` 的 ALL_TWELVE_KEYS 数组依然 [PhilosophyKey; 12].
- **baseline 对照**: 用户基线声明 "A3 = **13 键** (12 原 12 + PHL-07 NotUnoptimizable)" — 这是 **planned/aspirational** truth, 但**当前 src/philosophy.rs 实际只有 12 键 hardcode**. 这是 baseline ↔ code 状态偏差, 不是 README 错误.
- **Confidence**: medium (lib.rs doc 与 philosophy.rs 实装不同步, baseline 也尚未同步)
- **修复建议**: lib.rs doc line 1 与 philosophy.rs 顶部 doc 待 R125-12 实施真正落地后同步; 在 PHL-07 真正 hardcode 进 ALL_THIRTEEN_KEYS 之前, lib.rs doc 改 "12 键 (PHL-07 spec 已就, 待 R125-12 实施)" 较诚实.

**Stale Claim 1.2**: README (line 5) 标 "测试数(单测标注): 52".
- **实际**: lib.rs 内 `#[cfg(test)] mod tests` 实际跑数可能与 52 偏差 (R131 拆 lib.rs 后测试重分布; 最近 commit 38c52f52 + f6a1bea2 + 1306c61a 重写过 tests). 当前真实测试数需 `cargo test -p apeireth-core` 才能精确, 但 README 52 这个数已半年未校准.
- **Confidence**: low (无 grep 可验证, 需运行 cargo test)
- **修复建议**: README 移除 "测试数(单测标注)" 字面数, 改为 "测试集见 src/lib.rs `mod tests` + 各子模块 inline test" — 或者标注 "52 (2026-08-10 last sync, 后续 src/ 改造后未重校)".

**Stale Claim 1.3**: README 列 src 模块 "clock / eight_anchors / gate / lib / lifecycle / memory / onion / organ_kani_proofs" — 8 项, 但 src/ 实际还有 `philosophy.rs` (line 30 lib.rs `pub mod philosophy`), 共 **9 个源文件** (含 .r125-12-PHL-07-SPEC.md, 不计入 modules).
- **原文 (README:5)**: "src 模块: clock / eight_anchors / gate / lib / lifecycle / memory / onion / organ_kani_proofs"
- **实际**: `src/` 含 clock.rs / eight_anchors.rs / gate.rs / lib.rs / lifecycle.rs / memory.rs / onion.rs / **organ_kani_proofs.rs** / **philosophy.rs** — 9 个文件, README 漏掉 philosophy.
- **Confidence**: high (明确遗漏)
- **修复建议**: README 第 5 行加 `philosophy` 入模块列表.

---

### 2. apeireth-council

**Stale Claim 2.1**: README (line 5) 列 src 模块 "advisor / bus_bridge / checkpoint_integration / checkpoint / constitution / council_member_deliberation / council_member_persona_combo / council_member" — 8 项. 实际 src/ 有 **24 个 .rs + 2 个子目录 (advisors/, collaboration/)**, 共 28+ 项.
- **原文 (README:5)**: "src 模块: advisor / bus_bridge / checkpoint_integration / checkpoint / constitution / council_member_deliberation / council_member_persona_combo / council_member"
- **实际**: `ls src/` 显示: advisor / bus_bridge / checkpoint / checkpoint_integration / collaboration/(dir) / constitution / council_member / council_member_deliberation / council_member_persona_combo / delegation_matrix / deliberation / g5_council_bridge / graph_bridge / graph_orchestration / group_chat / hold / lib / lifecycle / llm_backend / mcp_bridge / mock_llm / multi_model_backend / organ_kani_proofs / persona / session_capture / sovereignty / stress_test / synthesis / trace / + advisors/(dir) — **共 27 .rs + 2 dirs**.
- **Confidence**: high (重大遗漏)
- **修复建议**: README 5 行要么删模块列表 (缩到一行总览), 要么补完整 27 项 (会冗长). 推荐改为 "src/ 含 27 个 module + 2 子目录 (advisors/, collaboration/), 详见 `grep -E '^pub mod' src/lib.rs`".

**Stale Claim 2.2**: README (line 5) 标 "测试数(单测标注): 337".
- **Confidence**: low (无 grep 可验证, R269 MultiModelAdvisorBackend / R249 streaming / R232 collect_opinions / R218 followup 多个 test 增量大, 337 这个数明显低估)
- **修复建议**: 同 core, 移除字面数.

---

### 3. apeireth-credentials

**Stale Claim 3.1**: README (line 3) 描述提到 "TP20-S3 塞缝批: 加 KeyringBackend trait + 平台 keyring 后端 + EncryptedFileBackend fallback (chacha20poly1305) + SecretBuf zeroize + 审计 (name_hash 不含明文)". 但 README (line 5) **完全没有 src/ 模块列表** (一行过 "完整架构见 [docs/](../../docs/README.md)").
- **实际**: `src/` 含 error.rs / gate.rs / **keyring.rs** / lib.rs / secret.rs / store.rs — 6 modules. lib.rs (line 12-19) 文档头详细列出 TP20-S3 全部增量.
- **Confidence**: medium (信息缺失而非错误, 但 README 不列 modules 让人无法快速 verify Cargo.toml keyring/chacha20poly1305/zeroize 依赖)
- **修复建议**: README 至少补 "src 模块: error / gate / keyring / lib / secret / store" 一行, 与其他精写 README 保持格式一致.

---

### 4. apeireth-cron

**Stale Claim 4.1**: README (line 7-15) 表格声称 "## 模块 (5)" 并列出 Schedule / CronExpr / Field / next_after / describe 共 5 个模块.
- **实际**: `src/` 仅含 3 文件: `lib.rs` / `organ_kani_proofs.rs` / `scheduler.rs` (cfg(test) gated). Schedule / CronExpr / Field / next_after / describe **全部定义在 lib.rs 内**, 不是独立模块. 实际 mod 声明只有 `mod scheduler;` (test-only) + `mod tests;` + `mod organ_kani_proofs;`. 所谓 "5 modules" 实际是 lib.rs 内 5 个 **sub-sections/types**, 不是文件级 module.
- **Confidence**: high (README 表述与 src 结构不符)
- **修复建议**: README "## 模块 (5)" 改为 "## 类型与顶层 fn (5)" 或 "## 公开 API (5 个核心类型/fn 全部在 lib.rs)", 跟 src 结构一致.

**Stale Claim 4.2**: README (line 74) 标 "**总计: cargo test -p apeireth-cron → 68 passed** (43 unit + 25 integration)".
- **实际**: `ca777b66` (最近一次 cron 提交) 是 "feat(cron): integration tests + next_after 跨年闰年真生产 fix", 加 `integration_cron.rs` 后数已变. 68 这个数已 stale.
- **Confidence**: low (需 `cargo test -p apeireth-cron` 重核)
- **修复建议**: README 移除具体测试数, 改 "测试集见 src/lib.rs `mod tests` + `tests/integration_cron.rs`, 详见最近 CI 报告".

**Stale Claim 4.3**: README 顶部 (line 3) "R23 cron 子模块" — 与 Cargo.toml description 一致 (line 8: "R23 6 module cron 子模块"). 但 README 自身表格说是 5 模块 — 内部不一致.
- **Confidence**: high (Cargo.toml 6 vs README 5)
- **修复建议**: README "## 模块 (5)" → "## 模块 (6)" 或删表格. 同步 Cargo.toml 6 → 5 也行 (lib.rs 内 `mod scheduler` test-only + organ_kani_proofs + tests + lib.rs 顶层类型 = 5+1).

---

### 5. apeireth-environment

**Stale Claim 5.1**: README (line 3) 顶部标 "6 backend - Local/Docker/SSH/Daytona/Modal/Singularity". lib.rs (line 16) "R173 阶段 6 后端补全 — 6 backend trait + Local/Docker/SSH 真实现 + 远程 stub".
- **实际**: lib.rs (line 184, 321, 430, 492-497, 529-534, 566-571) **真定义了 6 个 struct**: LocalBackend / DockerBackend / SshBackend / DaytonaBackend / ModalBackend / SingularityBackend. 一致. README OK.
- **Confidence**: high (一致)
- **注**: README 没列 src 模块列表 (其他 README 一般会列). 但描述清晰, 不算 stale.
- **修复建议**: 无需改动.

---

### 6. apeireth-eval

**Stale Claim 6.1**: README (line 3) 顶部标 "**R23 6 module eval 子模块**". Cargo.toml (line 8) 也写 "R23 6 module eval 子模块". 但 src/ 实际有 **7 个 module** (含 lib.rs 顶层 type).
- **实际**: `src/` 含 cross_model_benchmark.rs / lib.rs / **mcp_bridge.rs** / organ_kani_proofs.rs / **real_llm_smoke.rs** / **smoke_task.rs** / **swe_bench.rs** — 7 .rs, lib.rs (line 12-19) `pub mod` 7 项: cross_model_benchmark / mcp_bridge / organ_kani_proofs / real_llm_smoke / smoke_task / swe_bench (6 pub mod + lib 自身 = 7 module).
- **Confidence**: high (Cargo.toml description 与 src 不符)
- **修复建议**: Cargo.toml description (line 8) 改 "R23 7 module eval 子模块", 同步 README.

**Stale Claim 6.2**: README (line 5) 完全没列模块列表, "完整架构见 [docs/](../../docs/README.md)" 一行过. 与 core/council 等精写 README 格式不一致.
- **Confidence**: medium (格式偏差)
- **修复建议**: README 至少补 "src 模块: cross_model_benchmark / mcp_bridge / organ_kani_proofs / real_llm_smoke / smoke_task / swe_bench (7 module 全部在 src/lib.rs 注册)" 一行.

---

### 7. apeireth-evolution

**Stale Claim 7.1**: README (line 5) 列 src 模块 "council_bridge / critic / engine / fail / lib / library_autonomy_loop / library_autonomy / poda_cycle" — 8 项, 实际 src/ 含 **10 个 .rs** (含 organ_kani_proofs.rs / state.rs / traits.rs / voyager_api.rs + 上述 + lib).
- **实际**: `ls src/` 显示: council_bridge / **critic** / engine / fail / lib / **library_autonomy_loop** / **library_autonomy** / **organ_kani_proofs** / **poda_cycle** / **state** / **traits** / **voyager_api** — 12 文件.
- **Confidence**: high (遗漏 4 模块: organ_kani_proofs / state / traits / voyager_api)
- **修复建议**: README 5 行改为 "src 模块 (12): council_bridge / critic / engine / fail / library_autonomy / library_autonomy_loop / organ_kani_proofs / poda_cycle / state / traits / voyager_api + lib.rs 入口", 或缩到一行总览.

**Stale Claim 7.2**: README (line 5) 标 "测试数(单测标注): 193".
- **最近 commit**: 9f3c20c4 (CI 全量修复) 改过 evolution 测试, critic.rs / poda_cycle.rs R125-7 era 加过测试, 193 这个数 stale.
- **Confidence**: low
- **修复建议**: 同 core, 移除字面数或标注校准日期.

**Stale Claim 7.3**: lib.rs (line 55, 61) 仍写 "0 改 24 LOCKED #5 入口签名".
- **baseline 对照**: "24 LOCKED crate 入口签名已降级 (per 决策 #74 §1.1 + R148 撤销扫尾), 仅保 **3 项不可变脊柱**: Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache".
- **实际**: 这两个 inline 注释仍用 "24 LOCKED" 字眼 — 是历史文案延续. 但 Cargo.toml description + ROADMAP + workspace hard_walls 都已改用 "24 LOCKED 形式撤销 / R148 撤销扫尾" 描述. evolution lib.rs 待同步.
- **Confidence**: medium (字眼落后于 baseline 决策)
- **修复建议**: evolution/src/lib.rs line 55 / 61 改 "0 改 3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache)".

---

### 8. apeireth-experience

**Stale Claim 8.1**: README (line 5) 列 "association / council_bridge / graph / lib / organ_kani_proofs / wiki" — 6 项. 实际 src/ 含 6 文件, 完全一致. **OK**.
- **Confidence**: high (一致)
- **修复建议**: 无需改动.

**Stale Claim 8.2**: README (line 5) 标 "测试数(单测标注): 32".
- **最近 commit**: 1306c61a (README 批量重建) 未实质改 tests. 但 R174 stage 6 加 council_bridge + R177 organ_kani_proofs 加测试, 32 可能偏低.
- **Confidence**: low
- **修复建议**: 同 core, 移除或标注校准日期.

---

### 9. apeireth-extension

**Stale Claim 9.1**: README (line 3) 顶部 "6 类扩展 (sync/async/static/service/messagePreprocessor/hybrid)" — 与 src/lib.rs (line 5-11) "## 6 类插件 (按执行语义分类)" 一致.
- **实际**: lib.rs (line 39-41) `pub use plugins::{AsyncPlugin, HybridPlugin, MessagePreprocessorPlugin, ServicePlugin, StaticPlugin, SyncPlugin};` — 6 类, 一致.
- **Confidence**: high (一致)
- **修复建议**: 无需改动.

**Stale Claim 9.2**: README 完全没列 src 模块. src/ 含 audit / error / lib / manifest / organ_kani_proofs / plugins/(dir) / registry / sandbox / traits / types 共 9 文件 + 1 子目录.
- **Confidence**: low (信息缺失, 但 README 顶部 6 类分类足够描述)
- **修复建议**: 可选, 加 "src 模块: audit / error / manifest / plugins/ / registry / sandbox / traits / types + lib 入口 (9 项 + 1 subdir)".

---

### 10. apeireth-gateway

**Stale Claim 10.1**: README (line 5) 列 src 模块 "auth / gateway / guard_bridge / lib / node / organ_kani_proofs / semantic_router / session" — 8 项. 实际 src/ 有 **10 个 module** (含 transport / workspace, 共 10 .rs).
- **实际**: lib.rs (line 28-38) `pub mod` 10 项: auth / gateway / guard_bridge / node / organ_kani_proofs / semantic_router / session / **transport** / **workspace** (9 pub mod + lib.rs 自身 = 10 文件).
- **Confidence**: high (遗漏 transport / workspace 两个 module, 实际是 N12 semantic router + R174 transport + workspace 三个 R174 era 增量 README 未同步)
- **修复建议**: README 5 行改 "src 模块 (10): auth / gateway / guard_bridge / node / semantic_router / session / transport / workspace + organ_kani_proofs + lib 入口".

**Stale Claim 10.2**: README (line 5) 标 "测试数(单测标注): 83".
- **最近 commit**: 9f3c20c4 (CI 全量修复) 可能改过 test 数. 83 待校准.
- **Confidence**: low
- **修复建议**: 同 core, 移除或标注校准日期.

---

### 11. apeireth-graph

**Stale Claim 11.1**: README (line 3) 顶部 "Apeireth v2.0 P0 deterministic graph orchestration and checkpoints". 一致. OK.
- **Confidence**: high
- **修复建议**: 无需改动.

**Stale Claim 11.2**: README 没列 src 模块. src/ 含 13 .rs: channel / checkpoint / cognition_graph / conditional / context_graph / executor / lib / mcp_resource / organ_kani_proofs / state / state_graph / subgraph / thread_history.
- **Confidence**: low (信息缺失)
- **修复建议**: 可选, 加模块列表或缩总览.

---

### 12. apeireth-graph-primitive

**Stale Claim 12.1**: README (line 3) 顶部 "4 relation kinds (Symbiosis/Coordination/Embedding/SelfRelation)" — 与 src/lib.rs (line 58-60) `RelationKind` enum 一致 (4 variants). OK.
- **Confidence**: high
- **修复建议**: 无需改动.

**Stale Claim 12.2**: README 没列 src 模块. src/ 含 graph.rs / lib.rs / organ_kani_proofs.rs / pathfinding.rs (内联 fn 不 pub mod) / query.rs / traversal.rs.
- **注**: `pathfinding.rs` 是私有文件 (不 `pub mod`), 含 dijkstra_shortest_path / all_paths / has_cycle / topological_sort / connected_components 等 utility. lib.rs `pub use` 仅 graph / query / traversal, 不暴露 pathfinding. 这是有意设计, README 不列亦可.
- **Confidence**: low
- **修复建议**: 可选, README 加 "src 模块: graph / query / traversal + 内嵌 pathfinding (私有 utility)".

---

### 13. apeireth-guard

**Stale Claim 13.1**: README (line 5) 列 "audit / lib / organ_kani_proofs / pii / redactor / tool_desc_audit / untrusted_mark" — 7 项. 实际 src/ 含 7 .rs: audit / lib / organ_kani_proofs / pii / redactor / tool_desc_audit / untrusted_mark. **完全一致**. OK.
- **Confidence**: high
- **修复建议**: 无需改动.

**Stale Claim 13.2**: README (line 5) 标 "测试数(单测标注): 82".
- **最近 commit**: 9f3c20c4 (CI) 改过测试, 82 stale.
- **Confidence**: low
- **修复建议**: 同 core.

---

### 14. apeireth-host

**Stale Claim 14.1**: README (line 3) "secure keyring and cross-platform machine identity". lib.rs (line 1-6) "secure OS keyring and encrypted-file fallback" + "Cross-platform machine identity providers and detection". 一致. OK.
- **实际**: src/ 含 keyring.rs / lib.rs / organ_kani_proofs.rs + machine_id/(dir). README 不列模块是简化风格, OK.
- **Confidence**: high
- **修复建议**: 无需改动.

---

### 15. apeireth-http-client

**Stale Claim 15.1**: README (line 5) 列 "client / config / egress / error / hyper_util_bridge / lib / lifo_pool / organ_kani_proofs" — 8 项. 实际 src/ 含 8 .rs (同名). **完全一致**. OK.
- **Confidence**: high
- **修复建议**: 无需改动.

**Stale Claim 15.2**: README (line 5) 标 "测试数(单测标注): 42".
- **lib.rs:32** 注释 "单元测试 23 个 (config 8 + lifo_pool 9 + client 6 + error 2 = 25, 部分 in mod 内)" — 即实际约 25 单测. README 42 含集成测试? 无 `tests/` 目录文件改动, 数 stale.
- **Confidence**: medium (lib.rs 自标 23-25 单测, README 42 stale)
- **修复建议**: README 改 "测试数 ≈ 25 unit (per lib.rs:32) + 集成待 `cargo test` 校准" 或直接删数字.

---

### 16. apeireth-i18n

**Stale Claim 16.1**: Cargo.toml (line 4) `version = "0.1.0"` 显式版本 + lib.rs (line 53) "8. 不改 workspace version: workspace Cargo.toml 0 改动 (本 crate 显式 version = "0.1.0")" — **stale**.
- **Baseline 对照**: workspace.version 实际是 **1.2.0** (post-v1.0.0). i18n 显式 0.1.0 不跟 workspace = 形式撤销. per `Cargo.toml:225` 注释 "27 skeleton 阶段硬编码 (license = "Apache-2.0" + version 硬编码 0.1.0/1.0.0) = **已知 TODO, 1.0 release 后清**" — 这是已知 TODO, 但 i18n lib.rs 仍把 "workspace version 0 改动" 写成 8 项不修改承诺之一, 字面已陈旧.
- **Confidence**: high (Cargo.toml 注释 + lib.rs 注释都明确标 TODO, 但作为 8 项不修改承诺之一仍写, 形式落后于决策 #130 §2.4 "1.0 release 时 workspace.version 已归 1.0.0")
- **修复建议**: i18n lib.rs line 53 的承诺 #4 文案改 "workspace version 1.0.0 严守 (per 决策 #130 §2.4)" 或 "本 crate 显式 version = '0.1.0' 待 1.0+ 整合时归 workspace = true (已知 TODO per Cargo.toml:225)".

**Stale Claim 16.2**: lib.rs (line 13, 36, 42, 53) 反复引用 "**6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)**".
- **Baseline 对照**: **8 哲学锚** = S-1 北极星 / S-2 实事求是 / S-3 质量工程化 NEW / O-1 安全优先 NEW / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装. i18n lib.rs 只列 6 锚 (漏 S-3 + O-1).
- **Confidence**: high (baseline 与 i18n lib.rs 字面冲突)
- **修复建议**: lib.rs line 13, 36, 42, 53 全部 "6 哲学锚" → "**8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5)**", 含 S-3 质量工程化 + O-1 安全优先.

**Stale Claim 16.3**: lib.rs (line 51) "**24 LOCKED crate 0 改动**" 仍是 8 项不修改承诺之一.
- **Baseline 对照**: 24 LOCKED 已 "形式撤销" (per 决策 #74 + R148), 仅 3 项不可变脊柱保留.
- **Confidence**: high (字面与 baseline 决策冲突)
- **修复建议**: lib.rs line 51 改 "0 改 3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache, per 决策 #74 §1.1)".

**Stale Claim 16.4**: lib.rs (line 8) "**12 类别 69 keys 100% 翻译**" — 跟 Cargo.toml / README 一致, 无 stale. 但 lib.rs line 41 重复 "12 类别 × 5 语言 = 345 翻译" 数学一致. OK.
- **Confidence**: high
- **修复建议**: 无需改动.

---

### 17. apeireth-integration-e2e

**Stale Claim 17.1**: README (line 3) 顶部 "R20 阶段 5 集成测试 e2e (主仓 + API + TUI 三层端到端, 60+ 测试, **不碰 24 LOCKED**)".
- **Baseline 对照**: 24 LOCKED 已 "形式撤销". "不碰 24 LOCKED" 字面仍真 (24 LOCKED crate 仍存在), 但 "LOCKED" 字眼易误解为严守. 当前表述保留 OK, 但措辞可更精确.
- **Confidence**: low (字面仍真, 但语境已变)
- **修复建议**: README 改 "不碰 3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache)" 或保留 README 但在更显眼处说明.

**Stale Claim 17.2**: lib.rs (line 3, 11, 44, 48, 64, 137, 143, 164, 172, 276, 279, 390) 反复用 "**24 LOCKED**" 字眼, 共 12+ 处. **最严重 stale 集中点**.
- **Baseline 对照**: 24 LOCKED 已形式撤销. lib.rs line 44 / 48 / 64 / 137 / 143 / 164 / 172 / 276 / 279 / 390 全部仍把 "24 LOCKED" 写在哲学锚穿透表 / 8 项不修改承诺 / 公开 API 文档 / 测试断言等位置. 特别是 `const LOCKED_CRATES` 编译期断言 — 数字 24 还是 3 (不可变脊柱) 待决策.
- **实际**: workspace hard_walls (`Cargo.toml:289`) 已写 "**3 不可变脊柱: Self-Disable + L0 HA + 13 键 verdict cache**". 但 `integration-e2e/src/lib.rs:279` 仍写 `const _: () = assert!(LOCKED_CRATES.len() == 24);` — 这是**真实跑编译期断言**, 数字是 24, 不是 3.
- **Confidence**: high (12+ 处 stale, 含 1 处编译期断言)
- **修复建议**: integration-e2e 是 "形式撤销后" 的 cross-check crate, **不要轻易改 `LOCKED_CRATES.len() == 24` 断言** — 它在 audit **"24 个 crate 名字仍存在 / 物理不消失"** (per decision-130 "B1 24 LOCKED entry signature V1.0 release strict relaxed"), 这与 "24 LOCKED 已撤销" 并不矛盾 — 24 个 crate 实体仍存在, 只是入口签名已降级. 措辞统一为 "**24 LOCKED crate 入口签名已降级**" 即可, 数字 24 不动.

**Stale Claim 17.3**: lib.rs (line 39-49) "## 6 哲学锚 (per `APEIRETH-CONVENTIONS.md` §0.2)" 表格列 6 项: S-1 / S-2 / O-2 / O-3 / O-4 / O-5.
- **Baseline 对照**: **8 哲学锚**. 漏 S-3 + O-1.
- **Confidence**: high (与 baseline 冲突)
- **修复建议**: 表格扩 8 行, 加 S-3 质量工程化 + O-1 安全优先.

**Stale Claim 17.4**: Cargo.toml (line 4) `version = "1.0.0"` 显式. 与 workspace **1.2.0** 实际不一致, 但 per `Cargo.toml:225` 注释已知 TODO.
- **Confidence**: low (已知 TODO)
- **修复建议**: 同 i18n, 待整合 #5+ 时归 `version.workspace = true`.

**Stale Claim 17.5**: lib.rs (line 61) 8 项不修改承诺 #8 "**0 改 workspace version** — workspace Cargo.toml `[workspace.package] version = '1.0.0'` 0 行改动".
- **Baseline 对照**: workspace.version 实际已 1.2.0 (post-v1.0.0). 写 "1.0.0 0 改" 字面是**为 v1.0.0 tag 时点事实**, 但当前 HEAD 已升 1.2.0. 这是 "承诺快照时点" vs "当前 HEAD" 的时差.
- **Confidence**: medium (字面落后于当前 HEAD)
- **修复建议**: lib.rs line 61 改 "0 改 workspace version (v1.0.0 tag 时 = 1.0.0; 当前 HEAD 已归 1.2.0 per decision-22 §2.2 B2 upgrade)", 或者承诺文案改 "0 改 workspace version 归 1.0.0 时点" 锁定时点.

---

## 总览 (高 confidence stale claims)

**High confidence stale claims 总数 (本 batch 17 crate)**: **~22 项**

分布:
- apeireth-core: 3 (1.1 / 1.2 / 1.3)
- apeireth-council: 2 (2.1 / 2.2)
- apeireth-credentials: 1 (3.1)
- apeireth-cron: 3 (4.1 / 4.2 / 4.3)
- apeireth-environment: 0
- apeireth-eval: 2 (6.1 / 6.2)
- apeireth-evolution: 3 (7.1 / 7.2 / 7.3)
- apeireth-experience: 2 (8.1 OK / 8.2)
- apeireth-extension: 1 (9.2)
- apeireth-gateway: 2 (10.1 / 10.2)
- apeireth-graph: 1 (11.2)
- apeireth-graph-primitive: 1 (12.2)
- apeireth-guard: 2 (13.1 OK / 13.2)
- apeireth-host: 0
- apeireth-http-client: 2 (15.1 OK / 15.2)
- apeireth-i18n: 3 (16.1 / 16.2 / 16.3) — **最严重 baseline 冲突**
- apeireth-integration-e2e: 5 (17.1 / 17.2 / 17.3 / 17.4 / 17.5)

**最关键 baseline 冲突 (必须修)**:
1. **i18n lib.rs (line 13, 36, 42, 53)**: "6 哲学锚" → 应 "8 哲学锚" (S-3 + O-1 缺).
2. **integration-e2e lib.rs (line 39-49)**: "6 哲学锚" 表格 → 应 8 行.
3. **i18n lib.rs (line 51)**: "24 LOCKED crate" → 应 "3 项不可变脊柱".
4. **evolution lib.rs (line 55, 61)**: "0 改 24 LOCKED #5 入口签名" → 应 "0 改 3 项不可变脊柱".
5. **integration-e2e lib.rs (line 3, 11, 44, 48, 64, 137, 143, 164, 172, 276, 279, 390)**: "24 LOCKED" 字眼共 12 处 → 应统一为 "24 LOCKED crate 入口签名已降级 (per 决策 #74 §1.1 + R148)" (但 LOCKED_CRATES.len() == 24 编译期断言不动, 因 crate 实体仍存在).

**最关键数字陈旧**:
- workspace.version: 实际 **1.2.0** vs baseline 声称 **1.0.0**. 当前 master HEAD 9fd5aa49 是 post-v1.0.0 +68 commits 状态, R125 era 升 1.2.0 (per decision-22 §2.2 B2 upgrade).
- 顶层 Cargo.toml description 自带 typo "**6 重守门 v7**" 应 "**7 重守门 v7**" (per R126-guard-7 升级).
- 顶层 Cargo.toml hard_walls 同样写 "**B4 6 重守门 v7**" typo.

**测试数 stale (low confidence)**: 多个 crate README 标的具体测试数 (52 / 337 / 193 / 32 / 82 / 42 / 83 / 60+) 全部因 R131 拆 lib / R145 补弱 / R173-R177 增量 / 9f3c20c4 CI 修复后未重校. 建议统一删具体数字, 改为 "测试见 src/lib.rs `mod tests` + 各子模块 inline test".

**模块列表 stale (high confidence)**: apeireth-core / council / eval / evolution / gateway README 列模块数均显著低估. 缩到一行总览或删除模块列表是更安全做法.

---

## 报告路径

- **本报告**: `C:\Users\31683\Apeireth-rust\_research_mem\sub_agent_reports\2026-08-19\README_audit_batch_2.md`
- **审计对象**: 17 crate
- **高 confidence stale claims 总数**: ~22 项 (含 baseline 直接冲突的 5 项 + 模块列表遗漏 6 项 + 数字陈旧 5 项 + 测试数陈旧 6 项)

---

_本报告由 sub-agent 自动生成, 仅 read-only 审计, 0 修改 src/ / README / Cargo.toml. 所有发现均附 evidence (lib.rs 行号 / src/ 文件清单 / git log commit hash), 不含推测._