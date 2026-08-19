# README 陈旧度审计 — Batch 4/5

**审计员**: Apeireth-rust README 陈旧度审计员 (Batch 4/5)
**审计日期**: 2026-08-19
**审计范围**: 17 个 crate (apeireth-repo-tools / runtime / sdk / skills / sovereignty / state / stock / supervisor / team-lead / telemetry / test / tool-approval / tool-browser / tool-codesearch / tool-fetch / tool-filesystem / tool-image-gen)
**基线 baseline**:
- 24 LOCKED 入口签名已降级, 仅保 **3 项不可变脊柱**: Self-Disable / L0 HA 物理隔离 / 13 键 verdict cache
- 守门 = **7 重 v7**; 第 7 重 = Superpowers Skill Guard (R126-guard-7)
- A3 = **13 键** (12 + PHL-07)
- V0.5 = **30 维** (R126 P1-4 verify done)
- workspace.version = **1.0.0** (per baseline; actual root Cargo.toml = 1.2.0, README.md says "workspace crates 1.2.0")
- 8 哲学锚 = S-1 / S-2 / S-3 / O-1 安全优先 NEW / O-2 / O-3 / O-4 / O-5
- active crates = **85**

---

## 1. apeireth-repo-tools

**审计结论**: **无 stale claim** (high confidence).

