# Rust SDK (`apeireth-sdk`) 详细 API

> **依据**: `crates/apeireth-sdk/src/{client,wire,error,version,abi}.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: 1.0 release 真接

---

## 1. 依赖

```toml
[dependencies]
apeireth-sdk = "1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

> workspace 锁定的 reqwest 0.12 / tokio 1.40 / serde 1.0 直接用，不需重声明

---

## 2. 模块结构

```
apeireth-sdk/
├── lib.rs              # 公共 API
├── client.rs           # Client 主入口
├── wire.rs             # 协议 + 序列化
├── abi.rs              # 4 协议 ABI
├── error.rs            # Error 类型
├── version.rs          # SDK_VERSION 常量
├── auth/               # 鉴权（auto refresh）
├── ws/                 # WebSocket 客户端
├── tools/              # 6 工具便捷方法
└── provider/           # 5 Provider 客户端
```

---

## 3. Client 主入口

### 3.1 构造

```rust
use apeireth_sdk::Client;

// 1. 简单构造
let client = Client::new("https://api.apeireth.dev");

// 2. 带 token
let client = Client::new("https://api.apeireth.dev")
    .with_token("access_token_15min");

// 3. 带 AuthClient（自动 refresh）
let auth = AuthClient::new("https://api.apeireth.dev");
let client = Client::new("https://api.apeireth.dev")
    .with_auth(auth)
    .build();

// 4. 自定义超时
let client = Client::new("https://api.apeireth.dev")
    .timeout(std::time::Duration::from_secs(60))
    .build();
```

### 3.2 Builder 模式

```rust
pub struct ClientBuilder {
    base_url: String,
    token: Option<String>,
    auth: Option<AuthClient>,
    timeout: Duration,
    retry_policy: RetryPolicy,
    user_agent: String,
}

impl ClientBuilder {
    pub fn new(base_url: impl Into<String>) -> Self;
    pub fn with_token(self, token: impl Into<String>) -> Self;
    pub fn with_auth(self, auth: AuthClient) -> Self;
    pub fn timeout(self, t: Duration) -> Self;
    pub fn retry_policy(self, p: RetryPolicy) -> Self;
    pub fn user_agent(self, ua: impl Into<String>) -> Self;
    pub fn build(self) -> Client;
}
```

---

## 4. 6 工具调用

### 4.1 通用方法

```rust
impl Client {
    pub fn tool(&self, name: impl Into<String>) -> ToolInvocation<'_>;
}

pub struct ToolInvocation<'a> {
    client: &'a Client,
    tool: String,
}

impl<'a> ToolInvocation<'a> {
    pub fn action(self, action: impl Into<String>) -> ActionInvocation<'a>;
    pub fn params(self, params: serde_json::Value) -> ActionInvocation<'a>;
}

impl<'a> ActionInvocation<'a> {
    pub async fn invoke<T: DeserializeOwned>(self) -> Result<T, Error>;
    pub async fn invoke_raw(self) -> Result<Bytes, Error>;
}
```

### 4.2 calendar 示例

```rust
use apeireth_sdk::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct ListEventsResult {
    events: Vec<Event>,
    next_page_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Event {
    id: String,
    summary: String,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
}

let client = Client::new("https://api.apeireth.dev").with_token(token);

let result: ListEventsResult = client
    .tool("calendar")
    .action("list_events")
    .params(json!({
        "start": "2026-08-05T00:00:00Z",
        "end": "2026-08-12T00:00:00Z"
    }))
    .invoke()
    .await?;

for event in result.events {
    println!("{}: {}", event.start, event.summary);
}
```

### 4.3 强类型便捷方法（per 工具）

```rust
// calendar
client.calendar().list_events(start, end).await?;
client.calendar().get_event(event_id).await?;

// message
client.message().send(channel, to, subject, body).await?;

// drive
client.drive().upload(name, bytes, mime).await?;
client.drive().download(drive_id).await?;

// search
client.search().query(query, mode, limit).await?;
```

> 这些便捷方法是对通用 `tool().action()` 的语法糖，避免重复写 action 名 + params 结构

---

## 5. WebSocket 客户端

```rust
use apeireth_sdk::ws::{WsClient, Frame};
use futures::StreamExt;

let mut ws = WsClient::connect(
    "wss://api.apeireth.dev/v1/ws",
    "ws_token_5min_ttl"
).await?;

// 发消息
ws.chat("apeireth 1.0 release 进度？").await?;

// 接收流
while let Some(frame) = ws.next().await {
    match frame? {
        Frame::Auth(_) => {}
        Frame::Chat(_) => {}
        Frame::Delta(d) => {
            print!("{}", d.text);
            if d.is_final { break; }
        }
        Frame::ToolCall(tc) => {
            let result = match tc.tool.as_str() {
                "search" => call_search(&tc.params).await?,
                _ => return Err("unknown tool".into()),
            };
            ws.tool_result(&tc.call_id, result).await?;
        }
        Frame::ToolResult(_) => {}
        Frame::Error(e) => return Err(e.into()),
        Frame::Ping => ws.pong().await?,
        Frame::Pong => {}
        Frame::Close(_) => break,
    }
}
```

