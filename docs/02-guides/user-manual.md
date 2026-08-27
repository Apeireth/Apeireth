# Apeireth 用户手册（v2 工程重构线）

> 给运行 Apeireth 的人：它在做什么、我怎么控制它。机制描述以真实代码为准（v2 工程重构：默认分支 `main`，13-crate 工作区，tag `v2.0.0-alpha.1`）。
> 快速跑起来见 [quick-start.md](quick-start.md)；部署见 [deployment.md](deployment.md)；架构总览见根 [ARCHITECTURE.md](../../ARCHITECTURE.md)。

```
[Document-Meta]
Document:        docs/02-guides/user-manual.md
Version:         Manual-Rev-M + R131-D6 (v2 重写)
Last-Modified:   2026-08-27
Status:          🟢 活跃
```

## 1. Apeireth 是什么（v2）

**基地，不是 AI 本身**：Apeireth 是给 LLM 的操作系统——提供工具、记忆、治理边界、协议、运行时；不定义 AI 是什么。接入一个 LLM（默认 MiniMax-M3，Anthropic / OpenAI 兼容也可），基地给它：工具、记忆生命周期、治理闸门、可观测 trace。

**v2 两种运行形态**（v1 的 TUI / companion_serve / 三件套目录装配 已进 `legacy/`，恢复排期见根 `ROADMAP.md` §4）：

| 形态 | 命令 | 端口 | 说明 |
| | | | |
| **HTTP gateway** | `apeireth gateway serve --port 8080` | :8080 | canonical 入口，HTTP/SSE；OpenAI Chat Completions 兼容端点；薄壳适配器，全部委托 `Runtime::execute` |
| **CLI chat** | `apeireth chat "<prompt>"` | — | 单轮会话，直接走 agent loop（governance → provider → tool dispatch → 回灌续轮）|

`apeireth session` 启动交互会话；`apeireth --help` 看完整命令。`frontend/companion-desktop/` 是独立 Svelte 5 + Tauri 2 前端 workspace，不在根 cargo workspace 内。

## 2. 她怎么记得你（v2 形态）

v2 真实在 13-crate 工作区里运行的：

