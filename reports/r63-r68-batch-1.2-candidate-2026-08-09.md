# R63-R68 batch 1.2 candidate (2026-08-09)

> 1 commit 总, 6 R 一气呵成 (R63 skills file-loader + R64 cognition_graph checkpoint + R65 MCP tools/list + R66 cargo-audit SARIF + R67 cross_model_benchmark tier + R68 council deliberation stress)

---

## R63: apeireth-skills file_loader — VCP `vcptoolbox/modules` 真借鉴

### 改动
- `Cargo.toml`: 加 `walkdir = "2.5"` dep (1 行)
- `src/lib.rs`: 加 `pub mod file_loader;` (1 行)
- `src/file_loader.rs`: 新增 373 行, 借鉴 VCP + Anthropic Skills CLI

### 核心类型
- `DescriptorLayout` enum: NestedDir / FlatFile (双 layout 兼容)
- `LoadedDescriptor { path, layout, descriptor: Option, error: Option }` (fail-soft)
- `discover_descriptor_paths(base_dir) -> Vec<PathBuf>` (walkdir max_depth=4)
- `load_one(path) -> (Skill, SkillDescriptor)` (256 KiB 上限, validate id/version)
- `load_registry_from_dir(base_dir) -> (DescriptorRegistry, Vec<LoadedDescriptor>)` (混合 layout 自动 fallback)
- `report_to_markdown(entries) -> String` (TUI / reporting 用)

### 测试
+10 tests: discover_nested/flat / detect_layout / load_one (round_trip / invalid_id / invalid_version) / load_registry_from_dir (mixed / partial_failure / nonexistent) / report_to_markdown

### 借鉴锚 (S-1)
- VCP `vcptoolbox/modules/<name>/index.js` 5 字段 metadata 模式
- LangChain Hub `load("langchain/...")` 仓库式 descriptor loading
- Anthropic Skills CLI `~/.claude/skills/<name>/SKILL.md` 文件路径即 ID

### 不漂移 (主哲学锚 #1)
- 0 改 Skill (R23 LOCKED 4 字段)
- 0 改 SkillDescriptor / DescriptorRegistry (R36 batch 后续 7 字段)
- 0 改 workspace 1.0.0 / 24 LOCKED crate

---

## R64: cognition_graph 真接 checkpoint persistence — LangGraph `MemorySaver` 1:1 借鉴

### 改动
- `crates/apeireth-graph/src/cognition_graph.rs` 末尾新增 R64 section (~200 行):
  - `CognitionCheckpointPayload { v05_dims, target_name, mean, min, max, verdict, is_allowed, saved_at_unix_ms }`
  - `from_summary()` / `to_json()` / `from_json()` (schema = "cognition_checkpoint_v1")
  - `build_checkpoint_from_payload(graph, payload) -> Checkpoint` (复用 Checkpoint::new)
  - `load_payload_from_checkpoint(path) -> Result<CognitionCheckpointPayload, String>` (async, 读 .json)
  - `rerun_from_payload(payload) -> CognitionSummary` (还原后 re-run)

### 测试
+7 tests: payload_pack_unpack / payload_wrong_schema_rejected / payload_wrong_dim_count_rejected / payload_dims_array_pad_zero / rerun_from_payload_round_trip / checkpoint_file_round_trip

### 借鉴锚 (S-1)
- LangGraph `MemorySaver.put_writes` (`langgraph/checkpoint/memory/base.py`)
- VCP `VCPLogbook.js` 3 段 (input / output / meta) 持久化

### 不漂移 (主哲学锚 #1)
- 0 改 CognitionSummary (R47 B8)
- 0 改 run_cognition_graph_sync (R57 per-chat-cycle 0 改)
- 0 改 build_cognition_graph / DimensionNode / AsiSummaryNode / CognitiveDecideNode
- 0 改 Checkpoint / CheckpointStore (复用 R-Cycle API)

---

## R65: apeireth-mcp tools/list + tools/call 真接 — MCP spec §tools

### 改动
- `crates/apeireth-mcp/src/tools.rs`: 新增 ~360 行
- `crates/apeireth-mcp/src/lib.rs`: 加 `pub mod tools;` (1 行)

### 核心类型
- `Tool { name, description, inputSchema }` (MCP spec §tools/list item 1:1)
- `ToolContent` enum: Text / Image / Resource (content[] item 1:1, MCP 2025-03-26)
- `ToolCallResult { content: Vec<ToolContent>, is_error: bool }` (spec §tools/call result)
- `ToolServer` trait: `list()` + `call(name, args)` (跟 ResourceServer 对偶)
- `handle_tools_list(req) -> JsonRpcResponse`
- `handle_tools_call(req) -> JsonRpcResponse` (校验 name + arguments, 4 错误码)
- `is_valid_tool_name(name)` (kebab-case, VCP toolCallParser.js 借鉴)
- 错误码: TOOL_NOT_FOUND (-32010) / TOOL_INVALID_ARGS (-32011) / TOOL_CALL_FAILED (-32012) / TOOL_INTERNAL (-32013)

