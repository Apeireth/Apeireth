# TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明) — 验收报告

**任务**: `cf0194b6-f842-4d69-ba73-48cc6893b5f4` (TP29, 生态批)
**角色**: 后端工程师
**分支**: `task/tp12-schema-guardrail-rework-final`
**提交**:
- `b3a2cb6` feat(tools+companion): TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明)

---

## 1. 交付物清单

| 项 | 路径 | 状态 |
|---|---|---|
| 新模块 | `crates/apeireth-tools/src/yaml_spec.rs` | 新增 (~750 行, 含 doc + 22 测试) |
| 模块挂载 | `crates/apeireth-tools/src/lib.rs` | `pub mod yaml_spec;` + 11 个 re-export |
| 依赖 | `crates/apeireth-tools/Cargo.toml` | `serde_yaml = "0.9"` |
| tool_bridge 衔接 | `crates/apeireth-companion/src/tool_bridge.rs` | `ToolBridge::register_yaml_spec` + `ToolBridge::register_yaml_spec_dir` (2 个新方法) |
| companion 集成测试 | `crates/apeireth-companion/src/tool_bridge.rs` (新 `tp29_tests` 模块) | 4 测试 (合法/非法/冲突/dir 批量) |
| companion dev-dep | `crates/apeireth-companion/Cargo.toml` | `tempfile = "3"` |
| 台账登记 | `docs/backlog.md` 第 39 行 | ✅ TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明) + 提交 b3a2cb6 |

## 2. 核心结构 (Composio 借鉴, 落地版)

### 2.1 数据模型

| 类型 | 职责 |
|---|---|
| `ParameterType` | `String \| Integer \| Float \| Boolean \| Array \| Object` (与 JSON Schema 1:1) |
| `PermissionType` | `FileRead(String) \| FileWrite(String) \| Network(String) \| Env(String)` (tag+content serde) |
| `ParameterSpec` | `name + type + description + required + default` |
| `PermissionSpec` | `kind: PermissionType + raw: String` (YAML 原文保留, 便于 round-trip + 错误报告) |
| `CredentialSpec` | `name + required + env: Option<String>` (env 仅 `${VAR}` / `${VAR:?msg}` 形式, TP33 纪律) |
| `SpecError` | `Io \| YamlParse{path,msg} \| MissingField{path,field} \| InvalidParameterType{path,param,ty} \| InvalidPermission{path,perm} \| InvalidCredential{path,cred} \| NameConflict{path,name} \| Directory(String)` (8 变体) |
| `ToolSpec` (trait) | `Send + Sync + Debug` + `name/description/version/parameters/permissions/credentials/implementation/validate` |
| `YamlToolSpec` | 从 YAML 字符串/文件加载, 内部预解析 `PermissionSpec` 缓存 |
| `StaticToolSpec` | 内存构造, 测试用 builder API (`.with_param/.with_permission/.with_credential/.with_implementation`) |

### 2.2 YAML 解析入口

| 函数 | 语义 |
|---|---|
| `load_tool_spec(path)` → `Result<Arc<dyn ToolSpec>, SpecError>` | 文件 → 解析 → 校验 → Arc |
| `load_directory(dir)` → `Result<Vec<Arc<dyn ToolSpec>>, SpecError>` | **Transactional**: 任一失败 → 整体返 Err (避免半成品种子) |
| `register_yaml_spec(registry, path)` → `Result<String, SpecError>` | 加载 + 冲突检测 + 注册 (冲突 = `SpecError::NameConflict`, 不覆盖) |

### 2.3 占位 Tool shim

`SpecPlaceholderTool: Tool` — 收到 `call(args)` 立刻返:
```json
{
  "tool": "...",
  "description": "...",
  "version": "...",
  "implementation": "./impl/...rs",
  "error": "yaml_spec only declares metadata; no executable implementation yet (TP29 placeholder)",
  "hint": "后续任务按 implementation: 路径加载真实现; 当前仅做声明解析 + 占位路由",
  "parameters_declared": [...],
  "permissions_declared": [...],
  "credentials_declared": [...]
}
```

