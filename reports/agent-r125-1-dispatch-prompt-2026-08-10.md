# R125-1 Sub-Agent Dispatch Prompt (LiteLLM Provider Registry)

**Date**: 2026-08-10 17:28
**Author**: R125 P0 supervisor (mvs_47dd64fb4fc24e23b30edd5f649bfebb session, dispatched 17:23)
**Receiving agent**: R125-1 sub-agent (Mavis 派)

---

## 任务 (per 主人 17:22 升级授权 + decision-33)

**主题**: LiteLLM Provider Registry — 借鉴 LiteLLM 的多 Provider 路由注册表模式, 在 `apeireth-pipeline` crate 新增 `provider_registry` 模块, 统一管理 50+ LLM Provider 的注册/选择/降级.

**借鉴 ID**: `R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\LiteLLM\`

**目标文件**:
- `Apeireth-rust/crates/apeireth-pipeline/src/provider_registry.rs` (NEW mod, ~500-800 行)
- `Apeireth-rust/crates/apeireth-pipeline/src/lib.rs` (M: add `pub mod provider_registry;` + 1 re-export)
- `Apeireth-rust/crates/apeireth-pipeline/examples/provider_registry_demo.rs` (NEW example, 1 file)
- `Apeireth-rust/crates/apeireth-pipeline/tests/provider_registry_test.rs` (8 unit tests, NEW)

**整合依赖**: R122-5 semantic_router 0 漂移 (`crates/apeireth-api/src/llm/semantic_router.rs` 已存, 你的 Provider Registry 要能接 semantic_router 的路由结果)

**估时**: 50 min (含 8 单元测试 + 1 example + 1 doc)

**截止**: 8/10 17:30 (0 含实施, 跑过夜明早), 实际交付 = final 报告

---

## 0 装解除 (主人 17:22) — 重要

**借鉴源码状态** (verify 实施前):
```bash
Test-Path '.openclaw\workspace\borrowed-repos\LiteLLM\.git'  # 必须 True
```

**3 种状态对应动作**:
1. ✅ **cloned** (`.git` 存在) = 真实施, 报告里写 "借鉴源码 ✅ cloned, 已实施"
2. ⏳ **限流中** (`.git` 0 存在) = 等 30 min 再 verify, 仍 0 实施, 报告里写 "借鉴源码 ⏳ 限流中, 0 实施, 借鉴 ID 索引完成"
3. ❌ **永久失败** (24h 后仍 0 cloned) = 报 supervisor + 取消任务, 0 假装"已借鉴"

**0 装 PASS 严守**: ❌ 0 假装"已借鉴", ❌ 0 写 src 假装 import 借鉴代码, ❌ 0 写 doc 假装 API 兼容. 借鉴源码 0 在手 = 0 实施, 报告诚实标.

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

| # | 硬墙 | 你 (R125-1) 必守 |
|---|------|-----------------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 你 0 再升) | ✅ 0 触碰 `Cargo.toml` `version` 字段 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | ✅ 0 触碰 `integration_r_measure.rs` |
| 3 | **B1** 24 LOCKED crate mtime 16:34 baseline 0 触碰 (apeireth-pipeline 不在 24 LOCKED, 实施可改) | ✅ 仅改 apeireth-pipeline, 0 触碰 24 LOCKED crate |
| 4 | **B5** 6→8 哲学锚 (R125 末升) | ✅ 0 改 6 哲学锚原 6 实质, 8 锚是扩展 |
| 5 | **B3** V0.5 25 维 (R125 末升) | ✅ 0 改 V0.5 公式, 25 维是扩展 |
| 6 | **B4** 6 重守门 v6 (R125-5 实施) | ✅ 0 改 5 重原 5 重, 6 重是扩展 |
| 7 | **A3** 12 键 + PHL-07 = 13 键 (R125-12 后) | ✅ 0 改 12 键原 12, 13 键是扩展 |
| 8 | **C1** 0 主动 commit (你 sub-agent 0 commit) + **C2** 0 装 解除 (主人 17:22) + **C3** 0 装 5 项 升 6 重 v6 + 0 主动 push 严守 | ✅ 0 commit, 0 push, 借鉴源码 ✅ cloned 才真实施 |

**新增 mod 0 触碰 workspace.version**: apeireth-pipeline 自身 Cargo.toml 是 `version.workspace = true`, 你 0 触碰 workspace root 的 Cargo.toml.

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴源码 study (10 min)
```bash
# verify cloned
Test-Path '.openclaw\workspace\borrowed-repos\LiteLLM\.git'
# 读 LiteLLM 核心: litellm/__init__.py + litellm/main.py + litellm/router.py + litellm/utils.py
Get-ChildItem '.openclaw\workspace\borrowed-repos\LiteLLM\litellm\llms' -Directory | Select-Object Name
```
提取 3 个核心 pattern:
1. **Provider Registration**: 如何把 50+ provider (openai/anthropic/cohere/...) 统一抽象
2. **Cost Calculation**: per-token cost + model pricing table
3. **Router/Completion**: 选 model → 调 API → 处理 response

### 阶段 2: Rust 实施 (25 min)
**provider_registry.rs** 结构:
```rust
//! Provider Registry — 借鉴 LiteLLM 的多 Provider 路由注册表 (R124-1-BORROW-BerriAI/litellm)
//!
//! 50+ LLM Provider 统一注册/选择/降级, 整合 apeireth-api semantic_router 0 漂移.

