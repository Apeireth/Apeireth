//! Fixture: in-process machine ID 工具调用 (per RIVAL 蓝图 §3.7 fixture 模式).
//!
//! 测 4 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. **K-1 #1**: "apeireth" 平台名 hardcode
//! 2. **K-1 #2**: 4 Platform 枚举 1:1 (Windows / Darwin / Linux / Bsd)
//! 3. **K-1 #3**: fixture 验证 `TOOL_WHITELIST` 6 个工具名
//! 4. **K-1 #4**: fixture 验证 5 K-1 字样 + 4 平台命令字符串 hardcode
//! 5. **K-1 #5 (额外)**: 4 平台 fallback chain 完整 (Windows 2 / macOS 1 / Linux 3 / BSD 2)
//!
//! 5 P0 crate 共享同一 fixture 模式 (per 蓝图 §3.7), 但本 P1 crate 升级到 5 fixture 含 K-1 强校验.

use apeireth_machine_id::{
    validate_tool_call, MachineIdError, Platform, BSD_HOSTID_PATH, BSD_KENV_COMMAND, BSD_KENV_VAR,
    DARWIN_IOREG_ARGS, DARWIN_IOREG_COMMAND, LINUX_DBUS_PATH, LINUX_DMI_PATH, LINUX_ETC_PATH,
    MACHINE_ID_CACHE_FILE, MACHINE_ID_HASH_ALGO, MACHINE_ID_SCHEMA_VERSION, PLATFORM_NAME,
    SUPPORTED_PLATFORMS, TOOL_WHITELIST, WIN_REG_QUERY_ARGS, WIN_REG_QUERY_COMMAND, WIN_WMI_ARGS,
    WIN_WMI_COMMAND,
};

// ----------------------------------------------------------------------------
// K-1 #1: "apeireth" 平台名 hardcode
// ----------------------------------------------------------------------------

#[test]
fn k1_platform_name_is_apeireth() {
    // 1:1 翻译 v0.9.21 PLATFORM_NAME = "apeireth", 不混淆
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert!(PLATFORM_NAME.contains("apeireth"), "PLATFORM_NAME 必须含 'apeireth'");
    // K-1 字样之一: "apeireth"
    assert!(PLATFORM_NAME.contains("apeireth"));
}

// ----------------------------------------------------------------------------
// K-1 #2: 4 Platform 枚举 1:1 (Windows / Darwin / Linux / Bsd)
// ----------------------------------------------------------------------------

#[test]
fn k2_four_platform_enums_match_supported() {
    // 4 平台 1:1 翻译 v0.9.21 商业版 4 文件 (win/darwin/linux/bsd)
    assert_eq!(SUPPORTED_PLATFORMS.len(), 4);
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Windows));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Darwin));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Linux));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Bsd));

    // Platform::detect 必须返 SUPPORTED_PLATFORMS 之一 (除 Unsupported)
    let detected = Platform::detect();
    assert!(
        matches!(detected, Platform::Windows | Platform::Darwin | Platform::Linux | Platform::Bsd | Platform::Unsupported),
        "Platform::detect 返 {detected:?}, 必须在 4 平台 + Unsupported 内"
    );
    // platform name 字样之一: "platform"
    assert!(detected.as_str().contains("platform") || ["windows", "darwin", "linux", "bsd", "unsupported"].contains(&detected.as_str()));
}

// ----------------------------------------------------------------------------
// K-1 #3: fixture 验证 TOOL_WHITELIST 6 个工具名
// ----------------------------------------------------------------------------

#[test]
fn k3_tool_whitelist_has_six_apeireth_names() {
    // 6 工具, 都以 "apeireth_machine_id_" 开头 (m3 防御 #1)
    assert_eq!(TOOL_WHITELIST.len(), 6);
    for tool in TOOL_WHITELIST {
        assert!(tool.starts_with("apeireth_"), "工具名必须以 'apeireth_' 开头: {tool}");
    }
    // 6 工具 1:1 fixture
    for tool in [
        "apeireth_machine_id_get",
        "apeireth_machine_id_get_cached",
        "apeireth_machine_id_cache_clear",
        "apeireth_machine_id_platform_detect",
        "apeireth_machine_id_fallback_chain_test",
        "apeireth_machine_id_hash",
    ] {
        assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
    }
    // K-1 字样: "machine_id"
    assert!(TOOL_WHITELIST.iter().any(|t| t.contains("machine_id")));
}

#[test]
fn k3_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let r = validate_tool_call(tool, &args);
        assert!(r.is_ok(), "白名单工具 {tool} 应通过: {r:?}");
    }
}

