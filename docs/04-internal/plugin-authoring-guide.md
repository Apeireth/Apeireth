# Apeireth 社区插件开发规范（v2 工程重构线, 2026-08-27）

> **现状 (2026-08-27)**：本文**重写**为 v2 插件契约（`crates/foundation/plugin`），取代 v1 时代的 `apeireth-companion` Plugin/Tool trait 与 ToolBridge 装配模型（现 `legacy/`）。当前基线：默认分支 `main`、13-crate 工作区、tag `v2.0.0-alpha.1` @ `d6910cf7`；v2 下一步见根 [ROADMAP.md](../../ROADMAP.md) §4。内核（哲学 8 锚、文档同步自觉、0 装 PASS）不变。

```
[Document-Meta]
Document:        docs/04-internal/plugin-authoring-guide.md
Version:         Manual-Rev-N (v2 重写)
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v2 插件契约)
```

>给谁看：社区插件作者 + 官方套件维护者。
>读法：先读 §1（最小可可跑插件）→ §2（注册模型，决定能否被 runtime 看见）→ §3（ToolCapability 工具接口）→ §4（ProviderCapability 接口）→ §5（生命周期 + 治理 + 测试模板）→ §6（发布检查单）。
>0 假装：本指南所有示例均**摘自真实 v2 代码**（`crates/foundation/plugin` + `crates/capabilities/tools`）；未实装的机制如实标注，R不假装。

---

## 0. 一句话

**v2 插件 = 一个 `Plugin` trait 实现 + 一个 `PluginManifest`**。插件注册到 **`PluginManager`**，由它统一维护 *plugin* 与 *capability* 两个 registry（**唯一**事实源）。能力分两类：`ToolCapability`（内置工具 + 第三方工具）与 `ProviderCapability`（LLM provider 适配器）。一个插件可以同时提供两类能力的任意多。

---

## 1. 最小可可跑插件（HelloTool）

### 1.1 完整示例（基于真实代码）

来源：`crates/foundation/plugin/src/plugin.rs:71-98`（Plugin trait）+ `crates/foundation/plugin/src/manifest.rs:38-54`（PluginManifest）+ `crates/capabilities/tools/src/plugin.rs:39-90`（BuiltinToolsPlugin 真实使用模式）。

```rust
//! `<your_plugin>` — 社区插件模板（v2）.
//!
//! 0 装 PASS（必做做）：
// - 写清"做了什么" + "没做什么"，未接的 trait 口必须标注 "未接".
// - 任何网络调用 = 用 `PluginContext::credentials` 取 secret，R不用 `std::env::*`.
//! 
//! 装配：PluginManager 注册 → initialize() 拿 PluginContext → tools()/providers() 暴露能力 → shutdown() 真清理。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, PluginId, TraceId};
use apeireth_plugin::{
    CapabilityDescriptor, CapabilityKind, Plugin, PluginContext, PluginManifest,
    PluginManager, PluginResult, ToolCapability,
};
use apeireth_protocol::canonical::NormalizedTool;
use async_trait::async_trait;
use serde_json::{json, Value};
```

### 1.2 工具实现（ToolCapability trait）

来源：`crates/foundation/plugin/src/tool.rs:44`（trait 定义）。

```rust
/// 社区示例工具：一个只读计算器，返回两个数相加.
pub struct HelloCalc;

#[async_trait]
impl ToolCapability for HelloCalc {
    /// 规范标识：runtime 用这个 id 在 `BuiltinToolsPlugin::tools()` 注册表里查找.
    fn id(&self) -> &CapabilityId {
        use std::sync::OnceLock;
        static ID: OnceLock<CapabilityId> = OnceLock::new();
        ID.get_or_init(||::::CapabilityId::new("tool.hello_calc").unwrap())
    }

    /// 展示 schema——provider 端可读的 tool declaration（OpenAI tool_calls 兼容）.
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::function(
            "hello_calc",
            "Return a+b (community template demo).",
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                },
                "required": ["a", "b"]
            }),
        )
    }

    /// runtime 调用入口。Err 直接回灌模型，R不假装成功.
    async fn call(&self, arguments: serde_json::Value) -> PluginResult<serde_json::Value> {
        let a = arguments.get("a").and_then(|v| v.as_f64())
            .ok_or_else(|| apeireth_plugin::PluginError::call_failed("hello_calc", "需要数字参数 a"))?;
        let b = arguments.get("b").and_then(|v| v.as_f64())
            .ok_or_else(|| apeireth_plugin::PluginError::call_failed("hello_calc", "需要数字参数 b"))?;
        Ok(json!({ "sum": a + b }))
    }
}
```

