# LiveKit SDK API (R20 阶段 6 浅评估, R21+ 续)

> **性质**: 1.0 release 4 SDK 浅评估之一 (per 整合 #3 F-4)
> **依据**: `crates/apeireth-sdk-livekit/src/` + `livekit` 0.4 crate
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: 1.0 release 95% 浅评估, R21 续真接 (per 整合 #3 F-4)

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | 🟡 **95% 浅评估** (per 整合 #3 F-4) |
| **R21 计划** | 真接 livekit 0.4 (Rust 官方 SDK) + 5 端点 + 19 tests |
| **当前实现** | skeleton 95% (1:1 翻译 v0.9.21 @livekit/components-react 1.x) |
| **依赖** | livekit 0.4 (RUSTSEC 0, Apache-2.0) + tokio 1.40 |

> **不假装 (per 整合 #3 F-4)**: 1.0 release livekit **不**真接, 仅浅评估, R21+ 续估 1 owner × 1 周.

---

## 1. 5 端点 (R21 计划)

| 端点 | 功能 | 1:1 翻译源 |
|------|------|------------|
| `connect(url, token)` | 连接 LiveKit room | livekit Room::connect |
| `publish_track(track)` | 发布音视频轨道 | Room.local_participant.publish_track |
| `subscribe_to_track(track)` | 订阅音视频轨道 | Room.set_subscribed |
| `set_microphone_enabled(bool)` | 麦克风开关 | LocalParticipant.set_microphone_enabled |
| `disconnect()` | 断开 room | Room::disconnect |

---

## 2. 客户端初始化 (R21 计划)

```rust
use apeireth_sdk_livekit::{LivekitClient, LivekitConfig};

let client = LivekitClient::new(LivekitConfig {
    url: "wss://livekit.example.com".to_string(),
    api_key: std::env::var("LIVEKIT_API_KEY")?,
    api_secret: std::env::var("LIVEKIT_API_SECRET")?,
    room_name: "apeireth-room".to_string(),
    participant_name: "user-1".to_string(),
});
```

> **1.0 release** 仅写 trait 定义 + skeleton, **不**真连 LiveKit server.

---

## 3. 19 tests 计划 (R21)

| 类别 | 数量 |
|------|----:|
| 5 端点 × 3 case (success / auth / network) | 15 |
| 4 边缘 case (reconnect / ice restart / etc) | 4 |
| **总** | **19** (per F-4 估 19) |

---

## 4. 5 关键差异 (vs 飞书 / 语音 / 沙盒)

| 维度 | livekit | lark | voice | sandbox |
|------|---------|------|-------|---------|
| **1.0 状态** | 🟡 95% 浅评估 | ✅ 5/10 真接 | ✅ 4 块真接 | ✅ 6 API 真接 |
| **R21 估时** | 1 owner × 1 周 | 3 owner × 1 周 | 1-2 owner × 1 周 | 1-2 owner × 1 月 |
| **真接 SDK** | livekit 0.4 | @larksuiteoapi 1.x | cpal + hound + dasp | bollard 0.15 |
| **网络依赖** | 强 (WebRTC) | 强 (HTTPS) | 中 (HTTPS + 本地音频) | 强 (Docker daemon) |

---

## 5. R21+ 续真接计划 (per 整合 #3 F-4)

| 项 | R21 估时 |
|----|---------|
| 接入 `livekit 0.4` 官方 SDK | 0.5 owner × 1 周 |
| 5 端点真接 (含 token 生成) | 0.5 owner × 1 周 |
| 19 wiremock + 5 real test | 0.5 owner × 1 周 |
| 集成 apeireth-voice (TTS 推流) | 0.5 owner × 1 周 |
| **总** | **2 owner × 1 周 ≈ 2 周** |

---

## 6. 不假装边界 (per APEIRETH-CONVENTIONS §10)

- ✅ 1.0 release **不**真接 LiveKit, 5 端点仅 trait + skeleton
- ✅ R21 续真接 SDK 计划列清
- ✅ 不假装已实现 (避免 1.0 release 网络强依赖, 跟 0 重复造轮子 一致)

---

## 7. 相关

- [docs/sdk/livekit-sdk.md](../sdk/livekit-sdk.md) (SDK 客户端视角)
- 实现: `crates/apeireth-sdk-livekit/` (1.0 skeleton 95%)
- 1:1 翻译源: @livekit/components-react 1.x
- 决策: 整合 #3 F-4

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