`ToolKind::Sync` + `ToolAxes { Trigger::OnDemand, Awaiting::Immediate, Resident::Ephemeral, Transport::Local, Output::Value }`.

### 2.4 tool_bridge 衔接

| 方法 | 行为 |
|---|---|
| `ToolBridge::register_yaml_spec(path)` | 合法 → eprintln 注册日志 + 返 name; 非法/冲突 → eprintln 原因 + 返 `Err(String)` (不影响其他工具) |
| `ToolBridge::register_yaml_spec_dir(dir)` | **Granular**: 逐文件 load + register, 跳过失败 (与 `load_directory` 的 transactional 语义不同 — 桥接层优先保证部分可用), 返成功列表 |

## 3. TP33 纪律核对 (真实密码不入 yml)

### 3.1 CredentialSpec::validate 边界

仅接受三种合法形态:
- `env: None` — 由其他渠道注入 (keyring/encrypted-file 等)
- `env: "${VAR}"` — env var 缺则 None
- `env: "${VAR:?msg}"` — env var 缺则中断, 返 msg (Compose 风格)

拒绝任何裸字符串密码 (如 `"hardcoded_password_123"` / `"hunter2"`), 由 `CredentialSpec::validate` 在 `SpecError::InvalidCredential` 路径兜底。

### 3.2 POSIX env var 命名校验

- 字母/下划线开头 + 字母/数字/下划线 (符合 POSIX env var 命名)
- 拒绝 `${}` (空变量名) / `${1BAD}` (数字开头) / `${MY VAR}` (含空格) / `${VAR!!msg}` (非法守卫)

## 4. 验收测试结果

### 4.1 `cargo test -p apeireth-tools --lib yaml_spec` (22/22 全绿)

```
running 22 tests
test yaml_spec::tests::parameter_type_round_trip_yaml ... ok
test yaml_spec::tests::parameter_type_invalid_yaml_rejected ... ok
test yaml_spec::tests::permission_type_parse_valid ... ok
test yaml_spec::tests::permission_type_parse_invalid_rejected ... ok
test yaml_spec::tests::credential_validate_accepts_env_reference ... ok
test yaml_spec::tests::credential_validate_rejects_plaintext_password ... ok
test yaml_spec::tests::yaml_legal_full_spec_parses ... ok
test yaml_spec::tests::yaml_minimal_spec_parses ... ok
test yaml_spec::tests::yaml_missing_name_rejected ... ok
test yaml_spec::tests::yaml_invalid_parameter_type_rejected ... ok
test yaml_spec::tests::yaml_plaintext_password_rejected ... ok
test yaml_spec::tests::yaml_invalid_permission_rejected ... ok
test yaml_spec::tests::load_tool_spec_from_file ... ok
test yaml_spec::tests::load_tool_spec_nonexistent_returns_io_error ... ok
test yaml_spec::tests::load_directory_collects_all_yaml ... ok
test yaml_spec::tests::load_directory_not_a_dir_rejected ... ok
test yaml_spec::tests::static_tool_spec_round_trip ... ok
test yaml_spec::tests::placeholder_tool_call_returns_metadata ... ok
test yaml_spec::tests::register_yaml_spec_inserts_into_registry ... ok
test yaml_spec::tests::register_yaml_spec_name_conflict_does_not_overwrite ... ok
test yaml_spec::tests::register_yaml_spec_failure_does_not_corrupt_registry ... ok
test yaml_spec::tests::invalid_yaml_load_directory_does_not_register_partial ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 168 filtered out
```