---

## 6. 鉴权

```rust
use apeireth_sdk::auth::AuthClient;

let auth = AuthClient::new("https://api.apeireth.dev");

// 登录
let tokens = auth.login("alice@apeireth.dev", "password").await?;
println!("access: {}", tokens.access_token);

// refresh
let new_tokens = auth.refresh(&tokens.refresh_token).await?;

// 登出
auth.logout(&tokens.refresh_token).await?;
```

**自动 refresh**:
```rust
let client = Client::new("https://api.apeireth.dev")
    .with_auth(auth)
    .build();

// 内部 tower middleware: access_token 剩余 < 60s 时自动 refresh
client.tool("calendar").action("list_events").invoke().await?;
```

---

## 7. 错误处理

```rust
use apeireth_sdk::Error;

match client.tool("calendar").action("list_events").invoke::<ListEventsResult>().await {
    Ok(events) => { /* success */ }
    Err(Error::Auth(e)) => { /* AUTH_* 错误 */ }
    Err(Error::RateLimit { retry_after }) => { /* 429 */ }
    Err(Error::ToolNotFound { tool }) => { /* 404 */ }
    Err(Error::NotImplemented { tool, action }) => { /* 501 stub */ }
    Err(Error::Validation(msg)) => { /* 400 */ }
    Err(Error::Upstream(msg)) => { /* 502 */ }
    Err(Error::Network(e)) => { /* 连接错 */ }
    Err(Error::Server(msg)) => { /* 500 */ }
}
```

---

## 8. 重试策略

```rust
use apeireth_sdk::RetryPolicy;
use std::time::Duration;

let policy = RetryPolicy::default()
    .max_retries(3)
    .backoff(Backoff::Exponential {
        initial: Duration::from_millis(100),
        max: Duration::from_secs(5),
    })
    .retry_on(|e| matches!(e, Error::Network(_) | Error::Upstream(_)));

let client = Client::new("https://api.apeireth.dev")
    .retry_policy(policy)
    .build();
```

---

## 9. 5 Provider 客户端

```rust
use apeireth_provider_claude_code::{Client as ClaudeClient, ClaudeModel, Message};

let client = ClaudeClient::new(std::env::var("ANTHROPIC_API_KEY")?);

let resp = client
    .model(ClaudeModel::Sonnet)
    .messages(vec![Message::user("Hello")])
    .invoke()
    .await?;

println!("{}", resp.text);
```

其他 4 Provider 同模式，stub 返 501。

---

## 10. 版本

```rust
use apeireth_sdk::version;

println!("apeireth-sdk {}", version::SDK_VERSION);  // "1.0.0"
println!("api {}", version::API_VERSION);            // "v1"
```

**SDK_VERSION** 跟 workspace version 同步（1.0.0，per D-05 拍板）

---

## 11. 完整示例

```rust
use apeireth_sdk::{Client, auth::AuthClient};
use apeireth_sdk::ws::{WsClient, Frame};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 登录
    let auth = AuthClient::new("https://api.apeireth.dev");
    let tokens = auth.login("alice@apeireth.dev", "password").await?;

    // 2. REST 调用
    let client = Client::new("https://api.apeireth.dev")
        .with_auth(auth.clone())
        .build();

    let events = client.tool("calendar").action("list_events")
        .params(json!({
            "start": "2026-08-05T00:00:00Z",
            "end": "2026-08-12T00:00:00Z"
        }))
        .invoke().await?;
    println!("Found {} events", events["events"].as_array().unwrap().len());

    // 3. WebSocket 流式
    let ws_token = auth.ws_token(&tokens.access_token).await?;
    let mut ws = WsClient::connect("wss://api.apeireth.dev/v1/ws", &ws_token).await?;
    ws.chat("本周有哪些会议？").await?;

    while let Some(frame) = ws.next().await {
        match frame? {
            Frame::Delta(d) => print!("{}", d.text),
            Frame::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}
```

---

## 12. 不假装

- ✅ Client + 6 工具 + WS + Auth + Error 全真接
- ✅ 编译期 hardcode：工具白名单 + 协议 schema
- ✅ 5 Provider 客户端：claude-code 真接，4 stub
- ✅ 自动 refresh-on-use 中间件

---

## 13. 相关

- 实现: `crates/apeireth-sdk/src/`
- API: [`docs/api/README.md`](../api/README.md)
- 决策: [`docs/adr/0018-rust-sdk-design.md`](../adr/0018-rust-sdk-design.md)
