//! S1 沙盒加固集成测试 (验证 SandboxConfig + 新模块按 JSON 解析/字段断言行为).
//!
//! 设计: 沙盒相关逻辑放 _lib 单测_ 更合适 (在 `sandbox.rs`/`restricted_token.rs`/
//! `directory_acl.rs`/`app_container.rs` 内部 `mod tests`), 但当前 lib 编译被并行
//! WIP (tool_bridge 旧代码、approval_requests arg mismatch) 阻塞— 因此 S1 沙盒
//! 集成验证走 tests/ 集成文件, 跑 lib 直链桩 + 内部 trait 行为.
//!
//! 验收点:
//! 1. SandboxConfig::from_json 解析 S1 字段 (向后兼容老 B3 JSON)
//! 2. SandboxConfig::has_privilege_hardening 与 has_limits 判定
//! 3. IntegrityLevel::parse / as_str 双向
//! 4. WellKnownSid::parse 多重别名
//! 5. AppContainer 0 装 PASS 诚实标注
//! 6. 跨平台 `prepare_child` 行为 (Windows 可真接, 其他平台返回 Err)
//! 7. S1 状态 eprintln 错误信息含"非 Windows" (跨平台可断言)

use apeireth_companion::app_container::{
    windows_backends, AppContainerBackend, AppContainerProfile,
};
use apeireth_companion::sandbox::{
    backends, IntegrityLevel, SandboxBackend, SandboxConfig, WellKnownSid,
};
use serde_json::json;

#[test]
fn sandbox_config_from_json_backward_compatible() {
    let c = SandboxConfig::from_json(&json!({"memory_limit_mb": 256, "timeout_secs": 60}));
    assert_eq!(c.memory_limit_mb, Some(256));
    assert_eq!(c.timeout_secs, 60);
    // 老 B3 JSON 没有任何 S1 字段 → 全部默认 (0 装 PASS)
    assert_eq!(c.integrity_level, None);
    assert!(c.deny_only_sids.is_empty());
    assert!(c.directory_acl_roots.is_empty());
    assert!(!c.use_app_container);
    assert!(c.has_limits());
    assert!(!c.has_privilege_hardening());
}

#[test]
fn sandbox_config_from_json_s1_full() {
    let c = SandboxConfig::from_json(&json!({
        "integrity_level": "low",
        "deny_only_sids": ["BUILTIN\\Administrators", "world"],
        "directory_acl_roots": ["C:\\sandbox\\root"],
        "use_app_container": true,
    }));
    assert_eq!(c.integrity_level, Some(IntegrityLevel::Low));
    assert_eq!(
        c.deny_only_sids,
        vec![WellKnownSid::BuiltinAdministrators, WellKnownSid::World]
    );
    assert_eq!(
        c.directory_acl_roots,
        vec![std::path::PathBuf::from("C:\\sandbox\\root")]
    );
    assert!(c.use_app_container);
    assert!(c.has_privilege_hardening());
}

#[test]
fn sandbox_config_from_json_invalid_items_dropped_not_blocking() {
    let c = SandboxConfig::from_json(&json!({
        "deny_only_sids": ["world", "garbage", "BUILTIN\\Administrators"],
        "directory_acl_roots": ["C:\\valid", "", "C:\\also-valid"],
        "integrity_level": "extreme", // 落到 Untrusted
    }));
    assert_eq!(
        c.deny_only_sids,
        vec![WellKnownSid::World, WellKnownSid::BuiltinAdministrators]
    );
    assert_eq!(
        c.directory_acl_roots,
        vec![
            std::path::PathBuf::from("C:\\valid"),
            std::path::PathBuf::from("C:\\also-valid")
        ]
    );
    assert_eq!(c.integrity_level, Some(IntegrityLevel::Untrusted));
}

#[test]
fn integrity_level_roundtrips() {
    assert_eq!(IntegrityLevel::parse("low"), IntegrityLevel::Low);
    assert_eq!(IntegrityLevel::parse("LOW"), IntegrityLevel::Low);
    assert_eq!(IntegrityLevel::parse("Medium"), IntegrityLevel::Medium);
    assert_eq!(IntegrityLevel::parse("garbage"), IntegrityLevel::Untrusted);
    assert_eq!(IntegrityLevel::Low.as_str(), "low");
    assert_eq!(IntegrityLevel::Medium.as_str(), "medium");
    assert_eq!(IntegrityLevel::Untrusted.as_str(), "untrusted");
}

#[test]
fn wellknown_sid_aliases() {
    assert_eq!(
        WellKnownSid::parse("BUILTIN\\Administrators"),
        Some(WellKnownSid::BuiltinAdministrators)
    );
    assert_eq!(
        WellKnownSid::parse("administrators"),
        Some(WellKnownSid::BuiltinAdministrators)
    );
    assert_eq!(WellKnownSid::parse("world"), Some(WellKnownSid::World));
    assert_eq!(
        WellKnownSid::parse("authenticated_user"),
        Some(WellKnownSid::AuthenticatedUser)
    );
    assert_eq!(
        WellKnownSid::parse("interactive"),
        Some(WellKnownSid::Interactive)
    );
    assert_eq!(WellKnownSid::parse("garbage"), None);
}

