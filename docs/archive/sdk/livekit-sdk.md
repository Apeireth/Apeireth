# LiveKit SDK stub

> **依据**: `crates/apeireth-sdk-livekit/src/lib.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: 🟡 stub（连接 stub，音视频流 stub）

---

## 1. 概览

**功能**: 实时音视频通信（per 5 organs 视觉化、未来多模态）
**目标 SDK**: livekit-rs 0.5.x
**1.0 状态**: stub

---

## 2. API

### 2.1 连接

```rust
use apeireth_sdk_livekit::{Client, RoomConfig};

let client = Client::new(
    "wss://livekit.apeireth.dev",
    "access_token_jwt"
);

let config = RoomConfig {
    room: "apeireth-session-uuid".to_string(),
    identity: "user-uuid".to_string(),
};

// stub: NotImplemented
let room = client.connect(config).await?;
```

### 2.2 音视频发布

```rust
// 1.0 stub
room.publish_audio(audio_track).await?;  // NotImplemented
room.publish_video(video_track).await?;  // NotImplemented
```

### 2.3 订阅

```rust
// 1.0 stub
room.subscribe_all().await?;  // NotImplemented
```

### 2.4 数据通道

```rust
room.send_data("hello", reliable=true).await?;  // NotImplemented
```

---

## 3. 错误

```rust
pub enum Error {
    NotImplemented(&'static str),  // 1.0 全部
    Connection(String),              // R21
    Auth(String),                    // R21
    Track(String),                   // R21
}
```

---

## 4. R21 计划

| 功能 | R21 实装 | 估时 |
|---|---|---|
| `connect` | livekit-rs 0.5.x | 1 owner × 1 周 |
| `publish_*` | 音视频发布 | 1 owner × 1 周 |
| `subscribe_*` | 订阅 | 1 owner × 1 周 |
| `send_data` | 数据通道 | 0.5 owner × 1 周 |

**总估**: 3.5 owner × 1 周

---

## 5. 与 5 organs 视觉化

**未来场景** (R22+):
- 实时显示 AI 9 器官状态给用户
- 多用户协同编辑 session
- 视频会议 + AI 助手

**当前 1.0 用途**: 仅占位（per 主人 2026-08-05 拍板"先做好后端"）

---

## 6. 不假装

- ✅ API 签名清楚
- 🟡 livekit-rs 真接 R21
- ✅ 不假装已实现

---

## 7. 相关

- 实现: `crates/apeireth-sdk-livekit/src/lib.rs`
- 决策: R20 阶段 1 拍板"SDK stub 留 R21 续"
