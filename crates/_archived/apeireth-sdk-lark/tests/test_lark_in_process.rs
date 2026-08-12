//! # apeireth-sdk-lark 集成测试 (in-process, per task spec §5)
//!
//! 12-15 测试覆盖:
//! - 6 消息类型演示
//! - 5 鉴权演示
//! - 6 K-1 强校验
//! - 8 核心 API 全返 NotImplemented
//!
//! **STUB MODE**: 跑在 `cargo test -p apeireth-sdk-lark` 阶段, 不真接飞书 API.
//! R21 续真接后, 加 mock server / wiremock 集成测试.
//!
//! ## 跑测试
//!
//! ```bash
//! cargo test -p apeireth-sdk-lark
//! ```

use apeireth_sdk_lark::{
    lark_verify_webhook, AppIdHolder, AppSecretHolder, ApprovalFormField, ApprovalInstance,
    ApprovalTask, CalendarEvent, CalendarEventQuery, CardContent, Department, Document,
    DocumentType, EventStatus, FileContent, ImageContent, InstanceStatus, InteractiveContent,
    LarkClient, LarkClientImpl, LarkError, LarkWebhookEvent, Message, MessageType, PostContent,
    PostElement, PostLocale, PostParagraph, ReceiveIdType, TenantAccessToken, TextContent,
    User, UserAccessToken, UserIdType, UserQuery, WebhookToken, AUTH_METHOD_COUNT, CORE_API_COUNT,
    DEFAULT_LARK_API_BASE, ENTITY_COUNT, K1_STRONG_VALIDATION_COUNT, LARK_SCHEMA_VERSION,
    LARK_TOOL_WHITELIST, LARK_TOOL_WHITELIST_COUNT, MESSAGE_TYPE_COUNT, MAX_TOKEN_TTL_SECONDS,
    PLATFORM_NAME, PROVIDER_NAME, STUB_MODE, SUPPORTED_MESSAGE_TYPES, TASK_STATUS_PENDING,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// §1 6 消息类型演示
// ============================================================================

#[test]
fn test_message_6_kinds() {
    // 6 类型全覆盖演示
    // 1. Text
    let m1 = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello, 飞书!".to_string(),
    )
    .expect("text message");
    assert_eq!(m1.msg_type, MessageType::Text);
    assert!(m1.content.contains("Hello"));

    // 2. Post
    let para = PostParagraph {
        elements: vec![PostElement::Text {
            text: "标题".to_string(),
        }],
    };
    let post = PostContent {
        locale: {
            let mut h = HashMap::new();
            h.insert(
                "zh_cn".to_string(),
                PostLocale {
                    title: "通知".to_string(),
                    content: vec![para],
                },
            );
            h
        },
    };
    let m2 = Message::post(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        post,
    )
    .expect("post message");
    assert_eq!(m2.msg_type, MessageType::Post);

    // 3. Image
    let m3 = Message::image(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "img_v2_abc123".to_string(),
    )
    .expect("image message");
    assert_eq!(m3.msg_type, MessageType::Image);

    // 4. File
    let m4 = Message::file(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "file_v2_abc".to_string(),
    )
    .expect("file message");
    assert_eq!(m4.msg_type, MessageType::File);

    // 5. Card
    let card = CardContent::plain("标题", "正文");
    let m5 = Message::card(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        card,
    )
    .expect("card message");
    assert_eq!(m5.msg_type, MessageType::Card);

    // 6. Interactive
    let interactive = InteractiveContent::plain("标题", "正文");
    let m6 = Message::interactive(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        interactive,
    )
    .expect("interactive message");
    assert_eq!(m6.msg_type, MessageType::Interactive);

    // 6 类型守门
    assert_eq!(MESSAGE_TYPE_COUNT, 6);
    assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 6);
}

// ============================================================================
// §2 5 鉴权演示
// ============================================================================