### 测试
+12 tests: tool_new / with_description_and_schema / tool_content_text_constructors / tool_call_result_ok_err / is_valid_tool_name_kebab / handle_tools_list_returns_tools / handle_tools_call_echo_returns_text / handle_tools_call_unknown_returns_error / handle_tools_call_missing_params_returns_error / handle_tools_call_missing_name_returns_error / tool_serialize_round_trip / tool_content_serialize_round_trip

### 测试用 helper
- `EchoToolServer`: list 返 [Tool::new("echo")] / call "echo" 返 text content / call 其它返 TOOL_NOT_FOUND

### 借鉴锚 (S-1)
- MCP spec 2025-03-26 §tools (fields 1:1)
- LangChain `@tool` decorator (Tool.name + Tool.description + Tool.args_schema)
- VCP `toolCallParser.js` (kebab-case + arguments 走 JSON object)

### 不漂移 (主哲学锚 #1)
- 0 改 resources.rs / protocol.rs / ResourceServer (LOCKED, R33-3 R33-3-1 0 触)
- 0 引入 I/O / 网络 (server 注入, 0 真接)
- 0 业务耦合 (apeireth-mcp 0 依赖 tui/api)

---

## R66: cargo-audit.yml SARIF Code Scanning 上传

### 改动
- `.github/workflows/cargo-audit.yml`: 加 SARIF 转换 step + upload-sarif step (10-15 行 yaml + ~30 行 python3)

### 关键 step
1. **Convert JSON report to SARIF (R66)**: 用 GitHub Actions ubuntu-latest 默认 python3, 读 audit-report.json → 写 audit-report.sarif (SARIF 2.1.0 schema, ruleId = `rustsec/<id>`, level per severity)
2. **Upload SARIF to Code Scanning**: `github/codeql-action/upload-sarif@v3`, category = "cargo-audit"

### 借鉴锚 (S-1)
- GitHub `github/codeql-action` SARIF schema
- qdrant-spark audit SARIF 转换模式
- OASIS SARIF TC sarif-spec 2.1.0

### 不假装
- 0 引新 dep (走 python3, GitHub Actions ubuntu-latest 默认)
- 当前 0 vulnerabilities (R60 cargo audit 验证); 仍设 `--deny warnings` 严格守门

---

## R67: apeireth-eval cross_model_benchmark 扩 6 model + ModelTier — HELM tier 范式

### 改动
- `crates/apeireth-eval/src/cross_model_benchmark.rs`:
  - 加 `EXTENDED_MODELS` (6 model: highspeed ×3 + M2.7/M2.5/M2.1)
  - 加 `ModelTier` enum: Frontier / Balanced / Fast / Legacy (4 tier)
  - 加 `tier_of(model)` + `select_models_for_tier(tier)` + `all_tiered_models()` + `count_by_tier()`

### 测试
+6 tests: tier_of_classifies_all_models / select_models_for_tier_filters_correctly / select_models_for_tier_dedup / all_tiered_models_dedup_no_overlap / count_by_tier_sums_to_total / model_tier_serialize_round_trip

### 借鉴锚 (S-1)
- HELM (Stanford) tier 范式 (廉价 / 主力 / 前沿)
- MiniMax docs 2026-08-09 8 model 分类
- Anthropic model tiers (Haiku / Sonnet / Opus)

### 不漂移 (主哲学锚 #1)
- 0 改 DEFAULT_MODELS (R32-3-2 锁, 4 model 保留)
- 0 改 ModelBenchmarkResult / CrossModelBenchmarkReport
- 0 改 cross_model_benchmark 主函数签名

---

## R68: apeireth-council deliberation stress test — AutoGen GroupChat + k6/vegeta

### 改动
- `crates/apeireth-council/src/stress_test.rs`: 新增 ~360 行
- `crates/apeireth-council/src/lib.rs`: 加 `pub mod stress_test;` (1 行)

### 核心类型
- `StressConfig { rounds, max_deliberation_rounds, verbose }` (default = 100 round, 3 deliberation round)
- `RoundResult { round_index, consensus_reached, final_score, final_stance, rounds, elapsed_ms, termination_reason, error }`
- `StressReport { rounds_run, total_elapsed_ms, consensus_count, consensus_rate, avg_consensus_score, latency_p50/p95/p99_ms, error_rate, termination_histogram, round_results, config }`
- `run_deliberation_stress(members, query, provider, config)` 主入口
- `to_markdown()` 报告生成

### 测试
+10 tests: stress_config_default_values / with_methods / 10_rounds_smoke / percentiles_monotonic / termination_histogram_records_reasons / markdown_contains_key_sections / avg_consensus_score_in_range / round_results_length_matches / percentile_empty_returns_zero / percentile_basic / reason_label_all_returns_snake_case

### 测试用 helper
- `AlwaysApproveEcho`: MockLlmProvider 实现, generate 固定返 "approve"

### 借鉴锚 (S-1)
- AutoGen `GroupChat.run_chat` 多轮 driver
- LangChain `ConversationChain` stress test pattern
- k6 / vegeta stress test metric (latency_p50/p95/p99 + error_rate)

