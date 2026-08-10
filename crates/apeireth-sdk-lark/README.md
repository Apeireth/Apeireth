# apeireth-sdk-lark (STUB MODE)

> ⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**

Lark 飞书 SDK skeleton, 1:1 翻译 `@larksuiteoapi/lark-sdk` v0.9.21 商业版 API 表面.

## 8 核心 API (全 STUB, 返 `LarkError::NotImplemented`)

| # | API | 1:1 翻译商业版 |
|---:|---|---|
| 1 | `send_message` | `Lark.Client.im.message.create` |
| 2 | `list_calendar_events` | `Lark.Client.calendar.event.list` |
| 3 | `get_user` | `Lark.Client.contact.user.get` |
| 4 | `get_department` | `Lark.Client.contact.department.get` |
| 5 | `create_doc` | `Lark.Client.docx.document.create` |
| 6 | `create_sheet` | `Lark.Client.sheet.spreadsheet.create` |
| 7 | `get_approval_instance` | `Lark.Client.approval.instance.get` |
| 8 | `verify_webhook` | `Lark.Client.im.event.verify` |

## 6 消息类型 / 5 鉴权 / 4 实体 / 6 K-1 强校验

- **6 消息类型**: `Text` / `Post` / `Image` / `File` / `Card` / `Interactive`
- **5 鉴权**: `App ID` + `App Secret` + `tenant_access_token` + `user_access_token` + `webhook_token`
- **4 实体**: `Message` / `CalendarEvent` / `User` / `Document`
- **6 K-1 强校验**: `app_id` / `app_secret` / `chat_id` (oc_/on_) / `open_id` (ou_) / `email` (RFC 5322) / `mobile` (E.164)

## 状态: ⏳ STUB skeleton (R20 阶段 4 效果)

当前 stage 跑 `cargo check` + 14+ fixture + 6 K-1 验证. **0 真接 SDK** — R21 续真接.

## 跑 demo

```bash
cargo run --manifest-path crates/apeireth-sdk-lark/Cargo.toml --example lark_demo
```

## 跑 test

```bash
cargo test -p apeireth-sdk-lark
```
