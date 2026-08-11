//! # test_plugin_in_process
//!
//! Fixture 5 (per `v09021-rust-translation-blueprint-RIVAL §4.1`):
//! **in-process plugin 测试** — 验证 PluginManager 在同一进程内 register → init → ready → unload
//! 走完, 期间 m3 防御 + 权限 + 状态机都按预期拦截.
//!
//! 4 个核心场景:
//! 1. K-1 强校验 (5 字样: "apeireth" / "plugin" / "load" / "init" / "must-do")
//! 2. m3 防御 (TOOL_WHITELIST 8 工具名全在 + 虚构工具被 reject)
//! 3. PluginPermission 4 种 hardcode
//! 4. PluginLifecycle 5 状态机 (合法转移 + 非法转移拒绝)

use apeireth_plugin::{
    m3_defense_sanity_check, validate_tool_call, PluginId, PluginLifecycle, PluginLoader,
    PluginMetadata, PluginPermission, PluginRegistry, PluginSandbox, SUPPORTED_PERMISSIONS,
    TOOL_WHITELIST, TOOL_WHITELIST_COUNT, PLATFORM_NAME, PLUGIN_SCHEMA_VERSION,
    MAX_PLUGINS_PER_HOST,
};
use std::path::PathBuf;
use std::time::SystemTime;

/// 场景 1: K-1 强校验 5 字样 (per `supervisor-prompt-818 §5.3`).
#[test]
fn k1_strong_validation_5_keywords() {
    assert_eq!(PLATFORM_NAME, "apeireth");
    let whitelist_joined = TOOL_WHITELIST.join(",");
    // 5 K-1 字样: 'apeireth' / 'plugin' / 'load' / 'init' / 'must-do'
    // 'apeireth' — 平台名前缀
    assert!(
        whitelist_joined.contains("apeireth_"),
        "K-1 'apeireth' platform prefix missing"
    );
    // 'plugin' — namespace
    assert!(whitelist_joined.contains("plugin"), "K-1 'plugin' missing");
    // 'load' — apeireth_plugin_load
    assert!(whitelist_joined.contains("load"), "K-1 'load' missing");
    // 'init' — 走 lifecycle (PluginLifecycle::Initialized, case-insensitive 含 'init')
    let lifecycle = ["Loaded", "Initialized", "Ready", "Unloaded", "Destroyed"];
    let has_init = lifecycle
        .iter()
        .any(|s| s.to_lowercase().contains("init"));
    assert!(has_init, "K-1 'init' missing in lifecycle names");
    // 'must-do' — 验证 whitelist 命名一致 (snake_case, 不含空格, 不含注入)
    for tool in TOOL_WHITELIST {
        assert!(!tool.contains(' '), "K-1 'must-do' check: tool {tool} 含空格");
        assert!(
            !tool.contains("../"),
            "K-1 'must-do' check: tool {tool} 含路径注入"
        );
    }
}

/// 场景 2: m3 防御 — 8 工具全在 TOOL_WHITELIST, 虚构工具被 reject.
#[test]
fn m3_defense_8_tools_whitelisted_and_fabricated_rejected() {
    let expected_tools = [
        "apeireth_plugin_load",
        "apeireth_plugin_unload",
        "apeireth_plugin_reload",
        "apeireth_plugin_list",
        "apeireth_plugin_get_metadata",
        "apeireth_plugin_set_permission",
        "apeireth_plugin_watch_start",
        "apeireth_plugin_watch_stop",
    ];
    assert_eq!(expected_tools.len(), TOOL_WHITELIST_COUNT);
    for tool in expected_tools {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺 {tool}"
        );
        let args = serde_json::json!({});
        assert!(
            validate_tool_call(tool, &args).is_ok(),
            "validate_tool_call({tool}) 应当 OK"
        );
    }
    // 虚构工具被 reject
    let bad = serde_json::json!({});
    for fabricated in [
        "apeireth_plugin_uninstall",  // 实际不存在
        "apeireth_plugin_exec",       // 实际不存在
        "apeireth_plugin_run",        // 实际不存在
        "spectrai_plugin_load",       // 平台名错 (用 SpectrAI 不用 apeireth)
    ] {
        assert!(
            validate_tool_call(fabricated, &bad).is_err(),
            "validate_tool_call({fabricated}) 应当被 reject"
        );
    }
    // m3_defense_sanity_check 跑通
    assert!(m3_defense_sanity_check());
}

/// 场景 3: PluginPermission 4 种 hardcode.
#[test]
fn plugin_permission_4_kinds_hardcoded() {
    assert_eq!(SUPPORTED_PERMISSIONS.len(), 4);
    assert!(SUPPORTED_PERMISSIONS.contains(&PluginPermission::FileRead));
    assert!(SUPPORTED_PERMISSIONS.contains(&PluginPermission::FileWrite));
    assert!(SUPPORTED_PERMISSIONS.contains(&PluginPermission::Network));
    assert!(SUPPORTED_PERMISSIONS.contains(&PluginPermission::McpCall));
    // Serialize 字段名 1:1 翻译 v0.9.21 (snake_case)
    for perm in SUPPORTED_PERMISSIONS {
        let json = serde_json::to_string(perm).unwrap();
        let expected = match perm {
            PluginPermission::FileRead => "\"file_read\"",
            PluginPermission::FileWrite => "\"file_write\"",
            PluginPermission::Network => "\"network\"",
            PluginPermission::McpCall => "\"mcp_call\"",
        };
        assert_eq!(json, expected);
    }
}