### 1.3 插件实现（Plugin trait）

来源：`crates/foundation/plugin/src/plugin.rs:71-98`（4 方法：manifest/initialize/shutdown/tools/providers）。

```rust
pub struct HelloCalcPlugin {
    manifest: PluginManifest,
}

impl HelloCalcPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::new(
            PluginId::new("community.hello_calc").unwrap(),
            "0.1.0",
            "社区插件模板：只读计算器示例（v2）",
        )
        .declare(
            CapabilityDescriptor::new(
                CapabilityId::new("tool.hello_calc").unwrap(),
                CapabilityKind::Tool,
                "Return a+b（社区模板演示）",
            )
            .unwrap()
            .with_metadata("risk", "low")
            .with_metadata("category", "demo"),
        )
        .unwrap();
        Self { manifest }
    }
}

#[async_trait]
impl Plugin for HelloCalcPlugin {
    fn manifest(&self) -> &PluginManifest { &self.manifest }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        // 0 装 PASS：明确"做了什么没做做"——本插件无外部资源，initialize 为空.
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        // 幂等：可能重复调用，R不能 panic. 默认工具无额外状态=空实现.
        Ok Ok(())
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::new(HelloCalc)]
    }

    fn providers(&self) -> Vec<Arc<dyn apeireth_plugin::ProviderCapability>> {
        vec![]   // 本插件只提供工具；provider 在 §4.
    }
}
```

### 1.4 注册入口（运行时 bootstrap）

来源：`crates/adapters/cli/src/lib.rs::build_canonical_runtime_from_env` + `crates/capabilities/tools/src/plugin.rs:39-90`（BuiltinToolsPlugin 实际装配）。

```rust
use apeireth_plugin::{PluginManager, CredentialResolver};
use apeireth_provider::credentials::EnvCredentialResolver;
use apeireth_tools_canonical::BuiltinToolsPlugin;
use apeireth_core::kernel::{system_clock, TraceId};
use std::sync::Arc;

let mut manager = PluginManager::new();
let resolver: Arc<dyn CredentialResolver> = Arc::new(EnvCredentialResolver::new());

// 官方内置：filesystem / search / repo 三个只读工具（shell/fetch 默认 opt-in）.
let tools_plugin = BuiltinToolsPlugin::new(std::env::current_dir().unwrap());
manager.register(Arc::new(tools_plugin)).unwrap();

// 社区插件：HelloCalcPlugin 接进同一个 manager.
manager.register(Arc::new(HelloCalcPlugin::new())).unwrap();

// 3 家 canonical provider 作为插件注册：
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
manager.register(Arc::new(MinimaxProviderPlugin::from_env()?)).unwrap();
manager.register(Arc::new(AnthropicProviderPlugin::from_env()?)).unwrap();

// runtime 拿 manager 的 capability 视图（推荐 path）：
let capabilities = manager.capabilities();
for (id, owner) in capabilities.iter() {
    println!("{} -> {}", id, owner);
}
```

**插件 Manager 是 Runtime 唯一的能力登记点**（`crates/engine/runtime/src/canonical/runtime.rs::Runtime` 持有 `Arc<PluginManager>`）。旧版 v1 的"插件+ToolBridge+ToolRegistry+PackRegistry 四处注册"已收敛为 v2 的"manager 一处"。这是 v2 收敛的核心契约。

---

## 2. 注册模型（为什么 PluginManager 是唯一事实源）

### 2.1 两个 registry，绝不并列

来源：`crates/foundation/plugin/src/lib.rs:19-24`（明确陈述）。

- **`PluginRegistry`**（`plugin::registry.rs:38`）—— 持有 `Arc<dyn Plugin>` 实例与 lifecycle
- **`CapabilityRegistry`**（`plugin::registry.rs:122`）—— `CapabilityId -> PluginId` 索引，**index, 不是 copy**

