//! Lark SDK Stub Demo (1:1 翻译 v0.9.21 商业版 `@larksuiteoapi/node-sdk@1.59` 集成流程).
//!
//! 演示 8 stub 工具全部返 `LarkError::NotImplemented(api_name)`, 强调 STUB MODE 守门.
//! 真实集成 (调飞书 Open API 5 端点) 留 **R20 阶段 3** 实施.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-lark --example lark_stub_demo
//! ```
//!
//! ## 期望输出 (STUB 阶段)
//!
//! ```text
//! [lark_stub_demo] STUB_MODE = true (per 编译期守门)
//! [lark_stub_demo] TOOL_WHITELIST 9 项: send_message, list_calendars, ...
//! [lark_stub_demo] send_message -> Err(NotImplemented("send_message"))
//! [lark_stub_demo] list_calendars -> Err(NotImplemented("list_calendars"))
//! [lark_stub_demo] create_event -> Err(NotImplemented("create_event"))
//! [lark_stub_demo] get_document -> Err(NotImplemented("get_document"))
//! [lark_stub_demo] search_documents -> Err(NotImplemented("search_documents"))
//! [lark_stub_demo] list_bitable_records -> Err(NotImplemented("list_bitable_records"))
//! [lark_stub_demo] create_bitable_record -> Err(NotImplemented("create_bitable_record"))
//! [lark_stub_demo] auth_refresh -> Err(NotImplemented("auth_refresh"))
//! [lark_stub_demo] completed (STUB MODE — R20 阶段 3 真接飞书 SDK 待补)
//! ```
//!
//! ## 6 哲学 anchor 验证 (per 主人 19:37 "全用 rust" 强调)
//!
//! - S-1 北极星导向: 1:1 翻译 v0.9.21 商业版飞书 SDK 集成面 (5 端点: 消息/日历/文档/Bitable/Auth)
//! - S-2 实事求是: 用 v0.9.21 `package.json` line 23 `@larksuiteoapi/node-sdk@^1.59.0` 实查
//!   5 端点 URL + 5 MessageType 枚举实证, 不假装 1:1
//! - O-2 走在前人肩上: 编译期 hardcode 5 MessageType + 9 TOOL_WHITELIST, 借鉴 5 P0 crate 同模式
//! - O-5 不假装: 8 stub 工具明确返 NotImplemented, STUB_MODE = true 编译期守门, 真实实现留 R20 阶段 3
//! - O-3 干到底: STUB skeleton 落地, R20 阶段 3 真接 reqwest 或 lark-rs 社区 crate
//! - O-4 任何人都能接手: 6 § 结构 + 10 LarkError + 8 工具 + 5 fixture 跟主草稿 1:1

use apeireth_lark::{
    is_stub_mode, validate_tool_call, LarkClient, LarkClientImpl, LarkConfig, LarkError, MessageType,
    TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) 编译期守门: STUB_MODE 必须 == true
    println!("[lark_stub_demo] STUB_MODE = {} (per 编译期守门)", is_stub_mode());
    assert!(is_stub_mode(), "STUB_MODE 必须是 true");

    // 2) TOOL_WHITELIST 9 项展示 (m3 防御, 8 工具 + 1 stub_status)
    println!(
        "[lark_stub_demo] TOOL_WHITELIST {} 项: {}",
        TOOL_WHITELIST_COUNT,
        TOOL_WHITELIST.join(", ")
    );

    // 3) m3 防御: validate_tool_call 接受白名单内
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        validate_tool_call(tool, &args)?;
    }

    // 4) 创建 LarkClient (STUB 模式, 0 网络调用)
    let client = LarkClientImpl::new(LarkConfig::default())?;
    println!(
        "[lark_stub_demo] LarkClientImpl 创建成功: app_id={}",
        client.config().app_id
    );

    // 5) 演示 8 stub 工具返 NotImplemented (逐个手写, 不放 loop 避免 async 闭包)

    // 消息 (1)
    let r = client.send_message("user-1", MessageType::Text, "hello lark").await;
    println!("[lark_stub_demo] send_message -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    // 日历 (2)
    let r = client.list_calendars().await;
    println!("[lark_stub_demo] list_calendars -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    let r = client.create_event("cal-primary", "team meeting", 0, 0).await;
    println!("[lark_stub_demo] create_event -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    // 文档 (2)
    let r = client.get_document("doc-abc").await;
    println!("[lark_stub_demo] get_document -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    let r = client.search_documents("apeireth", 10).await;
    println!("[lark_stub_demo] search_documents -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    // Bitable (2)
    let r = client.list_bitable_records("app-bitable", "tbl-tasks", 50).await;
    println!("[lark_stub_demo] list_bitable_records -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    let r = client
        .create_bitable_record(
            "app-bitable",
            "tbl-tasks",
            serde_json::json!({"title": "R20 阶段 3", "status": "todo"}),
        )
        .await;
    println!("[lark_stub_demo] create_bitable_record -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    // Auth (1)
    let r = client.auth_refresh().await;
    println!("[lark_stub_demo] auth_refresh -> {:?}", r);
    assert!(matches!(r, Err(LarkError::NotImplemented(_))));

    // 6) m3 防御演示: 白名单外工具拒绝
    let bad = validate_tool_call("apeireth_lark_send_email", &args);
    println!(
        "[lark_stub_demo] m3 防御: 白名单外工具 -> {:?}",
        bad.as_ref().err()
    );
    assert!(bad.is_err());

    println!("[lark_stub_demo] completed (STUB MODE — R20 阶段 3 真接飞书 SDK 待补)");
    Ok(())
}