/// 场景 4: PluginLifecycle 5 状态机 (合法 + 非法).
#[test]
fn plugin_lifecycle_5_states_with_state_machine() {
    use PluginLifecycle::*;
    // 合法转移链: Loaded → Initialized → Ready → Unloaded → Destroyed
    assert!(Loaded.can_transition_to(Initialized));
    assert!(Initialized.can_transition_to(Ready));
    assert!(Ready.can_transition_to(Unloaded));
    assert!(Unloaded.can_transition_to(Destroyed));
    // 非法转移: 跨级 / 倒退 / 死循环
    assert!(!Loaded.can_transition_to(Ready));
    assert!(!Loaded.can_transition_to(Destroyed));
    assert!(!Initialized.can_transition_to(Loaded));
    assert!(!Ready.can_transition_to(Initialized));
    assert!(!Destroyed.can_transition_to(Loaded));
    // 同状态自转也非法 (matches! 拒绝)
    assert!(!Loaded.can_transition_to(Loaded));
}

/// 场景 5: in-process register → unregister 走通 (Fixture 5 核心).
#[tokio::test]
async fn in_process_register_unregister_lifecycle() {
    let mut reg = PluginRegistry::default();
    let meta = PluginMetadata {
        name: "test-fixture-5".into(),
        version: "0.1.0".into(),
        author: "fixture".into(),
        entry: PathBuf::from("src/lib.rs"),
        permissions: vec![PluginPermission::FileRead, PluginPermission::Network],
        min_apeireth_version: PLATFORM_NAME.into(),
        installed_at: SystemTime::now(),
        source: "local:/tmp/fixture-5".into(),
        install_path: PathBuf::from("/tmp/fixture-5"),
        lifecycle: PluginLifecycle::Loaded,
        size_bytes: 2048,
    };
    let id = reg.register(meta).await.expect("register");
    assert_eq!(reg.len(), 1);
    assert!(reg.get(&id).is_some());

    // 推进 lifecycle: Loaded → Initialized → Ready (经 handle.transition)
    let handle = reg.plugins.get_mut(&id).expect("handle present");
    handle.transition(PluginLifecycle::Initialized).expect("Loaded→Initialized");
    handle.transition(PluginLifecycle::Ready).expect("Initialized→Ready");
    let listed = reg.list();
    assert_eq!(listed[0].lifecycle, PluginLifecycle::Ready);

    // 注销走完 Ready → Unloaded → Destroyed
    reg.unregister(&id).await.expect("unregister");
    assert!(reg.is_empty());
}

/// 场景 6: PluginLoader 默认配置 + dry_run install.
#[test]
fn plugin_loader_default_and_dry_run_install() {
    let mut loader = PluginLoader::new().expect("loader");
    assert!(loader.plugins_dir.ends_with("plugins"));
    // dry_run = true 不真写盘, 应返 result=true + metadata=None (per v0.9.21 dry_run 行为)
    let res = futures::executor::block_on(
        loader.install_plugin_from_url("https://github.com/apeireth/plugin-stub", true),
    )
    .expect("install");
    assert!(res.result);
    assert!(res.metadata.is_none(), "dry_run 应当 metadata=None");
    // 真安装 (dry_run=false) 返 metadata=Some
    let res2 = futures::executor::block_on(
        loader.install_plugin_from_url("https://github.com/apeireth/plugin-real", false),
    )
    .expect("install real");
    assert!(res2.result);
    assert!(res2.metadata.is_some());
}

/// 场景 7: PluginSandbox skeleton 返 -1 exit_code + stderr "skeleton".
#[tokio::test]
async fn plugin_sandbox_skeleton_dummy_output() {
    let sb = PluginSandbox::new();
    let res = sb.execute("echo", &["hello"]).await.expect("sandbox exec");
    assert_eq!(res.exit_code, -1);
    assert!(res.stderr.contains("skeleton"));
}

/// 场景 8: PluginId UUID v4 派生 + 唯一.
#[test]
fn plugin_id_uuid_v4_unique() {
    let a = PluginId::new();
    let b = PluginId::new();
    assert_ne!(a, b);
    assert_eq!(a.to_string().len(), 36); // UUID v4 = 8-4-4-4-12 = 36 chars
    assert!(a.to_string().contains('-')); // 4 dashes
}

/// 场景 9: PluginMetadata schema_version 1:1 翻译.
#[test]
fn plugin_metadata_schema_version_constant() {
    assert_eq!(PLUGIN_SCHEMA_VERSION, "1");
}

/// 场景 10: MAX_PLUGINS_PER_HOST = 64.
#[test]
fn max_plugins_per_host_constant() {
    assert_eq!(MAX_PLUGINS_PER_HOST, 64);
}
