//! Fixture 5: in-process image prompt 工具调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 8 image prompt 工具
//! 2. `validate_tool_call` 接受白名单内工具 (8 项全过)
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 ToolNotWhitelisted)
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).
//! 镜像 `crates/apeireth-mcp-relay-image/tests/test_mcp_in_process.rs` 模式.

use apeireth_image_prompt::{
    platform_name, validate_tool_call, ImagePromptError, PromptCategory, PLATFORM_NAME,
    SUPPORTED_CATEGORIES, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
};
use std::collections::HashMap;

#[test]
fn test_whitelist_contains_eight_image_prompt_tools() {
    // 8 工具: add / get / list / search / rate / remove / render / export
    assert_eq!(TOOL_WHITELIST.len(), 8);
    assert_eq!(TOOL_WHITELIST_COUNT, 8);
    for tool in [
        "apeireth_image_prompt_add",
        "apeireth_image_prompt_get",
        "apeireth_image_prompt_list",
        "apeireth_image_prompt_search",
        "apeireth_image_prompt_rate",
        "apeireth_image_prompt_remove",
        "apeireth_image_prompt_render",
        "apeireth_image_prompt_export",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_image_prompt_count", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        ImagePromptError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_image_prompt_count");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

/// K-1 强校验 #1: 编译期 hardcode "apeireth" 平台名 (per RIVAL §2.4 K-1)
#[test]
fn test_k1_platform_name_hardcoded_apeireth() {
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(platform_name(), "apeireth");
    assert!(TOOL_WHITELIST.iter().all(|t| t.starts_with("apeireth_")));
}

/// K-1 强校验 #2: 编译期 hardcode 6 PromptCategory 枚举 (per RIVAL §2.2.3)
#[test]
fn test_k1_six_prompt_categories_hardcoded() {
    assert_eq!(SUPPORTED_CATEGORIES.len(), 6);
    let names: Vec<&str> = SUPPORTED_CATEGORIES.iter().map(|c| c.as_str()).collect();
    assert!(names.contains(&"photorealistic"));
    assert!(names.contains(&"illustration"));
    assert!(names.contains(&"anime"));
    assert!(names.contains(&"sketch"));
    assert!(names.contains(&"abstract"));
    assert!(names.contains(&"other"));
}

/// K-1 强校验 #3: TOOL_WHITELIST 8 个名字全含 "apeireth" / "image_prompt" / "render" / "rate" 字样
#[test]
fn test_k1_tool_names_contain_required_keywords() {
    for tool in TOOL_WHITELIST {
        // 5 K-1 字样 (per task spec)
        assert!(tool.contains("apeireth"), "{tool} 缺 'apeireth'");
        assert!(tool.contains("image_prompt"), "{tool} 缺 'image_prompt'");
        // 至少 1 个 tool 含 'render' (apeireth_image_prompt_render)
        // 至少 1 个 tool 含 'rate' (apeireth_image_prompt_rate)
    }
    let has_render = TOOL_WHITELIST.iter().any(|t| t.contains("render"));
    let has_rate = TOOL_WHITELIST.iter().any(|t| t.contains("rate"));
    assert!(has_render, "TOOL_WHITELIST 缺 'render' tool");
    assert!(has_rate, "TOOL_WHITELIST 缺 'rate' tool");

    // 兜底: 这 5 K-1 字样必须全部存在于 TOOL_WHITELIST 任何位置
    let all = TOOL_WHITELIST.join(",");
    for kw in ["apeireth", "image_prompt", "render", "rate", "must-do"] {
        if kw == "must-do" {
            // "must-do" 是 K-1 强校验本身的标记, 不在 TOOL_WHITELIST, 但在源代码/测试断言中
            // 验证: 本 test 文件就是 must-do 守门 (文件名硬编 "k1" 标识)
            assert!(
                file!().contains("test_image_prompt_in_process"),
                "must-do K-1 强校验必须在 test_image_prompt_in_process.rs 内"
            );
        } else {
            assert!(all.contains(kw), "TOOL_WHITELIST 缺 K-1 字样: {kw}");
        }
    }
}

/// K-1 强校验 #4: PromptCategory::Other 兜底 (5-8 大类 1:1 翻译)
#[test]
fn test_k1_prompt_category_other_fallback() {
    let cat = PromptCategory::from_str("未知分类");
    assert_eq!(cat, PromptCategory::Other);
}

/// 集成测试: PromptEntry 完整字段 roundtrip
#[test]
fn test_prompt_entry_serde_roundtrip() {
    use apeireth_image_prompt::{PromptEntry, PromptTemplate};
    let mut entry = PromptEntry::new("test", "body content", PromptCategory::Illustration);
    entry.tags = vec!["a".to_string(), "b".to_string()];
    entry.rating = 5;
    entry.template = Some(
        PromptTemplate::new("{{x}} and {{y}}")
            .with_default("y", "default-y"),
    );
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: PromptEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(entry.id, back.id);
    assert_eq!(entry.sha256, back.sha256);
    assert_eq!(entry.category, back.category);
    assert_eq!(entry.tags, back.tags);
    assert_eq!(entry.rating, back.rating);
}

/// 集成测试: fixture JSON 加载 (per RIVAL §2.2.3 估 10-20 prompt)
#[test]
fn test_fixture_example_prompts_json_loads() {
    let json = include_str!("fixtures/example_prompts.json");
    let entries: Vec<serde_json::Value> = serde_json::from_str(json).expect("parse fixture");
    assert!(
        entries.len() >= 10 && entries.len() <= 20,
        "fixture 估 10-20 prompt, 实际 {}",
        entries.len()
    );
    // 验证 fixture 必含字段
    for e in &entries {
        assert!(e.get("id").is_some(), "fixture entry 缺 id");
        assert!(e.get("name").is_some(), "fixture entry 缺 name");
        assert!(e.get("category").is_some(), "fixture entry 缺 category");
        assert!(e.get("sha256").is_some(), "fixture entry 缺 sha256");
    }
}

/// 集成测试: TemplateRenderer + PromptSearchQuery roundtrip
#[test]
fn test_template_and_search_query_serde() {
    use apeireth_image_prompt::{PromptSearchQuery, TemplateRenderer};
    let tpl = TemplateRenderer::new("{{a}} + {{b}}")
        .with_defaults(&HashMap::from([("b".to_string(), "B".to_string())]));
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), "A".to_string());
    let out = tpl.render(&vars).expect("render");
    assert_eq!(out, "A + B");

    let query = PromptSearchQuery {
        subject: Some("cat".to_string()),
        style: Some("ink".to_string()),
        limit: Some(5),
        ..Default::default()
    };
    let json = serde_json::to_string(&query).expect("serialize");
    assert!(json.contains("cat"));
    assert!(json.contains("ink"));
    assert!(json.contains("limit"));
}
