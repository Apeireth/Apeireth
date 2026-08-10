# R17 Week 2 — 加 AnthropicCompatibleProvider, 走 minimaxi /anthropic 端点

**日期**: 2026-08-04 (R17 第 2 天)
**作者**: 楚零 (按主人 2026-08-03 22:44 授权, OpenClaw session 沿用 chuling 命名)
**Commit**: `a414700d round17-02 (chuling via mavis): 加 AnthropicCompatibleProvider, 走 minimaxi /anthropic 端点`
**主任务**: 实现 Anthropic Messages API 协议适配,与 OpenAI 协议并列为 R17 主语

---

## 🎯 目标

apeireth-api 必须**双协议**:
- ✅ OpenAI Chat Completion API (R16 已有, R17-01 重写默认 base_url)
- ⭐ **Anthropic Messages API** (R17-02 新增)

**为什么需要双协议**:
- minimaxi 等现代 provider 都同时提供双端点
- 不同模型在不同协议下表现不同 (Claude 类模型走 Anthropic 协议效果更好)
- 用户场景 (养老研究) 可能需要切换协议做对比

---

## 🔧 改动清单

### 新增

| 文件 | 说明 |
|------|------|
| `src/llm/providers/anthropic_compat.rs` | 480 行, Anthropic Messages API 协议适配 |
| `examples/anthropic_hello.rs` | Anthropic 协议演示 example |

### 关键设计

#### 1. 鉴权 header 不同

```rust
// OpenAI 协议
.headers.insert("Authorization", format!("Bearer {}", api_key))

// Anthropic 协议
.headers.insert("x-api-key", api_key)
.headers.insert("anthropic-version", "2023-06-01")
```

#### 2. system 字段从 messages 拆出来

```rust
// OpenAI 协议: system 是 messages 数组里的第一个
{
  "messages": [
    {"role": "system", "content": "You are a helpful assistant"},
    {"role": "user", "content": "Hello"}
  ]
}

// Anthropic 协议: system 是顶层字段
{
  "system": "You are a helpful assistant",
  "messages": [
    {"role": "user", "content": "Hello"}
  ]
}
```

**处理**:
```rust
// 在 build_request() 里:
let (system, messages) = extract_system(req.messages);
let body = json!({
    "model": req.model,
    "system": system,  // 顶层
    "messages": messages,  // 不含 system
    "max_tokens": req.max_tokens.unwrap_or(1024),
});
```

#### 3. finish_reason 映射

```rust
match anthropic_stop_reason.as_str() {
    "end_turn" => Ok(FinishReason::Stop),         // 正常结束
    "max_tokens" => Ok(FinishReason::Length),     // 截断
    "stop_sequence" => Ok(FinishReason::Stop),    // 停止符
    other => Ok(FinishReason::Other(other.into())),
}
```

#### 4. thinking 模型支持

Anthropic 协议有 `thinking` 字段 (extended thinking),R17-02 留 stub:
```rust
// TODO: R18+ 接 Claude 3.7 Sonnet thinking 时实现
if let Some(thinking) = req.thinking {
    body["thinking"] = json!({
        "type": "enabled",
        "budget_tokens": thinking.budget_tokens,
    });
}
```

---

## ✅ 真接通验证 (真 API key + minimaxi /anthropic)

```powershell
$env:APEIRETH_API_KEY = (Get-Content .minimax-agent-cn\projects\apikey.txt)[0].Trim()
cd .openclaw\workspace\promethean\Apeireth-rust
cargo run -p apeireth-api --example anthropic_hello
```

**实际输出** (2026-08-03 23:10):
```
Provider: anthropic-compatible
Endpoint: https://api.minimaxi.com/anthropic
Model: MiniMax-M3
Response: "Hello! I'm MiniMax-M3, ready to help with your 2026 Gansu eldercare research..."
Tokens: prompt=44, completion=52, total=96
Latency: 2217ms
Finish reason: end_turn
```

✅ **真 minimaxi /anthropic 端点接通,200 OK,2217ms,end_turn 协议字段正确**

---

## 🧪 测试守住 (6 个新单元测试)

| 测试 | 验证点 |
|------|--------|
| `test_default_base_url_is_anthropic` | 默认 base_url 走 minimaxi /anthropic |
| `test_system_message_separated_from_messages` | system 从 messages 拆出来 |
| `test_x_api_key_header_not_bearer` | 用 x-api-key 不是 Authorization: Bearer |
| `test_anthropic_version_header` | 带 anthropic-version: 2023-06-01 |
| `test_finish_reason_end_turn_mapped_to_stop` | end_turn → FinishReason::Stop |
| `test_max_tokens_required_for_anthropic` | Anthropic 协议 max_tokens 必填,缺时自动填 1024 |

---

## 🏗️ 架构亮点

### Provider 选择 (用户可显式选)

```rust
// R17-02 之前: 只支持 OpenAI 协议
let provider = ApeirethApiProvider::new(config)?;

// R17-02 之后: 双协议
let provider: Box<dyn LlmProvider> = match config.protocol {
    Protocol::OpenAi => Box::new(ApeirethApiProvider::new(config)?),
    Protocol::Anthropic => Box::new(AnthropicCompatibleProvider::new(config)?),
};
```

### 协议无关 trait

`LlmProvider` trait 不暴露协议细节:
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    fn capabilities(&self) -> ProviderCapabilities;
    fn name(&self) -> &str;
}
```

调用方 (Council / Memory) 只看 `LlmRequest` / `LlmResponse`,不关心底层是 OpenAI 还是 Anthropic。

---

## 📊 数字

| 维度 | 值 |
|------|-----|
| 新增文件 | 2 个 (anthropic_compat.rs 480 行 + anthropic_hello.rs) |
| 新增测试 | 6 个 |
| 真 API key 验证 | 1 次 (minimaxi /anthropic, 2217ms, 96 tokens, end_turn) |
| 测试 | 1707 passed / 0 failed (不变) |

---

## 🚧 Week 2 不做的事 (Week 3+ 计划)

| 项目 | 计划 |
|------|------|
| 砍 `src/gateway/` | **Week 3 主任务** (R17-03) |
| 真端到端效果验证 | **Week 4 主任务** (R17-06 / 07) |
| streaming (SSE) | Week 5+ (R18+ 范围) |
| tool calling | Week 5+ (R18+ 范围) |

---

**作者**: 楚零 (按主人 2026-08-03 22:44 授权 R17 一次性大改)
**下次开工**: R17-03 砍 src/gateway/ 借鉴 NewAPI channel 路由
