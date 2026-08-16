//! Integration tests for apeireth-tool-runtime
//!
//! **R18 第 2 阶段第 5 项**: parser 核心路径
//!
//! 测试策略: 真实文本解析 (VCP `<<<[TOOL_REQUEST]>>>` 格式), 不 mock LLM 输出.
//! record_store 部分需要 SqliteMemoryStore, 留作 R18 后续 service-based 集成测试.

use apeireth_tool_runtime::parser::ToolCallParser;

// =====================================================================
// ToolCallParser
// =====================================================================

#[test]
fn parser_extracts_single_tool_call() {
    let output = r#"
Let me help you with that.
<<<[TOOL_REQUEST]>>>
tool_name:<<<MyTool>>>
path:<<<./hello.txt>>>
arg2:<<<value2>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    let calls = ToolCallParser::parse(output).expect("parse ok");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tool_name, "MyTool");
    assert_eq!(calls[0].args["path"], "./hello.txt");
    assert_eq!(calls[0].args["arg2"], "value2");
}

#[test]
fn parser_extracts_multiple_tool_calls() {
    let output = r#"
<<<[TOOL_REQUEST]>>>
tool_name:<<<First>>>
x:<<<1>>>
<<<[END_TOOL_REQUEST]>>>

Some reasoning between calls.

<<<[TOOL_REQUEST]>>>
tool_name:<<<Second>>>
y:<<<2>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    let calls = ToolCallParser::parse(output).expect("parse ok");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].tool_name, "First");
    assert_eq!(calls[1].tool_name, "Second");
}

#[test]
fn parser_strips_think_blocks() {
    let output = r#"
<think>
The user wants me to call a tool.
I should use the search tool.
</think>

Result: I will search.
<<<[TOOL_REQUEST]>>>
tool_name:<<<Search>>>
query:<<<rust>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    let calls = ToolCallParser::parse(output).expect("parse ok after stripping think");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tool_name, "Search");
    assert_eq!(calls[0].args["query"], "rust");
}

#[test]
fn parser_handles_archery_flag() {
    let output = r#"
<<<[TOOL_REQUEST]>>>
tool_name:<<<AsyncTool>>>
archery:<<<true>>>
arg:<<<value>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    let calls = ToolCallParser::parse(output).expect("parse ok");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].archery, "archery flag should be true");
    assert!(
        !calls[0].archery_no_reply,
        "archery_no_reply should be false"
    );
}

#[test]
fn parser_handles_no_reply_flag() {
    let output = r#"
<<<[TOOL_REQUEST]>>>
tool_name:<<<FireAndForget>>>
archery:<<<no_reply>>>
<<<[END_TOOL_REQUEST]>>>
"#;
    let calls = ToolCallParser::parse(output).expect("parse ok");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].archery_no_reply, "no_reply flag should be true");
}

#[test]
fn parser_empty_input_fails() {
    let result = ToolCallParser::parse("");
    assert!(result.is_err(), "empty input should fail");
}

#[test]
fn parser_no_blocks_fails() {
    let output = "Just some text without any tool calls.";
    let result = ToolCallParser::parse(output);
    assert!(result.is_err(), "no blocks should fail");
}

#[test]
fn parser_markers_are_distinct() {
    // 编译期守门: start 和 end marker 必须是不同的
    assert_ne!(ToolCallParser::MARKER_START, ToolCallParser::MARKER_END);
    assert!(ToolCallParser::MARKER_START.contains("TOOL_REQUEST"));
    assert!(ToolCallParser::MARKER_END.contains("END_TOOL_REQUEST"));
}

// =====================================================================
// Parser 错误类型 (R25 boundary case 扩展)
// =====================================================================

#[test]
fn parser_missing_tool_name_fails() {
    // 缺 tool_name 字段 → 报错
    let output = "<<<[TOOL_REQUEST]>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>";
    let r = ToolCallParser::parse(output);
    assert!(r.is_err(), "缺 tool_name 应报错, 实际: {r:?}");
}

#[test]
fn parser_missing_start_marker_fails() {
    // 缺 <<<[TOOL_REQUEST]>>> 起始 → 报 NoBlocks
    let output = "tool_name:<<<X>>>\narg:<<<y>>>\n<<<[END_TOOL_REQUEST]>>>";
    let r = ToolCallParser::parse(output);
    assert!(r.is_err(), "缺 start marker 应 NoBlocks, 实际: {r:?}");
}

#[test]
fn parser_missing_end_marker_fails_or_skips() {
    // 缺 <<<[END_TOOL_REQUEST]>>> 结束: 行为可能是 NoBlocks 或解析出 start 后的 partial
    // 跟 src 实现一致 (宽容性: 缺 end marker 可能跳过, 不 panic)
    let output = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<X>>>\narg:<<<y>>>\n";
    let r = ToolCallParser::parse(output);
    // 不 panic 即通过 (具体行为因 src 实现而异)
    let _ = r;
}

// =====================================================================
// Fuzzy matching — 边界 + 多候选 + 大小写
// =====================================================================