#[test]
fn test_auth_5_kinds() {
    // 1. App ID
    let mut id_holder = AppIdHolder::empty();
    id_holder
        .set("cli_a1b2c3d4e5f6".to_string())
        .expect("valid app id");
    assert!(id_holder.is_set());

    // 2. App Secret
    let mut secret_holder = AppSecretHolder::empty();
    secret_holder
        .set("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid app secret");
    assert!(secret_holder.is_set());

    // 3. tenant_access_token
    let t = TenantAccessToken::new(
        "cli_a1b2c3d4e5f6".to_string(),
        "t-abc123".to_string(),
        7200,
    )
    .expect("valid");
    assert!(!t.is_expired());
    assert!(t.remaining_ttl_secs() <= 7200);

    // 4. user_access_token
    let u = UserAccessToken::new(
        "cli_a1b2c3d4e5f6".to_string(),
        "u-abc".to_string(),
        "ur-xyz".to_string(),
        "ou_user1234567890abcdef".to_string(),
        7200,
    )
    .expect("valid");
    assert!(!u.is_expired());

    // 5. webhook_token
    let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
        .expect("valid");
    assert!(wh.verify("token_xxx"));
    assert!(!wh.verify("wrong"));

    // 5 鉴权守门
    assert_eq!(AUTH_METHOD_COUNT, 5);
    assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
}

// ============================================================================
// §3 6 K-1 强校验
// ============================================================================

#[test]
fn test_k1_app_id_empty() {
    // K-1 #1: app_id 空 → AppIdMissing
    let result = LarkError::validate_app_id("");
    assert!(matches!(result, Err(LarkError::AppIdMissing)));
    let result = LarkError::validate_app_id("   ");
    assert!(matches!(result, Err(LarkError::AppIdMissing)));
}

#[test]
fn test_k1_app_secret_empty() {
    // K-1 #2: app_secret 空 → AppSecretMissing
    let result = LarkError::validate_app_secret("");
    assert!(matches!(result, Err(LarkError::AppSecretMissing)));
    // K-1 #2: app_secret 太短 → AppSecretInvalid
    let result = LarkError::validate_app_secret("short");
    assert!(matches!(result, Err(LarkError::AppSecretInvalid(5))));
}

#[test]
fn test_k1_chat_id_invalid() {
    // K-1 #3: chat_id 必须 oc_ / on_ 前缀
    assert!(matches!(
        LarkError::validate_chat_id("oc_a1b2c3d4e5f6"),
        Ok(())
    ));
    assert!(matches!(
        LarkError::validate_chat_id("on_a1b2c3d4e5f6"),
        Ok(())
    ));
    assert!(matches!(
        LarkError::validate_chat_id("invalid"),
        Err(LarkError::ChatIdInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_chat_id(""),
        Err(LarkError::ChatIdInvalid(_))
    ));
}

#[test]
fn test_k1_open_id_empty() {
    // K-1 #4: open_id 必须 ou_ 前缀
    assert!(matches!(
        LarkError::validate_open_id("ou_user1234567890abcdef"),
        Ok(())
    ));
    assert!(matches!(
        LarkError::validate_open_id(""),
        Err(LarkError::OpenIdInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_open_id("cli_xxx"),
        Err(LarkError::OpenIdInvalid(_))
    ));
}

#[test]
fn test_k1_email_invalid() {
    // K-1 #5: email 必须 RFC 5322
    assert!(LarkError::validate_email("user@example.com").is_ok());
    assert!(matches!(
        LarkError::validate_email("not-email"),
        Err(LarkError::EmailInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_email("missing@domain"),
        Err(LarkError::EmailInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_email(""),
        Err(LarkError::EmailInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_email("@example.com"),
        Err(LarkError::EmailInvalid(_))
    ));
}

#[test]
fn test_k1_mobile_invalid() {
    // K-1 #6: mobile 必须 E.164 (+ + 7-15 数字)
    assert!(LarkError::validate_mobile("+8613800138000").is_ok());
    assert!(LarkError::validate_mobile("+14155552671").is_ok());
    assert!(matches!(
        LarkError::validate_mobile(""),
        Err(LarkError::MobileInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_mobile("13800138000"), // 缺 +
        Err(LarkError::MobileInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_mobile("+12345"), // < 7 位
        Err(LarkError::MobileInvalid(_))
    ));
    assert!(matches!(
        LarkError::validate_mobile("+12345678901234567"), // > 15 位
        Err(LarkError::MobileInvalid(_))
    ));
}

// ============================================================================
// §4 8 核心 API 全部返 NotImplemented (STUB 模式守门)
// ============================================================================

#[tokio::test]
async fn test_send_message_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let msg = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello".to_string(),
    )
    .expect("valid");
    let result = client.send_message(&msg).await;
    assert!(matches!(
        result,
        Err(LarkError::NotImplemented("send_message"))
    ));
}

#[tokio::test]
async fn test_list_calendar_events_returns_not_implemented() {
    let client = LarkClientImpl::new();
    use chrono::Utc;
    let query = CalendarEventQuery {
        calendar_id: "cal_xxx".to_string(),
        start_time: Utc::now(),
        end_time: Utc::now() + chrono::Duration::days(7),
        page_size: 50,
        page_token: None,
    };
    let result = client.list_calendar_events(&query).await;
    assert!(matches!(
        result,
        Err(LarkError::NotImplemented("list_calendar_events"))
    ));
}

#[tokio::test]
async fn test_get_user_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let query = UserQuery::new(
        "ou_user1234567890abcdef".to_string(),
        UserIdType::OpenId,
    )
    .expect("valid");
    let result = client.get_user(&query).await;
    assert!(matches!(result, Err(LarkError::NotImplemented("get_user"))));
}

#[tokio::test]
async fn test_get_department_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let result = client.get_department("od_dept123").await;
    assert!(matches!(
        result,
        Err(LarkError::NotImplemented("get_department"))
    ));
}

#[tokio::test]
async fn test_create_doc_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let doc = Document::new_docx("title".to_string(), None).expect("valid");
    let result = client.create_doc(&doc).await;
    assert!(matches!(result, Err(LarkError::NotImplemented("create_doc"))));
}

#[tokio::test]
async fn test_create_sheet_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let sheet = Document::new_sheet("title".to_string(), None).expect("valid");
    let result = client.create_sheet(&sheet).await;
    assert!(matches!(result, Err(LarkError::NotImplemented("create_sheet"))));
}

#[tokio::test]
async fn test_get_approval_instance_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let result = client.get_approval_instance("instance_001").await;
    assert!(matches!(
        result,
        Err(LarkError::NotImplemented("get_approval_instance"))
    ));
}

