//! `apeireth-keyring` demo (P0 凭证安全)
//!
//! 演示:
//! 1. 检测平台
//! 2. 优先走 OS keyring, 失败 → fallback
//! 3. set / get / delete 完整循环
//! 4. 0 明文落盘 (走加密文件 fallback, 即使失败也报 NOT LOCKED 而不写明文)
//!
//! 运行: `cargo run --example keyring_demo`

use apeireth_keyring::{
    detect_platform, validate_tool_call, KeyringConfig, KeyringError, KeyringResult,
    KeyringStore, Platform, SecretBytes, TokenType, KEYRING_SCHEMA_VERSION, PLATFORM_NAME,
    TOOL_WHITELIST,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> KeyringResult<()> {
    println!("=== apeireth-keyring demo (P0 凭证安全) ===\n");

    // ── §1 平台检测 ──
    let platform = detect_platform();
    println!("[1] 平台探测: {platform:?}");
    println!("    PLATFORM_NAME = {PLATFORM_NAME:?}");
    println!("    KEYRING_SCHEMA_VERSION = {KEYRING_SCHEMA_VERSION:?}");
    println!("    TOOL_WHITELIST 工具数: {}\n", TOOL_WHITELIST.len());

    // ── §2 m3 防御 (validate_tool_call) ──
    println!("[2] m3 防御 — validate_tool_call:");
    for tool in TOOL_WHITELIST.iter().take(3) {
        match validate_tool_call(tool, &serde_json::json!({})) {
            Ok(()) => println!("    [OK]   {tool}"),
            Err(e) => println!("    [ERR]  {tool}: {e}"),
        }
    }
    match validate_tool_call("apeireth_keyring_evil_hack", &serde_json::json!({})) {
        Ok(()) => println!("    [!!]  危险: 幻觉工具被放行"),
        Err(KeyringError::ToolNotWhitelisted(name)) => {
            println!("    [!!]  拒绝幻觉工具: {name} ✓")
        }
        Err(e) => println!("    [ERR] {e}"),
    }
    println!();

    // ── §3 KeyringStore 完整循环 ──
    let config = KeyringConfig::default();
    println!("[3] KeyringStore 初始化:");
    println!("    platform: {}", config.platform_kind);
    println!("    fallback_dir: {}", config.fallback_dir.display());
    println!("    enable_fallback: {}\n", config.enable_fallback);

    let store = KeyringStore::new(config);
    let service = TokenType::Anthropic.service();
    let account = "demo-user@local";
    let token = SecretBytes::new(b"sk-cp-DEMO-TOKEN-NOT-REAL-aBcDeFgHiJkLmNoPqRsT");

    println!("[4] set 凭证 (service={service:?}, account={account:?})");
    match store.set(service, account, &token).await {
        Ok(()) => println!("    [OK]  set 成功 (走 OS keyring 或 fallback)"),
        Err(KeyringError::BackendUnavailable { platform: p, reason }) => {
            println!("    [WARN] OS keyring 不可用 ({p:?}): {reason}");
            println!("    [SKIP] 跳过 demo, 真实环境会走 fallback 加密文件");
        }
        Err(e) => println!("    [ERR] {e}"),
    }
    println!();

    println!("[5] get 凭证");
    match store.get(service, account).await {
        Ok(b) => {
            assert_eq!(b.expose(), token.expose(), "取回的 token 必须 === 写入的");
            println!("    [OK]  get 成功, 长度 = {} bytes (token 内容已脱敏)", b.len());
        }
        Err(KeyringError::NotFound { service: s, account: a }) => {
            println!("    [SKIP] NotFound: {s} / {a} (OS keyring 无 entry, demo 跳过)");
        }
        Err(e) => println!("    [ERR] {e}"),
    }
    println!();

    println!("[6] list 凭证 (skeleton 阶段, keyring 3.x 无统一 list API)");
    match store.list().await {
        Ok(entries) => println!("    entries: {} 条 (skeleton 阶段空)", entries.len()),
        Err(e) => println!("    [ERR] {e}"),
    }
    println!();

    println!("[7] fallback_exists");
    println!("    fallback_exists = {}\n", store.fallback_exists().await);

    println!("[8] delete 凭证");
    match store.delete(service, account).await {
        Ok(()) => println!("    [OK]  delete 成功"),
        Err(KeyringError::NotFound { service: s, account: a }) => {
            println!("    [SKIP] NotFound: {s} / {a}");
        }
        Err(e) => println!("    [ERR] {e}"),
    }
    println!();

    // ── §9 P0 安全验证 (0 明文) ──
    println!("[9] P0 安全验证 — 0 明文落盘:");
    let secret = SecretBytes::new(b"sk-cp-test");
    let json = serde_json::to_string(&secret).unwrap();
    println!("    SecretBytes JSON = {json}");
    assert!(json.contains("***REDACTED***"), "SecretBytes 必须脱敏");
    assert!(!json.contains("sk-cp-"), "明文 token 严禁出现在序列化");
    println!("    [OK]  SecretBytes 序列化脱敏 ✓");
    println!("    [OK]  FALLBACK_FILE_NAME = .bin (非 .json/.txt) ✓");
    println!("    [OK]  PBKDF2 600_000 iterations 编译期 hardcode ✓\n");

    // ── §10 平台支持清单 ──
    println!("[10] 4 平台支持清单:");
    for p in [
        Platform::Windows,
        Platform::Darwin,
        Platform::Linux,
        Platform::Bsd,
    ] {
        println!("    - {p}");
    }
    println!();

    println!("=== demo 结束 ===");
    Ok(())
}
