# Voice SDK API (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接之一 (per 整合 #3 F-2)
> **依据**: `crates/apeireth-voice/src/` + `@anthropic-ai/voice` 商业版 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: 4 块真接, 1 STUB 标 R21+ 续 (per 整合 #3 F-2)

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ **4 块真接** + 🟡 1 STUB 标 R21+ 续 |
| **4 真接** | TTS / STT / 声纹 / 唤醒词 (粗略) |
| **1 STUB** | 唤醒词本地检测 (依赖 Porcupine 商业版, R21+ 续) |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | reqwest 0.12 + cpal 0.15 + hound 3.5 + dasp 0.11 |

---

## 1. 客户端初始化

```rust
use apeireth_voice::{VoiceClient, VoiceConfig};

let client = VoiceClient::new(VoiceConfig {
    api_key: std::env::var("VOICE_API_KEY")?,
    base_url: "https://api.apeireth.dev/v1/voice".to_string(),
    sample_rate: 16_000,
    channels: 1,
    audio_format: AudioFormat::Wav,
});
```

---

## 2. 4 块真接

### 2.1 TTS (Text-to-Speech)

```rust
let audio = client.tts(
    "Apeireth 是长程 AI 成长平台",
    TtsOptions {
        voice: "zh-CN-Xiaoxiao".to_string(),
        speed: 1.0,
        pitch: 0.0,
        format: AudioFormat::Wav,
    },
).await?;
// 返 Vec<u8> (WAV bytes)
```

**API**: `POST /v1/voice/tts`

### 2.2 STT (Speech-to-Text)

```rust
let text = client.stt(
    &audio_bytes,
    SttOptions {
        language: "zh-CN".to_string(),
        model: "default".to_string(),
    },
).await?;
// 返 String (识别文本)
```

**API**: `POST /v1/voice/stt`

### 2.3 声纹 (Voice Print)

```rust
// 注册声纹
let voiceprint = client.register_voiceprint(
    "user_1",
    &audio_bytes,
).await?;
// 返 Vec<f32> (512 维声纹特征)

// 验证声纹
let similarity = client.verify_voiceprint(
    "user_1",
    &test_audio_bytes,
).await?;
// 返 f32 ∈ [0, 1] (cosine similarity)
```

**API**: `POST /v1/voice/voiceprint/register` + `POST /v1/voice/voiceprint/verify`

### 2.4 唤醒词 (粗略, 1.0 临时方案)

```rust
// 1.0 临时方案: 调用 STT, 检 "apeireth" 字符串
let detected = client.detect_wake_word(
    &audio_bytes,
    WakeWordConfig {
        keyword: "apeireth".to_string(),
        sensitivity: 0.5,
    },
).await?;
// 返 bool (true = 检测到)
```

> **不假装 (per 整合 #3 F-2)**: 1 STUB warning 标 R21+ 续, 当前是粗略方案, R21 真接 Porcupine 0.3 / Snowboy.

---

## 3. 4 块依赖

| 块 | 1:1 翻译源 | HTTP 依赖 | 本地依赖 |
|----|-----------|----------|---------|
| **TTS** | @anthropic-ai/voice (TTS) | reqwest 0.12 | — |
| **STT** | @anthropic-ai/voice (STT) | reqwest 0.12 | — |
| **声纹** | Resemblyzer 0.1 | reqwest 0.12 | dasp 0.11 |
| **唤醒词** | Porcupine 0.3 (R21+) / STT 粗略 (1.0) | reqwest 0.12 | — |

---

## 4. 19 tests (4 块 × 4 case + 3 边缘)

| 类别 | 数量 |
|------|----:|
| TTS 4 case (success / audio format / 中文 / 英文) | 4 |
| STT 4 case (success / 语言 / 噪声 / 静音) | 4 |
| 声纹 5 case (register / verify / 阈值 / 拒识 / 重放攻击) | 5 |
| 唤醒词 5 case (命中 / 未命中 / 灵敏度 / 噪声 / 静音) | 5 |
| 集成 (1 真接 + 1 STUB 警告) | 1 |
| **总** | **19** |

---

## 5. 1 STUB warning 标 R21+ 续

> **不假装 (per 整合 #3 F-2)**: 1 STUB warning 标 R21+ 续:
> - "apeireth 唤醒词本地检测当前 0% 真接 (依赖 Porcupine 商业版 SDK, R21+ 续)"
> - 替代方案: 调用 STT HTTP 端点 (返回文本) 检 "apeireth" 字符串 (粗略, 1.0 release 临时方案)
> - R21 估 1-2 周真接 Porcupine / Snowboy

---

## 6. 错误处理

```rust
pub enum VoiceError {
    NotImplemented(&'static str),  // 1 STUB
    Auth(String),                    // 401 / 403
    Network(String),                 // 连接 / 超时
    AudioFormat(String),             // WAV 解析失败
    Upstream { code: i32, msg: String },
    RateLimit { retry_after: u64 },
    Internal(String),
}
```

---

## 7. 相关

- [docs/sdk/voice-sdk.md](../sdk/voice-sdk.md) (SDK 客户端视角)
- 实现: `crates/apeireth-voice/`
- 1:1 翻译源: @anthropic-ai/voice 商业版
- 决策: 整合 #3 F-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
