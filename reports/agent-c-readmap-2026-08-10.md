# Agent-C 战区读图 (C1 阶段产物)

**作者**: Agent-C (Mavis 派活, 2026-08-10 02:55 起)
**目标**: 9 product crate 补 integration test, 0 改 src/, 0 触碰 LOCKED

---

## 0. Baseline

```
$ python 算 baseline
baseline: passed=2293, failed=1, ignored=4, total=2298
```

**注意**: pre-existing 1 failed 是 `workspace_e2e::tests::test_workspace_8_promises_audit_passes_runs`
(跟 8 项不修改承诺 守门相关, Mavis 已知, 不归我修)。

**任务硬指标**: workspace 总 test count +75 (从 2298 → 2373, 忽略任务文档里 "4921→4996" 的旧数字)。

---

## 1. 9 个 product crate 公共 API + 已有 test 状态

| # | Crate | Public API 核心 | 已有 integration test | 缺哪类 |
|---|---|---|---|---|
| 1 | `apeireth-tools` | `ToolResult` / `FileOps`+`StdFileOps` / `GitOps`+`GitCliOps` / `CodeExec`+`ShellCodeExec` / `WebSearch`+`HttpWebSearch` / `apply_patch::parse_patch` / `conventions_scanner::ProjectConventions::scan` | `tests/e2e.rs` 17 tests (file_ops 7 / code_exec 4 / git_ops 4 / const 2) | apply_patch 解析 + 编辑 / conventions_scanner 扫 / ToolResult serde 边界 / FileOps edit 操作 |
| 2 | `apeireth-tool-registry` | `ToolRegistry` (CRUD) / `MockSyncTool` 等 6 mock / `TokenBudget` (`truncate_to_token_budget`) / `Tool` trait (async) | `tests/registry.rs` 11 tests (CRUD 9 / call 2) | token_budget 截断 / list_by_kind 边界 / 多工具并发 register / ToolKind::all() |
| 3 | `apeireth-tool-runtime` | `ToolCallParser::parse` / `FuzzyToolMatcher::match_tool` / `levenshtein_distance` / `PrivacyGuard::mask` / `RecordStore` | `tests/parser.rs` 8 tests (parser 全) | fuzzy 边界 (0/1/2/3 distance + case-insensitive) / privacy 嵌套 (13 敏感键 + 7 token + env assignment) / 私有 token_budget 边界 |
| 4 | `apeireth-tool-approval` | 5 rules (Trust/Risk/Frequency/Whitelist/Blacklist) + `ApprovalManager::check` + `match_tool_name` fuzzy 桥 | `tests/rules.rs` 16 tests (5 规则 + manager 9 + 黑/白 2) | FrequencyRule 真实频率窗口 / RiskRule categories 自定义 / fuzzy_bridge threshold / decision 序列化 / check 历史累积 |
| 5 | `apeireth-pipeline` | `Pipeline::run` 5 步 + `Pipeline::run_streaming` / `PipelineConfig` / `RetrySuppression` / `placeholder` / `force_translate` / `token_budget` | `tests/pipeline.rs` 8 tests (4 协议 e2e + 4 错误) | streaming 端到端 / suppression window 真抑制 / token_budget truncate 边界 / placeholder 递归 / force_translate 切换 / tool_loop 循环控制 |
| 6 | `apeireth-agent` | `Agent` 6 字段 / `AgentManager` CRUD + alias 解析 + LRU cache + event log | `tests/agent.rs` 14 tests (Agent 4 + manager 10) | cache LRU 触发 (容量 + 访问) / event 5 variant 区分 / alias 重复检测 / contains edge |
| 7 | `apeireth-protocol` | 4 Bridge (`OpenAiChatBridge` etc) / `ProtocolKind::parse` / `encode_for_kind` / `decode_for_kind` / `is_tool_result_error` / WS 8 帧 | `tests/wire_format.rs` 19 tests (4 协议 round-trip + bridge dispatch + parse) | tool_choice 归一化 5 模式 / tool_call id 提取 / finish_reason 4 种 / WS 8 帧 round-trip / bridge_ext (Passthrough/Queue/Stream) 5 个 |
| 8 | `apeireth-tui` (binary, 走 `#[path]` 模式) | `CognitionLiveTracker` (4 LiveEvent) / `Theme` / `App` 状态机 / 5 nav | `tests/` 25 文件 (organ_* / nav_* / app_state / theme / error / http / http_test) | `cognition_live::CognitionLiveTracker` 4 事件 + stale 边界 + mark_seen — 新开 `cognition_live.rs` |
| 9 | `apeireth-web` | `html_escape` / `render_error_page` (templates module) | `tests/templates.rs` 12 tests (escape 7 + page 5) | XSS 边界 (img onerror / javascript: URL) / Unicode / 大字符串 / 嵌套 escape / 重复转义防 double-encode — 新开 `templates_ext.rs` |

