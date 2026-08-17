# Provider 客户端 SDK（5 客户端）

> **依据**: `crates/apeireth-provider-{claude-code,gemini-cli,codex,copilot,opencode}/`
> **最后更新**: 2026-08-05
> **状态**: claude-code 真接；其他 4 stub

---

## 1. 5 客户端概览

| Provider | 真接 vs Stub | R21 工作量 |
|---|---|---|
| **claude-code** | ✅ 真接 | — (1.0 完成) |
| **gemini-cli** | 🟡 stub | 1 owner × 1 周 |
| **codex** | 🟡 stub | 1 owner × 1 周 |
| **copilot** | 🟡 stub | 1 owner × 1 周 |
| **opencode** | 🟡 stub | 1 owner × 1 周 |

详见 API 视角：[docs/api/provider-claude-code.md](../api/provider-claude-code.md)

---

## 2. 共同 trait: `ProviderClient`

```rust
#[async_trait]
pub trait ProviderClient: Send + Sync {
    fn name(&self) -> &'static str;
    fn models(&self) -> &[ModelKind];
    
    async fn invoke(
        &self,
        model: ModelKind,
        messages: Vec<Message>,
        tools: &[ToolSpec],
    ) -> Result<Completion, ProviderError>;
    
    async fn stream(
        &self,
        model: ModelKind,
        messages: Vec<Message>,
        tools: &[ToolSpec],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Delta, ProviderError>> + Send>>, ProviderError>;
}

pub struct Completion {
    pub text: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub usage: TokenUsage,
    pub stop_reason: StopReason,
}

pub struct Delta {
    pub text: String,
    pub index: usize,
    pub is_final: bool,
}
```

---

## 3. claude-code（真接）

```rust
use apeireth_provider_claude_code::{Client, ClaudeModel, Message};

let client = Client::new(std::env::var("ANTHROPIC_API_KEY")?);

// 非流式
let resp = client
    .model(ClaudeModel::Sonnet)
    .messages(vec![Message::user("Hello")])
    .invoke()
    .await?;

println!("text: {}", resp.text);
println!("tokens: {}", resp.usage.total_tokens);

// 流式
use futures::StreamExt;
let mut stream = client
    .model(ClaudeModel::Sonnet)
    .messages(vec![Message::user("Hello")])
    .stream()
    .await?;

while let Some(delta) = stream.next().await {
    let d = delta?;
    print!("{}", d.text);
    if d.is_final { break; }
}
```

**3 ModelKind**:
```rust
pub enum ClaudeModel {
    Sonnet,    // claude-3-5-sonnet-20241022
    Opus,      // claude-3-opus-20240229
    Haiku,     // claude-3-haiku-20240307
}
```

**工具调用**:
```rust
let resp = client
    .model(ClaudeModel::Sonnet)
    .messages(vec![Message::user("查一下日历")])
    .tools(&[
        ToolSpec::new("calendar", "list_events")
            .param("start", ParamType::String, true)
            .param("end", ParamType::String, true),
    ])
    .invoke()
    .await?;

for call in resp.tool_calls {
    println!("call: {} {}", call.tool, call.action);
}
```

---

## 4. gemini-cli（stub）

```rust
use apeireth_provider_gemini_cli::{Client, GeminiModel, Message};

let client = Client::new(std::env::var("GEMINI_API_KEY")?);

let resp = client
    .model(GeminiModel::Gemini25Pro)
    .messages(vec![Message::user("hi")])
    .invoke()
    .await;
// 1.0 stub: NotImplemented
```

**3 ModelKind**: Gemini25Pro / Gemini25Flash / Gemini20Flash

---

## 5. codex（stub）

```rust
use apeireth_provider_codex::{Client, CodexModel, Message, SandboxType};

let client = Client::new(std::env::var("OPENAI_API_KEY")?);

let resp = client
    .model(CodexModel::Codex)
    .sandbox(SandboxType::WorkspaceWrite)
    .messages(vec![Message::user("hi")])
    .invoke()
    .await;
// 1.0 stub
```

**4 ModelKind**: Codex / CodexMini / O3 / O4Mini
**3 SandboxType**: WorkspaceWrite / ReadOnly / DangerFullAccess

---

## 6. copilot（stub）

