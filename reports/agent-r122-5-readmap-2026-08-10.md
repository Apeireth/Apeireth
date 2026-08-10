# R122-5 readmap — 语义模型路由 (VCP semanticModelRouter 借鉴)

**时间**: 2026-08-10 13:58
**项目**: Apeireth-rust (`.openclaw\workspace\promethean\Apeireth-rust`)
**借鉴 ID**: R122-5-VCP-SemanticModelRouter-2026-08-10
**目标**: 新建 `crates/apeireth-pipeline/src/model_router.rs` + 1 example + 8+ tests
**VCP 借鉴源**: `SemanticModelRouter.json` + `SemanticModelRouter.json.example` (VCP `lioensky/VCPToolBox`)

---

## 1. VCP 借鉴源字段级分析

### 1.1 VCP `SemanticModelRouter.json` 顶层 schema (github API 拉取 base64 解码)

```json
{
  "enabled": true,                       // 全局开关 (我的 port: 0 port, 不加 rule 即禁用)
  "autoModelName": "VCPModelAuto",       // 虚拟模型名 (我的 port: 0 port, 调用方选 preset)
  "defaultPreset": "default",            // 默认 preset (我的 port: 0 port, 简化 flat rules)
  "matchThreshold": 0.18,                // 关键词匹配阈值 (我的 port: hardcode 0.18 in keyword match)
  "contextWeights": [0.7, 0.3],          // 当前 vs 累积权重 (我的 port: 0 port, V2.1 P1 out of scope)
  "presets": {
    "default": {
      "displayName": "VCPModelAuto",
      "defaultModel": "gemini-3.5-flash-thinking",  // 默认模型 → my `default_model`
      "fallbackModels": ["gpt-5.5"],                // fallback 池 (我的 port: 0 port, 简化)
      "matchThreshold": 0.18,
      "contextWeights": [0.7, 0.3],
      "routes": [
        {
          "name": "daily_chat",
          "model": "gemini-3.5-flash-thinking",
          "description": "日常对话、聊天、问候...",   // 关键词列表 (逗号分隔) → my `KeywordMatch(Vec<String>)`
          "failoverPool": true
        },
        {
          "name": "research_and_coding",
          "model": "gpt-5.5",
          "description": "信息检索、资料搜集、调试代码...",
          "failoverPool": true
        },
        {
          "name": "deep_reasoning",
          "model": "claude-opus-4-7-thinking",
          "description": "复杂综合任务、多步骤推理...",
          "failoverPool": false
        }
      ]
    }
  }
}
```

### 1.2 VCP 字段 → Rust port 映射

| VCP 字段 | Rust 字段 | 借鉴/简化决策 |
|----------|-----------|---------------|
| `enabled` | (无) | 简化: 不加 rule 即不启用, 不需要全局开关 |
| `autoModelName` | (无) | 简化: Rust 调用方直接持有 `SemanticModelRouter` 实例, 无虚拟模型层 |
| `defaultPreset` | (无) | 简化: flat rules with priority, 不用 preset 嵌套 |
| `matchThreshold: 0.18` | hardcode 进 keyword match 逻辑 | 简化: 0.18 阈值不暴露, 用 case-insensitive substring match (VCP 实际是 fuzzy match, 1:1 需 embeddings, V2.1 P1 out of scope) |
| `contextWeights: [0.7, 0.3]` | (无) | 简化: out of scope V2.1 P1 (需要真正的 embedding scoring) |
| `presets[name].defaultModel` | `default_model: String` | 1:1 |
| `presets[name].fallbackModels[]` | (无) | 简化: out of scope, 不在 V2.1 P1 |
| `presets[name].routes[]` | `rules: Vec<RoutingRule>` | 1:1 展开 |
| `routes[].name` | `RoutingRule.name: String` | 1:1 |
| `routes[].model` | `RoutingRule.target_model: String` | 1:1 |
| `routes[].description` (keyword list) | `RoutingCondition::KeywordMatch(Vec<String>)` | 1:1 (把逗号分隔字符串 split 成 Vec) |
| `routes[].failoverPool` | (无) | 简化: out of scope |

### 1.3 借鉴 ID (per 07 §1 O-2 走在前人经验上)

**R122-5-VCP-SemanticModelRouter-2026-08-10**

---

## 2. 目标 crate 状态

### 2.1 `apeireth-pipeline` 现状 (R17 LOCKED mtime: 待确认)