#[tokio::test]
async fn test_verify_webhook_returns_not_implemented() {
    let client = LarkClientImpl::new();
    let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
        .expect("valid");
    let event = LarkWebhookEvent::new_url_verification("challenge".to_string());
    let result = client.verify_webhook(&event, &wh_token).await;
    assert!(matches!(
        result,
        Err(LarkError::NotImplemented("verify_webhook"))
    ));
}

// ============================================================================
// §5 编译期 hardcode + 守门常量
// ============================================================================

#[test]
fn test_compile_time_hardcode_constants() {
    let _ = STUB_MODE;
    assert!(apeireth_sdk_lark::is_stub_mode());
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(PROVIDER_NAME, "lark");
    assert_eq!(LARK_SCHEMA_VERSION, "1");
    assert!(DEFAULT_LARK_API_BASE.starts_with("https://"));
    assert_eq!(ENTITY_COUNT, 4);
    assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
    assert_eq!(LARK_TOOL_WHITELIST_COUNT, 8);
    assert_eq!(CORE_API_COUNT, 8);
    assert_eq!(LARK_TOOL_WHITELIST.len(), 8);
}

#[test]
fn test_validate_tool_call_m3_defense() {
    // 白名单内通过
    let args = serde_json::json!({});
    let result = apeireth_sdk_lark::validate_tool_call("apeireth_sdk_lark_send_message", &args);
    assert!(result.is_ok());
    // 白名单外拒绝
    let result = apeireth_sdk_lark::validate_tool_call("apeireth_sdk_lark_bogus", &args);
    assert!(result.is_err());
}

// ============================================================================
// §6 端到端构造演示 (STUB 模式拼装完整 chain, R21 续真接后跑通)
// ============================================================================

#[tokio::test]
async fn test_end_to_end_construction() {
    // 1. 构造客户端
    let mut client = LarkClientImpl::new();
    client
        .set_app_id("cli_a1b2c3d4e5f6".to_string())
        .expect("valid app id");
    client
        .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid app secret");
    assert!(client.is_configured());

    // 2. 构造 tenant token
    let token = TenantAccessToken::new(
        client.app_id().unwrap(),
        "t-abc123def456".to_string(),
        7200,
    )
    .expect("valid");
    client.set_tenant_token(token);

    // 3. 调 8 API 全返 NotImplemented
    let msg = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello, 飞书!".to_string(),
    )
    .expect("valid");
    let r = client.send_message(&msg).await;
    assert!(matches!(r, Err(LarkError::NotImplemented("send_message"))));

    // 4. stub_status
    let status = client.stub_status();
    assert!(status.stub_mode);
    assert!(status.configured);
    assert!(status.tenant_token_set);
    assert!(!status.tenant_token_expired);
}

// ============================================================================
// §7 4 实体构造演示
// ============================================================================

