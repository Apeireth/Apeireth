# ADR 0007: 兼容组件层（Compat Components Layer）

> **性质**: 第七个 ADR —— 记录 Apeireth R14 Rust 重写工程期产生的"兼容组件层"设计决策，为 `apeireth-pybridge` (PyO3 桥) + `apeireth-mcp` (Model Context Protocol) + `apeireth-extension` (VCP 6 类插件协议) 三类兼容组件提供统一的扩展边界。
>
> **依据**: 阶段 1 §1 架构灵感 8 项之"可扩展边界 —— 通过兼容组件接入其他语言模块 (Python/Go/JS 等)" + 阶段 1 §14.4 crate 候选清单（`apeireth-pybridge` + `apeireth-mcp`） + 阶段 1 §16.5 Apeireth vs Hermes 对比"兼容组件：MCP/ACP → VCP 6 类插件协议 + 纯文本标记" + 阶段 2 §6 平台中立（platform neutrality） + 阶段 2 §11 一人/多人部署兼容 + 阶段 2 §8 模块化（modularity） + 阶段 2 §9 通信总线 5 层。
>
> **commit 锚**: P22 (`2d3ba512`) + A16 (`A16-mcp-integration-expert`) + A16.3 (`A16.3-mcp-integration-expert2-pybridge`) + R14-A17 `apeireth-extension` 6 类插件协议。
>
> **生成时间**: 2026-08-02
> **作者**: technical_writer (387832ef-17eb-4be6-bb01-fc4295b9d3e7)
> **约束**: ❌ 不修改任何 LOCKED 文档（阶段 1-5）；仅新增命名空间 `docs/adr/0007-...md` 独立 ADR。

---

## 状态

🟢 **Accepted**（兼容组件层作为 Apeireth 设计的扩展边界正式确立）。

---

## 背景（Context）

### 设计层 LOCKED 设想（不可修改，作为 ADR 输入）

| LOCKED 来源 | 关键设想 | 行号锚点 |
|---|---|---|
| 阶段 1 §1 架构灵感 8 项 | "可扩展边界 —— 通过兼容组件接入其他语言模块 (Python/Go/JS 等)" | `docs/stage1/inspiration-stage1-2026-07-30.md` |
| 阶段 1 §14.4 crate 候选 | `apeireth-pybridge` PyO3 桥 + `apeireth-mcp` MCP 客户端/服务端 | 同上 |
| 阶段 1 §16.5 Apeireth vs Hermes | "兼容组件：MCP/ACP → VCP 6 类插件协议 + 纯文本标记" | 同上 |
| 阶段 2 §6 持久化 | `apeireth-data/src/backends/wave.rs` 自研（兼容组件抽象基础） | `docs/stage2/stage2-decisions-persistence.md` |
| 阶段 2 §8 模块化 | trait vs dyn vs TypeId 三分发机制 + manifest 注册 + 沙箱 | `docs/stage2/stage2-decisions-modularity.md` |
| 阶段 2 §9 通信总线 | 5 层通信分层（inproc / UnixSocket / pipe / gRPC / WS） | `docs/stage2/stage2-decisions-communication-bus.md` |
| 阶段 2 §11 一人/多人部署兼容 | `DeploymentMode::Single/Multi/Offline` 自适应 | `docs/stage2/stage2-decisions-persistence.md` |

### 工程期已落地的兼容组件（事实证据，2026-08-02 实测）

| 兼容组件 | Cargo.toml 路径 | 代码行数 | 实测状态 |
|---|---|---:|---|
| `apeireth-pybridge` | `crates/apeireth-pybridge/Cargo.toml` | **859**（bridge.rs 253 + error.rs 85 + lib.rs 81 + python_bindings.rs 191 + r11_compat.rs 249） | 🟢 workspace member，单测 35 passed |
| `apeireth-extension` | `crates/apeireth-extension/src/lib.rs` | **339**（lib.rs 单文件） | 🟢 workspace member，`PluginKind::ALL: [Self; 6]`（VCP 6 类）+ 集成测试 3 passed |
| `apeireth-mcp` | LOCKED 设想，未实装 crate | 0 | 🔴 MISSING（LOCKED §14.4 候选，但无 `crates/apeireth-mcp/` 目录） |

