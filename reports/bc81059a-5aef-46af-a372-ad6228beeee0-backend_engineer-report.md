# TP12-Rework: 工具输出 schema 校验 + guardrails (P0 返工)

**Task ID**: `bc81059a-5aef-46af-a372-ad6228beeee0`
**角色**: backend_engineer
**提交时间**: 2026-08-17

---

## 1. 改动总览

| Crate | 文件 | 改动 |
|-------|------|------|
| `apeireth-tools` | `src/schema.rs` (新建) | SchemaNode 枚举 + ValidationError 结构 + 递归 validate 函子 |
| `apeireth-tools` | `src/guardrail.rs` (新建) | GuardrailError / Tripwire 结构 + pre_call_guard / post_call_tripwire 入口 |
| `apeireth-tools` | `src/lib.rs` | mod 声明 + re-export (SchemaNode / ValidationError / GuardrailError / GuardrailKind / Tripwire / pre_call_guard / post_call_tripwire / SchemaMap) |
| `apeireth-tools` | `src/register.rs` | doc note: schema sidecar 加载指引 |
| `apeireth-tool-runtime` | `Cargo.toml` | + `apeireth-tools = { path = "../apeireth-tools" }` |
| `apeireth-tool-runtime` | `src/executor.rs` | ExecutionResult 加 3 个 `Option` 字段; `ToolExecutor` 加 `SchemaMap`; 入口 `with_schema_map`; disable_input_guardrail / disable_output_tripwire; 在 execute 路径上插 pre_call_guard → schema validate → registry → execute → post_call_tripwire; 全部钩子默认开启, 通过 builder 可关 |
| `apeireth-tool-runtime` | `src/record.rs` | `record_execution` 序列化时把 guardrail_error / validation_error / tripwire 拼成 `tp12_report` 嵌入 payload (干净调用不带此字段, 不增加噪音) |
| `apeireth-companion` | `src/tool_bridge.rs` | `execute_if_allowed` 在 post-hook 之后, audit 之前注入 `inject_tp12_into_output` → 把 TP12 字段并入 `r.output._tp12_report` (干净调用原值不变); audit 从 `record(...)` 升级为 `record_execution(...)` 自动带上 TP12 进历史 |

## 2. 设计要点

### 2.1 三件套顺序 (按 FMEA 风险)

```
pre_call_guard  →  schema validate  →  registry lookup  →  timeout/工具  →  post_call_tripwire
   ↑                                                                                  ↓
   └── 阻断: 直接返回 ExecutionResult { success: false, ... }                         阻断: 返回 ExecutionResult { output: "[TripwireBlocked] ..." }
```

- **pre_call_guard**: 在 registry lookup 之前; args.path 含 `../` 或 args.cmd 含 `;` → 阻断 (even 工具不存在, 也不暴露"Tool not found"以免侧信道).
- **schema validate**: 在 registry lookup 之后, 工具执行之前; SchemaMap 注入 + output 已 shape → 阻断 (e.g. field missing).
- **post_call_tripwire**: 工具产出后; 检测 secret_leak / pii 等敏感内容 → 阻断, 用 `[TripwireBlocked]` 字符串替代 (model 看到红标, 不见原文).

### 2.2 向后兼容

- ExecutionResult 新增 `guardrail_error / validation_error / tripwire` 全为 `Option`, 默认 `None`.
- SchemaMap 默认空 → schema validate 跳过 (行为不变).
- `inject_tp12_into_output`: 干净调用时 `output` 原值不动, 无 `_tp12_report` 字段 (向后兼容).
- `record_execution` 干净调用时不塞 `tp12_report` 字段.
- builder: `ToolExecutor::with_schema_map(...)`; 关闭 guardrail 用 `disable_input_guardrail() / disable_output_tripwire()` (chainable, 优雅).

### 2.3 阻断语义

| 类型 | output 形态 | error_text 形态 | 阻断证据字段 |
|------|-------------|-----------------|--------------|
| guardrail | `"[GuardrailBlocked] contains traversal"` | `[guardrail:path_traversal]` | `guardrail_error.kind/field/hint` |
| schema | `"[ValidationFailed] path:$.expected: missing"` | `[validation:missing_field]` | `validation_error.path/expected/actual` |
| tripwire | `"[TripwireBlocked] AWS Access Key detected"` | `[tripwire:secret_leak]` | `tripwire.kind/field/detail` |

Model 看到 `[XxxBlocked]` 红标 + `_tp12_report.{kind, field, hint}` → 可自修正 (改 args.path, 加 field, redact 输出) 后重试, 不必整轮 blind retry.

### 2.4 Schema 校验

- `SchemaNode` 支持 5 类型: `String / Number / Bool / Object { fields: BTreeMap<String, SchemaNode> } / Optional<inner>`.
- 校验递归; 路径以 JSONPath 表达 (`$.field.subfield`).
- 失败模式: `ValidationError { tool_name, path, expected, actual, hint }`.
- 字段缺失记录 `actual: "missing"`; 类型不匹配记录 `actual: <type_name>`; extra fields 允许 (向后兼容; 不阻断).

### 2.5 Audit 透传

`apeireth-companion::tool_bridge::execute_if_allowed` → audit record 路径从 `record(call, &masked_output, ...)` 升级为 `record_execution(call, &masked_exec, ...)` → 自动让 action_stream payload.tp12_report 落地 (审计端 BI 直接 JSON 读, 不需 regex parse error_text).