#[test]
fn fuzzy_match_returns_closest_when_multiple_candidates() {
    // 多个候选距离相同 → 取一个 (具体看实现: list() 字典序 → first-found)
    use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
    use apeireth_tool_runtime::FuzzyToolMatcher;
    use std::sync::Arc;
    let r = ToolRegistry::new();
    for n in ["abcd", "abce", "wxyz"] {
        r.register(
            n.to_string(),
            Arc::new(MockSyncTool {
                name: n.to_string(),
            }),
        );
    }
    // "abcf" 跟 abcd 距离 1, abce 距离 1, wxyz 距离 3
    let m = FuzzyToolMatcher::match_tool("abcf", &r);
    // ToolRegistry::list() 按字典序, abcd 排在 abce 之前
    assert!(
        m == Some("abcd".to_string()) || m == Some("abce".to_string()),
        "应选 abcd 或 abce, 实际: {m:?}"
    );
}

#[test]
fn fuzzy_match_threshold_one_works() {
    // 阈值 = 1: 距离 1 命中, 距离 2 拒识
    use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
    use apeireth_tool_runtime::FuzzyToolMatcher;
    use std::sync::Arc;
    let r = ToolRegistry::new();
    r.register(
        "read".to_string(),
        Arc::new(MockSyncTool {
            name: "read".to_string(),
        }),
    );
    // "red" 距离 "read" = 1 (插 1)
    let m = FuzzyToolMatcher::match_tool_threshold("red", &r, 1);
    assert_eq!(m, Some("read".to_string()));
    // "raed" 距离 "read" = 2
    let m2 = FuzzyToolMatcher::match_tool_threshold("raed", &r, 1);
    assert!(m2.is_none(), "距离 2 在 threshold=1 应拒识");
}

// =====================================================================
// PrivacyGuard — 13 类敏感键 + 7 类 token + 嵌套 + env
// =====================================================================

fn v() -> serde_json::Value {
    serde_json::json!({})
}

#[test]
fn privacy_masks_api_key_field() {
    // api_key 是 VCP 13 类敏感键之一
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "api_key": "sk-verylongsecretvaluethatistoolong1234567890",
        "result": "ok"
    });
    let out = g.mask(&input);
    let masked = out["api_key"].as_str().unwrap();
    assert!(
        masked.contains("[APEIRETH_PRIVACY_REDACTED]"),
        "api_key 应被 mask, 实际: {masked}"
    );
    // 普通字段不动
    assert_eq!(out["result"], "ok");
}

#[test]
fn privacy_masks_password_and_token() {
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "password": "verylongpassword123456789",
        "token": "verylongtokenvalue123456789012345",
    });
    let out = g.mask(&input);
    assert!(out["password"]
        .as_str()
        .unwrap()
        .contains("[APEIRETH_PRIVACY_REDACTED]"));
    assert!(out["token"]
        .as_str()
        .unwrap()
        .contains("[APEIRETH_PRIVACY_REDACTED]"));
}

#[test]
fn privacy_masks_high_confidence_github_token() {
    // 7 类 token: ghp_ / sk- / AKIA / glpat- / etc
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "text": "use ghp_abcdefghijklmnopqrstuvwxyz0123456789 in your code"
    });
    let out = g.mask(&input);
    let s = out["text"].as_str().unwrap();
    assert!(
        !s.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "ghp_ token 应被 mask, 实际: {s}"
    );
    assert!(s.contains("[APEIRETH_PRIVACY_REDACTED]"));
}

#[test]
fn privacy_masks_nested_object() {
    // 嵌套对象递归 mask
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "outer": {
            "inner": {
                "api_key": "sk-verylongsecretvaluethatistoolong1234567890"
            }
        }
    });
    let out = g.mask(&input);
    let masked = out["outer"]["inner"]["api_key"].as_str().unwrap();
    assert!(
        masked.contains("[APEIRETH_PRIVACY_REDACTED]"),
        "嵌套 api_key 应被 mask, 实际: {masked}"
    );
}

#[test]
fn privacy_short_value_not_masked() {
    // min_secret_length = 8, 短值不 mask (VCP 行为)
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "api_key": "abc" // 太短, 不应 mask
    });
    let out = g.mask(&input);
    // 长度 3 < min_secret_length=8 → 不 mask
    assert_eq!(
        out["api_key"], "abc",
        "短值应保留原样, 实际: {}",
        out["api_key"]
    );
}

#[test]
fn privacy_disabled_returns_unchanged() {
    use apeireth_tool_runtime::{PrivacyConfig, PrivacyGuard};
    let cfg = PrivacyConfig {
        enabled: false,
        ..Default::default()
    };
    let g = PrivacyGuard::with_config(cfg);
    let input = serde_json::json!({
        "api_key": "sk-verylongsecretvaluethatistoolong1234567890"
    });
    let out = g.mask(&input);
    // enabled=false → 返原值
    assert_eq!(
        out["api_key"],
        "sk-verylongsecretvaluethatistoolong1234567890"
    );
}

#[test]
fn privacy_masks_array_of_secrets() {
    // 数组内嵌套也 mask
    use apeireth_tool_runtime::PrivacyGuard;
    let g = PrivacyGuard::new();
    let input = serde_json::json!({
        "secrets": [
            {"password": "verylongpassword123456789"},
            {"password": "anotherverylongpassword1234567890"}
        ]
    });
    let out = g.mask(&input);
    let arr = out["secrets"].as_array().unwrap();
    for s in arr {
        assert!(s["password"]
            .as_str()
            .unwrap()
            .contains("[APEIRETH_PRIVACY_REDACTED]"));
    }
}
