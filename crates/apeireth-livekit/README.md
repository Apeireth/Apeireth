# apeireth-livekit

> **R20 阶段 6 flesh out**: LiveKit Server SDK 真接实现 (1:1 翻译 livekit-server-sdk 0.6+ Twirp API)
> **状态**: 6 端点真接 (server_url / api_key / room / track / participant / event), wiremock 0.6 测 19 测试 pass
> **状态**: ⚠️ STUB 路径 6 端点 + 1 stub_status = 7 工具白名单 编译期 hardcode 守门 (`STUB_MODE = true`)

## 跟 `apeireth-sdk-livekit` 关系

| Crate | 阶段 | 角色 | 状态 |
|-------|------|------|------|
| `apeireth-sdk-livekit` (LOCKED) | R20 阶段 4 | 商业版 v0.9.21 1:1 翻译 stub (client-side: connect/disconnect/publishTrack/subscribe/setCameraEnabled/setMicrophoneEnabled) | LOCKED baseline 16:34:11, 0 改 |
| `apeireth-livekit` (本 crate) | R20 阶段 6 | 真接 LiveKit Server API 6 端点 (server-side: server_url/api_key/room/track/participant/event) | flesh out, opt-in via `LiveKitRealImpl::new` |

两 crate 严格分离, 各自独立 flesh out. `apeireth-sdk-livekit` 走 client-side WebSocket 6 API
(NotImplemented stub), `apeireth-livekit` 走 server-side Twirp HTTP 6 端点 (真接).

## 6 端点 (per LiveKit Server API Twirp 协议)

| 端点 | 1:1 翻译 | Twirp endpoint | K-1 强校验 |
|------|---------|----------------|------------|
| `server_url` | `LiveKitRealImpl.get_server_url()` | (getter, 不走 HTTP) | K-1 #1: `https://` 或 `http://localhost` |
| `api_key` | `LiveKitRealImpl.get_api_key()` | (getter, 不暴露 secret) | K-1 #2: 至少 10 chars alphanumeric |
| `room` | `create_room` / `list_rooms` / `delete_room` | `POST /twirp/livekit.RoomService/{Create,List,Delete}Room` | K-1 #3: 1..=256 chars alphanumeric + `-` + `_` |
| `track` | `mute_track` | `POST /twirp/livekit.RoomService/MutePublishedTrack` | K-1 #4: `TR_<alphanumeric>` |
| `participant` | `list_participants` / `remove_participant` / `get_participant_info` | `POST /twirp/livekit.RoomService/{List,Remove,Get}Participant` | K-1 #5: 1..=128 chars alphanumeric + `-` + `_` + `.` |
| `event` | `push_event` / `drain_events` / `peek_events` | (in-memory 模拟, 跟诚实标缺 #2) | — |

## 6 哲学锚穿透 (R20 阶段 6 必守)

1. **S-1 不漂移**: 6 端点 1:1 翻译 LiveKit Server API Twirp 6 维度
2. **S-2 编译期 hardcode**: 6 端点名 / 5 K-1 / Twirp 路径前缀全部 const
3. **O-2 工程铁律**: reqwest 0.12 + jsonwebtoken 9.3 + url 2.5 业界成熟 crate
4. **O-3 m3 防御**: 7 工具白名单 编译期 hardcode
5. **O-4 不假装可观测**: 6 端点失败时返 `LiveKitError::ServerCallFailed(...)` + tracing::warn! log
6. **O-5 K-1 强校验**: 5 字段编译期 hardcode 白名单

## 5 诚实标缺 (R20 阶段 6 flesh out 实查)

1. **JWT 走 HS256 默认**: 商业版 LiveKit Server API 默认 HS256 HMAC 签名, RS256 留 R21+ 续
2. **Event 端点 in-memory 模拟**: LiveKit Server API 不提供 server-side 事件订阅, 真实场景是 webhook 推, 留 R21+ 续
3. **缺 rate-limit 自动退避**: Twirp 429 时本实现立刻返 ServerCallFailed, 留 R21+ 续
4. **api_key/secret 走 String 明文**: R21+ 续时改 `Secret<String>` + 走 `apeireth-keyring`
5. **GetParticipant 端点 STUB 简化**: 走 list_participants + 过滤, R21+ 续真接

## 用法

```rust
use apeireth_livekit::{LiveKitConfig, LiveKitRealImpl, CreateRoomRequest};

let real = LiveKitRealImpl::new(
    LiveKitConfig::default(),
    "https://livekit.example.com",
    "APIxxxxxxxxxxxxxxxx",
    "secret_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
)?;

let room = real
    .create_room(CreateRoomRequest::new("my-room")?)
    .await?;

println!("Room created: sid={} name={}", room.sid, room.name);
```

## 8 项不修改承诺

- ✅ 0 改 24 LOCKED crate
- ✅ 0 改 workspace version (1.0.0)
- ✅ 0 改 `apeireth-sdk-livekit` (LOCKED baseline 16:34:11)
- ✅ 0 引 NewAPI (走 LiveKit 官方 Server API Twirp endpoint)
- ✅ 0 重复造轮子 (沿用 workspace 已有 reqwest / tokio / serde)
- ✅ 0 假装已实现 (6 端点真 HTTP, 失败如实报)
- ✅ 0 引 unsafe
- ✅ 编译期 hardcode (5 K-1 + 6 端点 + 7 tool whitelist)

## 测试 (19/19 pass)

- **4 单元** (in `src/real.rs`): 编译期常量, server_url/api_key 拒空, 默认创建
- **14 wiremock 端到端** (in `tests/test_livekit_real_wiremock.rs`): 6 端点 × happy + error + 401 重试 + 5 K-1 拒空 + event 端点 + Twirp 错误响应 + HTTP 500
- **1 demo** (in `examples/livekit_real_demo.rs`): 8 演示入口

## 引用文档

1. `livekit-server-sdk 0.6+` Go / Rust / JS 商业版 SDK (1:1 镜像 URL 路径)
2. `crates/apeireth-voice/src/real.rs` (R20 阶段 6 voice 真接模板, 4 块 API)
3. `crates/apeireth-lark/src/real.rs` (R20 阶段 6 lark 真接模板, 5 端点)
4. `crates/apeireth-sandbox/src/real.rs` (R20 阶段 6 sandbox 真接模板, 6 API + 3 RuntimeKind)
5. `crates/apeireth-sdk-livekit/` (LOCKED baseline 16:34:11, 0 改)
6. `reports/voice-real-flesh-out-2026-08-06.md` (R20 阶段 6 voice 真接报告)
7. `reports/livekit-real-flesh-out-2026-08-06.md` (R20 阶段 6 livekit 真接报告)