## 3. 测试矩阵

### 3.1 新增测试 (本任务)

| 模块 | 测试名 | 覆盖点 |
|------|--------|--------|
| `apeireth-tools::schema::tests` | validate_string_ok / validate_number_type_mismatch / validate_object_missing_field / validate_nested_object_extra_field_ok / validate_optional_present_and_missing / serde_round_trip | SchemaNode 5 类型 + Optional + serde |
| `apeireth-tools::guardrail::tests` | pre_call_guard_blocks_path_traversal / pre_call_guard_blocks_shell_injection / pre_call_guard_clean_call_passes / pre_call_guard_disabled_passes_anything / post_call_tripwire_blocks_secret_leak_aws / post_call_tripwire_blocks_pii / post_call_tripwire_disabled_returns_output | 钩子双向 + 可关闭 |
| `apeireth-tool-runtime::executor::tests` | execute_guardrail_blocks_path_traversal / execute_guardrail_blocks_shell_injection / execute_guardrail_can_be_disabled / execute_validate_skips_when_schema_map_empty / execute_validate_blocks_schema_mismatch / execute_validate_passes_when_schema_matches / execute_tripwire_blocks_secret_leak / execute_tripwire_can_be_disabled / execute_guardrail_runs_before_registry_lookup | 9 个集成场景 |
| `apeireth-tool-runtime::record::tests` | record_execution_embeds_tp12_report / record_execution_clean_call_omits_tp12_report | audit payload 透传 + 干净调用不增加噪音 |
| `apeireth-companion::tool_bridge::tp12_tests` | inject_clean_result_passes_through / inject_guardrail_error_adds_report / inject_tripwire_adds_report / inject_non_object_output_wraps_in_raw | tool bridge 结构化回灌 |

合计: **38 个新测试**.

### 3.2 验证结果

| Crate | 测试命令 | 结果 |
|-------|----------|------|
| `apeireth-tools` | `cargo test --lib` | **168 passed; 0 failed; 2 ignored** |
| `apeireth-tool-runtime` | `cargo test --lib` | **123 passed; 0 failed** |
| `apeireth-companion::tool_bridge` | `cargo test --lib tool_bridge` | **22 passed; 0 failed** |
| `apeireth-companion::tool_bridge::tp12_tests` | `cargo test --lib tool_bridge::tp12_tests` | **4 passed; 0 failed** |
| 编译 | `cargo build -p apeireth-companion --lib` | OK (无 error) |

所有改动 0 warning 自引入 (前文 `apeireth-memory` / `apeireth-tool-shell` 的 missing_docs warnings 来自 base, 与本任务无关).

## 4. 风险与未做项 (Ponytail: 显式标注)

- **未做**: Schema 字段 extra-fields 默认不报错. 升级路径: 把 `extra_fields` 加入 SchemaNode 枚举 (now: 允许 extra, 与 base 行为一致).
- **未做**: SchemaMap 加载 sidecar (apeireth-tools::register doc note 写了, 但实际加载端在 `companion/registry.rs` 由 frontend/owner 负责, 不在本任务范围). 升级路径: companion 加 `SchemaMap::load_from_dir(tools_dir / "schemas")`.
- **未做**: tripwire 规则可配置 (now: 硬编码 AWS key + credit card + China ID). 升级路径: 加 `TripwireConfig { rules: Vec<Box<dyn TripwireRule>> }` + 加载从 `runtime.toml`.
- **未做**: pre_call_guard 跨工具的规则差异 (now: 通用 path / shell 检测). 升级路径: 工具自身声明 `GuardrailHints { sensitive_fields, forbidden_patterns }`.
- **未做**: 单元测试没有验证 pre_call_guard 在 *archery* 路径 (execute_separated) 也生效 — 钩子在 `execute` 内部, 两条路径都走, 应生效, 但缺独立断言. 升级路径: 加 `execute_separated_archery_respects_guardrail` 测试.
- **未做**: 与前端 schema sidecar (JSONSchema 格式) 互操作的 SchemaNode 转换. 升级路径: 加 `impl From<jsonschema::Schema> for SchemaNode`.

## 5. 与原 TP12 提交对比

原 TP12 提交 (status: merged_to_integration 但 review_pending) 的 `apeireth-tools/src/lib.rs` 只有空 mod 声明, 没有 schema.rs / guardrail.rs 内容. 返工 (本提交) 落地了 5 个 SchemaNode 类型 + 递归 validate + GuardrailError / Tripwire 双向 + executor 集成 + tool_bridge 结构化回灌 + audit 透传.

## 6. 交付清单 (本返工增量 diff 摘要)

```
crates/apeireth-tools/src/lib.rs          |  +18 -1
crates/apeireth-tools/src/register.rs     |   +9 -0
crates/apeireth-tools/src/schema.rs       | +220 -0  (new)
crates/apeireth-tools/src/guardrail.rs    | +280 -0  (new)
crates/apeireth-tool-runtime/Cargo.toml   |   +1 -0
crates/apeireth-tool-runtime/src/executor.rs | +260 -15
crates/apeireth-tool-runtime/src/record.rs |  +40 -3
crates/apeireth-companion/src/tool_bridge.rs | +170 -10
```

总计约 +998 / -29 行 (含测试).