### 兼容组件的 3 种核心模式

| 模式 | 兼容组件 | 桥接协议 | 部署场景 |
|---|---|---|---|
| **同进程嵌入** | `apeireth-pybridge` (PyO3) | `pyo3::wrap_pyfunction!` + `extension-module` feature | Python mvp/ 内调用 Rust API |
| **同进程 trait 暴露** | `apeireth-extension` | `pub trait Plugin` + dyn dispatch | Rust 插件进程内加载 |
| **跨进程协议** | `apeireth-mcp` (LOCKED 未实装) | JSON-RPC 2.0 over stdio / HTTP | 外部 Python/Node 等进程通过 MCP 调用 |

---

## 问题（Problem）

1. **3 个组件各自孤立**：pybridge / extension / mcp 各自有 trait + manifest + 注册机制，**没有统一的"兼容组件层"边界概念**。后续阶段 4 / 阶段 5 增量（如 wasm sandbox / 浏览器嵌入 / 跨语言 FFI）容易各自重新发明轮子。
2. **pybridge 的 PyO3 ABI 风险**：`apeireth-pybridge/Cargo.toml:9` 引入 `pyo3 = { workspace = true }`（无条件依赖），但 `extension-module` 是 `python-ext` feature gated。**默认 build 时 pyo3 在依赖树但不链接 Python ABI**——这是 P28 阶段 6 的 `apeireth-verify/example walk_all_crates` 失败时 `Cargo.lock` 增量的诱因之一。
3. **mcp 是 LOCKED 但 0 代码**：`apeireth-mcp` 在 §14.4 候选清单 + §16.5 对比表都提到，但**没有任何 `crates/apeireth-mcp/` 目录**。如果阶段 3 蓝图（`docs/stage3-blueprints/`）要画兼容组件架构图，会发现 mcp 是空白。
4. **未确立扩展边界哲学**：LOCKED 文档分散提及"可扩展边界 / 平台中立 / 部署兼容"，但**没有一篇 ADR 把这三者整合为"兼容组件层"作为 Apeireth 设计的扩展边界**。

---

## 决策（Decision）

**正式确立"兼容组件层（Compat Components Layer）"作为 Apeireth 设计的扩展边界**：

> 兼容组件层 = **3 类统一抽象 + 3 种部署模式**的统一抽象。
>
> - **3 类统一抽象**：`Plugin` trait（apeireth-extension）+ PyO3 bridge（apeireth-pybridge）+ JSON-RPC bridge（apeireth-mcp，LOCKED 未实装）
> - **3 种部署模式**：同进程嵌入（PyO3 模式）/ 同进程 trait 暴露（Plugin 模式）/ 跨进程协议（MCP 模式）

### 3 类兼容组件的统一 trait 表面

```rust
// apeireth-extension 已是 6 类插件协议, 兼容组件层统一 trait 表面:
// 1) PluginKind::ALL (apeireth-extension::PluginKind)
//    -> 同步 / 异步 / 静态 / 服务 / 消息预处理 / 混合
// 2) CompatAdapter trait (本 ADR 新增, 阶段 5 实装)
//    -> fn language() -> LangTag  // "rust" / "python" / "javascript" / "wasm" / "external"
//    -> fn process_model() -> ProcessModel  // InProcEmbed / InProcTrait / CrossProcess
//    -> fn manifest() -> &CompatManifest  // 与 plugin manifest 同构
// 3) apeireth-mcp 的 JSON-RPC bridge trait (LOCKED, 阶段 4 实装)
//    -> apeireth_mcp::McpServer { handle_request(req: McpRequest) -> McpResponse }
```

### 兼容组件层与 Apeireth 核心架构的位置关系

