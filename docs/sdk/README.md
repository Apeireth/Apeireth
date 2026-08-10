# Apeireth SDK 总览

> **性质**: Apeireth 多语言 SDK 索引（Rust / Python / Lark / Voice / LiveKit / Sandbox）
> **依据**: `crates/apeireth-sdk/` + 5 stub SDK 实际实现
> **最后更新**: 2026-08-05
> **状态**: Rust SDK 1.0 真接；Python / Lark / Voice / LiveKit / Sandbox 5 个 stub

---

## 1. SDK 总览

| SDK | 实现位置 | 1.0 状态 | 文档 |
|---|---|---|---|
| **Rust SDK** | `crates/apeireth-sdk/` | ✅ 真接 | [rust-sdk.md](rust-sdk.md) |
| **Lark SDK** | `crates/apeireth-lark/` | 🟡 stub（8 NotImplemented） | [lark-sdk.md](lark-sdk.md) |
| **Voice SDK** | `crates/apeireth-voice/` | 🟡 stub（默认唤醒词 "apeireth"） | [voice-sdk.md](voice-sdk.md) |
| **LiveKit SDK** | `crates/apeireth-sdk-livekit/` | 🟡 stub | [livekit-sdk.md](livekit-sdk.md) |
| **Sandbox SDK** | `crates/apeireth-sdk-sandbox/` | 🟡 stub | [sandbox-sdk.md](sandbox-sdk.md) |
| **Provider 客户端** | 5 crate | 🟡 1 真接 + 4 stub | [provider-claude-code.md](provider-claude-code.md) |

> **不假装**: Python SDK 估补 [docs/architecture] R21 估补；当前只有 Rust + 5 stub 客户端

---

## 2. 选型决策（per R20 阶段 4 拍板）

| 维度 | 决策 | 理由 |
|---|---|---|
| **核心语言** | Rust | 后端主力，类型安全 + 性能 |
| **多语言策略** | 5 stub | R21 商业化再扩 (Python/Go/TS) |
| **HTTP 客户端** | reqwest 0.12 | workspace 锁 |
| **JSON** | serde + serde_json | workspace 锁 |
| **异步** | tokio 1.40 | workspace 锁 |
| **错误** | thiserror | workspace 锁 |

---

## 3. Rust SDK 用法（5 分钟上手）

### 3.1 安装

```toml
# Cargo.toml
[dependencies]
apeireth-sdk = "1.0"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### 3.2 同步调用

```rust
use apeireth_sdk::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.apeireth.dev")
        .with_token("your-access-token");

    // 列日历事件
    let events = client
        .tool("calendar")
        .action("list_events")
        .params(json!({
            "start": "2026-08-05T00:00:00Z",
            "end": "2026-08-12T00:00:00Z"
        }))
        .invoke()
        .await?;

    println!("Found {} events", events["events"].as_array().unwrap().len());
    Ok(())
}
```

### 3.3 流式 WebSocket

```rust
use apeireth_sdk::ws::WsClient;
use futures::StreamExt;

let mut ws = WsClient::connect(
    "wss://api.apeireth.dev/v1/ws",
    "ws_token_5min"
).await?;

ws.chat("apeireth 1.0 release 进度？").await?;

while let Some(frame) = ws.next().await {
    match frame? {
        Frame::Delta(d) => print!("{}", d.text),
        Frame::Close(_) => break,
        _ => {}
    }
}
```

### 3.4 鉴权 + 自动 refresh

```rust
use apeireth_sdk::auth::AuthClient;

let auth = AuthClient::new("https://api.apeireth.dev");
let tokens = auth.login("alice@apeireth.dev", "password").await?;
// access_token 15 min，refresh-on-use 自动

let client = Client::new("https://api.apeireth.dev")
    .with_auth(auth)  // 自动管理 token 生命周期
    .build();
```

---

## 4. 5 stub SDK 概览

### 4.1 Lark SDK (飞书)

**功能**: 飞书消息、文档、审批
**当前状态**: 8 `NotImplemented` 错误返 501
**R21 计划**: 真接 @larksuiteoapi/node-sdk 1.x

详见 [lark-sdk.md](lark-sdk.md)

### 4.2 Voice SDK

**功能**: 语音唤醒 + 识别
**默认唤醒词**: `"apeireth"`
**当前状态**: stub

详见 [voice-sdk.md](voice-sdk.md)

### 4.3 LiveKit SDK

**功能**: 实时音视频（per 5 organs 视觉化）
**当前状态**: stub

详见 [livekit-sdk.md](livekit-sdk.md)

### 4.4 Sandbox SDK

**功能**: 沙盒代码执行（per code_exec tool）
**当前状态**: stub

详见 [sandbox-sdk.md](sandbox-sdk.md)

### 4.5 Provider 客户端（5 个）

**功能**: LLM Provider 接入
**当前状态**: claude-code 真接，其他 4 stub

详见 [provider-claude-code.md](provider-claude-code.md)

---

## 5. SDK 不假装清单

| SDK | 真接功能 | stub 功能 |
|---|---|---|
| Rust | HTTP 调用 + WS 流 + 自动 refresh | — |
| Lark | 8 方法全 stub | 8 方法全 stub |
| Voice | 唤醒词检测 stub | 语音识别 stub |
| LiveKit | 连接 stub | 音视频流 stub |
| Sandbox | 进程隔离 stub | 完整执行 stub |
| Provider | claude-code 1 真接 | 其他 4 stub |

---

## 6. 8 项不修改承诺

- ✅ 编译期 hardcode：SDK 协议 schema 编译期固定
- ✅ 不改 LOCKED：API/SDK 协议严守向后兼容
- ✅ 不假装已实现：stub 状态明确标注
- ✅ 不依赖 NewAPI：自建 5 SDK
- ✅ 不重复造轮子：用 reqwest/serde/tokio 业界惯例

---

## 7. 相关

- 实现: `crates/apeireth-sdk/`
- API: [`docs/api/README.md`](../api/README.md)
- 决策: [`docs/adr/0018-rust-sdk-design.md`](../adr/0018-rust-sdk-design.md)
