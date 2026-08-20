# Apeireth README 陈旧度审计 — Batch 5/5

**审计员**: Apeireth-rust README 陈旧度审计 sub-agent (Batch 5/5)
**审计日期**: 2026-08-19
**审计范围**: 17 个 crate (apeireth-tool-image-process, apeireth-tool-registry, apeireth-tool-runtime, apeireth-tool-search, apeireth-tool-shell, apeireth-tools, apeireth-tui, apeireth-tui-e2e, apeireth-upgrade, apeireth-value, apeireth-vector, apeireth-verify, apeireth-voice, apeireth-web, apeireth-wiki, apeireth-workflow, release-tools)
**方法**: 读 README.md + Cargo.toml + src/lib.rs 顶部 doc + 列 src/ 文件 + 验证测试数 (grep `#[test]`)
**基线事实 (来自 master HEAD 9bf36b1e, v1.0.0 2026-08-18)**: 85 active crates, 24 LOCKED 仅保 3 项不可变脊柱, 7 重 v7 守门, A3 = 13 键, V0.5 = 30 维 (R126 P1-4 verify done), 8 哲学锚 (S-1/S-2/S-3/O-1 安全优先 NEW/O-2/O-3/O-4/O-5)

---

## 1. apeireth-tool-image-process

**README 摘要**: "R141 image processing tool (multimodal router, OCR placeholder, image hash, EXIF)" — 5 行

**Cargo.toml description**: 同上

**src/ 实际模块**: hash / compat / enhanced / exif / ocr / mcp / register / router / organ_kani_proofs / lib (10 个文件)

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 2. apeireth-tool-registry

**README 摘要**: "R17 战役 2-1: 工具注册中心 (6 类 enum + 5 轴正交 + token 预算三层 + notify 热加载 + 异步任务推送)" — 5 行

**Cargo.toml description**: 同 README 一致

**src/ 实际模块**: types / trait_def / token_budget / registry / async_task / catalog / chain / classifier / handoff / injection / vcp_category / organ_kani_proofs (12 个 pub mod, 远超 README 列举的"6 类 enum + 5 轴正交")

**Stale claim**: ❌ 无 (README 描述是抽象级别, 实际模块展开超出但描述未矛盾)

**Confidence**: —

**修复建议**: 维持现状。如要增详, 可添加模块清单段, 但极简 README 风格一致。

---

## 3. apeireth-tool-runtime

**README 摘要**:
> "R17 战役 2-2: 工具运行时 (parser + executor + record + privacy, 借鉴开源 runtime loop + toolCallRecordStore + toolResultPrivacyGuard + §6.2.2 #18 (origin: open-source))"
> "src 模块: executor / fuzzy / lib / mcp_protocol / organ_kani_proofs / parser / privacy / record"
> "测试数(单测标注): 76"

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: executor / fuzzy / lib / mcp_protocol / organ_kani_proofs / parser / privacy / record / **text_protocol / tool_pipeline** (10 个 .rs)

**Stale claim**:
- **#1**: README 列举的 src 模块清单缺 `tool_pipeline` 和 `text_protocol` 两个模块
  - 证据: `crates/apeireth-tool-runtime/src/lib.rs:60` 声明 `pub mod tool_pipeline;` (注释 "R132.4: pipeline-g5 接入 tool-runtime 生产路径")
  - 证据: `crates/apeireth-tool-runtime/src/lib.rs:62` 声明 `pub mod text_protocol;` (注释 "N10: 宽松文本工具协议层 (VCP vcpLoop TOOL_REQUEST 移植)")
  - 这两个模块 8/18 还在更新, 是 R132.4 / N10 添加的活跃模块

**Confidence**: **HIGH**

**修复建议**: README 第 5 行 src 模块清单应改为:
```
src 模块: executor / fuzzy / lib / mcp_protocol / organ_kani_proofs / parser / privacy / record / text_protocol / tool_pipeline
```

---

## 4. apeireth-tool-search

**README 摘要**: "R145 VSearch: 全文 + 聚合 + TF-IDF 排序内存搜索" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: register + lib (含 SearchEngine, Document, FieldFilter, SortBy 等) — 模块描述吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 5. apeireth-tool-shell

**README 摘要**: "R138: shell extension (real sandbox seccomp/JobObject + russh SSH + persistent tasks + streaming + multi-sig + calculator)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: sandbox / calculator / compat / enhanced / persist / preset / register / ssh / streaming / organ_kani_proofs (10 个 .rs) — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 6. apeireth-tools ⚠️ STALE

