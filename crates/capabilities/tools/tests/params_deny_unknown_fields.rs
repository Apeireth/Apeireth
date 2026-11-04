//! 工具参数面「未知字段拒绝即帧」集成测试。
//!
//! 背景: 内建工具的参数结构 (shell / fetch / content_feed) 此前**静默忽略**
//! 调用方多传的字段 —— 调用方以为自己传的值生效了, 参数面与对外声明的 schema
//! 可能悄悄漂移。本测试钉死三条 params 结构的 serde 口径:
//!
//! 1. 合法参数 (含全部字段) ⇒ 解析成功, 且取值逐字段落地;
//! 2. 含未知字段 ⇒ 解析**失败**, 且错误信息里**出现该未知字段名**
//!    (拒绝即帧, 不是静默忽略);
//! 3. 嵌套结构: 这三条 params 的字段**全部是标量** (String / Option<String> /
//!    Option<u64>), 仓内**没有 map / 对象型字段** —— 所以「嵌套 map 里塞未知
//!    字段」这一形态在本仓**不存在**。调用方唯一能塞进来的嵌套形态是
//!    「已知字段的位置放一个对象」, 该形态按类型错配拒绝 (实测: serde 报
//!    `invalid type: map, expected a string`), 同样不是静默接受。
//!
//! 另附工具面 (调用 → 参数解析 → 冻结) 的端到端断言: 多传字段在调用边界
//! 就得拒绝帧, 且帧里带该字段名。
//!
//! 口径说明: `tool.fetch` 的工具面另有一道**既有**的人工键位闸 (逐键点名
//! `unknown fetch parameter "..."`), 它先于 serde 命中, 因此 fetch 的工具面
//! 文案与本批前一致; serde 闸是结构级的第二道防线 (直接反序列化
//! `FetchParams` 时生效)。

use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::content_feed::tool::ContentFeedParams;
use apeireth_tools_canonical::fetch::FetchParams;
use apeireth_tools_canonical::shell::ShellParams;
use apeireth_tools_canonical::{
    ContentFeedConfig, ContentFeedTool, ControlledEgress, EgressPolicy, FetchConfig, FetchTool,
    ShellTool, TrustedShellConfig,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// shell 参数面
// ---------------------------------------------------------------------------

#[test]
fn shell_params_accept_all_declared_fields() {
    let parsed = serde_json::from_value::<ShellParams>(json!({
        "command": "echo hi",
        "cwd": "sub/dir",
        "timeout_ms": 1500,
    }))
    .expect("全部已声明字段必须解析成功");

    assert_eq!(parsed.command, "echo hi");
    assert_eq!(parsed.cwd.as_deref(), Some("sub/dir"));
    assert_eq!(parsed.timeout_ms, Some(1500));
}

#[test]
fn shell_params_accept_minimal_call() {
    let parsed = serde_json::from_value::<ShellParams>(json!({ "command": "echo hi" }))
        .expect("仅必填字段必须解析成功");

    assert_eq!(parsed.command, "echo hi");
    assert!(parsed.cwd.is_none());
    assert!(parsed.timeout_ms.is_none());
}

#[test]
fn shell_params_reject_unknown_field() {
    let error = serde_json::from_value::<ShellParams>(json!({
        "command": "echo hi",
        "timeout_ms": 1500,
        "unexpected_field": 1,
    }))
    .expect_err("未知字段必须拒绝 (不得静默忽略)");

    let text = error.to_string();
    assert!(
        text.contains("unexpected_field"),
        "错误信息必须点名未知字段: {text}"
    );
}

#[test]
fn shell_params_reject_nested_object_where_scalar_expected() {
    // 本 params 结构无 map / 对象型字段 (全部标量), 故「嵌套 map 里的未知
    // 字段」这一形态不存在; 唯一能塞进来的嵌套形态 = 已知字段位置放对象 ⇒
    // 类型错配拒绝 (实测 `invalid type: map, expected a string`)。
    let error = serde_json::from_value::<ShellParams>(json!({
        "command": "echo hi",
        "cwd": { "nested": 1 },
    }))
    .expect_err("标量字段位置放对象必须拒绝");

    let text = error.to_string();
    assert!(
        text.contains("invalid type") && text.contains("map"),
        "错误信息必须说明类型错配: {text}"
    );
}

#[test]
fn shell_tool_rejects_unknown_parameter_at_call_boundary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
    let call = ToolCall {
        id: "call_shell_unknown".into(),
        name: "shell".into(),
        arguments: json!({ "command": "echo hi", "unexpected_field": 1 }),
    };

    let error = tool
        .freeze_invocation(&call)
        .expect_err("多传字段必须在调用边界拒绝")
        .render();

    assert!(
        error.contains("unexpected_field"),
        "拒绝帧必须点名未知字段: {error}"
    );
}

// ---------------------------------------------------------------------------
// fetch 参数面
// ---------------------------------------------------------------------------

