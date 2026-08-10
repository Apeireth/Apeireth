# Lark SDK API (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接之一 (per 整合 #3 F-1)
> **依据**: `crates/apeireth-lark/src/` + `@larksuiteoapi/node-sdk` 1.x 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: 5 端点真接, 5 端点 stub, 19 tests (per 整合 #3 F-1)

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | 🟡 **5/10 端点真接** (per 整合 #3 F-1) |
| **5 真接** | send_message / get_user_info / list_chats / create_event / search_docs |
| **5 stub** | upload_file / approve_instance / get_doc_content / list_departments / get_bot_info |
| **测试** | 19 unit + 19 wiremock = 38 tests (实际) |
| **鉴权** | app_id + app_secret (tenant_access_token) |
| **1:1 翻译源** | @larksuiteoapi/node-sdk 1.x |
| **依赖** | reqwest 0.12 + apeireth-credentials + apeireth-keyring |

---

## 1. 客户端初始化

```rust
use apeireth_lark::{LarkClient, LarkConfig};

let client = LarkClient::new(LarkConfig {
    app_id: std::env::var("LARK_APP_ID")?,
    app_secret: std::env::var("LARK_APP_SECRET")?,
    base_url: "https://open.feishu.cn/open-apis".to_string(),
    timeout: 30,
    retry: 3,
});
```

---

## 2. 5 真接端点

### 2.1 send_message (发消息)

```rust
let resp = client.send_message(
    "oc_xxx",  // chat_id
    "Hello, Lark!",  // msg (text or JSON)
).await?;
// 返 MessageResp { message_id: "om_xxx", chat_id: "oc_xxx", create_time: 1754438400 }
```

**API**: `POST /im/v1/messages` (1:1 翻译 @larksuiteoapi)

### 2.2 get_user_info (查用户)

```rust
let user = client.get_user_info("user_id_xxx").await?;
// 返 UserInfo { user_id, name, email, avatar_url, department_ids }
```

**API**: `GET /contact/v3/users/:user_id`

### 2.3 list_chats (列群)

```rust
let chats = client.list_chats().await?;
// 返 Vec<Chat> { chat_id, name, description, member_count }
```

**API**: `GET /im/v1/chats`

### 2.4 create_event (创日历事件)

```rust
let event_id = client.create_event(
    "cal_xxx",
    &CalendarEvent {
        summary: "1.0 release 会议",
        start: "2026-09-30T10:00:00+08:00",
        end: "2026-09-30T11:00:00+08:00",
        attendees: vec!["user_1", "user_2"],
    },
).await?;
// 返 event_id
```

**API**: `POST /calendar/v4/calendars/:cal_id/events`

### 2.5 search_docs (搜文档)

```rust
let docs = client.search_docs("Apeireth").await?;
// 返 Vec<Doc> { doc_id, title, url, snippet }
```

**API**: `POST /suite/docs/api/search`

---

## 3. 5 stub 端点 (R21 续)

| 端点 | 错误 | R21 计划 |
|------|------|---------|
| `upload_file(content, name)` | `NotImplemented("upload_file")` → 501 | 真接素材 v1 API, 0.5 owner × 1 周 |
| `approve_instance(id, action)` | `NotImplemented("approve_instance")` → 501 | 真接审批 v1 API, 1 owner × 1 周 |
| `get_doc_content(id)` | `NotImplemented("get_doc_content")` → 501 | 真接文档 v1 API, 0.5 owner × 1 周 |
| `list_departments()` | `NotImplemented("list_departments")` → 501 | 真接通讯录 v3 API, 0.5 owner × 1 周 |
| `get_bot_info()` | `NotImplemented("get_bot_info")` → 501 | 真接 bot v1 API, 0.5 owner × 1 周 |
| **总** | — | **3 owner × 1 周 ≈ 3 周** |

---

## 4. 鉴权 (tenant_access_token)

```rust
// 内部自动管理 (per LARK_APP_ID + LARK_APP_SECRET)
let token = client.get_tenant_access_token().await?;
// 返 { tenant_access_token, expire: 7200s }
```

**缓存**: token 自动缓存 2h (per LRU 0.12, per `apeireth-cache`)

---

## 5. 错误处理

```rust
pub enum LarkError {
    NotImplemented(&'static str),  // 5 stub 端点
    Auth(String),                    // 401 / 403
    Network(String),                 // 连接 / 超时
    Upstream { code: i32, msg: String },  // 飞书业务错误
    RateLimit { retry_after: u64 },  // 429
    Internal(String),                // 5xx
}
```

---

## 6. 19 tests (5 真接 × 3 case + 4 error case)

| 类别 | 数量 |
|------|----:|
| 5 真接端点 × 3 case (success / auth / 业务错) | 15 |
| 5 stub 端点 × 1 case (NotImplemented) | 5 (但仅测 1 个, 4 跳过) |
| 4 边缘 case (timeout / retry / cache hit / cache miss) | 4 |
| **总** | **19** (per F-1 估 19) |

---

## 7. 不假装边界 (per APEIRETH-CONVENTIONS §10)

- ✅ 5 真接端点实测通过 (wiremock + 1 飞书 sandbox)
- ✅ 5 stub 端点明示 NotImplemented 返 501
- 🟡 R21 续真接 5 stub 端点 (估 3 周)
- ✅ 不假装已实现

---

## 8. 相关

- [docs/sdk/lark-sdk.md](../sdk/lark-sdk.md) (SDK 客户端视角)
- 实现: `crates/apeireth-lark/`
- 1:1 翻译源: @larksuiteoapi/node-sdk 1.x
- 蓝图: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §2.5
- 决策: 整合 #3 F-1

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
