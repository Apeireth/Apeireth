# 心 (Heart) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-core` (基器官, 24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 心 / **i18n 解剖名词**: 心脏

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | heart (心 / 心脏) |
| **6 command** | status / pulse / health / restart / halt / stats |
| **关键 dep** | tokio 1.40 / serde 1.0 / thiserror 1.0 / chrono 0.4 |
| **状态** | ✅ 24 LOCKED 之一 (R11 baseline 严守) |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2, async t()) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `status` | 心脏状态 (心率 / 血压 / 输出一致性) | 心状态 |
| `pulse` | 心跳 (每秒 1 次, 给其他 8 器官同步) | 心跳 |
| `health` | 健康检查 (liveness probe) | 健康检查 |
| `restart` | 重启 (仅受限用户) | 重启 |
| `halt` | 停机 (受 Self-Disable 防护) | 停机 |
| `stats` | 统计 (心跳次数 / 重启次数 / etc) | 统计 |

---

## 2. API 调用

```rust
use apeireth_core::organ::heart::{Heart, HeartStatus};

let heart = Heart::new();
let status = heart.status().await?;
// HeartStatus { bpm: 60, pressure: 120/80, output: OutputStatus::Consistent }
```

---

## 3. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/heart.rs (R20 阶段 6 借 Golutra)
use apeireth_tui::organ::command::{Command, CommandResult};

pub struct HeartCommand {
    pub organ: Heart,
}

impl Command for HeartCommand {
    fn name(&self) -> &str { "heart" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* ... */ }
}
```

---

## 4. 相关

- [docs/api/v1-tools.md](v1-tools.md) (6 工具概览)
- [docs/api/v1-observability.md](v1-observability.md) (3 observability 端点)
- 实现: `crates/apeireth-core/src/`
- 1:1 翻译源: v0.9.21 SpectrAI heart organ
- 决策: 整合 #3 C-1 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
