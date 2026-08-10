# 声 (Voice) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-voice` (R20 阶段 6 真接, TTS 主动发声)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 声 / **i18n 解剖名词**: 声带

> **注**: 跟 "耳" 器官共用 `apeireth-voice` crate, 但 API 焦点不同 (耳 = 输入, 声 = 输出).

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | voice (声 / 声带) |
| **6 command** | speak / whisper / shout / pause / resume / stop |
| **关键 dep** | tokio 1.40 / reqwest 0.12 / cpal 0.15 / hound 3.5 |
| **状态** | ✅ R20 阶段 6 真接 (per 整合 #3 F-2) |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `speak` | 说话 (TTS 默认) | 说话 |
| `whisper` | 低声 (TTS 速度 0.5) | 低声 |
| `shout` | 大声 (TTS 速度 1.5) | 大声 |
| `pause` | 暂停 (输出中) | 暂停 |
| `resume` | 继续 (暂停后) | 继续 |
| `stop` | 停止 (输出中) | 停止 |

---

## 2. API 调用

```rust
use apeireth_voice::organ::voice::{Voice, SpeakOptions};

let voice = Voice::new();
voice.speak(
    "Apeireth 1.0 release 估补 88%",
    SpeakOptions {
        voice: "zh-CN-Xiaoxiao".to_string(),
        speed: 1.0,
        pitch: 0.0,
        volume: 0.8,
    },
).await?;
```

---

## 3. 5 TTS 后端 (per `provider-voice.md` §2.1)

| 后端 | 1.0 状态 |
|------|---------|
| **apeireth-voice TTS HTTP** (自家) | ✅ |
| **Azure TTS** | ⚪ R21 续 |
| **Google TTS** | ⚪ R21 续 |
| **AWS Polly** | ⚪ R21 续 |
| **本地 RHVoice** | ⚪ R21+ 续 |

---

## 4. 4 边缘 case

| 边缘 | 处理 |
|------|------|
| 音频设备不可用 | 降级 (仅 text output) |
| 静音 / 0 音量 | 跳过 (0 报错) |
| 长文本 (> 1000 字) | 分段 (per 5 段) |
| 失败 / 网络断 | 重试 3 次, 0 强依赖 |

---

## 5. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/voice.rs
impl Command for VoiceCommand {
    fn name(&self) -> &str { "voice" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* speak / whisper / etc */ }
}
```

---

## 6. 跟 "耳" 器官区别

| 维度 | 耳 (输入) | 声 (输出) |
|------|----------|----------|
| **方向** | 听 → 文本 | 文本 → 说 |
| **API** | `hear(audio) → text` | `speak(text) → audio` |
| **默认命令** | hear | speak |
| **依赖 STT** | ✅ | ⚪ |
| **依赖 TTS** | ⚪ | ✅ |
| **声纹** | ✅ 注册 / 验证 | ⚪ |
| **唤醒词** | ✅ 监听 | ⚪ |

---

## 7. 相关

- [docs/api/provider-voice.md](provider-voice.md) (Voice SDK 视角)
- [docs/api/organ-ear.md](organ-ear.md) (耳器官, 输入)
- 实现: `crates/apeireth-voice/`
- 决策: 整合 #3 C-1 + F-2 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