#[test]
fn fetch_params_accept_all_declared_fields() {
    let parsed = serde_json::from_value::<FetchParams>(json!({
        "url": "https://example.com/feed.xml",
    }))
    .expect("全部已声明字段必须解析成功");

    assert_eq!(parsed.url, "https://example.com/feed.xml");
}

#[test]
fn fetch_params_reject_unknown_field() {
    let error = serde_json::from_value::<FetchParams>(json!({
        "url": "https://example.com/",
        "unexpected_field": 1,
    }))
    .expect_err("未知字段必须拒绝 (不得静默忽略)");

    let text = error.to_string();
    assert!(
        text.contains("unexpected_field"),
        "错误信息必须点名未知字段: {text}"
    );
}

#[test]
fn fetch_params_reject_nested_object_where_scalar_expected() {
    // 同 shell: 本结构无 map / 对象型字段, 唯一嵌套形态 = 对象占位 ⇒ 拒绝。
    let error = serde_json::from_value::<FetchParams>(json!({
        "url": { "nested": 1 },
    }))
    .expect_err("标量字段位置放对象必须拒绝");

    let text = error.to_string();
    assert!(
        text.contains("invalid type") && text.contains("map"),
        "错误信息必须说明类型错配: {text}"
    );
}

#[test]
fn fetch_tool_rejects_unknown_parameter_at_call_boundary() {
    let tool = FetchTool::new(FetchConfig::new(Arc::new(ControlledEgress::new(
        EgressPolicy::DenyAll,
    ))));
    let call = ToolCall {
        id: "call_fetch_unknown".into(),
        name: "fetch".into(),
        arguments: json!({ "url": "https://example.com/", "unexpected_field": 1 }),
    };

    let error = tool
        .freeze_invocation(&call)
        .expect_err("多传字段必须在调用边界拒绝")
        .render();

    assert!(
        error.contains("unexpected_field"),
        "拒绝帧必须点名未知字段: {error}"
    );
}

// ---------------------------------------------------------------------------
// content_feed 参数面
// ---------------------------------------------------------------------------

#[test]
fn content_feed_params_accept_all_declared_fields() {
    let parsed = serde_json::from_value::<ContentFeedParams>(json!({
        "action": "feed_add",
        "id": "news",
        "url": "https://example.com/rss",
        "kind": "rss",
    }))
    .expect("全部已声明字段必须解析成功");

    assert_eq!(parsed.action, "feed_add");
    assert_eq!(parsed.id.as_deref(), Some("news"));
    assert_eq!(parsed.url.as_deref(), Some("https://example.com/rss"));
    assert_eq!(parsed.kind.as_deref(), Some("rss"));
}

#[test]
fn content_feed_params_accept_minimal_call() {
    let parsed = serde_json::from_value::<ContentFeedParams>(json!({ "action": "feed_list" }))
        .expect("仅必填字段必须解析成功");

    assert_eq!(parsed.action, "feed_list");
    assert!(parsed.id.is_none());
    assert!(parsed.url.is_none());
    assert!(parsed.kind.is_none());
}

#[test]
fn content_feed_params_reject_unknown_field() {
    let error = serde_json::from_value::<ContentFeedParams>(json!({
        "action": "feed_add",
        "id": "news",
        "url": "https://example.com/rss",
        "kind": "rss",
        "unexpected_field": 1,
    }))
    .expect_err("未知字段必须拒绝 (不得静默忽略)");

    let text = error.to_string();
    assert!(
        text.contains("unexpected_field"),
        "错误信息必须点名未知字段: {text}"
    );
}

#[test]
fn content_feed_params_reject_nested_object_where_scalar_expected() {
    // 同 shell: 本结构无 map / 对象型字段, 唯一嵌套形态 = 对象占位 ⇒ 拒绝。
    let error = serde_json::from_value::<ContentFeedParams>(json!({
        "action": "feed_add",
        "id": { "nested": 1 },
    }))
    .expect_err("标量字段位置放对象必须拒绝");

    let text = error.to_string();
    assert!(
        text.contains("invalid type") && text.contains("map"),
        "错误信息必须说明类型错配: {text}"
    );
}

#[test]
fn content_feed_tool_rejects_unknown_parameter_at_call_boundary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let writer: Arc<dyn Fn(&Episode) -> Result<(), String> + Send + Sync> =
        Arc::new(|_episode: &Episode| Ok(()));
    let tool = ContentFeedTool::new(ContentFeedConfig::new(
        dir.path(),
        Arc::new(ControlledEgress::new(EgressPolicy::DenyAll)),
        writer,
    ));
    let call = ToolCall {
        id: "call_feed_unknown".into(),
        name: "content_feed".into(),
        arguments: json!({ "action": "feed_list", "unexpected_field": 1 }),
    };

    let error = tool
        .freeze_invocation(&call)
        .expect_err("多传字段必须在调用边界拒绝")
        .render();

    assert!(
        error.contains("unexpected_field"),
        "拒绝帧必须点名未知字段: {error}"
    );
}