- **版本**: workspace = 1.1.0 (per `Cargo.toml:246`) — **0 改**
- **已 lockdep**: `apeireth-protocol`, `apeireth-http-client`, `tokio`, `futures`, `serde`, `serde_json`, `thiserror`, `tracing`, `regex`, `parking_lot`, `bytes`
- **未 lockdep**: `serde_yaml` (但 `apeireth-workflow` 已用 `serde_yaml = "0.9"`, 复用同版本, 0 新增 workspace dep)
- **现有 mod 6 个**: `force_translate`, `placeholder`, `retry_suppression`, `streaming`, `token_budget`, `tool_loop` (R32-2)
- **现有 example**: `examples/pipeline_demo.rs` (236 行, minimaxi OpenAI Chat 真接)
- **现有 `lib.rs:79-84` mod 声明**:
  ```rust
  pub mod force_translate;
  pub mod placeholder;
  pub mod retry_suppression;
  pub mod streaming;
  pub mod token_budget;
  pub mod tool_loop; // R32-2
  ```
- **测试 mod**: `lib_tests` (~9 tests, pipeline 5 步 + hardcode + wiremock e2e)
- **baseline build 状态**: `cargo build -p apeireth-pipeline --quiet` 通过 (0 error, 无 warning)

### 2.2 MessageRole 借鉴 (`apeireth-protocol::normalized.rs:30-39`)

```rust
pub enum MessageRole {
    System, User, Assistant, Tool,
}
```

我的 `RoutingCondition::RoleBased(Role)` 借用此 enum, 不重新定义。

### 2.3 R122-2 / R122-3 协调

- **R122-2**: 也在 `apeireth-pipeline` 加自己的 mod (待查)
- **R122-3**: 也在 `apeireth-pipeline` 加 `tiktoken_counter` (待 R122-3 实施)
- **协调原则**: 我只加 `pub mod model_router;` 到 `pipeline/src/lib.rs` 1 行, **0 改其他 mod 声明**, **0 改 Cargo.toml 已有 dep**, 仅在 `[[example]]` 加自己的 demo 段
- **冲突核验**: `tiktoken_counter.rs` 在写 readmap 时**未存在** (R122-3 还没动), 0 命名冲突

---

## 3. 目标文件清单 (新建, 0 改 LOCKED)

| 文件 | 类型 | 行数估算 | 内容 |
|------|------|---------|------|
| `crates/apeireth-pipeline/src/model_router.rs` | 新建 | ~300 | SemanticModelRouter + RoutingRule + RoutingCondition + 8 tests |
| `crates/apeireth-pipeline/examples/model_router_demo.rs` | 新建 | ~60 | 演示 4 路由规则 |
| `crates/apeireth-pipeline/src/lib.rs` | 改 1 行 | +1 | 加 `pub mod model_router;` (在 mod 声明块) |
| `crates/apeireth-pipeline/Cargo.toml` | 改 1 行 | +1 | 加 `serde_yaml = "0.9"` 依赖 (跟 workflow 一致, 0 新增 workspace dep) |
| `crates/apeireth-pipeline/Cargo.toml` | 改 1 行 | +5 | 加 `[[example]] model_router_demo` |

**0 改**:
- `Cargo.toml:246` workspace.version = "1.1.0" 
- 24 LOCKED crate mtime (含 apeireth-asi)
- 9 器官 logic (body/brain/ear/eye/hand/heart/memory/mind/voice)
- 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱
- 11 agent 公共 API 签名
- 其他 5 mod (`force_translate` / `placeholder` / `retry_suppression` / `streaming` / `token_budget` / `tool_loop`)

---

## 4. 实施计划 (50 min)

### 4.1 `model_router.rs` 设计 (~300 行)

**结构**:
```rust
//! SemanticModelRouter — 借鉴 VCP `SemanticModelRouter.json` (R122-5)
//! 
//! **VCP 借鉴源**: `lioensky/VCPToolBox/SemanticModelRouter.json` (2.7KB JSON config)
//! **借鉴 ID**: R122-5-VCP-SemanticModelRouter-2026-08-10
//! **简化声明**: 0 装 1:1 替代 VCP (per 哲学锚 #1 "不假装已实现")
//! - 0 装 fuzzy embedding scoring (VCP 实际是 0.18 阈值 fuzzy match)
//! - 0 装 failover pool (VCP `failoverPool` 字段 out of scope V2.1 P1)
//! - 0 装 preset 嵌套 (VCP `presets: { name: {...} }` 简化成 flat rules)
//! - 0 装 context weight 累积 (VCP `contextWeights: [0.7, 0.3]` out of scope)
//! 
//! **架构**:
//! - 5 种 condition: KeywordMatch / TokenCountRange / RoleBased / Complexity / Custom (Arc<dyn Fn>)
//! - priority 降序排序 (priority=100 最先匹配)
//! - 第一个匹配的 rule 胜出 (first-match-wins, 跟 VCP `routes[]` 顺序一致)
//! - 无 match 时返 default_model
//! 
//! **借鉴字段** (per 07 §1 O-2 走在前人经验上):
//! - VCP `defaultModel` → Rust `default_model: String` (1:1)
//! - VCP `routes[].name` → Rust `RoutingRule.name: String` (1:1)
//! - VCP `routes[].model` → Rust `RoutingRule.target_model: String` (1:1)
//! - VCP `routes[].description` (keyword list) → Rust `RoutingCondition::KeywordMatch(Vec<String>)` (1:1, split 逗号)
//! - VCP `routes[].failoverPool` → out of scope (V2.1 P1)
//! - VCP `matchThreshold: 0.18` → hardcode 进 keyword match (case-insensitive substring, 1:1 简化)

// 类型定义 ...
// RoutingCondition enum (5 variants)
// RoutingRule struct
// SemanticModelRouter struct
// new, add_rule, route, explain, from_yaml, rules_count
// 8 tests
```

