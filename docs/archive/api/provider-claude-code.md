# Provider 客户端 API（5 Provider 视角）

> **依据**: `crates/apeireth-provider-{claude-code,gemini-cli,codex,copilot,opencode}/src/`
> **最后更新**: 2026-08-05
> **状态**: claude-code 真接 SDK；其他 4 Provider stub（R21 续真接）

---

## 1. 5 Provider 概览

| Provider | crate | 8 工具 | 3-4 ModelKind | 真接 vs Stub | 1:1 翻译源 |
|---|---|---|---|---|---|
| **claude-code** | `apeireth-provider-claude-code` | ✅ | 3 (Sonnet/Opus/Haiku) | ✅ **真接** | @anthropic-ai/claude-agent-sdk 0.2.112 |
| **gemini-cli** | `apeireth-provider-gemini-cli` | ✅ | 3 (Gemini25Pro/Flash/2Flash) | 🟡 stub | @google/gemini-cli 0.9.21 |
| **codex** | `apeireth-provider-codex` | ✅ | 4 (codex/codex-mini/o3/o4-mini) | 🟡 stub | @openai/codex 0.9.21 |
| **copilot** | `apeireth-provider-copilot` | ✅ | 3 (Chat/Enterprise/Immersive) | 🟡 stub | @github/copilot-sdk 0.9.21 |
| **opencode** | `apeireth-provider-opencode` | ✅ | 3 (llm/embed/rerank) | 🟡 stub | @opencode-ai/sdk 1.17.15 |

> 1:1 翻译指 v0.9.21 商业版 client.js 行为对齐。**claude-code 1.0 release 真接 SDK**，其他 4 个 stub 留 R21 续。

---

## 2. 共同 8 工具

所有 5 Provider 客户端暴露同一组 8 工具（per `apeireth-tools`）:

| 工具 | 功能 |
|---|---|
| `ReadFile` | 读文件 |
| `WriteFile` | 写文件 |
| `Edit` | 编辑（per 搜索+替换） |
| `Bash` | shell 命令（per `code_exec.rs`） |
| `Grep` | 文本搜索 |
| `Glob` | 文件 glob |
| `WebFetch` | HTTP GET |
| `WebSearch` | Web 搜索 |

每 Provider 都有 `TOOL_WHITELIST` 编译期 hardcode（per `apeireth-protocol`），防止 LLM 调未授权工具。

---

## 3. claude-code（真接）

**实现**: `crates/apeireth-provider-claude-code/`

**3 ModelKind**:
```rust
pub enum ClaudeModel {
    Sonnet,    // claude-3-5-sonnet-20241022
    Opus,      // claude-3-opus-20240229
    Haiku,     // claude-3-haiku-20240307
}
```

**SDK 接入**:
```rust
use apeireth_provider_claude_code::{Client, ClaudeModel, Message};

let client = Client::new(std::env::var("ANTHROPIC_API_KEY")?);

let resp = client
    .model(ClaudeModel::Sonnet)
    .messages(vec![Message::user("Hello")])
    .invoke()
    .await?;

println!("{}", resp.text);
```

**5 K-1 强校验** (per `apeireth-protocol/security.rs`):
1. **token**: API key 启动期非空
2. **model**: 必须在白名单
3. **scope**: tool 调用 scope 覆盖
4. **rate_limit**: token bucket 检查
5. **org**: org 隔离（per user scope）

---

## 4. gemini-cli（stub）

**实现**: `crates/apeireth-provider-gemini-cli/`

**3 ModelKind**:
```rust
pub enum GeminiModel {
    Gemini25Pro,
    Gemini25Flash,
    Gemini20Flash,
}
```

**当前状态**:
- ✅ 8 工具全实现
- ✅ 3 ModelKind 枚举
- ✅ TOOL_WHITELIST 编译期
- ✅ 4 K-1 强校验
- 🟡 SDK 调用 stub（`unimplemented!()` 返 501）→ R21 续

---

## 5. codex（stub）

**实现**: `crates/apeireth-provider-codex/`

**4 ModelKind**:
```rust
pub enum CodexModel {
    Codex,        // o1
    CodexMini,    // o1-mini
    O3,
    O4Mini,
}
```

**特有**: 3 SandboxType
```rust
pub enum SandboxType {
    WorkspaceWrite,
    ReadOnly,
    DangerFullAccess,
}
```

**当前状态**: 同 gemini-cli，stub 留 R21 续

---

## 6. copilot（stub）

**实现**: `crates/apeireth-provider-copilot/`

**3 ModelKind**:
```rust
pub enum CopilotModel {
    CopilotChat,
    CopilotEnterprise,
    CopilotImmersive,
}
```

**特有**: GitHub OAuth 2.0 device / web flow

**5 K-1 强校验**:
1. token (GitHub PAT)
2. model
3. scope (tool)
4. org
5. enterprise (Enterprise tier)

---

## 7. opencode（stub）

**实现**: `crates/apeireth-provider-opencode/`

**3 ModelKind**:
```rust
pub enum OpencodeModel {
    OpencodeLlm,
    OpencodeEmbed,    // 特有：向量 embedding
    OpencodeRerank,   // 特有：rerank
}
```

**特有**:
- 向量 embedding (opencode-embed) 集成到 `apeireth-vector`
- 相似度检索 (L2 / dot / cosine)
- 3 similarity metric

---

## 8. Provider 路由

LLM 调用走 `apeireth-protocol` ProtocolRouter：

```rust
let router = ProtocolRouter::new()
    .register("claude-code", claude_code_client)
    .register("gemini-cli", gemini_cli_client)
    .register("codex", codex_client)
    .register("copilot", copilot_client)
    .register("opencode", opencode_client);

let resp = router
    .provider("claude-code")
    .model("sonnet")
    .messages(vec![Message::user("hi")])
    .invoke()
    .await?;
```

**协议**: 4 协议 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Google Gemini)，per `apeireth-protocol/src/abi.rs`

---

## 9. m3 hallucination 防御（per `apeireth-protocol`）

4 P0 守门（per `docs/stage4/m3-hallucination-defense-2026-08-05.md`）:
1. **TOOL_WHITELIST 编译期 hardcode** — 8 工具枚举固定
2. **validate_tool_call schema 校验** — 拒绝幻觉 tool
3. **scope 校验** — tool scope 覆盖
4. **rate_limit** — 防 tool flood

---

## 10. 指标

| 指标 | 标签 | 类型 |
|---|---|---|
| `apeireth_llm_tokens_total` | `provider`, `model` | counter |
| `apeireth_llm_request_duration_seconds` | `provider`, `model` | histogram |
| `apeireth_tool_calls_total` | `provider`, `tool` | counter |
| `apeireth_tool_call_errors_total` | `provider`, `tool`, `code` | counter |

---

## 11. 不假装

- ✅ claude-code 1.0 release 真接（@anthropic-ai/claude-agent-sdk 0.2.112）
- 🟡 其他 4 Provider stub 留 R21 续真接
- ✅ 5 客户端 8 工具全实现
- ✅ TOOL_WHITELIST + 5 K-1 强校验全实装
- ✅ 不依赖 NewAPI（自建 5 客户端）

---

## 12. 相关

- 5 crates: `crates/apeireth-provider-{claude-code,gemini-cli,codex,copilot,opencode}/`
- 协议: `crates/apeireth-protocol` (4 协议 + ProtocolRouter)
- 工具: `crates/apeireth-tools` (8 工具)
- 安全: `docs/stage4/m3-hallucination-defense-2026-08-05.md`
- 蓝图: `docs/stage4/5-provider-tool-mapping-2026-08-05.md`
