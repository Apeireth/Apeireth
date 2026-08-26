//! Integration tests for apeireth-tool-registry
//!
//! **R18 第 2 阶段第 4 项**: 7 测试覆盖 registry CRUD + mock Tool impl

use apeireth_tool_registry::{
    registry::ToolRegistry,
    trait_def::Tool,
    types::{
        AwaitingAxis, OutputAxis, ResidentAxis, ToolAxes, ToolKind, TransportAxis, TriggerAxis,
    },
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

// =====================================================================
// Mock Tool — 1 个 async + Send + Sync, 供测试用
// =====================================================================

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        Ok(args) // echo back
    }
}

struct FailingTool;

#[async_trait]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }
    async fn call(&self, _args: Value) -> Result<Value, String> {
        Err("intentional failure".to_string())
    }
}

// =====================================================================
// registry CRUD
// =====================================================================

#[test]
fn registry_register_and_get() {
    let r = ToolRegistry::new();
    r.register("echo".to_string(), Arc::new(EchoTool));
    let tool = r.get("echo").expect("should be registered");
    assert_eq!(tool.name(), "echo");
}

#[test]
fn registry_get_nonexistent_returns_none() {
    let r = ToolRegistry::new();
    assert!(r.get("nope").is_none());
}

#[test]
fn registry_unregister_removes_tool() {
    let r = ToolRegistry::new();
    r.register("echo".to_string(), Arc::new(EchoTool));
    assert!(r.get("echo").is_some());
    let removed = r.unregister("echo");
    assert!(removed.is_some());
    assert!(r.get("echo").is_none());
}

#[test]
fn registry_list_sorted() {
    let r = ToolRegistry::new();
    r.register("zebra".to_string(), Arc::new(EchoTool));
    r.register("alpha".to_string(), Arc::new(FailingTool));
    r.register("mike".to_string(), Arc::new(EchoTool));
    let names = r.list();
    assert_eq!(names, vec!["alpha", "mike", "zebra"]); // sorted
}

#[test]
fn registry_len_and_is_empty() {
    let r = ToolRegistry::new();
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    r.register("echo".to_string(), Arc::new(EchoTool));
    assert!(!r.is_empty());
    assert_eq!(r.len(), 1);
}

#[test]
fn registry_clear_empties_all() {
    let r = ToolRegistry::new();
    r.register("echo".to_string(), Arc::new(EchoTool));
    r.register("failing".to_string(), Arc::new(FailingTool));
    assert_eq!(r.len(), 2);
    r.clear();
    assert!(r.is_empty());
}

#[test]
fn registry_overwrite_same_name() {
    let r = ToolRegistry::new();
    r.register("tool".to_string(), Arc::new(EchoTool));
    r.register("tool".to_string(), Arc::new(FailingTool)); // overwrite
                                                           // last one wins
    let t = r.get("tool").unwrap();
    assert_eq!(t.name(), "failing");
}

#[tokio::test]
async fn registry_call_through_get() {
    let r = ToolRegistry::new();
    r.register("echo".to_string(), Arc::new(EchoTool));
    let tool = r.get("echo").unwrap();
    let result = tool.call(json!({"msg": "hi"})).await.unwrap();
    assert_eq!(result, json!({"msg": "hi"}));
}

#[tokio::test]
async fn registry_failing_tool_returns_err() {
    let r = ToolRegistry::new();
    r.register("failing".to_string(), Arc::new(FailingTool));
    let tool = r.get("failing").unwrap();
    let result = tool.call(json!({})).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "intentional failure");
}

#[test]
fn registry_list_by_kind() {
    let r = ToolRegistry::new();
    r.register("echo".to_string(), Arc::new(EchoTool));
    r.register("failing".to_string(), Arc::new(FailingTool));
    let by_kind = r.list_by_kind();
    // 2 tools, both ToolKind::Sync
    let sync_tools = by_kind.get(&ToolKind::Sync).unwrap();
    assert_eq!(sync_tools.len(), 2);
}

// =====================================================================
// token_budget (VCP §6.2.2 #15) — 估算 + 截断
// =====================================================================

#[test]
fn token_pieces_handles_latin_and_cjk() {
    use apeireth_tool_registry::token_budget::token_pieces;
    // 拉丁按词分 (VCP 启发式)
    let pieces = token_pieces("hello world foo");
    assert_eq!(pieces.len(), 3, "3 latin words: {pieces:?}");
    // CJK 每字 1 token (每个 char 独立 push, 因 multi-byte char 各自算 1 token)
    let cjk = token_pieces("你好世界");
    assert_eq!(cjk.len(), 4, "4 CJK chars: {cjk:?}");
    // 混合: 1 拉丁 word + 2 CJK chars (世/界 各自 1 token, 不合并)
    let mixed = token_pieces("hello 世界");
    assert_eq!(mixed.len(), 3, "1 latin + 2 CJK: {mixed:?}");
}

