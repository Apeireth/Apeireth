//! # repo_scan_demo
//!
//! apeireth-repo-scan 端到端 demo (per R20 阶段 4 续 P1 估补).
//!
//! 跑通 5 步:
//! 1. 打印 K-1 强校验 7 项编译期 hardcode 常量
//! 2. 创建 RepoScanner (默认配置: 深度 10, skip 8 dir, 4 sensitive 模式)
//! 3. 调 `scan` 扫当前目录 (skeleton 返空结果)
//! 4. 调 `report_json` + `report_markdown` 生成 2 格式报告
//! 5. 调 m3 防御 sanity check (8 工具 + 13 Language + 11 KEY_FILE_PATTERNS + K-1 字样)
//!
//! 跑法: `cargo run --example repo_scan_demo --manifest-path crates/apeireth-repo-scan/Cargo.toml`
//!
//! ## 期望输出 (skeleton 阶段)
//!
//! ```text
//! [K-1] platform_name = apeireth
//! [K-1] schema_version = 1
//! [K-1] max_scan_depth = 10
//! [K-1] cache_ttl_days = 7
//! [K-1] supported_languages.len() = 13
//! [K-1] key_file_patterns.len() = 11
//! [K-1] tool_whitelist.len() = 8
//! [§3 scan] schema_version=1, files=0, duration_ms=0
//! [§4 report_json] length=NN
//! [§4 report_markdown] starts with # Repo Scan Report
//! [§5 cache] skeleton put/get roundtrip OK
//! [§6 m3] sanity_check = true
//! [§6 m3] validate_tool_call('apeireth_repo_scan_wipe', _) = Err(ToolNotWhitelisted)
//! [§6 m3] validate_tool_call('apeireth_repo_scan_scan', _) = Ok
//! ```

use apeireth_repo_scan::{
    m3_defense_sanity_check, validate_tool_call, Language, RepoScanCache, RepoScanner,
    RepoScannerConfig, RepoScannerTrait, KEY_FILE_PATTERNS, MAX_SCAN_DEPTH, PLATFORM_NAME,
    REPO_SCAN_SCHEMA_VERSION, SCAN_CACHE_TTL_DAYS, SUPPORTED_LANGUAGES, TOOL_WHITELIST,
    TOOL_WHITELIST_COUNT,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-repo-scan demo ===\n");

    // §1 K-1 强校验 — 7 项编译期 hardcode 守门
    println!("[K-1] platform_name = {PLATFORM_NAME}");
    println!("[K-1] schema_version = {REPO_SCAN_SCHEMA_VERSION}");
    println!("[K-1] max_scan_depth = {MAX_SCAN_DEPTH}");
    println!("[K-1] cache_ttl_days = {SCAN_CACHE_TTL_DAYS}");
    println!("[K-1] supported_languages.len() = {}", SUPPORTED_LANGUAGES.len());
    println!("[K-1] key_file_patterns.len() = {}", KEY_FILE_PATTERNS.len());
    println!("[K-1] tool_whitelist.len() = {TOOL_WHITELIST_COUNT}\n");

    // §2 创建 RepoScanner (默认配置)
    let scanner = RepoScanner::with_defaults();
    let config = RepoScannerConfig::default();
    println!("[§3 scanner] max_depth = {}", config.max_depth);
    println!("[§3 scanner] skip_hidden = {}", config.skip_hidden);
    println!("[§3 scanner] skip_dirs.len = {}", config.skip_dirs.len());
    println!(
        "[§3 scanner] sensitive_patterns = {:?}\n",
        config.sensitive_patterns
    );

    // §3 scan 当前目录 (skeleton 返空结果)
    let r = scanner.scan(Path::new(".")).await?;
    println!(
        "[§3 scan] schema_version={}, files={}, duration_ms={}\n",
        r.schema_version,
        r.files.len(),
        r.duration_ms
    );

    // §4 报告生成 (JSON + Markdown)
    let json = scanner.report_json(&r).await?;
    let md = scanner.report_markdown(&r).await?;
    println!("[§4 report_json] length = {} bytes", json.len());
    println!(
        "[§4 report_markdown] starts_with = {:?}",
        md.lines().next()
    );
    println!();

    // §5 缓存 skeleton put/get (tmpdir 跑通)
    let tmp = tempfile::tempdir()?;
    let cache = RepoScanCache::new(tmp.path().to_path_buf());
    cache.put(Path::new("."), &r)?;
    let entry = cache.get(Path::new("."))?;
    println!(
        "[§5 cache] put/get roundtrip OK, schema_version = {}\n",
        entry.schema_version
    );

    // §6 m3 防御 — sanity check + validate_tool_call
    println!("[§6 m3] sanity_check = {}", m3_defense_sanity_check());
    let bad = serde_json::json!({});
    println!(
        "[§6 m3] validate_tool_call('apeireth_repo_scan_wipe', _) = {:?}",
        validate_tool_call("apeireth_repo_scan_wipe", &bad)
    );
    println!(
        "[§6 m3] validate_tool_call('apeireth_repo_scan_scan', _) = {:?}",
        validate_tool_call("apeireth_repo_scan_scan", &bad)
    );
    println!();

    // §7 Language 13 项 + KeyFile 11 项守门
    let rust = Language::Rust;
    println!("[§2 Language] Rust as_debug = {rust:?}");
    println!(
        "[§1 KEY_FILE_PATTERNS] first 3 = {:?}\n",
        &KEY_FILE_PATTERNS[..3]
    );

    println!("=== demo done ===");
    Ok(())
}
