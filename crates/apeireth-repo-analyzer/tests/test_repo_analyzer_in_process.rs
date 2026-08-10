//! Fixture 6: in-process Repo Analyzer 工具调用 (per RIVAL 蓝图 §3.7 缺口 6).
//!
//! 测 5 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 8 Repo Analyzer 工具
//! 2. `validate_tool_call` 接受白名单内工具
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 AnalyzerError::ToolNotWhitelisted)
//! 4. 5 K-1 不变量字符串全在 K1_INVARIANTS
//! 5. SUPPORTED_TECH_DEBT 编译期 hardcode 5 个枚举
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_repo_analyzer::{
    validate_tool_call, validate_tool_schema, AnalyzerError, K1_INVARIANTS, SUPPORTED_TECH_DEBT,
    TechDebtType, TOOL_WHITELIST,
};

#[test]
fn test_whitelist_contains_eight_repo_analyzer_tools() {
    // 8 工具: complexity / tech_debt / deps / security / functions / report_json / report_markdown / report_sarif
    assert_eq!(TOOL_WHITELIST.len(), 8);
    for tool in [
        "apeireth_repo_analyzer_complexity",
        "apeireth_repo_analyzer_tech_debt",
        "apeireth_repo_analyzer_deps",
        "apeireth_repo_analyzer_security",
        "apeireth_repo_analyzer_functions",
        "apeireth_repo_analyzer_report_json",
        "apeireth_repo_analyzer_report_markdown",
        "apeireth_repo_analyzer_report_sarif",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    // 白名单内应通过 (返回 Ok(()))
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    // m3 hallucination 防御: 不在白名单的工具必须拒绝
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_repo_analyzer_audit", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        AnalyzerError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_repo_analyzer_audit");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

#[test]
fn test_k1_invariants_all_five_present() {
    // K-1 强校验 5 字样 (per 任务规范):
    // - "apeireth" 平台名
    // - "repo_analyzer" 工具前缀
    // - "analyze" 操作动词
    // - "complexity" 核心指标
    // - "must-do" 设计哲学
    assert_eq!(K1_INVARIANTS.len(), 5);
    for s in [
        "apeireth",
        "repo_analyzer",
        "analyze",
        "complexity",
        "must-do",
    ] {
        assert!(
            K1_INVARIANTS.contains(&s),
            "K-1 invariant 缺: {s}"
        );
    }
}

#[test]
fn test_supported_tech_debt_is_five() {
    // K-1 强校验: 5 个 TechDebtType 枚举值编译期 hardcode
    assert_eq!(SUPPORTED_TECH_DEBT.len(), 5);
    assert!(SUPPORTED_TECH_DEBT.contains(&TechDebtType::Todo));
    assert!(SUPPORTED_TECH_DEBT.contains(&TechDebtType::Fixme));
    assert!(SUPPORTED_TECH_DEBT.contains(&TechDebtType::Hack));
    assert!(SUPPORTED_TECH_DEBT.contains(&TechDebtType::Bug));
    assert!(SUPPORTED_TECH_DEBT.contains(&TechDebtType::SecurityIssue));
}

#[test]
fn test_schema_validate_rejects_non_object_args() {
    // m3 防御 #2: args 必须是 JSON object
    let tool = "apeireth_repo_analyzer_complexity";
    // 4 个非 object 类型都拒绝
    for bad in [
        serde_json::json!(null),
        serde_json::json!("string"),
        serde_json::json!(42),
        serde_json::json!([1, 2, 3]),
    ] {
        let result = validate_tool_schema(tool, &bad);
        assert!(result.is_err(), "non-object args 必须拒绝: {bad}");
    }
    // object 通过
    let good = serde_json::json!({"file": "src/lib.rs"});
    assert!(validate_tool_schema(tool, &good).is_ok());
}

#[test]
fn test_platform_name_is_apeireth() {
    // K-1 强校验 #2: 平台名编译期 hardcode "apeireth"
    assert_eq!(apeireth_repo_analyzer::PLATFORM_NAME, "apeireth");
    assert_eq!(apeireth_repo_analyzer::REPO_ANALYZER_SCHEMA_VERSION, "1");
}

#[test]
fn test_supported_dep_formats_count() {
    // 编译期守门: 3 种依赖格式
    assert_eq!(apeireth_repo_analyzer::SUPPORTED_DEP_FORMATS.len(), 3);
    assert!(apeireth_repo_analyzer::SUPPORTED_DEP_FORMATS.contains(&"Cargo.toml"));
    assert!(apeireth_repo_analyzer::SUPPORTED_DEP_FORMATS.contains(&"package.json"));
    assert!(apeireth_repo_analyzer::SUPPORTED_DEP_FORMATS.contains(&"pyproject.toml"));
}

#[test]
fn test_supported_report_formats_count() {
    // 编译期守门: 3 种报告格式
    assert_eq!(apeireth_repo_analyzer::SUPPORTED_REPORT_FORMATS.len(), 3);
    assert!(apeireth_repo_analyzer::SUPPORTED_REPORT_FORMATS.contains(&"json"));
    assert!(apeireth_repo_analyzer::SUPPORTED_REPORT_FORMATS.contains(&"markdown"));
    assert!(apeireth_repo_analyzer::SUPPORTED_REPORT_FORMATS.contains(&"sarif"));
}