**README 摘要**:
> "R17 战役 2-5: 工具集成 (5 trait 真实现: web_search / file_ops / git_ops / code_exec / tool_result)"
> "src 模块: apply_patch / classifier / code_exec / conventions_scanner / file_ops / git_ops / github_accel / grep_ops"
> "测试数(单测标注): 117"

**Cargo.toml description**: 同样声明 "5 trait 真实现"

**src/ 实际模块 (19 个 .rs)**:
- apply_patch / classifier / code_exec / conventions_scanner / file_ops / git_ops / github_accel / grep_ops (README 列了 8 个)
- **未列**: guardrail / lib / long_task / organ_kani_proofs / register / result / schema / web_crawl / web_fetch / web_search / yaml_spec (11 个)

**src/ 实际 TRAIT 数**: `pub const TRAIT_COUNT: usize = 7;` — README/Cargo 都说 "5 trait", 但实际是 7 个 trait (WebSearch / FileOps / GitOps / CodeExec / GrepOps / WebFetch + ToolResult enum)

**测试数**: `grep -c #\[test\]` = 117, 实际匹配。

**Stale claim**:
- **#1 (HIGH)**: README "5 trait 真实现" 描述与实际 `TRAIT_COUNT = 7` 不一致
  - 证据: `crates/apeireth-tools/src/lib.rs:134` `pub const TRAIT_COUNT: usize = 7;`
  - 证据: `crates/apeireth-tools/src/lib.rs:168` 注释 "TRAIT_COUNT = 7 (WebSearch / FileOps / GitOps / CodeExec / GrepOps / WebFetch + ToolResult enum)"
- **#2 (MEDIUM)**: README src 模块清单严重过时, 漏列 11 个模块 (其中 guardrail/schema 是 TP12 (A2, P0) 新增, yaml_spec 是 TP29 新增, long_task / register / result 是 24 LOCKED baseline, web_crawl / web_fetch 是 R230 / R30 U2 新增)
- **#3 (LOW)**: Cargo.toml description 同样声明 "5 trait" — 需同步修正

**Confidence**: **HIGH** (3 处 stale)

**修复建议**:
1. README 描述改为 "7 trait 真实现 (WebSearch / FileOps / GitOps / CodeExec / GrepOps / WebFetch + ToolResult enum)"
2. README src 模块清单应扩展为:
   ```
   src 模块: apply_patch / classifier / code_exec / conventions_scanner / file_ops / git_ops / github_accel / grep_ops / guardrail / long_task / register / result / schema / web_crawl / web_fetch / web_search / yaml_spec
   ```
3. Cargo.toml description 同步改为 "7 trait 真实现"

---

## 7. apeireth-tui

**README 摘要**:
> "R19 + R155 - ratatui 终端版, 5 nav 页面 (舰桥/对话/生长/历史/设置) + 9 器官 + 30+ 后端 crate 全接 + R155 RuntimeBridge 拉 apeireth-runtime 7 模块 (HeartbeatScheduler/AsyncTaskStore/ChanneledBus/ArbitrationLog/SearchEngine/GroupChat/EmotionEngine) 状态供 TUI main loop 渲染"

**Cargo.toml description**: 同 README 一致

**src/ 实际模块**: app / backend / cognition_live / config_watcher / http_llm / observability / organ / pages / command / llm_config / onboarding / persistence / theme / runtime_bridge / organ_kani_proofs / lib — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 8. apeireth-tui-e2e

**README 摘要**: "TUI 5 nav + 9 器官 端到端集成测试 (R20 阶段 5 估补, ratatui TestBackend 测 TUI 设计契约, 干 TUI 不干前端)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: backend / error / harness / nav_e2e / organ_e2e / render / organ_kani_proofs — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 9. apeireth-upgrade

**README 摘要**: "升级器官 (A15 落点 — R14 Phase 5 OTA 升级 + sandbox-validator + 5 重治理)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: council / governance / intent / manifest / monitor / multisig / ota / rollback / sandbox / organ_kani_proofs / cross_crate / self_update — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 10. apeireth-value

**README 摘要**: "价值器官 (A11.3 落点 — R14 Phase 4 动机/价值评估: ValueEvaluation + ValuePrioritization + 5 层原则洋葱一致性 + motivation_score 0.85 门槛)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: evaluation / onion_consistency / prioritization / organ_kani_proofs — 描述与 README 吻合 (motivation_score ≥ 0.85 硬门槛, 5 层洋葱 E/S/A/M/O 一致)