#[test]
fn estimate_token_count_basic() {
    use apeireth_tool_registry::token_budget::estimate_token_count;
    assert_eq!(estimate_token_count(""), 0);
    assert_eq!(estimate_token_count("hello"), 1);
    assert_eq!(estimate_token_count("hello world"), 2);
    // "hello 世界" → 1 latin + 2 CJK = 3
    assert_eq!(estimate_token_count("hello 世界"), 3);
}

#[test]
fn truncate_to_token_budget_short_input_unchanged() {
    use apeireth_tool_registry::token_budget::truncate_to_token_budget;
    let s = "hello world";
    let out = truncate_to_token_budget(s, 10);
    assert_eq!(out, s, "短输入 (< 10 tokens) 不应被截断");
}

#[test]
fn truncate_to_token_budget_long_input_truncated() {
    use apeireth_tool_registry::token_budget::truncate_to_token_budget;
    let s = "alpha beta gamma delta epsilon zeta eta theta"; // 8 tokens
    let out = truncate_to_token_budget(s, 3); // 限制 3 tokens
                                              // 截断应: 保留 2 + 1 marker = 3 tokens (per `truncate_to_token_budget` 实现)
    let pieces: Vec<&str> = out.split_whitespace().collect();
    assert!(
        pieces.len() <= 3,
        "应 ≤ 3 tokens, got {}: {out}",
        pieces.len()
    );
    assert!(out.contains("…"), "应含 ellipsis marker: {out}");
}

#[test]
fn truncate_to_token_budget_min_two_when_budget_zero() {
    // budget = 0 时 max(2) 保底, 不 panic
    use apeireth_tool_registry::token_budget::truncate_to_token_budget;
    let s = "x y z w v";
    let out = truncate_to_token_budget(s, 0);
    assert!(!out.is_empty(), "budget=0 也应返非空 (max(2) 保底)");
    assert!(out.len() < s.len(), "应被截断");
}

// =====================================================================
// ToolKind 6 类 + 5 轴 (VCP §6.2.1 #12 + #13)
// =====================================================================

#[test]
fn tool_kind_all_returns_six_vcp_strings() {
    // 6 类 as_legacy_str 应返 VCP 真值字符串 (字段级引用)
    let kinds = ToolKind::all();
    let strings: Vec<&str> = kinds.iter().map(|k| k.as_legacy_str()).collect();
    assert_eq!(strings.len(), 6);
    assert!(strings.contains(&"synchronous"));
    assert!(strings.contains(&"asynchronous"));
    assert!(strings.contains(&"static"));
    assert!(strings.contains(&"service"));
    assert!(strings.contains(&"messagePreprocessor"));
    assert!(strings.contains(&"hybridservice"));
}

#[test]
fn tool_kind_from_legacy_str_round_trip() {
    // 6 类往返解析: as_legacy_str → from_legacy_str → 同一 variant
    for k in ToolKind::all() {
        let s = k.as_legacy_str();
        let back = ToolKind::from_legacy_str(s);
        assert_eq!(back, Some(k), "{s} 应能 round-trip 回 {k:?}");
    }
    // 未知字符串返 None
    assert_eq!(ToolKind::from_legacy_str("nonsense"), None);
    assert_eq!(ToolKind::from_legacy_str(""), None);
}

// =====================================================================
// 6 类 mock 工具真调 (per registry.rs re-exports)
// =====================================================================

#[tokio::test]
async fn mock_sync_tool_call_returns_args() {
    // MockSyncTool::call 读 args.input, 返 {tool, kind, echo, result}
    use apeireth_tool_registry::MockSyncTool;
    let tool = MockSyncTool {
        name: "echo".to_string(),
    };
    let r = tool.call(json!({"input": "hi"})).await.expect("call ok");
    assert_eq!(r["tool"], "echo", "应 echo back tool name");
    assert_eq!(r["echo"], "hi", "应 echo back args.input");
    assert_eq!(r["result"], "processed", "MockSyncTool 应有 result 字段");
}

#[tokio::test]
async fn mock_async_tool_call_with_zero_delay() {
    use apeireth_tool_registry::MockAsyncTool;
    let tool = MockAsyncTool {
        name: "fast".to_string(),
        delay_ms: 0,
    };
    let start = std::time::Instant::now();
    let r = tool.call(json!({})).await.expect("async call");
    let elapsed = start.elapsed();
    // delay_ms=0 应 0 等待 (允许小幅度调度开销)
    assert!(
        elapsed.as_millis() < 500,
        "delay=0 应 0 等待, got {}ms",
        elapsed.as_millis()
    );
    assert!(
        r.is_object() || r.is_string() || r.is_number(),
        "MockAsyncTool 返值: {r}"
    );
}

#[test]
fn mock_names_are_six_unique() {
    use apeireth_tool_registry::MOCK_NAMES;
    let mut names: Vec<&str> = MOCK_NAMES.to_vec();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 6, "MOCK_NAMES 必须 6 个唯一, got {names:?}");
}