设计约束（v2 不可违反）：`CapabilityDescriptor` 的定义**只在** `PluginManifest` 里出现一次；capability registry 只是**索引**；runtime 拿真实描述永远去 plugin 拉 (`manager.record(id) -> CapabilityRecord<'_>`，`record.rs:202`)。

### 2.2 注册 + lifecycle 语义

来源：`crates/foundation/plugin/src/manager.rs:37-89`。

| 方法 | 行为 | 失败语义 |
| | | |
| `PluginManager::register(plugin)` | 验证 manifest vs `tools()/providers()` 1:1（`manager.rs::register`）；重复 id 直接拒（**0 装 PASS：R假装重复装）| Err |
| `PluginManager::state(id)` | 返回 `Lifecycle`（Inactive/Starting/Active/Stopping/Stopped/Failed）| — |
| `initialize(ctx)` | 自动按 `.depend_on()` 拓扑序调用 | 任一失败→该 plugin 进 Failed，但**已初始化的不回滚**（manager:80-89 注释明确） |
| `shutdown()` | 逆拓扑序调用 | 错误记录但**不停止其余** shutdown（plugin.rs:86-87 注释明确：a shutdown that abandons half its plugins leaks more than it protects） |

### 2.3 v1 → v2 失去的东西（诚实标注）

v1 companion 模型里你用过的工具，**有些 v2 没有等价物**——这是设计选择，不是 bug：

| v1 概念 | v2 状态 | 替代路径 |
| | | |
| `ToolKind::Sync/Async/Static/Service/MessagePreprocessor/Hybridservice`（v1 6 类） | ****：v2 只有 `CapabilityKind::Tool / Provider / Memory`（3 大类）） | `ToolCapability` trait 自描述异步/同步语义，不靠枚举 |
| `ToolAxes` 5 轴（trigger/awaiting/resident/transport/output） | **未接** | `with_metadata("axis", "...")` 自定义 metadata 表达 |
| `PermissionPack`（v1 授权包）| ****：v2 改走 `crates/foundation/governance::PermissionPolicy` + `PermissionGovernanceHook`（P0 接线后生效）| 工具运行受 governance pipeline 管控，R**非**插件自授权 |
| `watch_plugin_dir`（v1 动态加载注释）| **未接**：v2 明确"Static, in-process plugins only"（plugin.rs:34-35）| 动态加载 / WASM / 远程 plugin / marketplace = 明确 out of scope |
| `Pluggable provider/套件/三层交付` | **partial**：v2 只保留 plugin 一个交付层；套件/模块 v1 三层模型已弃用 | 套件边界由 PluginManifest 的 CapabilityKind + depend_on 表达 |
| `on_load/on_unload`（v1 生命周期）| | **已合并**：`initialize(ctx)/shutdown()` |

---

## 3. ToolCapability 接口（你的工具怎么被 runtime 调）

### 3.1 trait 三个方法

来源：`crates/foundation/plugin/src/tool.rs`。

```rust
#[async_trait]
pub trait ToolCapability: Send + Sync {
    fn id(&self) -> &CapabilityId;                              // 规范 id（必 unique）
    fn declaration(&self) -> NormalizedTool;                  // OpenAI 兼容 tool_calls schema
    async fn call(&self, arguments: serde_json::Value)         // runtime 调用入口
        -> PluginResult<serde_json::Value>;
}
```

**`declaration` 直接生成 `NormalizedTool`**（`crates/foundation/protocol/src/canonical/normalized.rs`）—— v2 抽象掉了"工具 schema"的 OpenAI/Anthropic 协议差异，provider 用统一 DTO 转发。**你的 declaration 写一次，所有协议自动兼容**。

### 3.2 PluginManager 提供的工具视图

来源：`crates/foundation/plugin/src/manager.rs:316-368`。

| 方法 | 返回 | 用例 |
| | | |
| `manager.tool(id) -> Result<Arc<dyn ToolCapability>>` | 单一工具 | runtime 拿工具执行 |
| `manager.active_tools() -> Vec<Arc<dyn ToolCapability>>` | 全部 active 工具 | runtime 启动时声明 |
| `manager.tool_declarations() -> Vec<NormalizedTool>` | 全部工具的协议层 schema | 给 provider 的 tool_calls 系统消息 |
| `manager.tool_by_name(name) -> Option<Arc<dyn ToolCapability>>` | 按名字查 | tracing/logging |

