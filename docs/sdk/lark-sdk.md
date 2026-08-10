# Lark SDK（飞书 stub）

> **依据**: `crates/apeireth-lark/src/lib.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: 🟡 **8 方法全 stub**（`NotImplemented` 返 501）R21 续

---

## 1. 概览

**功能**: 飞书（Feishu / Lark）开放平台接入
**目标 SDK**: @larksuiteoapi/node-sdk 1.x（v0.9.21 1:1 翻译源）
**1.0 状态**: 8 方法全 stub（per R20 阶段 1 拍板）

---

## 2. 8 stub 方法

```rust
// crates/apeireth-lark/src/lib.rs
pub struct LarkClient {
    app_id: String,
    app_secret: String,
}

impl LarkClient {
    pub fn new(app_id: impl Into<String>, app_secret: impl Into<String>) -> Self;
    
    // 8 stub 方法（全部 NotImplemented）
    pub async fn send_message(&self, chat_id: &str, msg: &str) -> Result<String, Error> {
        Err(Error::NotImplemented("send_message"))
    }
    
    pub async fn get_user_info(&self, user_id: &str) -> Result<UserInfo, Error> {
        Err(Error::NotImplemented("get_user_info"))
    }
    
    pub async fn list_chats(&self) -> Result<Vec<Chat>, Error> {
        Err(Error::NotImplemented("list_chats"))
    }
    
    pub async fn create_event(&self, cal_id: &str, event: &CalendarEvent) -> Result<String, Error> {
        Err(Error::NotImplemented("create_event"))
    }
    
    pub async fn upload_file(&self, content: &[u8], name: &str) -> Result<String, Error> {
        Err(Error::NotImplemented("upload_file"))
    }
    
    pub async fn approve_instance(&self, instance_id: &str, action: ApprovalAction) -> Result<(), Error> {
        Err(Error::NotImplemented("approve_instance"))
    }
    
    pub async fn search_docs(&self, query: &str) -> Result<Vec<Doc>, Error> {
        Err(Error::NotImplemented("search_docs"))
    }
    
    pub async fn get_doc_content(&self, doc_id: &str) -> Result<String, Error> {
        Err(Error::NotImplemented("get_doc_content"))
    }
}
```

---

## 3. 用法（1.0 调用示例）

```rust
use apeireth_lark::LarkClient;

let client = LarkClient::new(
    std::env::var("LARK_APP_ID")?,
    std::env::var("LARK_APP_SECRET")?,
);

// 1.0 调用 → 501 NotImplemented
match client.send_message("chat_id", "hello").await {
    Err(e) if e.is_not_implemented() => {
        println!("Lark SDK 1.0 stub; 计划 R21 实装");
    }
    _ => unreachable!(),
}
```

---

## 4. 错误

```rust
pub enum Error {
    NotImplemented(&'static str),  // 1.0 全部返此
    Auth(String),                    // R21
    Network(String),                 // R21
    Upstream(String),                // R21
    RateLimit,                       // R21
}

impl Error {
    pub fn is_not_implemented(&self) -> bool;
}
```

---

## 5. R21 计划

| 方法 | R21 实装来源 | 工作量估 |
|---|---|---|
| `send_message` | 飞书消息 v1 API | 1 owner × 1 周 |
| `get_user_info` | 通讯录 v3 API | 0.5 owner × 1 周 |
| `list_chats` | 群列表 v1 API | 0.5 owner × 1 周 |
| `create_event` | 日历 v4 API | 1 owner × 1 周 |
| `upload_file` | 素材 v1 API | 0.5 owner × 1 周 |
| `approve_instance` | 审批 v1 API | 1 owner × 1 周 |
| `search_docs` | 文档 v1 API | 1 owner × 1 周 |
| `get_doc_content` | 文档 v1 API | 0.5 owner × 1 周 |

**总估**: 6 owner × 1 周 ≈ 6 周

---

## 6. 与 message 工具的关系

`/v1/tools/message/invoke` 的 `channel: "lark"` 路径在 1.0 走 **直发 SMTP** fallback（per `apeireth-protocol`），Lark 真接等 R21。

---

## 7. 不假装

- ✅ 8 方法签名定义清楚
- ✅ 8 NotImplemented 错误明示
- 🟡 R21 实装计划列清
- ✅ 不假装已实现

---

## 8. 相关

- 实现: `crates/apeireth-lark/src/lib.rs`
- 1:1 翻译源: @larksuiteoapi/node-sdk 1.x (v0.9.21)
- 蓝图: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §2.5
- 决策: R20 阶段 1 拍板"SDK stub 留 R21 续"