#[test]
fn wellknown_sid_as_str_includes_authority() {
    // as_str 给日志/序列化用, 必须显示完整 SID 来源 (BUILTIN / NT AUTHORITY)
    let s = WellKnownSid::BuiltinAdministrators.as_str();
    assert!(
        s.contains("BUILTIN"),
        "Administrators 须含 BUILTIN 前缀: {s}"
    );
    assert!(WellKnownSid::World.as_str().contains("WORLD"));
    assert!(WellKnownSid::AuthenticatedUser
        .as_str()
        .contains("NT AUTHORITY"));
}

#[test]
fn sandbox_backends_all_honest_unavailable() {
    // B3 同边界: Sandboxie/landlock 0 装 PASS, status 须诚实标注 "未接"
    for b in backends() {
        assert!(
            !b.available(),
            "B3 backend {} 未接 → available() 必须 false",
            b.name()
        );
        assert!(
            b.status().contains("未接"),
            "B3 backend {} status 须标注未接",
            b.name()
        );
    }
}

#[test]
fn app_container_0_install_passes_loudly() {
    // S1 高危档: 0 装 PASS, available=false, status 诚实
    let b = AppContainerProfile;
    assert!(!b.available(), "AppContainer 0 装 → available() 必须 false");
    assert!(
        b.status().contains("未接"),
        "AppContainer status 须标注未接"
    );
    assert_eq!(b.name(), "AppContainer");
}

#[test]
fn app_container_render_params_contains_timeout() {
    let cfg = SandboxConfig {
        timeout_secs: 90,
        ..Default::default()
    };
    let p = AppContainerProfile.render_params(&cfg);
    assert!(
        p.iter().any(|s| s.contains("90")),
        "参数模板应含 timeout=90: {p:?}"
    );
}

#[test]
fn app_container_backends_listing() {
    let list = windows_backends();
    assert!(!list.is_empty(), "AppContainer backend 清单非空");
    for b in &list {
        assert!(!b.available());
    }
}

#[test]
fn privilege_hardening_flag_logic() {
    let mut sc = SandboxConfig::default();
    assert!(!sc.has_privilege_hardening());

    sc.integrity_level = Some(IntegrityLevel::Low);
    assert!(sc.has_privilege_hardening());

    sc.integrity_level = None;
    sc.deny_only_sids.push(WellKnownSid::BuiltinAdministrators);
    assert!(sc.has_privilege_hardening());

    sc.deny_only_sids.clear();
    sc.directory_acl_roots
        .push(std::path::PathBuf::from("C:\\root"));
    assert!(sc.has_privilege_hardening());

    sc.directory_acl_roots.clear();
    sc.use_app_container = true;
    assert!(sc.has_privilege_hardening());
}

#[test]
fn has_limits_only_resource_flags() {
    let mut sc = SandboxConfig::default();
    assert!(!sc.has_limits(), "default 不应有资源限额");

    sc.memory_limit_mb = Some(512);
    assert!(sc.has_limits());

    sc.memory_limit_mb = None;
    sc.cpu_percent = Some(50);
    assert!(sc.has_limits());

    sc.cpu_percent = None;
    sc.cpu_time_secs = Some(30);
    assert!(sc.has_limits());

    // S1 字段不算 has_limits (resource 专属)
    sc.cpu_time_secs = None;
    sc.integrity_level = Some(IntegrityLevel::Low);
    assert!(!sc.has_limits(), "S1 完整性级别不计入资源限额");
    assert!(sc.has_privilege_hardening(), "S1 完整性级别计入权限加固");
}

#[cfg(not(windows))]
#[test]
fn prepare_child_off_windows_returns_err() {
    let sc = SandboxConfig {
        integrity_level: Some(IntegrityLevel::Low),
        ..Default::default()
    };
    let r = apeireth_companion::sandbox::prepare_child(&sc);
    assert!(r.is_err(), "非 Windows 平台请求 hardening 应诚实返 Err");
    let err = r.unwrap_err();
    assert!(err.contains("非 Windows"), "错误信息应诚实标注: {err}");
}

#[cfg(not(windows))]
#[test]
fn prepare_child_default_is_passthrough() {
    // default cfg → 不需要 harden → 跨平台 Ok 占位
    let sc = SandboxConfig::default();
    let r = apeireth_companion::sandbox::prepare_child(&sc);
    assert!(r.is_ok(), "default cfg 无 harden 应 Ok: {r:?}");
}

#[test]
fn from_json_empty_or_null_returns_default() {
    assert_eq!(
        SandboxConfig::from_json(&json!({})),
        SandboxConfig::default()
    );
    assert_eq!(
        SandboxConfig::from_json(&json!(null)),
        SandboxConfig::default()
    );
}