**BuiltinToolsPlugin 真代码示例**（`crates/capabilities/tools/src/plugin.rs:44-90`）展示了完整形状：3 个 ToolCapability（filesystem/search/search / repo），每个都有 `id()` / `declaration()` / `call()`，通过 manifest 的 `.declare(CapabilityDescriptor...)` 注册。

### 3.3 工具名（id）命名规范

- 格式：`<scope>.<name>`（例：`tool.hello_calc`、`tool.filesystem`）
- `tool.shell` / `tool.fetch` 是 **BuiltinToolsPlugin 的 opt-in 名**——**不要命名冲突**
- 风险词（`file`/`patch`/`task`/`exec`/`shell`/`network`）会让 GovernancePipeline 第 5 闸 LLM 评审触发（per `crates/foundation/governance/src/input_security.rs`）；非该类工具避免
- `id()` 必须返回**静态** `&CapabilityId`（推荐 `OnceLock<CapabilityId>`）

### 3.4 执行链：runtime + governance 怎么用你的工具

1. runtime 在 agent loop 拿到模型的 `tool_calls`
2. runtime 经 **`Arc<PluginManager>::tool(id)`** 找工具
3. runtime 构造 **`GovernanceRequest { action: CapabilityDispatch { capability, arguments }, ... }`**，调 `Arc<dyn GovernanceHook>::evaluate`
4. governance 返回 `Decision::Allow/Deny/RequireApproval`（runtime 严格区分"拒绝"与"挂起"，`crates/foundation/governance/src/lib.rs:117-139`）
5. Allow → `tool.call(arguments).await` → 结果回灌 transcript
6. Deny → tool error 直接回灌模型（不终止回合，`crates/engine/runtime/src/canonical/execute.rs` 哲学）
7. RequireApproval → turn 挂起为 `PendingApproval`，不调工具，等 resume

> **生产 governance 接线（✅ 已通过 upstream `873d2857` 落地）**：`build_canonical_runtime_from_env` 装 `GovernancePipeline` = `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`；runtime 每个 `CapabilityDispatch` 都先经 pipeline 评估，**不再**默认 `AllowAll`。你的插件不需要自己实现拒绝逻辑——只需要在 `CapabilityDescriptor::with_metadata("risk", ...)` 声明风险级别（low/medium/high），governance 闸会自动响应。MaxRounds 是 runtime 结构性约束，AuditHashChain 按部署需要挂。
>
> **敏感 workspace 路径保护（✅ upstream `ac5cbf5a`）**：`tool.filesystem` 与 `tool.search` 通过 `crates/capabilities/tools/src/sensitive_path.rs` 自动屏蔽 `.env` / `.ssh` / `.aws` / `.gnupg` / `.secret` / `.config/gcloud` 等敏感路径（普通项目元数据如 `.gitignore` / `.cargo/config.toml` 仍可读）。**你的插件在 fork filesystem/search 行为时必须继承这个保护**，不要绕过 `is_sensitive_path` 检查。

---

## 4. ProviderCapability 接口（你要不要提供 LLM 接入）

来源：`crates/foundation/plugin/src/provider.rs:96-148`。

```rust
#[async_trait]
pub trait ProviderCapability: Send + Sync {
    fn name(&self) -> &str;                             // 唯一标识符（runtime 用它做 fallback order）
    fn model_ids(&self) -> Vec<String>;                // 该 provider 支持的全部 model
    fn credential_name(&self) -> &str;                  // "provider.minimax.api_key" 形式 logical name
    async fn complete(&self, request: CompletionRequest)
        -> PluginResult<CompletionResponse>;            // 核心：与 normalized protocol DTO 对接
    async fn stream(&self, request: CompletionRequest)
        -> PluginResult<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>>>>>;
}
```

**真实现示例**（`crates/engine/provider/src/canonical_minimax.rs`）：3 家 canonical provider（`MinimaxProviderPlugin` / `AnthropicProviderPlugin` / `OpenAiCompatibleProviderPlugin`），命名遵循 `provider.<vendor>` logical name，凭据走 `CredentialResolver::resolve(name)`，不复制定 adapter 翻译逻辑。