### 4.2 example 设计 (~60 行)

**结构**:
- 演示 4 路由规则: daily_chat / coding / long_input / default fallback
- 用 route() 演示匹配结果
- 用 explain() 演示 matched_rule + reason

### 4.3 `lib.rs` 改 1 行

```diff
 pub mod force_translate;
 pub mod placeholder;
+pub mod model_router; // R122-5: 借鉴 VCP SemanticModelRouter (R122-5-VCP-SemanticModelRouter-2026-08-10)
 pub mod retry_suppression;
 pub mod streaming;
 pub mod token_budget;
 pub mod tool_loop; // R32-2
```

### 4.4 `Cargo.toml` 改 2 行 (1 dep + 1 example)

```diff
 serde = { workspace = true }
+serde_yaml = "0.9"  # R122-5: 借鉴 VCP SemanticModelRouter.json, 复用 workflow 已有版本
 serde_json = { workspace = true }
```

```diff
 [[example]]
 name = "pipeline_demo"
 path = "examples/pipeline_demo.rs"
+
+[[example]]
+name = "model_router_demo"
+path = "examples/model_router_demo.rs"
```

---

## 5. 验收硬指标 checklist

- [ ] `cargo build -p apeireth-pipeline` 0 error
- [ ] `cargo test -p apeireth-pipeline --lib model_router_tests` 8+ passed, 0 failed
- [ ] `cargo test --workspace` 0 failed (19972 + 8+ tests)
- [ ] 0 改 11 agent 公共 API 签名
- [ ] 0 触碰 24 LOCKED (apeireth-asi 0 触碰)
- [ ] 0 改 workspace.version (1.1.0)

---

## 6. 风险 & 决策日志

| # | 决策 | 理由 |
|---|------|------|
| 1 | VCP `description` 字段用 case-insensitive substring match, 不用 fuzzy embedding | VCP 真用 fuzzy match, 0.18 阈值是 fuzzy 阈值; 1:1 需引入 embedding 模型 (e.g. fastembed), V2.1 P1 out of scope; 0 装 |
| 2 | VCP `presets: { name: {...} }` 简化成 flat `Vec<RoutingRule>` | V2.1 P1 单 preset 足够, preset 嵌套是 VCP 多租户设计, 简化掉 |
| 3 | `RoutingCondition::Custom(Arc<dyn Fn(&str) -> bool + Send + Sync>)` | 用户任务明确要求 Arc<dyn Fn>, 借鉴 VCP `routes[].description` 之外的扩展能力 (e.g. ML 评分) |
| 4 | priority 降序排序 (priority=100 最先) | 跟 VCP `routes[]` 顺序一致 (first-match-wins), 但 Rust 显式 priority 字段更灵活 |
| 5 | `serde_yaml` 加到 `apeireth-pipeline/Cargo.toml` 而非 workspace deps | `apeireth-workflow` 已用 `serde_yaml = "0.9"`, 复用同版本避免 workspace dep 膨胀; 0 改 workspace deps 段 |
| 6 | YAML schema 自行设计, 跟 VCP JSON 字段对应 | 任务要求 "VCP 风格 yaml 规则", VCP 实际是 JSON; Rust 端用 serde_yaml 解析, schema 字段对齐 VCP JSON (1:1 字段映射, 0 装) |
| 7 | 0 装 fuzzy match 1:1 替代 VCP | 哲学锚 #1 "不假装已实现"; 在 rustdoc 显式声明 0 装 4 项 (fuzzy / failover / preset / contextWeight) |

---

## 7. 时间预算

- **13:58** readmap (本文档, 10 min) ✓
- **14:08** 实施 (model_router.rs + example + lib.rs + Cargo.toml, 50 min)
- **14:58** verify (cargo build + test + 报告, 17 min)
- **15:15** 截止

---

**R122-5 readmap 完成, 等实施. Mavis 待 review.**
