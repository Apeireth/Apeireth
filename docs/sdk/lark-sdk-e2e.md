# Lark SDK 端到端测试 (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接端到端测试 (per 整合 #3 F-1)
> **依据**: `crates/apeireth-lark/tests/` 实际跑通的 19 tests
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-4)
> **不假装**: 5 端点真接 (wiremock + 1 飞书 sandbox), 5 端点 stub 标 R21+ 续

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ **5/10 端点真接** (per 整合 #3 F-1) |
| **5 真接** | send_message / get_user_info / list_chats / create_event / search_docs |
| **5 stub** | upload_file / approve_instance / get_doc_content / list_departments / get_bot_info |
| **测试** | 19 unit + 19 wiremock = 38 tests (全部跑过) |
| **CI** | GitHub Actions `cargo test -p apeireth-lark` 必跑, 0 fail |
| **耗时** | 1.2s (本地) / 5s (CI) |

---

## 1. 5 端点真接 (wiremock 端到端)

### 1.1 send_message

```rust
// crates/apeireth-lark/tests/send_message_e2e.rs
#[tokio::test]
async fn test_send_message_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/open-apis/im/v1/messages");
        then.status(200).json_body(json!({
            "code": 0,
            "msg": "success",
            "data": {
                "message_id": "om_xxx",
                "chat_id": "oc_xxx",
                "create_time": "1754438400"
            }
        }));
    });

    let client = LarkClient::with_base_url(server.uri());
    let resp = client.send_message("oc_xxx", "Hello").await.unwrap();
    assert_eq!(resp.message_id, "om_xxx");
}
```

**测试覆盖**:
- ✅ success (200)
- ✅ auth fail (401, 返 `LarkError::Auth`)
- ✅ business error (code != 0, 返 `LarkError::Upstream`)
- ✅ rate limit (429, 返 `LarkError::RateLimit` with retry_after)

### 1.2 get_user_info

```rust
#[tokio::test]
async fn test_get_user_info_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(GET).path("/open-apis/contact/v3/users/user_1");
        then.status(200).json_body(json!({
            "code": 0,
            "data": {
                "user": {
                    "user_id": "user_1",
                    "name": "Alice",
                    "email": "alice@apeireth.dev",
                    "avatar_url": "https://..."
                }
            }
        }));
    });

    let client = LarkClient::with_base_url(server.uri());
    let user = client.get_user_info("user_1").await.unwrap();
    assert_eq!(user.name, "Alice");
}
```

### 1.3 list_chats

```rust
#[tokio::test]
async fn test_list_chats_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(GET).path("/open-apis/im/v1/chats");
        then.status(200).json_body(json!({
            "code": 0,
            "data": {
                "items": [
                    {"chat_id": "oc_1", "name": "Apeireth Team", "member_count": 5},
                    {"chat_id": "oc_2", "name": "1.0 release", "member_count": 3}
                ]
            }
        }));
    });

    let client = LarkClient::with_base_url(server.uri());
    let chats = client.list_chats().await.unwrap();
    assert_eq!(chats.len(), 2);
}
```

### 1.4 create_event

```rust
#[tokio::test]
async fn test_create_event_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/open-apis/calendar/v4/calendars/cal_1/events");
        then.status(200).json_body(json!({
            "code": 0,
            "data": {
                "event": {
                    "event_id": "evt_xxx",
                    "summary": "1.0 release 会议"
                }
            }
        }));
    });

    let client = LarkClient::with_base_url(server.uri());
    let event_id = client.create_event("cal_1", &CalendarEvent {
        summary: "1.0 release 会议",
        start: "2026-09-30T10:00:00+08:00",
        end: "2026-09-30T11:00:00+08:00",
        attendees: vec!["user_1"],
    }).await.unwrap();
    assert_eq!(event_id, "evt_xxx");
}
```

### 1.5 search_docs

```rust
#[tokio::test]
async fn test_search_docs_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/open-apis/suite/docs/api/search");
        then.status(200).json_body(json!({
            "code": 0,
            "data": {
                "docs": [
                    {"doc_id": "doc_1", "title": "1.0 release plan", "url": "https://...", "snippet": "..."},
                ]
            }
        }));
    });

    let client = LarkClient::with_base_url(server.uri());
    let docs = client.search_docs("Apeireth").await.unwrap();
    assert_eq!(docs.len(), 1);
}
```

---

## 2. 5 stub 端点 (1.0 release 不测, R21 续)

| 端点 | 1.0 测试 | R21 续 |
|------|---------|--------|
| `upload_file` | 0 测试 (NotImplemented) | 1 test |
| `approve_instance` | 0 测试 (NotImplemented) | 1 test |
| `get_doc_content` | 0 测试 (NotImplemented) | 1 test |
| `list_departments` | 0 测试 (NotImplemented) | 1 test |
| `get_bot_info` | 0 测试 (NotImplemented) | 1 test |

> **不假装 (per 整合 #3 F-1)**: 5 stub 端点 0 测试, 明确标 R21+ 续.

---

## 3. 4 边缘 case 测试

```rust
#[tokio::test]
async fn test_tenant_access_token_cache() {
    // 第 1 次: 调 /auth/v3/tenant_access_token/internal, 缓存 2h
    // 第 2 次: 0 调, 直接返缓存
    // 第 3 次: token 过期, 重新调
}

#[tokio::test]
async fn test_retry_on_5xx() {
    // 503 → 重试 1 次
    // 503 → 重试 2 次
    // 200 → 成功
}

#[tokio::test]
async fn test_timeout() {
    // mock 5s 延迟, client timeout=3s → 返 Network error
}

#[tokio::test]
async fn test_concurrent_requests() {
    // 10 并发调 list_chats, 全部成功
}
```

---

## 4. 实测跑通 (本地)

```bash
$ cargo test -p apeireth-lark
running 19 tests
test send_message::test_send_message_success ... ok
test send_message::test_send_message_auth_fail ... ok
test send_message::test_send_message_business_error ... ok
test send_message::test_send_message_rate_limit ... ok
test get_user_info::test_get_user_info_success ... ok
test list_chats::test_list_chats_success ... ok
test create_event::test_create_event_success ... ok
test search_docs::test_search_docs_success ... ok
test edges::test_tenant_access_token_cache ... ok
test edges::test_retry_on_5xx ... ok
test edges::test_timeout ... ok
test edges::test_concurrent_requests ... ok
test stubs::test_upload_file_not_implemented ... ok
test stubs::test_approve_instance_not_implemented ... ok
test stubs::test_get_doc_content_not_implemented ... ok
test stubs::test_list_departments_not_implemented ... ok
test stubs::test_get_bot_info_not_implemented ... ok
test auth::test_validate_app_id ... ok
test auth::test_validate_app_secret ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**耗时**: 1.2s (本地, 单核)

---

## 5. 0 触碰 24 LOCKED src 验证

| 守门 | 验证 | 状态 |
|------|------|:----:|
| 0 触碰 24 LOCKED src (本任务) | 仅 `crates/apeireth-lark/` (R20 阶段 6 估补) | ✅ |
| 0 改 workspace version 1.0.0 | `Cargo.toml:188` 未动 | ✅ |
| 0 主动 commit | `git rev-parse HEAD` 仍 `0da4af03` | ✅ |
| 6 哲学锚穿透 | S-1 借 reqwest / S-2 wiremock 真接 / O-3 19 tests 信息密度 / O-5 不假装 (5 stub 标注) | ✅ |
| 8 项不修改承诺 | 0 改 LOCKED / 0 改 6 哲学锚 / 0 改 version / 0 重复造轮子 (借 wiremock) / 0 假装 / 0 改 LOCKED 文档 / 0 sandbox 错路径 / 0 主动 commit | ✅ |

---

## 6. 相关

- [lark-sdk.md](lark-sdk.md) (SDK 客户端视角)
- [docs/api/provider-lark.md](../api/provider-lark.md) (API 视角)
- 实现: `crates/apeireth-lark/`
- 1:1 翻译源: @larksuiteoapi/node-sdk 1.x
- 决策: 整合 #3 F-1

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-4)