**关键契约**：你的 provider 应复用 `apeireth_protocol::canonical::NormalizedRequest` / `NormalizedResponse`（见 `crates/foundation/protocol/src/canonical/`），**不要自己定义协议 DTO**——这是 v2 收敛点之一（v1 的 vendor-wire 翻译散在 4+ 处）。

---

## 5. 生命周期 + 治理 + 测试

### 5.1 PluginContext 拿什么

来源：`crates/foundation/plugin/src/plugin.rs:28-53`。

```rust
pub struct PluginContext {
    pub clock: Arc<dyn Clock>,            // apeireth_core::kernel::Clock——必须用，R不用 Utc::now()
    pub credentials: Arc<dyn CredentialResolver>,  // 取 secret 唯一路径
    pub trace: TraceId,                    // 启动事件的 trace id（与你自己的事件相关）
}
```

**0 装 PASS 红线**：禁止 `std::env::var("OPENAI_API_KEY")`、禁止 `std::fs::read("apikey.txt")`、禁止 `Utc::now()`。这些在 v1 是常见的"简化"模式，v2 **强制**走 ctx——这是为了可移植 (virtual clock) + 安全 (secret 不落 struct) + 可可观测 (trace 相关性)。

### 5.2 凭据解析（怎么取 API key）

`CredentialResolver`（`crates/foundation/plugin/src/credentials.rs:58`）：
- `NoCredentials`：明确"没有凭据"（避免 `Option<>` 不可见）
- `StaticCredentials`：测试用，直接注入
- `EnvCredentialResolver`（生产默认在 `provider::credentials`）：logical name → env var，**secret 走 `Secret<T>` 类型（debug 不打印）**

默认映射（`crates/engine/provider/src/credentials.rs:88-99`）：
- `provider.minimax.api_key` → `APEIRETH_API_KEY`
- `provider.anthropic.api_key` → `APEIRETH_ANTHROPIC_KEY`
- `provider.openai-compatible.api_key` → `OPENAI_API_KEY`

插件要加新 provider → 在 `EnvCredentialResolver::new()` 加一行 default mapping，**R**改 provider trait。

### 5.3 治理（P0 未接线，但请按允许接入设计）

可用的 hook（已在 `crates/foundation/governance`）：
- `PermissionGovernanceHook`（`permission.rs:164`）：PermissionPolicy 包装（grant/revoke/需审批能力）
- `PromptInjectionHook`（`input_security.rs:280`）：启发式注入信号
- `CredentialDisclosureHook`（`input_security.rs:316`）：凭据泄漏拦截
- `AuditHashChain`（`audit.rs:104`）：append-only 审计

你的 plugin 应**只声明**风险等级（`with_metadata("risk", "low"/"medium"/"high")`），**不**自己实现拒绝逻辑——拒绝/批准是 governance crate 的责任。

### 5.4 0 装 PASS 写法

来源：`crates/foundation/plugin/src/error.rs::PluginError`（全 API 形态）+ `crates/foundation/governance/src/lib.rs` 治理决策 API。

| 禁止 | 正确 |
| | | |
| 返 `Ok` 假装成功 | 返 `Err(PluginError::...)` + **可行动提示**（"需要参数 a"，R"失败请重试"） |
| 文档/注释写"已支持"实际未接 | 标注 `// 0 装 PASS: <TODO> trait 口已备，实现未接` |
| 静默吞错 | `tracing::warn!` 记录 + 降级路径明确 |
| `std::env::*` 取 secret | `ctx.credentials.resolve("your.credential.name")` |
| `Utc::now()` 取时间 | `ctx.clock.now_ms()` |
| 文档写 plugin 名字就锁住 | `id()` 返 `OnceLock<CapabilityId>` 静态字串 |
| 工具命含 `file`/`patch` 词 | 除真是文件系统工具，R触发 governance Medium risk + LLM 评审 |

### 5.5 测试模板

来源：`crates/foundation/plugin/src/plugin.rs::tests` + `crates/foundation/plugin/src/manager.rs::tests`。

