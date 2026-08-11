//! # test_repo_scan_in_process
//!
//! Fixture 5 (per 蓝图 §2.3.1 + 任务要求): 5 in-process 测试覆盖
//! K-1 强校验 + m3 防御 + 编译期 hardcode 守门 + RepoScanner skeleton roundtrip.
//!
//! 5 场景:
//! 1. K-1 强校验 5 字样 (per `supervisor-prompt-818 §5.3`)
//! 2. m3 防御 8 工具白名单 + 虚构 reject + sanity_check
//! 3. 13 Language 枚举 + SUPPORTED_LANGUAGES 数组守门 (K-1 #3)
//! 4. KEY_FILE_PATTERNS 11 项 + 7 项编译期常量 hardcode
//! 5. RepoScanner skeleton scan → report → cache roundtrip

use apeireth_repo_scan::{
    m3_defense_sanity_check, validate_tool_call, RepoScanCache, RepoScanner, RepoScannerTrait,
    KEY_FILE_PATTERNS, MAX_SCAN_DEPTH, PLATFORM_NAME, REPO_SCAN_SCHEMA_VERSION,
    SCAN_CACHE_TTL_DAYS, SUPPORTED_LANGUAGES, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
};
use apeireth_repo_scan::Language;
use std::path::Path;

/// 场景 1: K-1 强校验 5 字样 — 任务要求 5 字样守门.
#[test]
fn fixture_1_k1_strong_validation_5_keywords() {
    assert_eq!(PLATFORM_NAME, "apeireth");
    let joined = TOOL_WHITELIST.join(",");
    assert!(joined.contains("apeireth_"), "K-1 'apeireth' missing");
    assert!(joined.contains("repo_scan"), "K-1 'repo_scan' missing");
    assert!(joined.contains("scan"), "K-1 'scan' missing");
    assert!(joined.contains("stats"), "K-1 'stats' missing");
    for tool in TOOL_WHITELIST {
        assert!(!tool.contains(' '), "K-1 'must-do' check: tool {tool} 含空格");
        assert!(!tool.contains("../"), "K-1 'must-do' check: tool {tool} 含路径注入");
    }
}

/// 场景 2: m3 防御 — 8 工具全在白名单, 虚构 reject + sanity_check.
#[test]
fn fixture_2_m3_defense_8_tools_and_sanity() {
    assert_eq!(TOOL_WHITELIST_COUNT, 8);
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        assert!(validate_tool_call(tool, &args).is_ok());
    }
    for bad in [
        "apeireth_repo_scan_wipe",
        "apeireth_repo_scan_exec",
        "spectrai_repo_scan_scan",
    ] {
        assert!(validate_tool_call(bad, &args).is_err());
    }
    assert!(m3_defense_sanity_check());
}

/// 场景 3: 13 Language 枚举 + SUPPORTED_LANGUAGES 数组守门.
#[test]
fn fixture_3_language_13_enums_hardcoded() {
    assert_eq!(SUPPORTED_LANGUAGES.len(), 13);
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("unknown"), Language::Other);
}

/// 场景 4: KEY_FILE_PATTERNS 11 + 7 项编译期 hardcode 常量守门.
#[test]
fn fixture_4_key_file_patterns_11_and_constants() {
    assert_eq!(KEY_FILE_PATTERNS.len(), 11);
    assert!(KEY_FILE_PATTERNS.contains(&"README*"));
    assert!(KEY_FILE_PATTERNS.contains(&"Cargo.toml"));
    assert!(KEY_FILE_PATTERNS.contains(&"package.json"));
    assert_eq!(REPO_SCAN_SCHEMA_VERSION, "1");
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(MAX_SCAN_DEPTH, 10);
    assert_eq!(SCAN_CACHE_TTL_DAYS, 7);
    assert_eq!(SUPPORTED_LANGUAGES.len(), 13);
    assert_eq!(TOOL_WHITELIST_COUNT, 8);
    assert_eq!(KEY_FILE_PATTERNS.len(), 11);
}

/// 场景 5: RepoScanner skeleton scan → report → cache roundtrip.
#[tokio::test]
async fn fixture_5_scanner_skeleton_roundtrip() {
    let scanner = RepoScanner::with_defaults();
    let r = scanner.scan(Path::new(".")).await.expect("scan");
    assert_eq!(r.schema_version, "1");
    let json = scanner.report_json(&r).await.expect("json");
    assert!(json.contains("schema_version"));
    let md = scanner.report_markdown(&r).await.expect("md");
    assert!(md.contains("# Repo Scan Report"));
    let tmp = tempfile::tempdir().expect("tmpdir");
    let cache = RepoScanCache::new(tmp.path().to_path_buf());
    cache.put(Path::new("."), &r).expect("put");
    let entry = cache.get(Path::new(".")).expect("get");
    assert_eq!(entry.schema_version, "1");
}
