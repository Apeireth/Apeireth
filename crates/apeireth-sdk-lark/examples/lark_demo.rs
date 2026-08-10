//! # apeireth-sdk-lark stub demo (R20 阶段 4 效果)
//!
//! 演示 8 stub 工具返 `LarkError::NotImplemented` + 6 消息类型 + 5 鉴权 + 4 实体
//! + 6 K-1 强校验 + 8 TOOL_WHITELIST + m3 防御.
//! **R21 续真接 @larksuiteoapi/lark-sdk 后, 本 demo 会被替换成真 API demo.**
//!
//! ## 运行
//!
//! ```bash
//! cargo run --manifest-path crates/apeireth-sdk-lark/Cargo.toml --example lark_demo
//! ```

use apeireth_sdk_lark::{
    is_stub_mode, lark_verify_webhook, validate_tool_call, ApprovalInstance, CalendarEvent,
    CalendarEventQuery, CardContent, Department, Document, EventStatus, InstanceStatus, LarkClient,
    LarkClientImpl, LarkError, Message, MessageType, ReceiveIdType, TenantAccessToken,
    UserAccessToken, WebhookToken, AUTH_METHOD_COUNT, CORE_API_COUNT, DEFAULT_LARK_API_BASE,
    DEFAULT_TENANT_TOKEN_TTL_SECONDS, ENTITY_COUNT, K1_STRONG_VALIDATION_COUNT,
    LARK_SCHEMA_VERSION, LARK_TOOL_WHITELIST, LARK_TOOL_WHITELIST_COUNT, MESSAGE_TYPE_COUNT,
    MAX_TOKEN_TTL_SECONDS, PLATFORM_NAME, PROVIDER_NAME, STUB_MODE, SUPPORTED_MESSAGE_TYPES,
};
use chrono::Utc;
use std::collections::HashMap;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-sdk-lark stub demo (R20 阶段 4 效果) ===");
    println!();

    // 1) 编译期 hardcode 守门 (K-1 强校验)
    println!("[§1 编译期 hardcode 守门]");
    println!("  LARK_SCHEMA_VERSION         = {}", LARK_SCHEMA_VERSION);
    println!("  PLATFORM_NAME               = {}", PLATFORM_NAME);
    println!("  PROVIDER_NAME               = {}", PROVIDER_NAME);
    println!("  STUB_MODE                   = {}", STUB_MODE);
    println!("  is_stub_mode()              = {}", is_stub_mode());
    println!("  DEFAULT_LARK_API_BASE       = {}", DEFAULT_LARK_API_BASE);
    println!("  DEFAULT_TENANT_TOKEN_TTL    = {}", DEFAULT_TENANT_TOKEN_TTL_SECONDS);
    println!("  MAX_TOKEN_TTL_SECONDS       = {}", MAX_TOKEN_TTL_SECONDS);
    println!("  CORE_API_COUNT              = {}", CORE_API_COUNT);
    println!("  MESSAGE_TYPE_COUNT          = {}", MESSAGE_TYPE_COUNT);
    println!("  AUTH_METHOD_COUNT           = {}", AUTH_METHOD_COUNT);
    println!("  ENTITY_COUNT                = {}", ENTITY_COUNT);
    println!("  K1_STRONG_VALIDATION_COUNT  = {}", K1_STRONG_VALIDATION_COUNT);
    println!();

    // 2) 6 消息类型
    println!("[§2 6 消息类型 (K-1 强校验守门)]");
    for mt in SUPPORTED_MESSAGE_TYPES {
        println!("  MessageType::{:?} -> \"{}\"", mt, mt.as_str());
    }
    println!();

    // 3) 8 TOOL_WHITELIST (m3 防御)
    println!("[§3 8 工具白名单 (m3 防御)]");
    println!("  LARK_TOOL_WHITELIST_COUNT = {}", LARK_TOOL_WHITELIST_COUNT);
    for (i, tool) in LARK_TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    println!();

    // 4) m3 防御: validate_tool_call 测试
    println!("[§4 m3 防御: validate_tool_call]");
    let args = serde_json::json!({});
    let valid = validate_tool_call("apeireth_sdk_lark_send_message", &args);
    println!("  白名单内工具: {:?}", valid);
    let invalid = validate_tool_call("apeireth_sdk_lark_bogus", &args);
    println!("  非白名单工具: {:?}", invalid);
    println!();

    // 5) 6 K-1 强校验演示
    println!("[§5 6 K-1 强校验演示]");
    println!("  K-1 #1 App ID:     \"\" -> {:?}", LarkError::validate_app_id(""));
    println!(
        "  K-1 #1 App ID:     \"cli_a1b2c3d4e5f6\" -> {:?}",
        LarkError::validate_app_id("cli_a1b2c3d4e5f6")
    );
    println!(
        "  K-1 #1 App ID:     \"app_xxx\" -> {:?}",
        LarkError::validate_app_id("app_xxx")
    );
    println!("  K-1 #2 App Secret: \"\" -> {:?}", LarkError::validate_app_secret(""));
    println!(
        "  K-1 #2 App Secret: \"abcdef1234567890abcdef1234567890\" -> {:?}",
        LarkError::validate_app_secret("abcdef1234567890abcdef1234567890")
    );
    println!("  K-1 #3 Chat ID:    \"\" -> {:?}", LarkError::validate_chat_id(""));
    println!(
        "  K-1 #3 Chat ID:    \"oc_a1b2c3d4e5f6\" -> {:?}",
        LarkError::validate_chat_id("oc_a1b2c3d4e5f6")
    );
    println!(
        "  K-1 #3 Chat ID:    \"xx_a1b2c3d4e5f6\" -> {:?}",
        LarkError::validate_chat_id("xx_a1b2c3d4e5f6")
    );
    println!("  K-1 #4 Open ID:    \"\" -> {:?}", LarkError::validate_open_id(""));
    println!(
        "  K-1 #4 Open ID:    \"ou_user1234567890abcdef\" -> {:?}",
        LarkError::validate_open_id("ou_user1234567890abcdef")
    );
    println!("  K-1 #5 Email:      \"\" -> {:?}", LarkError::validate_email(""));
    println!(
        "  K-1 #5 Email:      \"user@example.com\" -> {:?}",
        LarkError::validate_email("user@example.com")
    );
    println!(
        "  K-1 #5 Email:      \"not-email\" -> {:?}",
        LarkError::validate_email("not-email")
    );
    println!("  K-1 #6 Mobile:     \"\" -> {:?}", LarkError::validate_mobile(""));
    println!(
        "  K-1 #6 Mobile:     \"+8613800138000\" -> {:?}",
        LarkError::validate_mobile("+8613800138000")
    );
    println!(
        "  K-1 #6 Mobile:     \"13800138000\" -> {:?}",
        LarkError::validate_mobile("13800138000")
    );
    println!();

    // 6) LarkClientImpl 构造 + 5 鉴权
    println!("[§6 LarkClientImpl 构造 + 5 鉴权配置]");
    let mut client = LarkClientImpl::new();
    client
        .set_app_id("cli_a1b2c3d4e5f6".to_string())
        .expect("valid app id");
    client
        .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
        .expect("valid app secret");
    println!(
        "  app_id   = {}",
        client.app_id().as_deref().unwrap_or("None")
    );
    println!(
        "  app_secret   = {}",
        if client.app_secret().is_some() {
            "*** (set)"
        } else {
            "None"
        }
    );
    println!("  is_configured = {}", client.is_configured());

    // 5. tenant_access_token
    let token = TenantAccessToken::new(
        "cli_a1b2c3d4e5f6".to_string(),
        "t-abc123def456".to_string(),
        7200,
    )
    .expect("valid");
    client.set_tenant_token(token);
    println!(
        "  tenant_token   = set, remaining_ttl = {}s",
        client
            .tenant_token()
            .map(|t| t.remaining_ttl_secs())
            .unwrap_or(0)
    );
    println!();

    // 7) 8 核心 API stub 返 NotImplemented
    println!("[§7 8 核心 API stub 返 NotImplemented]");
    let msg = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello, 飞书!".to_string(),
    )
    .expect("valid");
    println!("  send_message            : {:?}", client.send_message(&msg).await);

    let query = CalendarEventQuery {
        calendar_id: "cal_xxx".to_string(),
        start_time: Utc::now(),
        end_time: Utc::now() + chrono::Duration::days(7),
        page_size: 50,
        page_token: None,
    };
    println!(
        "  list_calendar_events    : {:?}",
        client.list_calendar_events(&query).await
    );

    let user_query = apeireth_sdk_lark::UserQuery::new(
        "ou_user1234567890abcdef".to_string(),
        apeireth_sdk_lark::UserIdType::OpenId,
    )
    .expect("valid");
    println!("  get_user                : {:?}", client.get_user(&user_query).await);

    println!(
        "  get_department          : {:?}",
        client.get_department("od_dept123").await
    );

    let doc = Document::new_docx("项目计划".to_string(), None).expect("valid");
    println!("  create_doc              : {:?}", client.create_doc(&doc).await);

    let sheet = Document::new_sheet("预算表".to_string(), None).expect("valid");
    println!("  create_sheet            : {:?}", client.create_sheet(&sheet).await);

    println!(
        "  get_approval_instance   : {:?}",
        client.get_approval_instance("instance_001").await
    );

    let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
        .expect("valid");
    let event = apeireth_sdk_lark::LarkWebhookEvent::new_url_verification("challenge_xxx".to_string());
    println!(
        "  verify_webhook (client) : {:?}",
        client.verify_webhook(&event, &wh_token).await
    );
    // 但 free function `lark_verify_webhook` 已经能跑 (token 校验 + challenge 提取)
    let free_result = lark_verify_webhook(&event, &wh_token);
    println!("  verify_webhook (free)   : {:?}", free_result);
    println!();

    // 8) 4 实体构造演示
    println!("[§8 4 实体构造演示]");
    let start = Utc::now();
    let end = start + chrono::Duration::hours(1);
    let event_entity = CalendarEvent::new("cal_xxx", "团队周会", start, end).expect("valid");
    println!(
        "  CalendarEvent: {} ({:?})",
        event_entity.summary, event_entity.status
    );
    let _ = EventStatus::Confirmed;

    let user = apeireth_sdk_lark::User::new(
        "ou_user1234567890abcdef".to_string(),
        "Alice".to_string(),
    )
    .expect("valid");
    println!("  User: {} ({})", user.name, user.open_id);

    let dept = Department::new("od_dept123".to_string(), "工程部".to_string()).expect("valid");
    println!("  Department: {} ({})", dept.name, dept.open_department_id);

    let approval_inst = ApprovalInstance::new(
        "approval_code_xxx".to_string(),
        "ou_user1234567890abcdef".to_string(),
    )
    .expect("valid")
    .with_form_field(
        "reason".to_string(),
        "textarea".to_string(),
        "出差".to_string(),
    );
    println!(
        "  ApprovalInstance: code={}, status={:?}, form fields={}",
        approval_inst.approval_code,
        approval_inst.status,
        approval_inst.form.len()
    );
    let _ = InstanceStatus::Pending;
    println!();

    // 9) 6 消息类型构造演示
    println!("[§9 6 消息类型构造演示]");
    let _ = Message::text(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "Hello".to_string(),
    )
    .expect("valid");
    let _ = Message::image(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "img_v2_abc".to_string(),
    )
    .expect("valid");
    let _ = Message::file(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        "file_v2_abc".to_string(),
    )
    .expect("valid");
    let _ = Message::card(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        CardContent::plain("标题", "正文"),
    )
    .expect("valid");
    let _ = Message::interactive(
        "oc_a1b2c3d4e5f6".to_string(),
        ReceiveIdType::ChatId,
        CardContent::plain("标题", "正文"),
    )
    .expect("valid");
    for mt in SUPPORTED_MESSAGE_TYPES {
        println!("  MessageType::{:?} 构造演示: OK", mt);
    }
    let _ = MessageType::Text;
    let mut _h: HashMap<String, String> = HashMap::new();
    _h.insert("k".to_string(), "v".to_string());
    println!();

    // 10) stub_status
    println!("[§10 stub_status (R21 续真接后删)]");
    let status = client.stub_status();
    println!("  stub_mode              : {}", status.stub_mode);
    println!("  platform               : {}", status.platform);
    println!("  api_base               : {}", status.api_base);
    println!("  schema_version         : {}", status.schema_version);
    println!("  app_id_set             : {}", status.app_id_set);
    println!("  app_secret_set         : {}", status.app_secret_set);
    println!("  tenant_token_set       : {}", status.tenant_token_set);
    println!("  tenant_token_expired   : {}", status.tenant_token_expired);
    println!("  configured             : {}", status.configured);
    println!();

    // 11) User access token 演示
    println!("[§11 UserAccessToken 演示]");
    let _ = UserAccessToken::new(
        "cli_a1b2c3d4e5f6".to_string(),
        "u-abc".to_string(),
        "ur-xyz".to_string(),
        "ou_user1234567890abcdef".to_string(),
        7200,
    )
    .expect("valid user token");
    println!("  user_access_token: 构造 OK, 不返 NotImplemented");
    println!();

    println!("=== demo 完 (R21 续真接: 整合 #2 sub-agent 1 commit 落地, 改 STUB_MODE=false + 接 @larksuiteoapi/lark-sdk) ===");
}