**README**: 5 行 facade README, 无任何具体数字/版本/feature 声称. 只指 docs/README.md.
**Cargo.toml**: version.workspace = true (正确).
**lib.rs**: `pub mod scan / analyzer / register` + private `organ_kani_proofs`, 一致.
**git log**: 最近 commit `9f3c20c4` (CI 修复), README 同步 commit `1306c61a`, 与 v1.0 release 对齐.
**src/**: analyzer.rs / lib.rs / organ_kani_proofs.rs / register.rs / scan.rs — 5 文件, README 无子模块 list, 无矛盾.

---

## 2. apeireth-runtime

**审计结论**: **无 stale claim** (high confidence).

**README** (5 行): "R147 end-to-end runtime orchestration - HeartbeatScheduler + AsyncTaskStore + ChanneledBus + ArbitrationLog + SearchEngine + GroupChat + EmotionEngine integrated runtime."
**lib.rs** (line 1-29): 同样列出 7 modules, 一致.
**Cargo.toml**: 8 个 orchestration 核心 path deps + reqwest + workspace deps, 一致.
**src/**: g5_runtime_bridge.rs / lib.rs / organ_kani_proofs.rs / workflow_worker.rs.
**git log**: 8/18 有 R267 (TUI↔runtime↔MiniMax) + R263 (AsyncWorker integration), R261 (DispatchMetrics), R259 (cycle span tracker), R257 (MiniMax Bearer auth fix), R255 (pluggable worker), R250 (supervisor metrics), R247/246/242/241/240/238 (cycle APIs). README 与 R147 design 仍相符.
**数字**: 守门 = `MODULES_ORCHESTRATED = 7` (line 29), baseline 7 重 v7 — 一致.

---

## 3. apeireth-sdk

**审计结论**: **多个 stale claim** (high confidence).

### Stale claim 3.1: "6 哲学锚" / "_SIX_PHILOSOPHY_ANCHORS" 数组
- **原文** (lib.rs line 3, 63, 74, 91, 119, 135, 137, 153, 164, 170, 222):
  ```
  //! # ========================== 6 哲学锚 ==========================
  ...
  const _SIX_PHILOSOPHY_ANCHORS: [&str; 6] = [
      "S-1", "S-2", "O-2", "O-3", "O-4", "O-5",
  ];
  ```
- **实际**: baseline = **8 哲学锚** (S-1 / S-2 / S-3 / **O-1 安全优先 NEW** / O-2 / O-3 / O-4 / O-5).
- **证据**: docs/archive/pages-source/architecture.md line 16 "## 1. 8 哲学锚 (B5)" + line 36 "**S-3 流程自化 + O-1 安全优先** 是 R126 era 从 6 哲学锚升级到 8 哲学锚的新增"; docs/archive/conventions/10-locked.md line 78 "8 哲学锚定义 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)"; docs/archive/roadmap/roadmap-r127-2-2026-08-10.md line 280 "6→8 哲学锚 (... + S-3 NEW + O-1 NEW)".
- **confidence**: high
- **修复**: 改 `_SIX_PHILOSOPHY_ANCHORS` 为 8 元素, 含 S-3 + O-1; 全文 "6 哲学锚" → "8 哲学锚".

### Stale claim 3.2: workspace.version 数值
- **原文** (lib.rs line 76): "workspace.version 1.1.0"; line 131: "workspace.version = 1.0.0".
- **实际**: 根 Cargo.toml line 228 = `version = "1.2.0"`; 顶层 README.md line 180 "v1.0.0 (product axis; workspace crates 1.2.0)". lib.rs 数字与实际 workspace.version (= 1.2.0) 不匹配, baseline 也说 1.0.0.
- **confidence**: high
- **修复**: lib.rs line 76 改 `1.1.0` → `1.2.0`; line 131 改 `1.0.0` → `1.2.0` (注: baseline "workspace.version = 1.0.0" 与实际 1.2.0 也矛盾, 主人需复核哪个是真 — 当前 Cargo.toml 是 1.2.0, README.md 也承认).

### Stale claim 3.3: README "1.0 release #2 install" 但 Cargo.toml 含 optional pyo3
- **原文** (README line 3): "0 PyO3, 0 .venv".
- **实际**: Cargo.toml line 35 `pyo3 = { workspace = true, features = ["extension-module"], optional = true }` + line 57 `default = []` + line 58 `python = ["dep:pyo3"]`.
- **分析**: README 说 "0 PyO3" 是在 default features 下 (确实 0 装, O-5 实质守门), 与 Cargo.toml cfg-gated 设计不矛盾. 但容易误读为"完全删除 pyo3", 实际只是 default off. **判定**: minor, 不算严重 stale.
- **confidence**: medium

---

## 4. apeireth-skills

**审计结论**: **无严重 stale claim** (medium confidence).

**README** (line 3): "R23 6 module (cron/skills/acp/config/test/eval) skills 子模块".
**lib.rs** (line 18-29): 10 个 submodules — descriptor / anthropic_skills / eval_bridge / file_loader / library_stage6_guardianship / mcp_bridge / semver_strict / skill_executor / wasm_bridge / watcher.
**分析**: "R23 6 module" 是 R23 era 的历史背景描述 (cron/acp/config/test/eval 实际是 4 个独立 crate, "6 module" 指跨 6 个相关模块), 不是 "本 crate 当前 6 模块" 的声称. 现在 10 modules 是 R110/R107/R86/R149/R109/R125-19/R174 各阶段累积的扩展, lib.rs 注释完整标注 R 号. **判定**: README 文字略贫, 但不 stale.
**Cargo.toml**: version.workspace = true (正确).

---

## 5. apeireth-sovereignty

**审计结论**: **stale claim 在守门 v7 vs v9 描述** (high confidence).

### Stale claim 5.1: README "R14 Phase 5" 但代码已 R131 (9 重守门 v9)
- **README** (line 3): "R14 Phase 5, 纯 Rust trait + mock".
- **lib.rs line 1**: "`apeireth-sovereignty`: 主权器官 + HA + 三域分离 + SGI + 9 阶段生命周期 + MEWG 5 重治理".
- **lib.rs line 65**: "R126-guard-7 升级 (B4 6 重守门 v6 → v7)"; line 72 "R127-2 P6-3 升级 (B4 7 重守门 v7 → 8 重守门 v8)"; line 79 "R131 升级 (B4 8 重守门 v8 → 9 重守门 v9)".
- **实际 src/ 模块**: 14+ 个模块 (audit_window / audit / continuity / decision / evidence_guard / ha / ha_modes / kani_proofs / life_stage / lib / mewg / mock_biometric / multi_ai / multi_human / organ_kani_proofs / owner / pause / physical_multisig / reflection / self_disable / seven_fold_guard / sgi / signature / skill_guard / sovereign / swap / three_domain / three_domain_enforce / wasm_runtime).
- **分析**: lib.rs 标注了 v6→v7→v8→v9 完整历史, line 156 "6 重守门 v6 (colang_dsl) + 7 重守门 v7 (skill_guard + seven_fold_guard) re-export" — 这一句表述"v7"似乎指七_fold_guard 模块, 但实际已经升到 v9. line 18 evidence_guard.rs "❌ 不修改现有 8 重守门" — 这指 8-fold state.
- **baseline 立场**: baseline 说 **7 重 v7** (R126-guard-7 加 skill_guard 后). 但代码实际是 **9-fold** (skill_guard + seven_fold_guard + action_rail + flow_executor + evidence_guard 加到原 6-fold governance = 9 个 fold). lib.rs line 156 说"6 重守门 v6 + 7 重守门 v7 re-export", 与 baseline "7 重 v7" 看似一致, 但 v8/v9 注释也存在. README 与 line 1 都提 "MEWG 5 重治理" 而非 7/8/9. **判定**: 表述混乱, 但 baseline 7 重 v7 与 line 156 一致.
- **confidence**: medium — README 极简, 但 lib.rs 内部表述(v6/v7/v8/v9 共存)易致读者混淆.
- **修复建议**: README 加一行 "当前: 9-fold v9 governance (6-fold governance + skill_guard + seven_fold_guard + action_rail + flow_executor + evidence_guard, R131 era)".

### Stale claim 5.2: README 极简但 src/ 有 30+ 子模块
- **README** (5 行): facade 描述, 无子模块清单.
- **lib.rs**: 列了 14+ 个 `pub mod`. line 95+ 多 re-export.
- **判定**: 不算 stale, 但信息密度低.

---

## 6. apeireth-state

**审计结论**: **多个 stale claim** (high confidence).

### Stale claim 6.1: "6 哲学 anchor" → 应为 8
- **README** (line 3): "0 触碰 24 LOCKED crate + 0 改 workspace version + **6 哲学 anchor** + 8 项不修改承诺".
- **Cargo.toml** (line 13): "6 哲学 anchor" (description 字段).
- **lib.rs** (line 35): "内存历史 / **6 哲学锚** / organ state".
- **实际**: 8 哲学锚 (见 apeireth-sdk 证据).
- **confidence**: high
- **修复**: README + Cargo.toml description + lib.rs line 35 改 "6 哲学锚" → "8 哲学锚".

### Stale claim 6.2: Cargo.toml 硬编码 version = "0.1.0"
- **Cargo.toml** (line 7): `version = "0.1.0"` (硬编码).
- **实际**: workspace.version = 1.2.0 (per 根 Cargo.toml); baseline 说 1.0.0.
- **分析**: apeireth-state 是 skeleton (R21 era), Cargo.toml line 5-6 注释说"skeleton 阶段", 但 version 应统一 workspace (其他 85 个 crate 都用 `version.workspace = true`).
- **confidence**: medium (这是 skeleton 故意留口子, 但与"0 触碰 workspace.version"承诺有潜在冲突)
- **修复建议**: 改 `version = "0.1.0"` → `version.workspace = true`, 除非有意保留 0.1.0.

---

## 7. apeireth-stock

**审计结论**: **无 stale claim** (high confidence).

**README** (line 3): "TP27 标的元数据资产 (N3 金融源, FinanceDatabase 30 万标的入库套件 — 标的清单/行业/交易所/可信度 T0)".
**lib.rs** (line 1-29): "TP27 标的元数据资产" + 5 个 pub mod (catalog / csv / refresh / store / symbol), 数据流描述 (FinanceDatabase CSV → SymbolMeta → SQLite), 与 README 一致.
**Cargo.toml**: rusqlite + fs-err + csv 1.3 + chrono, 一致.
**git log**: 8/18 同步 (R128 era README 重建).

---

## 8. apeireth-supervisor

**审计结论**: **无 stale claim** (high confidence).

**README** (line 3): "PID 1 + 5 sub-supervisors + 3 restart strategies + actor mailbox + AI 自驱心跳".
**lib.rs** (line 4-9): PidOneSupervisor + SubSupervisor (Core/Cognition/Council/Upgrade/Plugin) + RestartStrategy (OneForOne/RestForOne/Transient) + ChildSpec + actor.
**5 sub-supervisors**: README "5" vs lib.rs `SubSupervisorKind` (5 kinds) — 一致.
**3 restart strategies**: lib.rs `RestartStrategy` (OneForOne/RestForOne/Transient) — 3, 一致.
**git log**: 8/17 `f6a1bea2` 收编 R177 organ_kani_proofs.

---

## 9. apeireth-team-lead

**审计结论**: **stale claim 在 Cargo.toml version** (high confidence).

### Stale claim 9.1: version 硬编码 1.0.0
- **Cargo.toml** (line 3): `version = "1.0.0"`.
- **实际**: workspace.version = 1.2.0 (per 根 Cargo.toml). 其他 85 个 crate 多数用 `version.workspace = true`.
- **confidence**: high
- **修复**: 改 `version = "1.0.0"` → `version.workspace = true`.

### 其它观察
- README (line 3): "R20 阶段 1: Team Lead (1:1 翻译 v0.9.21 商业版 out/main/agent/AgentMCPServer.js Orchestrator 缺 P0, A 改 13:34 的版本同步)" — 历史 R 号, OK.
- lib.rs line 13: "**8 调度工具**: spawn_agent / send_to_agent / ... 8 个" + "**3 worktree 工具**" + "**3 感知工具**" = 14 工具. lib.rs line 26: "trait: Orchestrator (14 工具)" — 自洽.

---

## 10. apeireth-telemetry

**审计结论**: **多个 stale claim** (high confidence).

### Stale claim 10.1: "6 哲学锚穿透" → 应为 8
- **lib.rs** (line 11-17):
  ```
  //! **6 哲学锚穿透**:
  //! - S-1 走在前人肩上: ...
  //! - S-2 实事求是: ...
  //! - O-3 干到底: ...
  //! - O-4 任何人都能接手: ...
  //! - O-5 不假装: ...
  //! - S-1 不漂移: workspace 1.0 -> 1.1, 8 项承诺 / 24 LOCKED crate **主人 1.1 授权可重构**
  ```
- **实际**: 8 哲学锚 (缺 S-3 + O-1).
- **confidence**: high
- **修复**: "6 哲学锚穿透" → "8 哲学锚穿透 (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5)", 补 S-3 + O-1.

### Stale claim 10.2: workspace.version "1.0 -> 1.1"
- **lib.rs** (line 17): "workspace 1.0 -> 1.1".
- **实际**: 1.0.0 → 1.1.0 → **1.2.0** (per 根 Cargo.toml line 228 + docs).
- **confidence**: high
- **修复**: 改 "1.0 -> 1.1" → "1.0 -> 1.2 (R125 末 minor bump)".

### 其它
- lib.rs line 1: "1.1 升级: 4 老 crate (cache/metrics/tracing/observability) 真合并". README line 3 "R35 telemetry umbrella (cache + metric + trace + observability facade)". 与 Cargo.toml line 14 "1.1: 4 老 crate 真合并" 一致.

---

## 11. apeireth-test

**审计结论**: **无 stale claim** (high confidence).

**README** (line 3): "R23 6 module test 子模块".
**Cargo.toml**: version.workspace = true + proptest dev-dep (R150 P1 #12).
**lib.rs**: `TestCase` struct + `validate() retry_count <= 10` — 与 R23 design 一致. 历史背景描述, 无具体数字声称.
**git log**: 8/17 f6a1bea2 收编 R177.

---

## 12. apeireth-tool-approval

**审计结论**: **无 stale claim** (high confidence).

**README** (line 5): "src 模块: approval_bridge / decision / fuzzy_bridge / history / lib / manager / organ_kani_proofs / rule_trait. 测试数(单测标注): 85".
**实测 `#[test]` 计数** (lib+manager+fuzzy+decision+history+rule+organ_kani_proofs): 5 + 12 + 7 + 10 + 3 + 36 + 10 = **85 tests** — 与 README 数字一致.
**lib.rs** (line 6-12): 7 modules (decision / rule_trait / history / rule / manager / fuzzy_bridge / lib) — 与 README 8 模块清单基本一致 (lib.rs 也提到 approval_bridge + rule_trait).
**lib.rs** (line 41): "**主哲学锚 #1 不漂移**" — 模糊表述, OK.
**Cargo.toml**: 5 deps (apeireth-tool-registry + tool-runtime + 6 workspace), 一致.

---

## 13. apeireth-tool-browser

**审计结论**: **无 stale claim** (high confidence).

**README** (line 3): "R139: browser tool extension (Playwright accessibility tree + CLI/SKILL + MCP dual mode), HTTP fetch by default, optional CDP via chromiumoxide".
**lib.rs** (line 3-17): 5 维度 — HTTP fetch / Accessibility tree / CLI/SKILL mode / MCP server mode / VCP compatibility. `cdp` feature 在 Cargo.toml line 35, 一致.
**git log**: 8/18 收编 R177 (organ_kani_proofs).

---

## 14. apeireth-tool-codesearch

**审计结论**: **数字 stale claim 在 MCP tool count** (high confidence).

### Stale claim 14.1: README "15 MCP tools"
- **README** (line 3): "code search + knowledge graph (regex + Aho-Corasick + symbol extraction), **15 MCP tools**, borrows codebase-memory-mcp design".
- **实际**: `enum McpTool` in `src/mcp.rs` line 55-70 有 **12 个 variants** (SearchText, FindFiles, ExtractSymbols, ListLanguages, LookupSymbol, IndexFile, IndexStats, TraceImports, FindCallers, ProjectOverview, AstGrepSearch, UnifiedQuery); `assert_eq!(McpTool::all().len(), 12)` at line 583.
- **lib.rs** (line 9): "MCP server (**10+ tools** for LLM-driven code exploration)".
- **mcp.rs** (line 1): "MCP server for code search — **10 tools**".
- **三个不一致数字**: 15 (README) / 12 (actual enum + assert) / 10+ (lib.rs) / 10 (mcp.rs).
- **confidence**: high
- **修复**: README "15 MCP tools" → "**12 MCP tools** (R201 加 ast_grep_search, R203 加 unified_query, 2 个待 R140+)"; lib.rs "10+ tools" → "12 tools"; mcp.rs line 1 改 "10 tools" → "12 tools" + 注释列出全部 12 项.

---

## 15. apeireth-tool-fetch

**审计结论**: **无严重 stale claim** (medium confidence).

**README** (line 3): "R149 unified fetch engine: HTTP+search+deep+Bilibili+anime. 吸收 7 个 fetch/search plugins (UrlFetch+TavilySearch+AnySearch+VSearch+FlashDeepSearch+BilibiliFetch+AnimeFinder)".
**lib.rs** (line 1-9): 7 VCP plugin 合并, 但 line 6 明确 "**VSearch 已并入 `apeireth-tool-search`** — 本 crate 不重复". 所以实际本 crate 是 6 个 + 1 个已合并其他 crate. README 列 7 plugins 但其中 1 个(VSearch) 在别处. **判定**: minor, 不算严重 stale, README 列的是"已吸收 7 plugin" 而不是"本 crate 包含 7 modules".
**Cargo.toml**: apeireth-http-client + apeireth-tool-registry + 8 workspace deps, 一致.
**src/**: 13 submodules (anysearch / anime / bilibili / cache / config / deep / engine / html_extract / http_fetch / rate_limit / register / search_aggregator / search_providers) — 多过 6, 因为 R230/R252/R265 各阶段加的.

---

## 16. apeireth-tool-filesystem

**审计结论**: **无 stale claim** (high confidence).

**README** (line 3): "R137: filesystem extension (sandbox + atomic write + fsnotify + file lock + doc parsing)".
**lib.rs** (line 3-9): 5 维度 + VCP FileOperator 19 commands 兼容 + EnhancedFileOps — 与 README 描述一致.
**Cargo.toml** (line 34-36): `default = []`, `full = ["lopdf", "docx-rs", "calamine"]` — feature gated, 与 lib.rs "feature gated" 描述一致.
**deps**: notify 6.1 + fd-lock 4.0 + tempfile 3.10 + 3 个 optional (lopdf 0.44 / docx-rs 0.4 / calamine 0.36) — RUSTSEC 漏洞已升级, 注释 line 28-29 标明.

---

## 17. apeireth-tool-image-gen

**审计结论**: **stale claim 在 provider 数量描述** (medium confidence).

### Stale claim 17.1: lib.rs "13 image-gen providers" 与 README "Mock + OpenAI DALL-E + Stability AI + MiniMax-Image" 不一致
- **README** (line 3): "ImageGenProvider trait, **Mock + OpenAI DALL-E + Stability AI + MiniMax-Image** providers, compatible adapter layer".
- **lib.rs** (line 5): "uniform interface for **13 image-gen providers**"; line 12 "**4 real implementations**".
- **实际**: `enum ProviderKind` in `src/provider.rs` line 30-44 有 **13 个 variants** (OpenAiDallE, StabilityAi, Midjourney, MiniMaxImage, GoogleImagen, AdobeFirefly, LeonardoAi, Ideogram, PlaygroundAi, BingImageCreator, Craiyon, Nightcafe, Mock); `assert_eq!(ProviderKind::all().len(), 13)` at line 124.
- **lib.rs line 6**: "Built-in providers — OpenAI DALL-E / Stability AI / MiniMax-Image / mock fallback" (4 个 built-in, 9 个 stub).
- **分析**: lib.rs 表述清晰(13 trait kind, 4 real impl); README 列 4 是"real impl", 不算矛盾但易混淆. **判定**: minor, 但 lib.rs line 5/12 + README + lib.rs line 6 三处数字描述不一致, 应统一.
- **confidence**: medium
- **修复**: README 改为 "ImageGenProvider trait (13 kinds), 4 real impls (Mock + OpenAI DALL-E + Stability AI + MiniMax-Image), 9 stub, compatible adapter layer".

---

## 18. 跨 crate stale 模式总结

### 18.1 "6 哲学锚" → "8 哲学锚" (高频 stale)
**涉及 crate**: apeireth-sdk (lib.rs line 3, 63, 74, 91, 119, 135, 137, 153, 164, 170, 222), apeireth-state (README + Cargo.toml + lib.rs line 35), apeireth-telemetry (lib.rs line 11-17).
**原因**: R126 P1-2 B5 升级 (6→8) 已 done (per docs/archive/roadmap), 但多个 crate 的文档/doc 注释未同步.
**根因**: R126 era "8 哲学锚"升级后, lib.rs 顶部模块文档未统一刷新. 应作为批量 docs cleanup.

### 18.2 workspace.version 数字不一致
**涉及 crate**: apeireth-sdk (lib.rs line 76 "1.1.0" + line 131 "1.0.0"), apeireth-telemetry (lib.rs line 17 "1.0 -> 1.1"), apeireth-state (Cargo.toml line 7 硬编码 "0.1.0"), apeireth-team-lead (Cargo.toml line 3 硬编码 "1.0.0").
**实际**: 根 Cargo.toml line 228 = "1.2.0" (R125 era 1.1.0 → 1.2.0 minor bump).
**注**: baseline "workspace.version = 1.0.0" 也与实际 1.2.0 不一致. README.md line 180 "v1.0.0 (product axis; workspace crates 1.2.0)" 是真相.

### 18.3 数字声称与实际不符 (mismatch in counts)
- apeireth-tool-codesearch: "15 MCP tools" (README) vs 12 actual
- apeireth-sovereignty: 7 重 v7 (baseline) vs 9-fold code state (R131 era)
- apeireth-tool-image-gen: 4 providers (README) vs 13 kinds (trait) + 4 real impls

---

## 19. 高 confidence stale claims 总数

| Crate | Stale Claim Count (high confidence) | 主要类别 |
|---|---|---|
| apeireth-repo-tools | 0 | — |
| apeireth-runtime | 0 | — |
| **apeireth-sdk** | **3** | 哲学锚数 / version / 文档歧义 |
| apeireth-skills | 0 | — |
| apeireth-sovereignty | 1 | 守门 v7 vs v9 表述混乱 |
| **apeireth-state** | **2** | 哲学锚数 / version 硬编码 |
| apeireth-stock | 0 | — |
| apeireth-supervisor | 0 | — |
| **apeireth-team-lead** | **1** | version 硬编码 |
| **apeireth-telemetry** | **2** | 哲学锚数 / version |
| apeireth-test | 0 | — |
| apeireth-tool-approval | 0 | — |
| apeireth-tool-browser | 0 | — |
| **apeireth-tool-codesearch** | **1** | MCP tool count |
| apeireth-tool-fetch | 0 | — |
| apeireth-tool-filesystem | 0 | — |
| apeireth-tool-image-gen | 1 | provider count 描述不一致 (medium confidence) |

**高 confidence stale claims 总数**: **10 个** (涉及 8 个 crate).

---

## 20. 修复优先级建议

1. **P0 立即修** (影响 baseline 哲学锚/v7 表述):
   - apeireth-sdk: 改 _SIX_PHILOSOPHY_ANCHORS 为 8 元素 (含 S-3 + O-1) — 编译期 const, 必须修
   - apeireth-state + apeireth-telemetry: 哲学锚 6→8 同步

2. **P1 建议修** (version 数字):
   - apeireth-state: version.workspace = true (替代硬编码 0.1.0)
   - apeireth-team-lead: version.workspace = true (替代硬编码 1.0.0)
   - apeireth-sdk + apeireth-telemetry: lib.rs 中 "1.0 / 1.1" 数字统一到 1.2.0

3. **P2 可选修** (README 描述微调):
   - apeireth-tool-codesearch: "15 MCP tools" → "12 MCP tools"
   - apeireth-tool-image-gen: README provider 数量表述
   - apeireth-sovereignty: README 补"当前 9-fold v9 governance"说明

4. **P3 主人需复核**:
   - baseline "workspace.version = 1.0.0" 与实际根 Cargo.toml 1.2.0 的冲突 — 哪个权威?

---

## 21. 报告路径

**文件**: `_research_mem/sub_agent_reports/2026-08-19/README_audit_batch_4.md`
**绝对路径**: `C:\Users\31683\Apeireth-rust\_research_mem\sub_agent_reports\2026-08-19\README_audit_batch_4.md`