```rust
use apeireth_provider_copilot::{Client, CopilotModel, Message};

let client = Client::new(std::env::var("GITHUB_TOKEN")?);

let resp = client
    .model(CopilotModel::CopilotChat)
    .messages(vec![Message::user("hi")])
    .invoke()
    .await;
// 1.0 stub
```

**3 ModelKind**: CopilotChat / CopilotEnterprise / CopilotImmersive
**特有**: GitHub OAuth 2.0 device / web flow

---

## 7. opencode（stub）

```rust
use apeireth_provider_opencode::{Client, OpencodeModel, Message};

let client = Client::new(std::env::var("OPENCODE_API_KEY")?);

// llm 模式
let resp = client
    .model(OpencodeModel::OpencodeLlm)
    .messages(vec![Message::user("hi")])
    .invoke()
    .await;
// 1.0 stub

// embed 模式（特有）
let embedding = client
    .model(OpencodeModel::OpencodeEmbed)
    .embed("text to embed")
    .await?;
// 1.0 stub

// rerank 模式（特有）
let ranked = client
    .model(OpencodeModel::OpencodeRerank)
    .rerank(&query, &candidates)
    .await?;
// 1.0 stub
```

**3 ModelKind**: OpencodeLlm / OpencodeEmbed / OpencodeRerank

---

## 8. ProtocolRouter（统一调度）

```rust
use apeireth_provider::{ProtocolRouter, ProviderClient};

let router = ProtocolRouter::new()
    .register("claude-code", Box::new(ClaudeClient::new(key))?)
    .register("gemini-cli", Box::new(GeminiClient::new(key)?))
    .register("codex", Box::new(CodexClient::new(key)?))
    .register("copilot", Box::new(CopilotClient::new(key)?))
    .register("opencode", Box::new(OpencodeClient::new(key)?));

// 按 provider 名路由
let resp = router
    .provider("claude-code")
    .invoke(model, messages, tools)
    .await?;
```

**协议**: 4 协议统一抽象 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Google Gemini)
per `crates/apeireth-protocol/src/abi.rs`

---

## 9. 错误处理

```rust
pub enum ProviderError {
    NotImplemented { provider: &'static str, action: &'static str },
    Auth(String),
    RateLimit { retry_after: Duration },
    Network(String),
    InvalidRequest(String),
    Upstream(String),
    ModelUnavailable(String),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool;
    pub fn retry_after(&self) -> Option<Duration>;
}
```

---

## 10. 5 K-1 强校验（per `apeireth-protocol`）

每个 Provider 客户端都过 5 守门:

1. **token**: API key 启动期非空
2. **model**: 必须在白名单
3. **scope**: tool scope 覆盖
4. **rate_limit**: token bucket 检查
5. **org/enterprise**: 视 Provider 特定

```rust
fn validate_invoke(req: &InvokeRequest) -> Result<(), ProviderError> {
    validate_token(&req.api_key)?;
    validate_model(req.model)?;
    validate_scope(req.tool_calls.iter().map(|tc| tc.tool))?;
    validate_rate_limit(req.user_id)?;
    validate_org(req.org_id)?;
    Ok(())
}
```

---

## 11. m3 hallucination 防御

per `docs/stage4/m3-hallucination-defense-2026-08-05.md` 4 守门:

1. **TOOL_WHITELIST 编译期 hardcode** — 8 工具枚举固定
2. **validate_tool_call schema 校验** — 拒绝幻觉 tool
3. **scope 校验** — tool scope 覆盖
4. **rate_limit** — 防 tool flood

---

## 12. 不假装

- ✅ claude-code 1.0 真接（@anthropic-ai/claude-agent-sdk 0.2.112）
- 🟡 其他 4 stub 留 R21
- ✅ 共同 trait 抽象
- ✅ 5 K-1 强校验
- ✅ 4 m3 守门
- ✅ ProtocolRouter 统一调度
- ✅ 不依赖 NewAPI（自建 5 客户端）

---

## 13. 相关

- 5 crates: `crates/apeireth-provider-{claude-code,gemini-cli,codex,copilot,opencode}/`
- 协议: `crates/apeireth-protocol`
- API 视角: [`docs/api/provider-claude-code.md`](../api/provider-claude-code.md)
- 蓝图: `docs/stage4/5-provider-tool-mapping-2026-08-05.md`
