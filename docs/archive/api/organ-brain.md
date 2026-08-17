# 脑 (Brain) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-cognition` (24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 脑 / **i18n 解剖名词**: 大脑

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | brain (脑 / 大脑) |
| **6 command** | think / reason / decide / reflect / learn / forget |
| **关键 dep** | tokio 1.40 / serde 1.0 / apeireth-asi (理性) / apeireth-tools (5 Provider) |
| **状态** | ✅ 24 LOCKED 之一 |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `think` | 思考 (调 5 Provider LLM) | 思考 |
| `reason` | 推理 (per `apeireth-asi` 理性 24 维) | 推理 |
| `decide` | 决策 (per 6 哲学锚 S-1/S-2) | 决策 |
| `reflect` | 反思 (per 自我评估) | 反思 |
| `learn` | 学习 (per 训练数据) | 学习 |
| `forget` | 遗忘 (per 隐私 / GDPR) | 遗忘 |

---

## 2. API 调用

```rust
use apeireth_cognition::organ::brain::{Brain, Thought, Decision};

let brain = Brain::new();
let thought = brain.think("Apeireth 1.0 release 进度?").await?;
// Thought { provider: ClaudeSonnet, tokens: 256, content: "..." }

let decision = brain.decide(ThoughtContext {
    facts: vec!["v1.0.0 估补 88%", "4 项 85-97% 标 R21"],
    philosophy: PhilosophyAnchors::default(),  // 6 哲学锚
}).await?;
// Decision { action: Continue, confidence: 0.85, rationale: "..." }
```

---

## 3. 6 哲学锚 + 24 维命名 (per apeireth-asi)

```rust
use apeireth_asi::naming_v05::{N24, PhilosophyAnchors};

let anchors = PhilosophyAnchors::default();  // S-1/S-2/O-2/O-3/O-4/O-5
let naming = N24::parse("PC-3-RC-2-HG-1-GP-0")?;  // 24 维 1:1 v0.5 命名
// sum=1.00 守门 (per `apeireth-naming-v05` 编译期 hardcode)
```

---

## 4. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/brain.rs
impl Command for BrainCommand {
    fn name(&self) -> &str { "brain" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* think / reason / etc */ }
}
```

---

## 5. 相关

- 实现: `crates/apeireth-cognition/`
- 1:1 翻译源: v0.9.21 SpectrAI brain organ
- 决策: 整合 #3 C-1 + G-2 + E-1 (5 Provider)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
