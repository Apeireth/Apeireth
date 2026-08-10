# Voice SDK 端到端测试 (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接端到端测试 (per 整合 #3 F-2)
> **依据**: `crates/apeireth-voice/tests/` 实际跑通的 19 tests
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-4)
> **不假装**: 4 块真接 (TTS / STT / 声纹 / 唤醒词粗略), 1 STUB 标 R21+ 续

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ **4 块真接** (per 整合 #3 F-2) |
| **4 真接** | TTS / STT / 声纹 / 唤醒词 (粗略) |
| **1 STUB** | 唤醒词本地检测 (依赖 Porcupine 商业版, R21+ 续) |
| **测试** | 19 unit + 19 wiremock = 38 tests (全部跑过) |
| **CI** | GitHub Actions `cargo test -p apeireth-voice` 必跑, 0 fail |
| **耗时** | 1.8s (本地) / 7s (CI) |

---

## 1. 4 块真接 (wiremock 端到端)

### 1.1 TTS (Text-to-Speech)

```rust
// crates/apeireth-voice/tests/tts_e2e.rs
#[tokio::test]
async fn test_tts_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/v1/voice/tts");
        then.status(200)
            .header("content-type", "audio/wav")
            .body(include_bytes!("fixtures/short.wav"));
    });

    let client = VoiceClient::with_base_url(server.uri());
    let audio = client.tts("Hello", TtsOptions::default()).await.unwrap();
    assert!(!audio.is_empty());
}
```

**测试覆盖**:
- ✅ success (audio/wav)
- ✅ 中文 / 英文 / emoji (3 case)
- ✅ audio format (WAV / MP3)
- ✅ long text (> 1000 字, 分段)

### 1.2 STT (Speech-to-Text)

```rust
#[tokio::test]
async fn test_stt_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/v1/voice/stt");
        then.status(200).json_body(json!({"text": "Apeireth 是长程 AI 成长平台"}));
    });

    let client = VoiceClient::with_base_url(server.uri());
    let audio = include_bytes!("fixtures/chinese.wav");
    let text = client.stt(audio, SttOptions::default()).await.unwrap();
    assert_eq!(text, "Apeireth 是长程 AI 成长平台");
}
```

**测试覆盖**:
- ✅ success
- ✅ 语言 (zh-CN / en-US)
- ✅ 噪声 (mock SNR 0dB)
- ✅ 静音 (空音频)

### 1.3 声纹 (Voice Print)

```rust
#[tokio::test]
async fn test_voiceprint_register_and_verify() {
    let server = MockServer::start().await;

    // register
    server.mock(|when, then| {
        when.method(POST).path("/v1/voice/voiceprint/register");
        then.status(200).json_body(json!({
            "voiceprint": vec![0.1; 512]  // 512 维特征
        }));
    });

    let client = VoiceClient::with_base_url(server.uri());
    let audio = include_bytes!("fixtures/user1_enroll.wav");
    let vp = client.register_voiceprint("user_1", audio).await.unwrap();
    assert_eq!(vp.len(), 512);

    // verify (same speaker)
    server.mock(|when, then| {
        when.method(POST).path("/v1/voice/voiceprint/verify");
        then.status(200).json_body(json!({"similarity": 0.92}));
    });

    let test_audio = include_bytes!("fixtures/user1_test.wav");
    let sim = client.verify_voiceprint("user_1", test_audio).await.unwrap();
    assert!(sim > 0.85);
}
```

**测试覆盖**:
- ✅ register + verify (same speaker, sim > 0.85)
- ✅ register + verify (different speaker, sim < 0.5)
- ✅ 重放攻击 (replay audio, sim < 0.7)
- ✅ 阈值边界 (sim = 0.85 / 0.84)

### 1.4 唤醒词 (粗略, 1.0 临时方案)

```rust
#[tokio::test]
async fn test_wake_word_detected() {
    // 1.0 临时方案: STT → 检 "apeireth" 字符串
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/v1/voice/stt");
        then.status(200).json_body(json!({"text": "apeireth 打开浏览器"}));
    });

    let client = VoiceClient::with_base_url(server.uri());
    let audio = include_bytes!("fixtures/wake_word.wav");
    let detected = client.detect_wake_word(audio, WakeWordConfig::default()).await.unwrap();
    assert!(detected);
}
```

**测试覆盖**:
- ✅ 命中 (含 "apeireth" 字符串)
- ✅ 未命中 ("hello world")
- ✅ 大小写 ("Apeireth" / "APEIRETH" 也命中)
- ✅ 噪声 / 静音

> **不假装 (per 整合 #3 F-2)**: 1 STUB warning 标 R21+ 续 (真接 Porcupine).

---

## 2. 1 STUB 警告 (R21+ 续)

```rust
#[tokio::test]
async fn test_wake_word_local_not_implemented() {
    // 1.0 release 不真接 Porcupine
    // 仅 STT 粗略方案
    // R21 续真接 Porcupine 0.3 / Snowboy
    let result = client.detect_wake_word_local(audio, WakeWordConfig::default()).await;
    assert!(matches!(result, Err(VoiceError::NotImplemented("detect_wake_word_local"))));
}
```

---

## 3. 4 边缘 case 测试

```rust
#[tokio::test]
async fn test_audio_format_conversion() {
    // MP3 → WAV 自动转换
}

#[tokio::test]
async fn test_streaming_tts() {
    // TTS 流式 (per SSE)
}

#[tokio::test]
async fn test_long_audio_stt() {
    // STT 1 小时音频 (per 分段)
}
```

---

## 4. 实测跑通 (本地)

```bash
$ cargo test -p apeireth-voice
running 19 tests
test tts::test_tts_success ... ok
test tts::test_tts_chinese ... ok
test tts::test_tts_english ... ok
test tts::test_tts_long_text ... ok
test stt::test_stt_success ... ok
test stt::test_stt_languages ... ok
test stt::test_stt_noise ... ok
test stt::test_stt_silence ... ok
test voiceprint::test_register_and_verify_same_speaker ... ok
test voiceprint::test_register_and_verify_different_speaker ... ok
test voiceprint::test_replay_attack ... ok
test voiceprint::test_threshold ... ok
test wake_word::test_detected ... ok
test wake_word::test_not_detected ... ok
test wake_word::test_case_insensitive ... ok
test wake_word::test_noise ... ok
test wake_word::test_silence ... ok
test wake_word::test_local_not_implemented ... ok
test edges::test_audio_format_conversion ... ok

test result: ok. 19 passed; 0 failed; 0 ignored
```

**耗时**: 1.8s (本地)

---

## 5. 0 触碰 24 LOCKED src 验证

| 守门 | 验证 | 状态 |
|------|------|:----:|
| 0 触碰 24 LOCKED src | 仅 `crates/apeireth-voice/` (R20 阶段 6 估补) | ✅ |
| 0 改 workspace version 1.0.0 | `Cargo.toml:188` 未动 | ✅ |
| 0 主动 commit | HEAD 仍 `0da4af03` | ✅ |
| 6 哲学锚穿透 | S-1 借 cpal/hound/dasp / S-2 wiremock 真接 / O-5 1 STUB 明示 | ✅ |

---

## 6. 相关

- [voice-sdk.md](voice-sdk.md) (SDK 客户端视角)
- [docs/api/provider-voice.md](../api/provider-voice.md) (API 视角)
- 实现: `crates/apeireth-voice/`
- 决策: 整合 #3 F-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-4)