**附注 (src/ 注释 stale, 不在 README 范围)**: `crates/apeireth-value/src/lib.rs:230` 注释 "关联 verdict (来自 apeireth-core 12 键 verdict 守门)" — baseline A3 = 13 键, 这是 lib.rs 注释 stale 不是 README

**Stale claim**: ❌ 无 (README 范围)

**Confidence**: —

**修复建议**: README 维持现状。如严格审计 lib.rs 注释, line 230 "12 键 verdict 守门" 应改为 "13 键 verdict 守门"。

---

## 11. apeireth-vector

**README 摘要**: "向量检索子系统 (VectorStore trait + SqliteVecBackend) — V2 P1 战区 4 skeleton" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: traits / sqlite_backend / qdrant_compat / distance / organ_kani_proofs / error — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 12. apeireth-verify

**README 摘要**: "Apeireth cross-crate regression verification mechanism" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: const_proofs / organ_kani_proofs — 主 lib 内容描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 13. apeireth-voice

**README 摘要**: "Apeireth voice subsystem" — 5 行 (极简, 几乎无描述)

**Cargo.toml description**: 同 README 一致 (极简)

**src/ 实际模块**: real / tone / consciousness_bridge / companion_bridge / bridge_kani_proofs / minimax_live / realtime — 描述与 README 极简风格一致, 未声称具体内容故无矛盾

**附注 (src/ 注释 stale, 不在 README 范围)**:
- `crates/apeireth-voice/src/lib.rs:116` 注释 "0 改 workspace version (1.2.0)"
- `crates/apeireth-voice/src/lib.rs:145` 注释 "0 改 workspace version (1.2.0)"
- 实际 workspace.version = 1.0.0 (baseline); Cargo.toml 中 apeireth-voice 写 `version = "0.1.0"` (没写 workspace inherit, 这是另一 stale 点)
- 这是 src/ 注释 stale, 不是 README stale

**Stale claim**: ❌ 无 (README 范围)

**Confidence**: —

**修复建议**: README 维持现状。如严格审计 src/ 注释, line 116/145 两处 "1.2.0" 应改为 "1.0.0"。

---

## 14. apeireth-web

**README 摘要**: "Web 前端 — Leptos 0.7 SSR + WASM hydration, 让主人能在浏览器真用 Apeireth Council 7 advisor (R18)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: api / api_endpoints (cfg ssr) / app / asi (cfg ssr) / council / council_history / memory / organ_kani_proofs / sovereignty / templates / tool_loop_adapter / verdict — 描述与 README 吻合

**附注 (验证事实)**: apeireth-web/lib.rs:9 注释提到 "ASI 24 维测量可视化" — 经查 `crates/apeireth-asi/src/lib.rs:56` `pub const V05_DIM_COUNT: usize = 24;`, **实际 V0.5 = 24 维** (与 src 描述吻合)。基线写 "V0.5 = 30 维 (R126 P1-4 verify done)" 跟代码事实不符, 但 README 没声明具体数字, 所以 README 不 stale。

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 15. apeireth-wiki

**README 摘要**: "TP28 Markdown 知识库 (llm_wiki 模式, 文件树 + 索引 + 检索)" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: WikiEntry / WikiIndex / WikiStore trait / FilesystemWiki / WikiBlock / WikiContextBlock trait — 描述与 README 吻合

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 16. apeireth-workflow

**README 摘要**: "R152: Temporal-style workflow engine (Activity trait + WorkflowRunner + EventHistory). Borrows temporalio/temporal (13K stars) design, self-impl 0 引外部 dep." — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际模块**: WorkflowError / Event / EventKind / Activity trait / WorkflowContext / Workflow trait / WorkflowRunner / WorkflowWorker — 描述与 README 吻合 (Activity trait + WorkflowRunner + EventHistory + 0 引外部 dep 全 match)

**Stale claim**: ❌ 无

**Confidence**: —

**修复建议**: 维持现状。

---

## 17. release-tools

**README 摘要**: "TP20-S5 塞缝批: 发布期供应链验证 (cargo vet/audit/deny + CycloneDX SBOM) 的工程化载体" — 5 行

**Cargo.toml description**: 与 README 一致