#[test]
fn test_4_entity_construction() {
    // 1. Message
    let _msg = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello".to_string(),
    )
    .expect("valid");

    // 2. CalendarEvent
    use chrono::Utc;
    let start = Utc::now();
    let end = start + chrono::Duration::hours(1);
    let event = CalendarEvent::new("cal_xxx", "会议", start, end).expect("valid");
    assert_eq!(event.status, EventStatus::Tentative);

    // 3. User
    let user = User::new(
        "ou_user1234567890abcdef".to_string(),
        "Alice".to_string(),
    )
    .expect("valid");
    assert_eq!(user.name, "Alice");

    // 4. Document
    let doc = Document::new_docx("title".to_string(), None).expect("valid");
    assert_eq!(doc.doc_type, DocumentType::Doc);

    // 4 实体守门
    assert_eq!(ENTITY_COUNT, 4);
}

// ============================================================================
// §8 辅助: task status 守门 (per 8 项不修改承诺守门, 防 3 variant 漂移)
// ============================================================================

#[test]
fn test_task_status_count_3() {
    use apeireth_sdk_lark::TaskStatus;
    assert_eq!(TASK_STATUS_PENDING, TaskStatus::Pending);
    let statuses = [
        TaskStatus::Pending,
        TaskStatus::Approved,
        TaskStatus::Rejected,
    ];
    assert_eq!(statuses.len(), 3);
    assert_eq!(apeireth_sdk_lark::TASK_STATUS_COUNT, 3);
}

// ============================================================================
// §9 辅助: 异步 sleep 演示 (tokio runtime 可用, R21 续真接时配合 reqwest timeout)
// ============================================================================

#[tokio::test]
async fn test_async_sleep_works() {
    // 验证 tokio runtime 可用, R21 续真接时配 reqwest timeout
    let start = std::time::Instant::now();
    sleep(Duration::from_millis(10)).await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(10));
}

// ============================================================================
// §10 辅助: webhook verify 守门 (per LarkWebhookEvent + verify_webhook_event 函数)
// ============================================================================

#[test]
fn test_webhook_verify_works_via_function() {
    // STUB 模式 client.verify_webhook 返 NotImplemented, 但 free function 可用
    // (因为它内部已经实现 token 校验 + challenge 提取)
    let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
        .expect("valid");
    let event = LarkWebhookEvent {
        event_type: apeireth_sdk_lark::WebhookEventType::UrlVerification,
        app_id: "cli_a1b2c3d4e5f6".to_string(),
        token: "token_xxx".to_string(),
        timestamp_secs: 0,
        challenge: Some("challenge_xxx".to_string()),
        encrypt: None,
        event: HashMap::new(),
    };
    let result = lark_verify_webhook(&event, &wh_token);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(matches!(
        result,
        apeireth_sdk_lark::WebhookVerifyResult::Challenge(_)
    ));
}

// ============================================================================
// §11 辅助: ApprovalFormField + ApprovalInstance + ApprovalTask 演示
// ============================================================================

#[test]
fn test_approval_chain_construction() {
    // ApprovalInstance
    let instance = ApprovalInstance::new(
        "approval_code_xxx".to_string(),
        "ou_user1234567890abcdef".to_string(),
    )
    .expect("valid")
    .with_form_field(
        "reason".to_string(),
        "textarea".to_string(),
        "出差".to_string(),
    );
    assert_eq!(instance.form.len(), 1);
    assert_eq!(instance.status, InstanceStatus::Pending);

    // ApprovalTask
    let task = ApprovalTask::new(
        "instance_001".to_string(),
        "ou_approver1234567890abcdef".to_string(),
    )
    .expect("valid");
    assert_eq!(task.instance_id, "instance_001");
}

// ============================================================================
// §12 辅助: Department 演示
// ============================================================================

#[test]
fn test_department_construction() {
    let dept = Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
    assert_eq!(dept.name, "工程部");
    assert_eq!(dept.status, "active");
}

// ============================================================================
// §13 辅助: CardContent + ImageContent + FileContent 字段演示
// ============================================================================

#[test]
fn test_content_types_field_round_trip() {
    // TextContent
    let text = TextContent::new("Hello");
    assert_eq!(text.text, "Hello");
    // CardContent
    let card = CardContent::plain("标题", "正文");
    assert!(!card.elements.is_empty());
    // ImageContent / FileContent (via 字段)
    let _ = ImageContent {
        image_key: "img_xxx".to_string(),
    };
    let _ = FileContent {
        file_key: "file_xxx".to_string(),
    };
}