```
┌─────────────────────────────────────────────────────────────┐
│  Apeireth 主干 (Rust 24 workspace crates)                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  apeireth-core + apeireth-memory + apeireth-cognition │    │
│  │  + apeireth-onion + apeireth-constraint + ...         │    │
│  └─────────────────────────────────────────────────────┘    │
│                          ▲                                    │
│                          │ trait Plugin / CompatAdapter       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  兼容组件层 (Compat Components Layer) — 本 ADR 确立   │    │
│  │  ┌──────────────┬──────────────┬──────────────┐      │    │
│  │  │ apeireth-    │ apeireth-    │ apeireth-mcp │      │    │
│  │  │ extension    │ pybridge     │ (LOCKED)     │      │    │
│  │  │ (VCP 6 类)   │ (PyO3)       │ (JSON-RPC)   │      │    │
│  │  └──────────────┴──────────────┴──────────────┘      │    │
│  └─────────────────────────────────────────────────────┘    │
│                          ▲                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  外部生态 (Python mvp/ + Node.js + 浏览器 + Go 等)    │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### 兼容组件层的 4 项硬规则

| # | 规则 | LOCKED 来源 |
|---|---|---|
| 1 | 兼容组件**不绕过 V1+V2+V3 AND 门** | 阶段 4 §1.6 + 阶段 5 §5 五重守门 |
| 2 | 兼容组件**必走 `apeireth-extension::Plugin` trait**（不论同进程/跨进程） | 阶段 2 §8 模块化 |
| 3 | 兼容组件**必须登记到 manifest**（含语义化版本 + owner + 审计钩子） | 阶段 2 §8.5 注册中心 |
| 4 | 兼容组件**L0 HA 永远不可变**作为统一体的核心（兼容组件层是外层装饰，不触及 L0） | ADR 0001 双洋葱统一体 |

---

## 后果（Consequences）

### 正面

- ✅ **统一抽象**：3 类兼容组件（Plugin + PyO3 + MCP）共用 `Plugin` trait 表面 + manifest 注册，**避免每类独立注册机制**
- ✅ **可扩展边界清晰**：未来 wasm 沙箱 / 浏览器嵌入 / Go FFI 都可纳入"兼容组件层"
- ✅ **阶段 3 蓝图参考**：兼容组件层架构图可直接参考本 ADR
- ✅ **LOCKED 设想追溯**：阶段 1 §1 + §14.4 + §16.5 + 阶段 2 §6 + §8 + §9 + §11 全部 LOCKED 设想落到 1 篇 ADR

### 负面

- ⚠️ **3 类组件集成测试需扩展**：当前 `apeireth-extension/tests/integration.rs` 仅测 1 类，需扩展到 3 类
- ⚠️ **`apeireth-mcp` 仍是 LOCKED 但 0 代码**：本 ADR 不创建新 crate（守"不修改 LOCKED"承诺），mcp 实装在阶段 4
- ⚠️ **CompatAdapter trait 新增**需在阶段 5 实装，本 ADR 仅作为设计层

### 中和

- 🛡️ **不修改 LOCKED**：阶段 1-5 文档未动，本 ADR 是独立新增
- 🛡️ **不写完整 Rust 代码**：本 ADR 仅 trait 定义示意，不实装 CompatAdapter
- 🛡️ **mcp 落地不在本 ADR**：明确为 LOCKED 待办，owner = 阶段 4 mcp-integration-expert

---

## 备选方案（Alternatives Considered）

### 选项 A: 不新增 ADR，让 3 类兼容组件各自独立

- ✅ 无新文档负担
- ❌ 阶段 3 蓝图无统一边界参考
- ❌ 后续 wasm / 浏览器 / Go FFI 各自重新发明轮子

### 选项 B: 在阶段 4 LOCKED 文档新增"兼容组件层"章节

- ✅ 与其他 LOCKED 章节同构
- ❌ 违反"不修改 LOCKED 文档"承诺（阶段 4 已 LOCKED）
- ❌ ADR 才是 LOCKED 之外的"补充记录"机制（MADR 4 工业标准）

### 选项 C: 在 `docs/adr/` 新增 ADR（本决策）

- ✅ 不修改任何 LOCKED
- ✅ ADR 是工业标准（MADR 4）
- ✅ 阶段 3 蓝图可直接参考
- ⚠️ 需新增 1 个 ADR 文件

---

## 实施路径（Implementation Path）

| 阶段 | 任务 | Owner | 依赖 |
|---|---|---|---|
| 阶段 4 | 实装 `apeireth-mcp` crate（LOCKED §14.4） | mcp-integration-expert | 本 ADR |
| 阶段 5 | 实装 `CompatAdapter` trait（与 `Plugin` 同构） | architect | 本 ADR + 阶段 5 §2 |
| 阶段 5 | 扩展 `apeireth-extension/tests/integration.rs` 覆盖 3 类 | qa_engineer | apeireth-mcp |
| 阶段 6 | 兼容组件层 milestone 验证（P28 阶段 6） | leader | 阶段 4 + 5 全部完成 |

---

## 关键不假装（Key Honesty Points）

- 🔴 **apeireth-mcp 是 LOCKED 但 0 代码**（本 ADR 不假装 mcp 已实装）
- 🟡 **CompatAdapter trait 仅设计层**（本 ADR 不实装 Rust 代码）
- 🟢 **apeireth-pybridge + apeireth-extension 已在 workspace 实装**（Cargo.toml + 实测代码 + 测试）
- 🟢 **3 类兼容组件的 LOCKED 设想来源全部登记**（阶段 1 §1/§14.4/§16.5 + 阶段 2 §6/§8/§9/§11）

---

## 主哲学 6 锚穿透

| 锚 | 落地表现 |
|---|---|
| 主 17:43 实事求是 | 3 类组件现状 = pybridge 实装 + extension 实装 + mcp LOCKED 0 代码（不假装 mcp 已实现） |
| 主 17:58 不假装 | 明确 mcp 是"LOCKED 设想未实装"而非"设计层遗漏" |
| 主 19:33 走在前人经验上 | ADR 是 MADR 4 工业标准 + trait 同构借鉴 Hermes Plugin trait + VCP 6 类插件协议 |
| 主 22:33 北极星 | 兼容组件层 = 主干 ↔ 外部生态的"桥梁"，直接服务 ASI 北极星 |
| 主 23:44 干到底 | 4 项硬规则 + 4 阶段实施路径 + owner 明确 |
| 主 00:56 任何人都能接手 | LOCKED 设想来源表 + 备选方案 A/B/C + 关键不假装 4 条 |

---

## 相关引用

- **前置 ADR**: [ADR 0001 双洋葱统一体](0001-double-onion-unity.md) + [ADR 0002 CLI 接入 core Session API](0002-cli-session-api-binding.md)
- **LOCKED 来源**: 阶段 1 §1 + §14.4 + §16.5 + 阶段 2 §6 + §8 + §9 + §11
- **实测代码**: `crates/apeireth-pybridge/src/{lib.rs, bridge.rs, python_bindings.rs, r11_compat.rs}` + `crates/apeireth-extension/src/lib.rs:40-51 (PluginKind::ALL)`
- **关联报告**: `reports/achievement-A16-mcp-integration-expert.md` + `reports/achievement-A16.3-mcp-integration-expert2-pybridge.md`

---

_V17 387832ef ADR 0007 (technical_writer) — 兼容组件层扩展边界正式确立, 不修改任何 LOCKED 文档._
_3 类组件统一 trait 表面 (Plugin + PyO3 + MCP) + 3 种部署模式 (InProcEmbed / InProcTrait / CrossProcess) + 4 项硬规则._
_阶段 1-5 LOCKED 全部 8 处设想落到 1 篇 ADR._
_任何接手者能查. 矩阵不可摘要替代._