pub struct ProviderSpec { /* name, base_url, model_family, cost_per_1k, capabilities */ }
pub struct ProviderRegistry { /* HashMap<String, ProviderSpec> */ }
pub enum ProviderCapability { Chat, Completion, Embedding, Tool, Vision, Audio, ... }
pub enum SelectionStrategy { RoundRobin, LowestLatency, LowestCost, Capability, Custom }

impl ProviderRegistry {
    pub fn new() -> Self { /* load 50+ built-in providers, LiteLLM parity */ }
    pub fn register(&mut self, spec: ProviderSpec) -> Result<(), RegistryError>
    pub fn select(&self, strategy: SelectionStrategy, caps: &[ProviderCapability]) -> Option<&ProviderSpec>
    pub fn all_providers(&self) -> impl Iterator<Item = &ProviderSpec>
    pub fn by_model(&self, model_name: &str) -> Option<&ProviderSpec>
}
```

**lib.rs 修改**:
- 加 `pub mod provider_registry;`
- 加 `pub use provider_registry::{ProviderRegistry, ProviderSpec, ProviderCapability, SelectionStrategy};`

### 阶段 3: 8 单元测试 (10 min)
- `test_register_new_provider` — register + get
- `test_register_duplicate_returns_error` — duplicate name → Err
- `test_select_round_robin_distribution` — 3 providers, 6 calls → 2 each
- `test_select_lowest_cost_filters_by_capability` — vision cap → only vision providers
- `test_select_no_match_returns_none` — impossible cap combination
- `test_by_model_lookup` — "gpt-4" → openai spec
- `test_builtin_50_providers_loaded` — count >= 50
- `test_semantic_router_compat` — 接 R122-5 路由结果

### 阶段 4: example + final 报告 (5 min)
- `examples/provider_registry_demo.rs` — 实际 run 1 round-robin + 1 cost-based
- final 报告: `Apeireth-rust/reports/agent-r125-1-final-2026-08-10.md`

---

## 0 主动 commit (C1 严守)

❌ **你 (R125-1 sub-agent) 0 commit, 0 push**. 实施完成 = 写 src/test/example + 写 final 报告. Mavis 整合 #3 拍板 17:30 (0 含 R125 实施, R125 续 mavis 整合 commit 链 8/15-9/10).

---

## final 报告 必含 6 段

```markdown
# R125-1 Final Report — LiteLLM Provider Registry
**Date**: 2026-08-10
**Author**: R125-1 sub-agent
**借鉴 ID**: R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10
**实施路径**: crates/apeireth-pipeline/src/provider_registry.rs (NEW)

## 1. 借鉴源码状态 (0 装解除 verify)
- ✅ cloned / ⏳ 限流中 / ❌ 永久失败 (3 选 1)

## 2. 实施步骤
- 阶段 1 借鉴 study: (3 提取 pattern)
- 阶段 2 Rust 实施: (provider_registry.rs 摘要, 公开 API 列表)
- 阶段 3 单元测试: (8 test pass/fail)
- 阶段 4 example + 报告: (demo 跑通)

## 3. 8 硬墙 verify (B1-B7 + A1-A3 + C1-C3)
- B2 ✅ 0 触碰 workspace.version
- A1 ✅ 0 触碰 R11 baseline 3 值
- B1 ✅ 0 触碰 24 LOCKED crate
- B5 ✅ 0 改 6 哲学锚实质
- B3 ✅ 0 改 V0.5 公式
- B4 ✅ 0 改 5 重守门实质
- A3 ✅ 0 改 12 键原 12
- C1-C3 ✅ 0 commit, 0 装 PASS, 0 push

## 4. 0 装解除 verify
- 借鉴源码状态: (✅/⏳/❌)
- 0 假装"已借鉴": (true/false)
- 真实实施 vs 索引完成: (真实施/索引完成)

## 5. 整合 verify
- 接 R122-5 semantic_router: (是/否 + 路径)
- apeireth-pipeline Cargo.toml: (0 触碰 / 改了哪些)

## 6. 下一步 + 风险
- 1 个风险 / 1 个待 R125-N 续协调
```

---

## 你的工具 (你 sub-agent 必知)

你有: read, write, edit, grep, glob, bash. 你 0 commit, 0 push. 你 0 假装.

---
**派活完成 17:28. 截止 8/10 17:30 (0 含, 跑过夜) → final 报告 17:30 后写, 8/11 8:00 前完成. 卡 30 min → 诊断 + kill + 派替代 (supervisor 监督).**
