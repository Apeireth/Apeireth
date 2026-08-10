//! apeireth-sdk smoke tests (≥ 5)
//!
//! 覆盖 docs/v2-strategy/03 §0.2 "apeireth-sdk" 验收点:
//! - 多语言 SDK 统一测试入口
//! - WireFormat 反序列化真跑
//! - 版本协商真跑
//! - 错误码双向真跑

use apeireth_sdk::{negotiate, Envelope, SdkError, SdkErrorCode, SdkVersion, WireCompat, WireKind};

/// ① SdkVersion 字符串解析.
#[test]
fn smoke_version_parse_and_stringify() {
    let v = SdkVersion::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.as_str(), "1.2.3");

    // 非法: 段数错
    assert!(SdkVersion::parse("1.2").is_none());
    // 非法: 非数字
    assert!(SdkVersion::parse("a.b.c").is_none());
}

/// ② 版本协商: 跨 major 一律不兼容.
#[test]
fn smoke_version_negotiate_incompatible_across_major() {
    let v1 = SdkVersion::new(1, 0, 0);
    let v2 = SdkVersion::new(2, 0, 0);
    assert_eq!(negotiate(v1, v2), WireCompat::Incompatible);
}

/// ③ 版本协商: 同 major 互转 (server newer / older / exact).
#[test]
fn smoke_version_negotiate_within_major() {
    let older = SdkVersion::new(1, 0, 0);
    let newer = SdkVersion::new(1, 2, 3);
    let exact = SdkVersion::new(1, 0, 0);

    assert_eq!(negotiate(older, newer), WireCompat::ServerNewer);
    assert_eq!(negotiate(newer, older), WireCompat::ServerOlder);
    assert_eq!(negotiate(older, exact), WireCompat::Exact);
}

/// ④ Envelope 序列化往返 (JSON wire-format).
#[test]
fn smoke_envelope_serde_roundtrip() {
    let env = Envelope::new(
        WireKind::Chat,
        "req-001",
        serde_json::json!({"prompt": "hi"}),
    );
    let line = env.encode().unwrap();
    assert!(line.contains("\"kind\":\"chat\""));
    assert!(line.contains("\"id\":\"req-001\""));
    assert!(line.contains("\"prompt\":\"hi\""));

    let back = Envelope::decode(&line).unwrap();
    assert_eq!(back.kind, WireKind::Chat);
    assert_eq!(back.id, "req-001");
    assert_eq!(back.body["prompt"], "hi");
}

/// ⑤ WireKind 自定义 (Other) 跨语言兼容: snake_case 字符串.
#[test]
fn smoke_wire_kind_other_roundtrip() {
    let env = Envelope::new(
        WireKind::Other("custom_event".to_string()),
        "x",
        serde_json::json!({}),
    );
    let line = env.encode().unwrap();
    // snake_case 字符串 Other("custom_event") 在 serde_json 序列化为:
    //   {"other": "custom_event"}  (为 untagged? 实际是：variant 名 + value)
    // 验证至少能 decode 回等价结构:
    let back = Envelope::decode(&line).unwrap();
    match back.kind {
        WireKind::Other(s) => assert_eq!(s, "custom_event"),
        _ => panic!("expected Other variant"),
    }
}

/// ⑥ 错误码数字 + 字符串化双向.
#[test]
fn smoke_error_code_numeric_and_names() {
    assert_eq!(SdkErrorCode::Unknown.numeric_code(), 1000);
    assert_eq!(SdkErrorCode::InvalidEnvelope.numeric_code(), 2001);
    assert_eq!(SdkErrorCode::VersionIncompatible.numeric_code(), 2002);
    assert_eq!(SdkErrorCode::NotFound.numeric_code(), 3001);
    assert_eq!(SdkErrorCode::PermissionDenied.numeric_code(), 4001);
    assert_eq!(SdkErrorCode::ToolNotApproved.numeric_code(), 4002);
    assert_eq!(SdkErrorCode::Internal.numeric_code(), 5001);

    // snake_case ↔ camelCase 双向往返
    assert_eq!(
        SdkErrorCode::InvalidEnvelope.snake_name(),
        "invalid_envelope"
    );
    assert_eq!(
        SdkErrorCode::InvalidEnvelope.camel_name(),
        "invalidEnvelope"
    );
    assert_eq!(
        SdkErrorCode::ToolNotApproved.snake_name(),
        "tool_not_approved"
    );
    assert_eq!(
        SdkErrorCode::ToolNotApproved.camel_name(),
        "toolNotApproved"
    );
    assert_eq!(SdkErrorCode::Unknown.snake_name(), "unknown");
    assert_eq!(SdkErrorCode::Unknown.camel_name(), "unknown");
}

/// ⑦ Envelope 版本字段透传.
#[test]
fn smoke_envelope_version_field() {
    let env = Envelope::new(WireKind::Health, "p", serde_json::json!({}));
    let v = env.expected_version().unwrap();
    assert_eq!(v.major, apeireth_sdk::SDK_VERSION.major);
    assert_eq!(v.minor, apeireth_sdk::SDK_VERSION.minor);
    assert_eq!(v.patch, apeireth_sdk::SDK_VERSION.patch);
}

/// ⑧ SdkError 业务错误构造 (含 non-trivial message).
#[test]
fn smoke_sdk_error_business_construct() {
    let err = SdkError::business(SdkErrorCode::NotFound, "vector id 42 not found");
    let msg = format!("{err}");
    assert!(msg.contains("not_found") || msg.contains("NotFound"));
    assert!(msg.contains("vector id 42 not found"));
}
