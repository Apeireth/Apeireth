# LiveKit SDK 端到端测试 (R21 续浅评估)

> **性质**: 1.0 release 4 SDK 浅评估 + R21 续真接 (per 整合 #3 F-4)
> **依据**: `crates/apeireth-sdk-livekit/src/` 1.0 skeleton 95%
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-4)
> **不假装**: 1.0 release 0 真接 (R21+ 续估 1 owner × 1 周)

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | 🟡 **95% 浅评估** (per 整合 #3 F-4) |
| **R21 计划** | 真接 livekit 0.4 (Rust 官方 SDK) + 5 端点 + 19 tests |
| **1.0 测试** | 0 真接测试 (1.0 仅 trait + skeleton) |
| **R21 估时** | 2 owner × 1 周 ≈ 2 周 |

> **不假装 (per 整合 #3 F-4)**: 1.0 release livekit **不**真接, 仅 5 端点 trait + skeleton. 0 wiremock 测试 (网络强依赖, 跟 0 重复造轮子 一致).

---

## 1. 1.0 release 现状 (95% 浅评估)

```rust
// crates/apeireth-sdk-livekit/src/lib.rs (1.0 release skeleton)
pub struct LivekitClient {
    config: LivekitConfig,
    // 1.0 release 0 真连 LiveKit server
}

impl LivekitClient {
    pub fn new(config: LivekitConfig) -> Self { /* ... */ }
    
    // 5 端点 trait 定义 (1.0 release 0 实现)
    pub async fn connect(&self) -> Result<Room, LivekitError> {
        Err(LivekitError::NotImplemented("connect"))
    }
    
    pub async fn publish_track(&self, _track: Track) -> Result<(), LivekitError> {
        Err(LivekitError::NotImplemented("publish_track"))
    }
    
    pub async fn subscribe_to_track(&self, _track: Track) -> Result<(), LivekitError> {
        Err(LivekitError::NotImplemented("subscribe_to_track"))
    }
    
    pub async fn set_microphone_enabled(&self, _enabled: bool) -> Result<(), LivekitError> {
        Err(LivekitError::NotImplemented("set_microphone_enabled"))
    }
    
    pub async fn disconnect(&self) -> Result<(), LivekitError> {
        Err(LivekitError::NotImplemented("disconnect"))
    }
}
```

---

## 2. 1.0 release 测试 (0 真接, 仅 trait)

```rust
// crates/apeireth-sdk-livekit/tests/skeleton.rs
#[test]
fn test_trait_definitions() {
    // 仅测 trait 存在 + 5 端点签名
    let _client = LivekitClient::new(LivekitConfig::default());
}

#[tokio::test]
async fn test_not_implemented() {
    // 5 端点全部返 NotImplemented
    let client = LivekitClient::new(LivekitConfig::default());
    assert!(matches!(client.connect().await, Err(LivekitError::NotImplemented(_))));
    assert!(matches!(client.publish_track(mock_track()).await, Err(LivekitError::NotImplemented(_))));
    // ... 3 剩余
}
```

> **1.0 release 仅 2 测试** (trait + NotImplemented), 0 端到端.

---

## 3. R21 续真接 SDK 计划 (per 整合 #3 F-4)

### 3.1 5 端点真接 + 19 tests

```rust
// R21 计划 (per 整合 #3 F-4 + D-4)
#[tokio::test]
async fn test_connect_success() {
    // 真连 LiveKit server (per wiremock 或 test server)
    let client = LivekitClient::new(LivekitConfig {
        url: "wss://livekit.example.com".to_string(),
        api_key: "test_key",
        api_secret: "test_secret",
        room_name: "test-room",
        participant_name: "test-user",
    });
    let room = client.connect().await.unwrap();
    assert_eq!(room.name, "test-room");
}

#[tokio::test]
async fn test_publish_track() {
    // 推 audio track
    let client = LivekitClient::new(LivekitConfig::default());
    let room = client.connect().await.unwrap();
    let track = create_audio_track();
    client.publish_track(track).await.unwrap();
}

// ... 17 剩余
```

### 3.2 19 tests 计划

| 类别 | 数量 |
|------|----:|
| 5 端点 × 3 case (success / auth / network) | 15 |
| 4 边缘 case (reconnect / ice restart / bandwidth / codec) | 4 |
| **总** | **19** |

### 3.3 集成 apeireth-voice (TTS 推流)

```rust
#[tokio::test]
async fn test_voice_to_livekit_stream() {
    // apeireth-voice TTS → LiveKit audio track
    let voice = VoiceClient::new();
    let livekit = LivekitClient::new();
    
    let audio = voice.tts("Apeireth 1.0 release", TtsOptions::default()).await.unwrap();
    let track = livekit.create_audio_track(&audio).unwrap();
    livekit.publish_track(track).await.unwrap();
}
```

---

## 4. R21 估时

| 项 | R21 估时 |
|----|---------|
| 接入 `livekit 0.4` 官方 SDK | 0.5 owner × 1 周 |
| 5 端点真接 (含 token 生成) | 0.5 owner × 1 周 |
| 19 wiremock + 5 real test | 0.5 owner × 1 周 |
| 集成 apeireth-voice (TTS 推流) | 0.5 owner × 1 周 |
| **总** | **2 owner × 1 周 ≈ 2 周** |

---

## 5. 不假装边界 (per APEIRETH-CONVENTIONS §10)

- ✅ 1.0 release 0 真接 LiveKit, 5 端点仅 trait + skeleton
- ✅ R21 续真接 SDK 计划列清
- ✅ 1.0 release 0 wiremock 测试 (避免 1.0 release 网络强依赖, 跟 0 重复造轮子 一致)
- ✅ 不假装已实现

---

## 6. 0 触碰 24 LOCKED src 验证

| 守门 | 验证 | 状态 |
|------|------|:----:|
| 0 触碰 24 LOCKED src | 仅 `crates/apeireth-sdk-livekit/` (1.0 skeleton 95%) | ✅ |
| 0 改 workspace version 1.0.0 | `Cargo.toml:188` 未动 | ✅ |
| 0 主动 commit | HEAD 仍 `0da4af03` | ✅ |

---

## 7. 相关

- [livekit-sdk.md](livekit-sdk.md) (SDK 客户端视角)
- [docs/api/provider-livekit.md](../api/provider-livekit.md) (API 视角)
- 实现: `crates/apeireth-sdk-livekit/` (1.0 skeleton 95%)
- 决策: 整合 #3 F-4

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-4)