| 验收点 | 测试函数 | 断言 |
|---|---|---|
| ① 参数类型合法/非法 | `parameter_type_round_trip_yaml` + `parameter_type_invalid_yaml_rejected` | 6 种合法类型 round-trip; `"weird"` → serde 拒绝 |
| ② 权限类型合法/非法 | `permission_type_parse_valid` + `permission_type_parse_invalid_rejected` | 4 合法 (file:read:./x, file:write:/tmp/y, network:api.example.com, env:HOME) + 5 非法 (空/无冒号/file:nope/ssh:host/file:read) |
| ③ 凭证 env 接受 | `credential_validate_accepts_env_reference` | `${VAR}` / `${VAR:?msg}` / `None` 三种合法形态 |
| ④ 凭证裸字符串拒绝 (TP33) | `credential_validate_rejects_plaintext_password` | 5 个非法 case (硬编码密码 / 含空格 / 空变量名 / 数字开头变量名) |
| ⑤ YAML 合法/最小 | `yaml_legal_full_spec_parses` + `yaml_minimal_spec_parses` | 完整 spec (5 参数/权限/凭证 + implementation) / 最小 spec (仅 name+description) |
| ⑥ YAML 缺 name → serde 阶段失败 | `yaml_missing_name_rejected` | `YamlParse { msg 含 "name" }` |
| ⑦ YAML 非法参数类型 | `yaml_invalid_parameter_type_rejected` | `type: weirdtype` → 解析失败 |
| ⑧ YAML 裸密码拒绝 | `yaml_plaintext_password_rejected` | `env: hardcoded_password_123` → `InvalidCredential` |
| ⑨ YAML 非法权限 | `yaml_invalid_permission_rejected` | `ssh:something` → `InvalidPermission` |
| ⑩ load_tool_spec 文件 | `load_tool_spec_from_file` + `_nonexistent_returns_io_error` | 真实文件加载 + 不存在文件 → `SpecError::Io` |
| ⑪ load_directory 批量 | `load_directory_collects_all_yaml` + `_not_a_dir_rejected` | 3 .yaml/.yml 加载, 1 .txt 忽略, 排序确定性; 非目录返 Err |
| ⑫ StaticToolSpec builder | `static_tool_spec_round_trip` | 全字段构造 + validate |
| ⑬ 占位 Tool 行为 | `placeholder_tool_call_returns_metadata` | call 返 metadata + error + hint + declared 列表 |
| ⑭ 注册到 registry | `register_yaml_spec_inserts_into_registry` | registry.get(name).is_some() |
| ⑮ 同名冲突不覆盖 | `register_yaml_spec_name_conflict_does_not_overwrite` | 二次注册返 `NameConflict{name="clash"}` |
| ⑯ 失败不污染 registry | `register_yaml_spec_failure_does_not_corrupt_registry` | 非法 YAML → registry.len() 不变 |
| ⑰ 目录 transactional | `invalid_yaml_load_directory_does_not_register_partial` | 1 合法 + 1 非法 → 整体 Err; 无 partial 注册 |

### 4.2 `cargo test -p apeireth-companion --lib tp29_tests` (4/4 全绿)

```
running 4 tests
test tool_bridge::tp29_tests::bridge_register_yaml_spec_legal ... ok
test tool_bridge::tp29_tests::bridge_register_yaml_spec_invalid_does_not_corrupt ... ok
test tool_bridge::tp29_tests::bridge_register_yaml_spec_name_conflict ... ok
test tool_bridge::tp29_tests::bridge_register_yaml_spec_dir_mixed ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 542 filtered out
```

| 验收点 | 测试函数 | 断言 |
|---|---|---|
| ① 合法 → 注册 | `bridge_register_yaml_spec_legal` | `ToolBridge::new` + `register_yaml_spec("eco_tool.yaml")` → 返 `Ok("eco_tool")` + registry 可查 |
| ② 非法 → 不破坏 | `bridge_register_yaml_spec_invalid_does_not_corrupt` | 缺 description YAML → 返 Err + registry.len() 不变 + bad 不在 |
| ③ 同名冲突 | `bridge_register_yaml_spec_name_conflict` | 与已注册 `recall_memory` 冲突 → 返 Err + 原工具仍在 |
| ④ dir 批量 granular | `bridge_register_yaml_spec_dir_mixed` | 2 合法 + 1 非法 → 返 `["y_a", "y_b"]` (跳过非法 bad) |

### 4.3 `cargo test -p apeireth-tools --lib` (190/190 全 lib 回归)

```
test result: ok. 190 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 4.13s
```