```rust
#[tokio::test]
async fn plugin_registers_and_resolves_tools() {
    use apeireth_core::kernel::{system_clock, CapabilityId, PluginId, TraceId};
    use apeireth_plugin::{NoCredentials, PluginManager};
    use std::sync::Arc;

    let mut mgr = PluginManager::new();
    let plugin = Arc::new(HelloCalcPlugin::new());
    let id = plugin.manifest().id().clone();
    
    // 1) 注册成功（manifest vs tools() 1:1 验证通过）
    mgr.register(plugin).unwrap();
    
    // 2) tool 可见 + declaration 与 manifest 一致
    let tool = mgr.tool(&CapabilityId::new("tool.hello_calc").unwrap()).unwrap();
    assert_eq!(tool.id().as_str(), "tool.hello_calc");
    
    // 3) execute: 正常路径
    let r = tool.call(serde_json::json!({"a": 1.0, "b": 2.0})).await.unwrap();
    assert_eq!(r["sum"], 3.0);
    
    // 4) execute: 失败路径（缺参 → Err + 可行动提示，R假装成功）
    let r = tool.call(serde_json::json!({})).await;
    assert!(r.is_err(), "缺参必须失败");
    
    // 5) initialize/shutdown 幂等
    let ctx = PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new());
    plugin.initialize(&ctx).await.unwrap();
    plugin.shutdown().await.unwrap();
    plugin.shutdown().await.unwrap();  // 幂等
}
```

测试命令：`cargo test -p apeireth-<your_plugin_crate> -j 4`（per `team-work-doc.md` §2.2）。

### 5.6 失败语义（manager 行为）

来源：`crates/foundation/plugin/src/manager.rs::register` + plugin.rs::tests。

| 场景 | 期望 |
| | |
| manifest 声明 `tool.foo` 但 `tools()` 返空 | register **失败** + `PluginError::manifest_mismatch` |
| manifest 声明 `tool.foo` 但 `tools()` 返 `tool.bar` | 同上 |
| 重复注册同 id plugin | register 失败 + `PluginError::duplicate_plugin` |
| `initialize` 返 Err | plugin 进 `Failed` lifecycle，**已成功的兄弟不回滚** |
| `shutdown` 返 Err | 错误记录，**其他 plugin 的 shutdown 不停止** |

---

## 6. 发布检查单

提交 PR 前逐项自查（0 装 PASS 红线 + 工程严守）：

| # | 项 | 标准 | 证据 |
|---|---|---|---|
| 1 | 测试覆盖 | `cargo test -p <your_crate> -j 4` 通过，含正常/失败/非法输入路径 | 测试输出 |
| 2 | 全链路测试 | §5.5 形状：注册 → manifest 验证 → tool 可见 → declaration 一致 → execute 正常 + 失败 → initialize/shutdown 幂等 | 测试代码 |
| 3 | 0 装 PASS | 模块头 `//!` 0 装块：做 + 没做明确分；Err 用 `PluginError::*` 而非字符串 | 模块头 |
| 4 | 不依赖全局 | 不读 `std::env::var(...)` 取 secret；不用 `Utc::now()` 取时间——全部走 `PluginContext` | grep 审查 |
| 5 | id() 静态 | `OnceLock<CapabilityId>` 缓存，不每次重 alloc | 源码 |
| 6 | declaration 与 manifest 一致 | `manager.register` 自动验证；测试覆盖 | 测试 |
| 7 | 命名合规 | `id()` 不含 `file`/`patch`/`task` 除非真是那类工具（避 governance Medium risk） | 自查 |
| 8 | 文档同步 | 本指南如涉及新形态或新模式，本批补 update | PR diff |
| 9 | 提交纪律 | 小步提交 + message 用中文（"为什么 + 做了什么 + 测试结果"）；无调试输出；`git status` 只含自己文件 | commit |
| 10 | 自审报告 | 改动文件 / 测试结果 / 集成点（`PluginManager::register` 调用点）/ 0 装 PASS 标注 / 给集成守门员的合并提示 | 报告文本 |

合入流程：PR → 集成守门员审查（`cargo check --workspace` + 相关 crate 测试 + 规范执法）→ merge。

---

## 附 A：v2 与 v1 概念对照（不再用 v1 术语写新代码）

