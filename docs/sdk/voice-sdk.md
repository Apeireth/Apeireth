# Voice SDK（语音 stub，默认唤醒词 "apeireth"）

> **依据**: `crates/apeireth-voice/src/lib.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: 🟡 stub（唤醒词检测 stub，语音识别 stub）

---

## 1. 概览

**功能**: 语音唤醒 + 语音识别
**默认唤醒词**: `"apeireth"`（编译期 hardcode）
**目标 SDK**: Porcupine (唤醒词) + Whisper (ASR)
**1.0 状态**: stub

---

## 2. 默认唤醒词

```rust
// crates/apeireth-voice/src/lib.rs
pub const DEFAULT_WAKE_WORD: &str = "apeireth";

pub struct VoiceConfig {
    pub wake_word: String,        // 默认 "apeireth"
    pub sensitivity: f32,         // 0.0 - 1.0, 默认 0.5
    pub language: String,         // "zh-CN" / "en-US"
}
```

**为什么选 "apeireth"**:
- 项目名一致
- 5+ 音节，误唤醒率低
- 不易与日常对话冲突
- 中英用户都能念

---

## 3. API

### 3.1 唤醒词检测

```rust
use apeireth_voice::{VoiceClient, VoiceConfig};

let client = VoiceClient::new(VoiceConfig {
    wake_word: "apeireth".to_string(),
    sensitivity: 0.5,
    language: "zh-CN".to_string(),
});

// stub: 永远返 false
let detected = client.detect_wake_word(audio_chunk).await?;
assert!(!detected);  // 1.0 stub
```

### 3.2 语音识别

```rust
let text = client.transcribe(audio_bytes, "zh-CN").await?;
// 1.0 stub: 返 ""
```

### 3.3 流式识别

```rust
use futures::StreamExt;

let mut stream = client.transcribe_stream(audio_stream, "zh-CN").await?;
while let Some(chunk) = stream.next().await {
    println!("partial: {}", chunk?);  // 1.0 stub: 永远 None
}
```

---

## 4. 错误

```rust
pub enum Error {
    NotImplemented(&'static str),  // 1.0 全部
    AudioDevice(String),            // R21
    ModelLoad(String),              // R21
}
```

---

## 5. R21 计划

| 功能 | R21 实装 | 估时 |
|---|---|---|
| `detect_wake_word` | Porcupine 集成 | 1 owner × 1 周 |
| `transcribe` | Whisper.cpp 集成 | 2 owner × 1 周 |
| `transcribe_stream` | Whisper streaming | 1 owner × 1 周 |

**总估**: 4 owner × 1 周

**模型文件**:
- Porcupine 唤醒词模型: `apeireth_zh.pv` (R21 估补)
- Whisper: `ggml-base.bin` (~150 MB)

---

## 6. 与 TUI 集成（per D-08 拍板）

TUI 1.0 release 不接 Voice（per 主人 2026-08-05 拍板"不要干前端的活儿"）。
R21 接 TUI 时，Voice SDK 作为可选 feature 集成。

---

## 7. 不假装

- ✅ 唤醒词常量定义 + 编译期 hardcode
- ✅ API 签名清楚
- 🟡 Porcupine + Whisper 真接 R21
- ✅ 不假装已实现

---

## 8. 相关

- 实现: `crates/apeireth-voice/src/lib.rs`
- 唤醒词: `"apeireth"` 编译期 hardcode
- 决策: R20 阶段 1 拍板"SDK stub 留 R21 续"