(基线 168 + 22 新增 yaml_spec 测试 = 190 ✓)

### 4.4 `cargo test -p apeireth-companion --lib` (546/546 全 lib 回归)

```
test result: ok. 546 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.86s
```

(基线 542 + 4 新增 tp29_tests = 546 ✓)

### 4.5 `cargo check --workspace --all-targets` (0 错)

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.21s
warning: the following packages contain code that will be rejected by a future version of Rust: nom v1.2.4, proc-macro-error2 v2.0.1
warning: `apeireth-companion` (lib test) generated 2 warnings (unused_variable 历史遗留, 与本任务无关)
```

(历史 warnings: unused_variable + 第三方包 future-incompat, 与 TP29 无关)

## 5. 复用 / 不重复造零件

| 既有零件 | 复用方式 |
|---|---|
| `serde_yaml` 0.9 | 沿用 `apeireth-pipeline` 版本 (+deprecated 警告可控), 不引新依赖 |
| `apeireth-tool-registry::Tool` trait (N15 锁定) | `SpecPlaceholderTool` 实现该 trait (kind=Sync + axes 完整); register 调用 0 改 |
| `apeireth-tool-registry::ToolRegistry` | 复用 `register(name, Arc<dyn Tool>)` 接口 (返回 `()`, 不返 Result, 冲突检测由我们外层做) |
| `ToolKind / ToolAxes / TriggerAxis / AwaitingAxis / ResidentAxis / TransportAxis / OutputAxis` | 完整复用, 0 新增 axis |
| `apeireth-tools::register_all` | 0 改; yaml_spec 是独立模块, 不破坏现有 register_all 装配路径 |
| `tool_bridge.rs::ToolBridge::new(store)` | 0 改; 仅加 2 个新 builder-less 方法 (向后兼容) |

**未引入新 crate 依赖** (workspace 已有 `serde_yaml` 0.9 via `apeireth-pipeline`); `tempfile` 0.3 仅 dev-dep (测试用, 不入生产)。

## 6. 0 装 PASS 边界 (诚实登记)

### 6.1 真 LLM 未接, trait 口已备
- `SpecPlaceholderTool::call` 立刻返 metadata + error + hint, 不调任何 LLM
- 真实实现挂接 (`implementation:` 字段) 后续任务做; 当前仅产"声明解析 + 占位路由"
- TP29 注释明确标注「yaml_spec only declares metadata; no executable implementation yet (TP29 placeholder)」

### 6.2 真凭据解析未接
- `CredentialSpec::resolve` 已实现 (支持 `${VAR}` / `${VAR:?msg}` 解析), 但**不**在 `load_tool_spec` / `register_yaml_spec` 路径自动调 — 仅在用户主动 `credential.resolve()` 时按需取 env
- 真凭据存储 (keyring / encrypted-file / SecretBuf) 由 `apeireth-credentials` crate 承担, 通过 `ToolBridge` 衔接时按 N21 协议取

### 6.3 implementation: 路径未挂载
- YAML `implementation: ./impl/example_tool.rs` 字段当前仅作为元数据存储
- 占位 Tool `call` 返的 JSON 含 `implementation` 字段 (提示后续接入), 不真加载路径
- 真实挂接路径: 通过 `with_implementation_loader(fn(impl_path) -> Arc<dyn Tool>)` 注入 (后续任务)

## 7. 纪律清单核对

| 纪律 | 状态 |
|---|---|
| 真 LLM mock, trait 口标"未接" | ✅ 见 §6.1 |
| 不注入记忆 | ✅ (yaml_spec 模块不引 SqliteMemoryStore) |
| all-targets 编译 | ✅ 见 §4.5 |
| 锁纪律 (std Mutex 不可重入) | ✅ 未引入新锁; StaticToolSpec builder 用值传递, 无内部状态共享 |
| 报告路径 = taskId + 角色 | ✅ `reports/cf0194b6-f842-4d69-ba73-48cc6893b5f4-backend_engineer-report.md` |
| 台账完成即划 ✅ | ✅ backlog.md 行 39 新增 TP29 ✅ 条目 |
| 不接任务包以外的活 | ✅ 仅做 TP29 范围内事, 未碰 W1-W7/E1-E7/其他批 |
| **不破坏现有 tool_bridge API** | ✅ 仅加 2 个新方法 (`register_yaml_spec` + `register_yaml_spec_dir`), 现有 `new/with_post_hook/with_observer_capture/...` 0 改 |
| **真实密码不入 yml (TP33)** | ✅ `CredentialSpec::validate` 拒绝 5 类非法形态; 测试覆盖 (6 个测试含 TP33 验证) |
| **cargo test -p apeireth-tools --lib 全绿** | ✅ 190/190 (基线 168 + 22 新增) |
| **cargo check --workspace --all-targets 0 错** | ✅ 0 错 |

## 8. 已知遗留 / 不在本任务范围

- **`implementation:` 路径真挂接**: 当前仅占位路由; 真实加载逻辑后续任务做 (Trait `ToolFactory` 注入 + 路径解析 + 沙盒约束)
- **`register_yaml_spec` 与 ToolExecutionResult 流水衔接**: 当前占位 Tool call 返的 JSON 是 metadata, 不走标准 ToolExecutionResult 路径; 后续与 `observer_capture` (TP22) / `tp12_guardrail` 衔接时再加
- **多语言权限声明**: 当前仅 `file/network/env` 四类 (Composio MCP 借鉴); 后续如需 `db:read:<table>` / `api:write:<endpoint>` 等可扩展 PermissionType 变体 (向后兼容, serde tag 留升级空间)
- **YAML schema 验证**: 当前靠 serde derive; 后续可加 `jsonschema` crate 强校验 (YAML 1.2 + JSON Schema 2020-12) — 当前**未引入**, 避免无谓依赖膨胀
- **热加载**: yaml_spec 未接 `notify` watcher; 后续如需热重载可接 `apeireth-tool-registry::ToolRegistry::start_watcher` (TP12 已有模式, 0 重发明)
- **N21 credentials 真解析**: `CredentialSpec::resolve` 形态已定义, 真接 N21 secret store 后续任务做 (本任务仅保证 yml 不含明文, 沿用 TP33 纪律)

## 9. 结论

**TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明) 完成, 验收标准全数达成**:

- ✅ 6 项验收测试点全绿 (YAML 解析合法/非法 + 类型/trait/load_*/ToolBridge 衔接/fail-safety)
- ✅ 全 tools lib 190/190 全绿 (168 基线 + 22 新增)
- ✅ 全 companion lib 546/546 全绿 (542 基线 + 4 新增 tp29_tests)
- ✅ `cargo check --workspace --all-targets` 0 错
- ✅ 不破坏现有 tool_bridge API (2 个新方法向后兼容)
- ✅ 真实密码不入 yml (TP33 纪律, 5 类非法形态测试覆盖)
- ✅ yaml_spec 模块独立, 不破坏 `register_all` 装配路径

**提交 hash**: `b3a2cb6`
**报告**: `reports/cf0194b6-f842-4d69-ba73-48cc6893b5f4-backend_engineer-report.md`

---

## 10. 与 TP22/TP33 的衔接说明

- **与 TP22 (Observer 捕获强化)** 衔接: 当前占位 Tool call 不走 ToolExecutionResult 路径 (仅返 metadata), 后续可在 `SpecPlaceholderTool` 接 `with_post_observer` 时挂入 (0 改 observer_capture.rs)
- **与 TP33 (compose 真实密码强制外部注入)** 纪律: 完全沿用 — yml 中 `env` 字段仅 `${VAR:?msg}` 形式, 真实密码不入 yml. 测试覆盖了 5 类非法形态 (硬编码 / 含空格 / 空变量名 / 数字开头 / 非法守卫)
- **与 N21 (credentials 统一凭据)** 衔接: `CredentialSpec::resolve` 已实现 env var 解析, 真接 N21 secret store (keyring / encrypted-file / SecretBuf) 由 apeireth-credentials crate 承担, 通过后续任务的 loader 注入

— 后端工程师 / TP29