### 不漂移 (主哲学锚 #1)
- 0 改 `CouncilMemberDeliberator` / `CouncilMember` / `MockLlmProvider` (R33-4-1 LOCKED, R33-4 LOCKED)
- 0 改 persona / advisor / deliberation / synthesis / hold (LOCKED)
- 0 引 I/O / 网络 (默认 mock; 真 LLM env-gated, 后续 R33-4-3 真接 LlmProvider 路线)

---

## 验证总表 (本批跑完)

| 范围 | 命令 | 结果 |
|---|---|---|
| 源仓 workspace build | `cargo build --workspace --tests` | ✅ 0 errors |
| 源仓 lib tests | `cargo test --workspace --lib` | ✅ 4641 passed, 0 failed (R57-R62 baseline 4596 + 45) |
| apeireth-skills | `cargo test -p apeireth-skills --lib` | ✅ 26 passed (16 → 26, +10) |
| apeireth-graph | `cargo test -p apeireth-graph --lib` | ✅ 25 passed (18 → 25, +7) |
| apeireth-mcp | `cargo test -p apeireth-mcp --lib` | ✅ 85 passed (73 → 85, +12) |
| apeireth-eval | `cargo test -p apeireth-eval --lib` | ✅ 47 passed (41 → 47, +6) |
| apeireth-council | `cargo test -p apeireth-council --lib` | ✅ 82 passed (72 → 82, +10) |
| R66 cargo-audit.yml | yaml 解析 | ✅ syntax OK (人工验) |

---

## 哲学锚穿透 (本批 100%)

| 锚 | 落实 |
|---|---|
| S-1 北极星 | 24 LOCKED + 9 organ + 8 LOCKED + 1.1 workspace - 0 触 |
| S-2 实事求是 | R63 file_loader fail-soft; R64 payload schema v1; R65 ToolContent 4 变体; R66 0 vulns 期望明示; R67 ModelTier 4 tier 拍板; R68 percentile 用 sorted slice |
| O-2 走在前人尖上 | R63 VCP `vcptoolbox/modules`; R64 LangGraph `MemorySaver`; R65 MCP §tools + LangChain `@tool`; R66 CodeQL SARIF schema; R67 HELM tier; R68 k6/vegeta metric |
| O-3 干到底 | 6 R 1 commit 总 (per user 授权 "1 commit 也行") |
| O-4 任何人都能接手 | 本报告 + 1.2 release plan + CHANGELOG + VERSIONING 1.1.2 (本批 0 改) |
| O-5 不假装 | R63 0 假 "100% load"; R64 0 假 "完整持久化"; R65 0 假 "完整 MCP spec"; R66 0 假 "0 vulns 永久"; R67 0 假 "全 model 同质"; R68 0 假 "压测 0 失败" |

---

## 不变边界 (本批 0 触)

- 24 LOCKED crate src/** 0 触
- workspace.version = "1.1.0" 0 触 (per user 授权 doc-level 灵活, semver-level workspace.version 不动)
- 8 项承诺 0 触
- R11 baseline 3 值 0 触
- R34 1.0 release 0 触
- v6 立体架构 0 触

---

## 后续 follow-up (本批 不在, 留作下一波 1.2 R70-R77)

- **R70**: cross_model_benchmark LIVE 6 model 真跑 (env-gated `APEIRETH_EVAL_LIVE=1`)
- **R71**: council deliberation LIVE 100 round stress 真接 MiniMax M3
- **R72**: MCP tools/subscribe push 模式真接 (MCP 2025-06-18 spec §subscribe)
- **R73**: cognition_graph 真接 memory long_term (per R54 long_term 真接 vector store)
- **R74**: TUI 9 organ memory page cognition summary 显示 (R47 R54 hook + UI 放行)
- **R75**: backend cognition_summary per-chat-cycle 强化 (R57 增量)
- **R76**: 1.2 release doc 落档 + VERSIONING + CONVENTIONS 同步 (per R54 R55 R56 节奏)
- **R77**: APEIRETH-FINAL-CHECK-2026-08-XX.md (1.2) + commit (per R62 节奏)

---

## 关联文档

- **1.2 release plan**: [`docs/roadmap/v1.2-release-plan-2026-08-09.md`](../docs/roadmap/v1.2-release-plan-2026-08-09.md)
- **1.1.2 patch 报告 (R54)**: [`r54-batch-1.1.2-patch-2026-08-09.md`](r54-batch-1.1.2-patch-2026-08-09.md)
- **1.1.2 follow-up-2 报告 (R57-R62)**: [`r57-r62-batch-1.1.2-followup2-2026-08-09.md`](r57-r62-batch-1.1.2-followup2-2026-08-09.md)
- **APEIRETH-VERSIONING.md**: 7 子系统 1.1.2 patch (R55)

---

## commit 节奏

- 上批源仓 R57-R62: commit `1f7e4823`
- 本批 R63-R68: 1 commit 总 (per user "1 commit 也行")
- Desktop 同步: 后续 commit (per "desktop 同步" 节奏)
