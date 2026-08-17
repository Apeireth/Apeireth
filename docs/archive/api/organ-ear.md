# 耳 (Ear) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-voice` (R20 阶段 6 真接, 声纹 + 唤醒词)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 耳 / **i18n 解剖名词**: 双耳

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | ear (耳 / 双耳) |
| **6 command** | listen / hear / decode / classify / train / stop |
| **关键 dep** | tokio 1.40 / reqwest 0.12 / cpal 0.15 / hound 3.5 / dasp 0.11 |
| **状态** | ✅ R20 阶段 6 真接 (per 整合 #3 F-2) |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `listen` | 监听 (持续录制) | 监听 |
| `hear` | 听 (单次 STT) | 听 |
| `decode` | 解码 (audio format 转换) | 解码 |
| `classify` | 分类 (声音类别: 语音 / 噪声 / 音乐) | 分类 |
| `train` | 训练 (声纹注册) | 训练 |
| `stop` | 停止 (listen) | 停止 |

---

## 2. API 调用

```rust
use apeireth_voice::organ::ear::{Ear, ListenConfig};

let ear = Ear::new(ListenConfig {
    sample_rate: 16_000,
    channels: 1,
    duration_sec: 5,
});
let text = ear.hear(&audio_bytes).await?;
// String ("Apeireth 是长程 AI 成长平台")
```

---

## 3. 5 声纹 + 唤醒词 (per `provider-voice.md`)

- **STT** (Speech-to-Text): HTTP 调 `apeireth-voice` STT 端点
- **TTS** (Text-to-Speech): HTTP 调 `apeireth-voice` TTS 端点
- **声纹** (Voice Print): 注册 + 验证 (per `provider-voice.md` §2.3)
- **唤醒词** (Wake Word): "apeireth" 粗略方案 (R21 续 Porcupine)
- **分类** (Classify): 3 类 (语音 / 噪声 / 音乐)

---

## 4. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/ear.rs
impl Command for EarCommand {
    fn name(&self) -> &str { "ear" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* listen / hear / etc */ }
}
```

---

## 5. 相关

- [docs/api/provider-voice.md](provider-voice.md) (Voice SDK 视角)
- 实现: `crates/apeireth-voice/`
- 决策: 整合 #3 C-1 + F-2 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
