# 手 (Hand) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-tools` (24 LOCKED 之一)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 手 / **i18n 解剖名词**: 双手

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | hand (手 / 双手) |
| **6 command** | invoke / list / approve / deny / cancel / status |
| **关键 dep** | tokio 1.40 / serde 1.0 / apeireth-tool-registry / apeireth-tool-approval |
| **状态** | ✅ 24 LOCKED 之一 |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `invoke` | 调 6 工具 (calendar / message / contact / task / search / drive) | 调工具 |
| `list` | 列 8 工具白名单 | 列工具 |
| `approve` | 批准 (per 4 门权限) | 批准 |
| `deny` | 拒绝 (per 4 门权限) | 拒绝 |
| `cancel` | 取消 (运行中调) | 取消 |
| `status` | 状态 (运行中调列表) | 状态 |

---

## 2. API 调用

```rust
use apeireth_tools::organ::hand::{Hand, ToolInvocation, ToolResult};

let hand = Hand::new();
let result = hand.invoke(
    "calendar",
    ToolInvocation::new("list_events")
        .param("start", "2026-08-01T00:00:00Z")
        .param("end", "2026-08-31T23:59:59Z"),
).await?;
// ToolResult { events: [...], count: 5, duration_ms: 42 }
```

---

## 3. 8 工具白名单 + 4 门权限 (per `apeireth-protocol`)

```rust
pub enum Tool {
    ReadFile, WriteFile, Edit, Bash, Grep, Glob, WebFetch, WebSearch,
}

pub enum Gate {
    Read,        // 1 门: 读
    Write,       // 2 门: 写
    Execute,     // 3 门: 执行
    Network,     // 4 门: 网络
}
```

**TOOL_WHITELIST** 编译期 hardcode, 防止 LLM 调未授权工具.

---

## 4. 6 工具 (per `v1-tools.md`)

| 工具 | 路径 | 真实接 | 文档 |
|------|------|:---:|------|
| **calendar** | `/v1/tools/calendar/invoke` | ✅ | [v1-tools-calendar.md](v1-tools-calendar.md) |
| **message** | `/v1/tools/message/invoke` | ✅ | [v1-tools-message.md](v1-tools-message.md) |
| **contact** | `/v1/tools/contact/invoke` | 🟡 | [v1-tools-contact.md](v1-tools-contact.md) |
| **task** | `/v1/tools/task/invoke` | 🟡 | [v1-tools-task.md](v1-tools-task.md) |
| **search** | `/v1/tools/search/invoke` | ✅ | [v1-tools-search.md](v1-tools-search.md) |
| **drive** | `/v1/tools/drive/invoke` | ✅ | [v1-tools-drive.md](v1-tools-drive.md) |

---

## 5. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/hand.rs
impl Command for HandCommand {
    fn name(&self) -> &str { "hand" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* invoke / approve / etc */ }
}
```

---

## 6. 相关

- 实现: `crates/apeireth-tools/`
- 1:1 翻译源: v0.9.21 SpectrAI hand organ
- 决策: 整合 #3 C-1 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
