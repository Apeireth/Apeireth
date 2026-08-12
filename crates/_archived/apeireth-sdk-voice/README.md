# apeireth-sdk-voice (STUB MODE)

⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**

Voice 语音 SDK stub (1:1 翻译 `@anthropic-ai/voice` v0.9.21 商业版).

## 1:1 翻译 v0.9.21 商业版 API 表面

| 模块 | 商业版 v0.9.21 | R20 阶段 4 stub |
|------|---------------|------------------|
| STT | `transcribe(audio, model)` | `VoiceError::NotImplemented` |
| TTS | `synthesize(text, voice)` | `VoiceError::NotImplemented` |
| Wake | `detectWake(audio, wake_word)` | `VoiceError::NotImplemented` |
| Listen | `startListening() / stopListening()` | `VoiceError::NotImplemented` |
| Stream | `streamAudio(stream)` | `VoiceError::NotImplemented` |

## 4 STT 模型 (per v0.9.21 商业版)

`Whisper` / `Wav2Vec` / `Deepgram` / `Google` — 编译期 hardcode 4 variant.

## 4 TTS 模型 (per v0.9.21 商业版)

`ElevenLabs` / `Azure` / `Google` / `OpenAI` — 编译期 hardcode 4 variant.

## 4 唤醒词类别 (per v0.9.21 商业版 + R20 设计拍板)

`Hardcoded` (默认 `"apeireth"`) / `Custom` / `Phonetic` / `Semantic` — 编译期 hardcode 4 variant.

## 3 VAD 算法 (per v0.9.21 商业版)

`Energy` / `Silence` / `WebRtc` — 编译期 hardcode 3 variant.

## 6 K-1 强校验 (per task spec §3)

1. **API Key** (非空 + 长度 ≥ 16)
2. **Audio Format** (wav/mp3/opus/flac)
3. **Sample Rate** (8000-48000 Hz)
4. **Bit Depth** (8/16/24/32)
5. **Channels** (1/2)
6. **Language** (ISO 639-1 e.g. en, zh-CN)

## 默认唤醒词

`"apeireth"` (per R20 设计拍板, 1:1 翻译 v0.9.21 商业版品牌一致).

## 状态

⚠️ skeleton (R20 阶段 4 效果, 1 owner × 1 周续真接)

当前 stage 跑 `cargo check` + 12-15 fixture + 6 K-1 验证. **0 真接 SDK** — R21 续真接.