**跳过**:
- `apeireth-vector` (agent A 在改 src/)
- `apeireth-mcp` (R70-R72 已加 9 tests)
- `apeireth-api` (agent B 在改 src/)

---

## 2. 跨 crate 集成测试覆盖空白

- **parser → fuzzy → registry** (runtime): 已有 partial e2e, 加 typo 容忍 (LLM 拼错 `WeatherQuary` 仍能匹配 `WeatherQuery`)
- **registry → approval**: 已有 partial, 加 5 规则按顺序短路 + Frequency 真实时间窗
- **pipeline → protocol**: 已有 4 协议 wiremock, 加 streaming + 5xx 流式 + 抑制窗口真抑制
- **privacy → tool-result**: 13 敏感键 + 7 token + env assignment 真测

---

## 3. 决策 (0 拍板)

- **mock 库**: 手写 mock Tool (task 已有示例, 0 引入新 dep)
- **property test 库**: 不上 (任务说 "workspace 已有 proptest" 但需要 dev-dep 改动, 风险大; 用固定 boundary input 覆盖)
- **test 文件组织**: 大部分追加到现有 `tests/<name>.rs` (1 个文件易查), tui 新开 `cognition_live.rs` (binary 模式), web 新开 `templates_ext.rs` (大文件分块), protocol 新开 `wire_format_ext.rs` (boundary case)
- **tui 走 `#[path]` 模式**: 跟现有 25 文件一致 (`app_state.rs:9-25` 模板)
- **不引入新 dev-dep**: `tempfile` / `wiremock` / `tokio` 全部 workspace 已有 (已验证)

---

## 4. 阶段分配 (75+ 新 test)

- **apeireth-tools**: +7 (apply_patch parse + apply 6 ops + conventions_scanner + ToolResult serde)
- **apeireth-tool-registry**: +6 (token_budget truncate + list_by_kind 多 + 6 mock call + ToolKind::all)
- **apeireth-tool-runtime**: +9 (fuzzy 5 距离 + privacy 4 边界)
- **apeireth-tool-approval**: +8 (Frequency 真实时间窗 + Risk 自定义 cat + fuzzy_bridge 3 + manager 历史累积 2)
- **apeireth-pipeline**: +7 (streaming e2e + suppression 真抑制 + token_budget truncate + placeholder 循环 + tool_loop)
- **apeireth-agent**: +8 (cache LRU 容量 + event 5 variant + alias 重复 + multi-alias + clear events)
- **apeireth-protocol**: +8 (tool_choice 4 + WS 4 帧 + bridge_ext)
- **apeireth-tui**: +8 (CognitionLiveTracker 4 事件 + stale + mark_seen + check_for_update)
- **apeireth-web**: +10 (XSS 5 + Unicode + 大字符串 + 嵌套 escape + 多特诊)

**目标 +71**, 容忍小幅浮动, ≥ 75 含扩展。

---

## 5. 风险

- **tui binary 编译慢**: 走 `-p apeireth-tui` 单独编译, 改 `#[path]` 跟现有模式一致
- **pipeline 4 协议 wiremock 现有**: 我不动现有 8 tests, 只在末尾追加
- **vector / api 跳过**: 任务明确说不要改 src, 我也不动 tests/ 避免跟 A/B 冲突
- **pre-existing 1 failed**: 不是我引入, baseline 已记录

---

**签**: Agent-C, 2026-08-10 02:55-03:25 C1 完成, 准备开 C2.