| v1 概念（legacy/apeireth-companion）| v2 替代 |
|---|---|
| `Plugin` trait 5 方法（id/version/description/on_load/on_unload） | `Plugin` trait 4 方法（manifest/initialize/shutdown/tools/providers） |
| `ToolRegistry` / `CapabilityCatalog` / `PluginRegistry` / `PackRegistry` 四处 | **`PluginManager` + 两个 registry（唯一事实源）** |
| `Tool` trait + `ToolKind` 6 类 + `ToolAxes` 5 轴 | `ToolCapability` trait 3 方法 + `declaration() -> NormalizedTool` |
| `ToolBridge` + `on_load` 自授权 `PermissionPack` | governance `Arc<dyn GovernanceHook>` + `PermissionPolicy`（P0 接线后生效）|
| `on_load` 注入 `registry.register` + `packs.grant` 两步 | `Plugin::tools()` 返 Arc<dyn ToolCapability> + `initialize()` 仅做资源初始化 |
| `on_unload` 撤销注册 + 撤销授权 | `shutdown()` 仅做资源释放（**v2 不再"撤销授权"——governance 接管**） |
| `<<<[TOOL_REQUEST]>>>` marker 解析 | provider 原生 `tool_calls` 数组（OpenAI 兼容），protocol 抽象已统一 |
| `crate::packs::PermissionPack` 三种期 | `Permission` + `PermissionPolicy`（governance/permission.rs） |
| 三层交付模型（模块/套件/插件）| **单层 plugin**（套件/模块边界由 manifest depend_on + CapabilityKind 表达） |
| `watch_plugin_dir` 动态加载 | **明确 out of scope**（plugin.rs:34-35） |

## 附 B：来源索引（v2 真实代码）

| API / 示例 | 位置 |
|---|---|
| `Plugin` trait（4 方法）/ `PluginContext` | `crates/foundation/plugin/src/plugin.rs:28-98` |
| `PluginManifest`（declare/depend_on/with_metadata）| `crates/foundation/plugin/src/manifest.rs:17-110` |
| `CapabilityDescriptor` / `CapabilityKind` 3 类 | `crates/foundation/plugin/src/capability.rs:71-160` |
| `PluginManager`（register/state/typed views）| `crates/foundation/plugin/src/manager.rs:37-368` |
| `PluginRegistry` / `CapabilityRegistry` | `crates/foundation/plugin/src/registry.rs:38-202` |
| `ToolCapability` trait（id/declaration/call）| `crates/foundation/plugin/src/tool.rs:44-...` |
| `ProviderCapability` trait | `crates/foundation/plugin/src/provider.rs:96-148` |
| `CredentialResolver` / `Secret` / `NoCredentials` / `StaticCredentials` | `crates/foundation/plugin/src/credentials.rs:24-99` |
| `PluginError`（init_failed/shutdown_failed/call_failed/manifest_mismatch/duplicate_plugin）| `crates/foundation/plugin/src/error.rs:138-200` |
| 真实插件示例（BuiltinToolsPlugin） | `crates/capabilities/tools/src/plugin.rs:39-235` |
| 真实 provider 示例（Minimax/Anthropic/OpenAI-compatible）| `crates/engine/provider/src/canonical_*.rs` |
| 治理（hooks + Decision）| `crates/foundation/governance/src/lib.rs` |
| 协议 DTO（NormalizedTool/Request/Response）| `crates/foundation/protocol/src/canonical/` |

> 相关文档：[team-work-doc.md](team-work-doc.md)（§1 三哲学 / §3 文档规范 / §2 工程规范）· [maintenance-guide.md](maintenance-guide.md)（13-crate 模块地图 + v2 维护流程）· [ROADMAP.md](../../ROADMAP.md)（§4 v2 路线 + §5 硬墙）· [ARCHITECTURE.md](../../ARCHITECTURE.md)（架构契约）· [architecture.md](../01-architecture/architecture.md)（foundation/engine/capabilities/adapters 详细归属）

---

_本指南 v2 重写 (2026-08-27)：取代 v1 companion Plugin/ToolBridge/Pack 装配模型；v2 单一事实源 = `PluginManager` + 两个 registry（plugin / capability），单一插件 trait = `Plugin`（4 方法），单一工具 trait = `ToolCapability`（3 方法 + declaration 直接出 `NormalizedTool`）。0 装 PASS：动态加载 / WASM / 远程 plugin / marketplace 明确 out of scope（plugin.rs:34-35），R**假装 v2 支持。_