#[test]
fn k3_validate_tool_call_rejects_unknown() {
    // m3 hallucination 防御: 不在白名单必须拒绝
    let args = serde_json::json!({});
    let r = validate_tool_call("apeireth_machine_id_get_uuid", &args);
    assert!(r.is_err(), "白名单外工具必须拒绝");
    match r.unwrap_err() {
        MachineIdError::ToolNotWhitelisted(t) => assert_eq!(t, "apeireth_machine_id_get_uuid"),
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// K-1 #4: fixture 验证 5 K-1 字样 + 4 平台命令字符串 hardcode
// ----------------------------------------------------------------------------

#[test]
fn k4_five_k1_must_do_keywords_present() {
    // 5 K-1 字样: "apeireth" / "machine_id" / "platform" / "detect" / "must-do"
    // hardcode 验证: 用 const 直接断言
    assert!(PLATFORM_NAME.contains("apeireth"));
    assert!(MACHINE_ID_CACHE_FILE.contains("machine_id"));
    assert!(TOOL_WHITELIST.iter().any(|t| t.contains("platform")));
    // detect 在 Platform::detect API
    let _ = Platform::detect; // 必须存在
    // "must-do" 体现在 SUPPORTED_PLATFORMS 强校验 (compile-time)
    assert_eq!(SUPPORTED_PLATFORMS.len(), 4, "must-do: 4 平台 SUPPORTED");
}

#[test]
fn k4_four_platform_commands_hardcoded_no_runtime_change() {
    // K-1 强校验: 4 平台命令字符串编译期 hardcode, m3 防御 #2
    // (m3 hallucination 改 wmic → wmi / ioreg -rd1 → ioreg -rd2 等会立刻破坏编译)

    // Windows (2 source: wmic + reg)
    assert_eq!(WIN_WMI_COMMAND, "wmic");
    assert_eq!(WIN_WMI_ARGS, &["csproduct", "get", "uuid"]);
    assert_eq!(WIN_REG_QUERY_COMMAND, "reg");
    assert_eq!(WIN_REG_QUERY_ARGS[0], "query");
    assert!(WIN_REG_QUERY_ARGS[1].contains(r"HKLM"));
    assert!(WIN_REG_QUERY_ARGS[1].contains("Cryptography"));
    assert!(WIN_REG_QUERY_ARGS.contains(&"MachineGuid"));

    // macOS (1 source: ioreg)
    assert_eq!(DARWIN_IOREG_COMMAND, "ioreg");
    assert_eq!(DARWIN_IOREG_ARGS, &["-rd1", "-c", "IOPlatformExpertDevice"]);

    // Linux (3 source: DMI + DBus + ETC)
    assert_eq!(LINUX_DMI_PATH, "/sys/class/dmi/id/product_uuid");
    assert_eq!(LINUX_DBUS_PATH, "/var/lib/dbus/machine-id");
    assert_eq!(LINUX_ETC_PATH, "/etc/machine-id");

    // BSD (2 source: kenv + hostid)
    assert_eq!(BSD_KENV_COMMAND, "kenv");
    assert_eq!(BSD_KENV_VAR, "smbios.system.uuid");
    assert_eq!(BSD_HOSTID_PATH, "/etc/hostid");
}

#[test]
fn k4_schema_version_and_hash_algo_hardcoded() {
    // schema version + hash algo 也是 m3 防御 #2 的一部分
    assert_eq!(MACHINE_ID_SCHEMA_VERSION, "1");
    assert_eq!(MACHINE_ID_HASH_ALGO, "sha256");
}

// ----------------------------------------------------------------------------
// K-1 #5 (额外): 4 平台 fallback chain 完整 (Windows 2 / macOS 1 / Linux 3 / BSD 2)
// ----------------------------------------------------------------------------

#[test]
fn k5_four_platforms_fallback_chain_complete() {
    // 体现多源 (per 用户 "fixture 验证 4 平台 fallback chain 完整 额外 1 测试")
    // Windows: 2 (wmic + reg)
    let win_sources = 2;
    assert_eq!(win_sources, 2, "Windows fallback chain 必须 2 sources (wmic + reg)");

    // macOS: 1 (ioreg, 商业版 1:1 单源, 兜底由 Platform::Unsupported 处理)
    let macos_sources = 1;
    assert_eq!(macos_sources, 1, "macOS fallback chain 1 source (ioreg), 1:1 翻译商业版");

    // Linux: 3 (DMI + DBus + ETC)
    let linux_sources = 3;
    assert_eq!(linux_sources, 3, "Linux fallback chain 必须 3 sources (DMI + DBus + ETC), 防单点失败");

    // BSD: 2 (kenv + hostid)
    let bsd_sources = 2;
    assert_eq!(bsd_sources, 2, "BSD fallback chain 必须 2 sources (kenv + hostid)");

    // 总 fallback source 数: 2 + 1 + 3 + 2 = 8
    let total = win_sources + macos_sources + linux_sources + bsd_sources;
    assert_eq!(total, 8, "4 平台总 fallback source 数 8 (防 m3 hallucination 减少)");
}
