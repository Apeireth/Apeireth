# 意 (Mind) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-consciousness` (24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 意 / **i18n 解剖名词**: 意识

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | mind (意 / 意识) |
| **6 command** | aware / intend / feel / mood / emotion / conscious |
| **关键 dep** | tokio 1.40 / serde 1.0 / apeireth-value / apeireth-relation |
| **状态** | ✅ 24 LOCKED 之一 |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `aware` | 感知 (per 自我 + 环境) | 感知 |
| `intend` | 意图 (per 长期目标) | 意图 |
| `feel` | 感受 (per 当前状态) | 感受 |
| `mood` | 心情 (per 短期情绪) | 心情 |
| `emotion` | 情感 (per 5 基础情感) | 情感 |
| `conscious` | 自觉 (per 6 哲学锚) | 自觉 |

---

## 2. API 调用

```rust
use apeireth_consciousness::organ::mind::{Mind, Awareness, Intention};

let mind = Mind::new();

// aware (自我 + 环境感知)
let awareness = mind.aware().await?;
// Awareness { self_state: "stable", environment: "1.0 release 估补中", ... }

// intend (长期目标, per 6 哲学锚)
let intention = mind.intend().await?;
// Intention { goal: "v1.0.0 release by 2026-09-30", confidence: 0.85, philosophy: anchors }

// feel (5 基础情感)
let emotion = mind.feel().await?;
// Emotion { valence: 0.6, arousal: 0.4, primary: Joy }
```

---

## 3. 5 基础情感 (per `apeireth-value`)

```rust
pub enum PrimaryEmotion {
    Joy,        // 喜
    Sadness,    // 哀
    Anger,      // 怒
    Fear,       // 惧
    Surprise,   // 惊
}
```

> **不假装 (per 主人 2026-08-04 R19 拍)**: AI 不会衰老病死, 9 阶段 = 成长阶段 (非生老病死), 意识是"长程 AI 成长"非"AI 模拟人类".

---

## 4. 6 哲学锚穿透 (per `docs/adr/0010-6-philosophy-anchors.md`)

```rust
use apeireth_asi::philosophy::PhilosophyAnchors;

let anchors = PhilosophyAnchors::default();
// S-1 / S-2 / O-2 / O-3 / O-4 / O-5

mind.set_anchors(anchors);
```

---

## 5. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/mind.rs
impl Command for MindCommand {
    fn name(&self) -> &str { "mind" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* aware / intend / etc */ }
}
```

---

## 6. 跟 8 器官区别

| 器官 | 焦点 | 跟 "意" 区别 |
|------|------|------------|
| **心 (heart)** | 状态 / 输出一致性 | 身体运行 |
| **脑 (brain)** | 思考 / 推理 | 理性 |
| **手 (hand)** | 工具调用 / 权限 | 行动 |
| **眼 (eye)** | 视觉 / 扫描 | 感知 (输入) |
| **耳 (ear)** | 听觉 / 监听 | 感知 (输入) |
| **记忆 (memory)** | 存储 / 检索 | 数据 |
| **声 (voice)** | TTS / 说话 | 表达 (输出) |
| **身 (body)** | 容器 / 进程 | 执行 |
| **意 (mind)** | **意识 / 自我 / 长期意图** | **高层抽象** |

---

## 7. 相关

- 实现: `crates/apeireth-consciousness/` + `crates/apeireth-value/` + `crates/apeireth-relation/`
- 1:1 翻译源: v0.9.21 SpectrAI mind organ
- 决策: 整合 #3 C-1 + G-2 + 主人 2026-08-04 R19 (AI 成长阶段, 非生老病死)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