- **`crates/engine/memory`** 持**域原语**（Episode / Session / IdentityCard 等通过 `apeireth_core::kernel` 复用），vector/graph 检索契约 primitive 已在 M1B 落地；ACT-R 全量记忆模型、记忆 v2 (importance/对账/排名/版本链) 的完整实现**留 legacy/`，排期见 ROADMAP §4 P3**
- **Session 生命周期**（`crates/engine/runtime/src/canonical/session.rs`）：transcript + 持久化接缝（`InMemorySessionStore` + 可换 SQLite 等）；approval 可恢复（pending approval 以 outcome 形式返回，不丢失 turn 上下文）
- **Continuous session**通过 `SessionId` 跨 CLI/gateway 同一会话；前端切换不影响 trace 重建
- **数据落本地**：默认 SQLite WAL + `PRAGMA user_version` 版本化迁移（`crates/engine/storage`）

记忆 v2 的"对话打分 / 对账 / 排名注入"等流水线**功能等价物**将在 P3 阶段从 `legacy/donor/apeireth-companion` 移植回 `crates/engine/memory`，详见 ROADMAP §4 P3/P6。当前 v2 状态下记忆引擎已能接上 plugin / session / SQLite，**但"打字 → 评分 → 写库 → 检索注入"的端到端管线不在 13-crate 工作区里**——这是一个明确的可观察缺口，不是文档不诚实。

## 3. 主动能力（v2 状态）

- **agent loop 单一入口**（`crates/engine/runtime/src/canonical/execute.rs`）：一次 turn = governance（completion）→ provider → 工具调用则 capability lookup + 插件分发 → 工具结果回灌 transcript → 继续；approval 是 outcome 不是 error；tool 失败不终止回合。
- **模型选择**：`apeireth chat --model MiniMax-M3` 或 gateway request body 显式；fallback 顺序：minimax → anthropic → openai-compatible（已启用时），按环境变量配置
- **当前不会主动找你**——v1 的"涌现循环 / 开口策略 / 安静窗 / 主动送达"机制**整体留在 legacy/**，未移植；ROADMAP §4 P6-P7 排期

## 4. 工具（v2 形态）

v2 唯一进程执行边界 = **`crates/capabilities/tools/src/process/`**（`ProcessExecutor`，Windows Job Object + CREATE_SUSPENDED 完整，Linux/macOS 进程组部分）。

**`BuiltinToolsPlugin::new(workspace_root)` 默认注册 3 个只读工具**：

| 工具 | 风险 | 用途 |
| | | |
| `tool.filesystem` | medium | 读 / 列 / stat workspace 内文件 |
| `tool.search` | low | 确定性本地文件名 + 文件内容搜索 |
| `tool.repo` | low | 只读 git 仓库探查（status / commit / diff / log）|

**`tool.shell` 与 `tool.fetch` 默认关闭**，需 `BuiltinToolsOptions { shell: Some(TrustedShellConfig), fetch: Some(FetchConfig) }` 显式开启（opt-in，非默认）。这与 v1 companion_serve `APEIRETH_GRANT=...` 的临时放行模型不同——v2 的批准模型是**编译时 / bootstrap 时**的显式配置 + **运行时** governance pipeline（见 §6）。

工具调用走 **`<<<[TOOL_REQUEST]>>>` 已废弃**：v2 工具调用由各 provider 的原生 tool_calls 流式协议处理（OpenAI `tool_calls` 数组），不再走 marker 解析。`crates/capabilities/tools/src/plugin.rs` 的 plugin descriptor（`CapabilityDescriptor`）声明每个工具的 CapabilityId / kind / risk / M2* stage 标注，是唯一权威来源。

## 5. 她怎么"想"（v2 状态）

v2 砍掉了"全套器官"复杂度，保留了**可执行机制**和**可插拔接口**：

- **可执行**：`crates/engine/runtime::canonical::execute` 的 single-shot agent loop（governance + provider + tool dispatch + trace）
- **可插拔**：`ProviderCapability` trait（`crates/foundation/plugin`）+ 3 家 canonical provider（minimax/anthropic/openai-compatible）

v1 的"世界模型 / 好奇心 / 假设检验 / 情感记忆 / 价值内化 / 自我诊断"等**实现全部在 `legacy/donor/apeireth-companion`**，未移植；详见 ROADMAP §4 P6。

## 6. 治理与安全（v2 现状 + 0 装 PASS 标注）

**合同层**（`crates/foundation/governance/src/lib.rs`）：一个 trait（`GovernanceHook`）+ 三种决策（`Allow` / `Deny{reason}` / `RequireApproval{reason}`，`Deny` 与 `RequireApproval` 不可混淆——runtime 必须区分"拒绝"与"挂起"）；`GovernancePipeline` 按顺序短路第一个非 allow。

**可用的 hook 实现**（全部在 governance crate，**生产默认不挂载**）：

| 类型 | 名字 | 用途 |
| | | |
| `AllowAll` | "allow_all" | 默认 + 测试用；honest default（可见） |
| `DenyCapabilities` | "deny_capabilities" | 拒绝指定 CapabilityId 列表 |
| `MaxRounds` | "max_rounds" | 限制单 turn 工具循环轮数 |
| `PermissionGovernanceHook` | "permission_policy" | PermissionPolicy 包装（grant/revoke/需审批能力）|
| `PromptInjectionHook` | "prompt_injection_heuristic" | 启发式注入信号检测 |
| `CredentialDisclosureHook` | "credential_disclosure" | 凭据泄漏拦截 |
| `AuditHashChain` | — | 防篡改审计哈希链（append-only）|
| `PiiDetector` | — | PII 检测与 redact |

**生产治理接线（✅ 已通过 upstream `873d2857` 落地，2026-08-27）**：

- `crates/adapters/cli/src/lib.rs::build_canonical_runtime_from_env` **挂载 `GovernancePipeline`** = `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`（3 个）；配置来源 `build_production_governance_from_env()` 按环境变量 `APEIRETH_GOVERNANCE_*` 装配
- runtime 每个 turn 的 `CapabilityDispatch` **都**先经 governance pipeline 评估，**不再**默认 `AllowAll`
- `MaxRounds` 是**结构性**约束（runtime.rs 内部），`AuditHashChain` 按部署需要挂
- 工具层兜底：`shell` / `fetch` 默认 opt-in 关闭；`ProcessExecutor` 是 Windows Job Object 完整 / Linux·macOS 进程组部分隔离（参见 [architecture.md](../01-architecture/architecture.md) Process ownership 表）
- `credentials` 走 **`EnvCredentialResolver`**（`crates/engine/provider/src/credentials.rs`）：逻辑名→环境变量映射（`provider.minimax.api_key` → `APEIRETH_API_KEY`、`provider.anthropic.api_key` → `APEIRETH_ANTHROPIC_KEY`、`provider.openai-compatible.api_key` → `OPENAI_API_KEY`），无 secret 留 struct、secret 走 `Secret<T>`（debug redact）
- **`apeireth-credentials` crate（keyring / encrypted file / KMS backend）代码存在但未接线**——本批运行时用的是 `EnvCredentialResolver`；legacy OS keyring 集成排期见 ROADMAP §4 P2

**0 装 PASS 总结**：治理机制全部就位且有单测，**生产 bootstrap 装 3 个 hook**（Permission + 凭据泄漏 + 注入检测），其余 hook（MaxRounds 结构性、AuditHashChain 按需、DenyCapabilities 显式）由部署方决定。PII/越权/凭据泄漏 3 类核心攻击面被上游 `873d2857` 守住；剩余延后项按 ROADMAP §4 P2-P8 排。

## 7. SDK（v2 stub）

`crates/adapters/sdk` 提供 `ApeirethClient`（HTTP + WS）客户端类型 + 6 工具 method 签名（`web_search` / `file_ops` / `git_ops` / `code_exec` / `calendar` / `message`）+ 鉴权 5 组件 stub（Bearer / keyring / token bucket / audit / quota）。**真实 HTTP / WebSocket 调 `apeireth-gateway` 走 `unimplemented!()` 守门**，R21 真接。这是显式 stub，不是半成品——`crates/adapters/sdk/src/client.rs` §32 "阶段 6 stub 边界"列出了已就位 vs 待 R21 接的清晰范围。

## 8. 诚实标注（v2.0.0-alpha.1, 2026-08-27）

| 项 | 状态 |
| | |
| **生产 governance 接线** | ✅ 已做（upstream `873d2857`）：`PermissionGovernanceHook` + `CredentialDisclosureHook` + `PromptInjectionHook` 安装到 CLI bootstrap |
| **敏感 workspace 路径保护**（`.env` / `.ssh` / `.aws` / `.gnupg` / `.secret` 等）| ✅ 已做（upstream `ac5cbf5a`）：`tool.filesystem` + `tool.search` 通过 `crates/capabilities/tools/src/sensitive_path.rs` 屏蔽（普通项目元数据如 `.gitignore` / `.cargo/config.toml` 仍可读）|
| **13 键 verdict cache 接线** | ✅ 降级完成（2026-08-27）：`crates/foundation/core/src/philosophy.rs::RUNTIME_ENFORCED = false` 显式标注"非 runtime 强制"；`VERDICT_KEYS_BY_PRINCIPLE` 映射到 5 原则洋葱（E 存在 / S 价值 / A 经验 / M 方法论）。13 键 v2 角色 = 哲学标准 / 判别词汇表（hook deny reason 引用 + CapabilityDescriptor risk 分级）。**v2 取代 13 键强制机制 = external hook 闸**（已装 3 个）。 |
| **`apeireth-credentials` 接线** | ⚠️ P2：env resolver 在 provider，生产足够；keyring 等后端未挂 |
| **M1B 记忆全量移植（ACT-R / 完整管线）** | ⏳ P3：当前 memory crate 有 primitive；端到端管线待 P3 |
| **MCP 动态能力注册** | ⏳ P4 |
| **ProcessSupervisor + 进程树** | ⏳ P5 |
| **companion 器官移植（W1/W2/W3 / E4 / F4 / F1 / F6 / E7）** | ⏳ P6 |
| **连续感知（voice/screen）** | ⏳ P7 |
| **前端产品化对接（companion-desktop ↔ gateway）** | ⏳ P8 |
| Docker 多架构构建 | ✅ 已修（commit 4596357, $TARGETARCH, linux/amd64 + linux/arm64）|
| SDK 真接（HTTP / WS 调 `apeireth-gateway`）| ⏳ R21（stub 守门）|
| 13 键编译期 hardcode + 13 键测试契约 | ✅ 在 core 跑（`crates/foundation/core/tests/verdict_keys.rs` 等）|

## 9. FAQ

| 问题 | 答案 |
| | |
| 怎么跑？ | `cargo run -p apeireth-cli -- gateway serve --port 8080`（HTTP）或 `cargo run -p apeireth-cli -- chat "<prompt>"`（CLI）。 |
| 换模型？ | `APEIRETH_MINIMAX_API_KEY` / `APEIRETH_ANTHROPIC_KEY` / `OPENAI_API_KEY` 任一即可，按 fallback 顺序生效。改 provider 不用重编译（plugin 注入）。 |
| 启用 shell/fetch？ | bootstrap 时 `BuiltinToolsPlugin::with_options(root, BuiltinToolsOptions { shell: Some(...), fetch: Some(...) })`。CLI 默认不启用。 |
| 工具被拒了？ | 预期行为：要么工具未启用（opt-in 关），要么 governance hook deny / RequireApproval。当前 AllowAll 默认下工具不拒；启用 hook 后才生效。 |
| 我想接一个 IDE 一样的自定义模型？ | 实现 `ProviderCapability` trait + 注册为 plugin；参考 `crates/engine/provider/src/canonical_minimax.rs` 作为示例。 |
| Docker？ | `Dockerfile` 多架构已修；本机无 docker 详见 [CI 实测为准](https://github.com/Apeireth/apeireth-rust/actions)。 |
| 我看到的"13 键"在哪？ | `crates/foundation/core/src/philosophy.rs` + 测试 `crates/foundation/core/tests/verdict_keys.rs` 等。当前未接 canonical 执行路径（见 §8）。|

## 10. 一句话

**v2.0.0-alpha.1 = canonical agent loop + 13-crate 工作区 + 3 provider + 5 工具（2 opt-in）+ 治理 hook 全在 + 生产已挂 3 个（P0 接线 `873d2857`）+ sensitive_path 路径保护 (`ac5cbf5a`)**。alpha 这个名是诚实的：骨架与主链绿了，3 个核心安全 hook 已挂，剩余延后项按 ROADMAP §4 P2-P8 排。

---

_本手册 v2 重写 (2026-08-27)：替换 v1 companion_serve / TUI / `<<<[TOOL_REQUEST]>>>` marker 全部形态为 gateway + canonical agent loop 现实，§6 治理与 §8 诚实标注严格 0 装 PASS（什么在、什么不在、为什么）。v1 形态在 legacy/，恢复排期见根 ROADMAP §4。_