**src/ 实际内容**: VERSION 常量 (env!("CARGO_PKG_VERSION")) + CYCLONEDX_SPEC_VERSION = "1.5" + SBOM_FILENAME = "cyclonedx-sbom.json" + SUPPLY_CHAIN_BLOCK_ON_FAIL = true + 3 个测试 — 描述与 README 吻合

**附注 (src/ 注释 stale, 不在 README 范围)**:
- `crates/release-tools/src/lib.rs:48` 注释 "workspace version 实际值 (1.2.0)"
- 实际 workspace.version = 1.0.0 (baseline)
- 这是 src/ 注释 stale, 不是 README stale

**Stale claim**: ❌ 无 (README 范围)

**Confidence**: —

**修复建议**: README 维持现状。如严格审计 src/ 注释, line 48 "1.2.0" 应改为 "1.0.0"。

---

# 总览

## Stale Claim 统计 (按 crate)

| Crate | Stale Claim 数 | Confidence | 类型 |
|-------|---------------|------------|------|
| apeireth-tool-image-process | 0 | — | — |
| apeireth-tool-registry | 0 | — | — |
| apeireth-tool-runtime | 1 | HIGH | src 模块清单遗漏 tool_pipeline / text_protocol |
| apeireth-tool-search | 0 | — | — |
| apeireth-tool-shell | 0 | — | — |
| **apeireth-tools** | **3** | **HIGH** | "5 trait" 描述与实际 7 不符; src 模块清单严重过时; Cargo.toml description 同步 stale |
| apeireth-tui | 0 | — | — |
| apeireth-tui-e2e | 0 | — | — |
| apeireth-upgrade | 0 | — | — |
| apeireth-value | 0 (README 范围) | — | lib.rs line 230 注释 "12 键" 应改 "13 键" (src/ stale, 非 README) |
| apeireth-vector | 0 | — | — |
| apeireth-verify | 0 | — | — |
| apeireth-voice | 0 (README 范围) | — | lib.rs line 116/145 注释 "1.2.0" 应改 "1.0.0" (src/ stale, 非 README) |
| apeireth-web | 0 | — | — |
| apeireth-wiki | 0 | — | — |
| apeireth-workflow | 0 | — | — |
| release-tools | 0 (README 范围) | — | lib.rs line 48 注释 "1.2.0" 应改 "1.0.0" (src/ stale, 非 README) |

## HIGH confidence stale claim 总数: **4** (apeireth-tool-runtime × 1 + apeireth-tools × 3)

## src/ 内 stale 注释 (附注, 不在 README 范围但应知会):
- apeireth-value/src/lib.rs:230 "12 键 verdict 守门" → 应为 "13 键"
- apeireth-voice/src/lib.rs:116 + 145 "workspace version (1.2.0)" → 应为 "1.0.0"
- release-tools/src/lib.rs:48 "workspace version 实际值 (1.2.0)" → 应为 "1.0.0"

## 报告路径

`_research_mem/sub_agent_reports/2026-08-19/README_audit_batch_5.md`

## 审计员建议 (优先级)

1. **HIGH**: 修正 apeireth-tools README + Cargo.toml description ("5 trait" → "7 trait") + 补全 src 模块清单 (漏列 11 个)
2. **HIGH**: 修正 apeireth-tool-runtime README src 模块清单 (补 tool_pipeline / text_protocol)
3. **LOW**: 修正 apeireth-value / apeireth-voice / release-tools 的 src/ 注释 stale (1.0.0 vs 1.2.0 / 12 键 vs 13 键) — 范围不在 README 审计但建议同步修

## 审计员备注

- Batch 5 范围内 17 crate, 14 个无 README stale, 2 个有 HIGH confidence stale (apeireth-tools + apeireth-tool-runtime)
- 极简风格 5 行 README 占大多数 (14/17), 这些 crate 内容描述在 Cargo.toml `description` 字段 + `src/lib.rs` 顶部 doc, 不易 stale
- 跟 baseline 数字 (85 crates / 24 LOCKED / 13 键 / 7 重 v7 / 30 维 V0.5) 直接矛盾的 README 数字未发现 — baseline 跟代码事实基本一致 (除 V0.5 = 30 维 vs 实际 24 维, 但 README 未声明具体维数)
- 严格审计 src/ 注释时发现 3 处 version 数字 stale (1.2.0 → 1.0.0), 跟 24 LOCKED 仅保 3 项不可变脊柱这条 baseline 升级历史一致 — 是 R128 LOCKED 降级前的注释残留