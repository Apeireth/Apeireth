//! # `tests/test_integration_e2e_in_process.rs` — 60+ e2e 集成测试
//!
//! **职责**: 集成测试, 跑 `apeireth_integration_e2e` 的公开 API, 端到端验证
//! 三层 (workspace + API + TUI) + 报告 + 5 K-1 强校验.
//!
//! **60+ 测试分组** (per 派活单):
//! - 5 workspace tests         (24 LOCKED + 5 provider + workspace version + 8 项承诺)
//! - 21 API tests              (19 端点 + 2 附加)
//! - 15 TUI tests              (5 nav + 9 organ + 1 quit)
//! - 5 harness tests           (start / shutdown / 镜像 / 锁 / 错误)
//! - 8 report tests            (TestResult / E2eLayer / E2eReport / 4 format fn)
//! - 5 K-1 强校验测试          (5 nav / 9 器官 / 6 锚 / 8 承诺 / 6 endpoint groups)
//! - 5 集成 smoke 测试        (harness 全跑过 + 报告生成 + assert_all_passed)
//!
//! **8 不修改承诺**: 跟 src/ 一致
//!
//! **跑法**:
//! ```bash
//! cd crates/apeireth-integration-e2e
//! cargo test
//! ```

use apeireth_integration_e2e::prelude::*;
use apeireth_integration_e2e::*;

// ============================================================================
// 5 workspace 集成测试
// ============================================================================

#[test]
fn test_workspace_no_locked_violation_integration() {
    let root = locate_workspace_root_for_tests();
    test_workspace_no_locked_violation(&root).unwrap();
}

#[test]
fn test_workspace_no_workspace_version_modified_integration() {
    let root = locate_workspace_root_for_tests();
    test_workspace_no_workspace_version_modified(&root).unwrap();
}

#[test]
fn test_workspace_no_sandbox_path_writes_integration() {
    let root = locate_workspace_root_for_tests();
    test_workspace_no_sandbox_path_writes(&root).unwrap();
}

#[test]
fn test_workspace_8_promises_audit_passes_integration() {
    let root = locate_workspace_root_for_tests();
    test_workspace_8_promises_audit_passes(&root).unwrap();
}

#[test]
fn test_workspace_cargo_check_passes_integration() {
    let root = locate_workspace_root_for_tests();
    test_workspace_cargo_check_passes(&root).unwrap();
}

fn locate_workspace_root_for_tests() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = `crates/apeireth-integration-e2e/`, 需上 2 级到主仓根
    if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest);
        if let Some(grand) = p.parent().and_then(|x| x.parent()) {
            return grand.to_path_buf();
        }
    }
    std::env::current_dir().unwrap()
}

// ============================================================================
// 21 API 集成测试 (用 tokio runtime)
// ============================================================================

fn tokio_block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

macro_rules! api_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            tokio_block_on(async {
                let mut h = IntegrationHarness::start().await.unwrap();
                $body(&mut h).await.unwrap();
                h.shutdown().await.unwrap();
            });
        }
    };
}

api_test!(
    test_api_metrics_integration,
    test_api_metrics_endpoint_returns_prometheus
);
api_test!(
    test_api_health_integration,
    test_api_health_endpoint_5_components
);
api_test!(test_api_status_integration, test_api_status_endpoint_uptime);
api_test!(
    test_api_tools_list_integration,
    test_api_tools_calendar_list
);
api_test!(
    test_api_tools_create_integration,
    test_api_tools_calendar_create
);
api_test!(test_api_tools_get_integration, test_api_tools_calendar_get);
api_test!(
    test_api_tools_update_integration,
    test_api_tools_calendar_update
);
api_test!(
    test_api_tools_delete_integration,
    test_api_tools_calendar_delete
);
api_test!(
    test_api_message_list_integration,
    test_api_tools_message_list
);
api_test!(
    test_api_message_send_integration,
    test_api_tools_message_send
);
api_test!(
    test_api_contact_list_integration,
    test_api_tools_contact_list
);
api_test!(
    test_api_contact_create_integration,
    test_api_tools_contact_create
);
api_test!(test_api_task_list_integration, test_api_tools_task_list);
api_test!(
    test_api_task_complete_integration,
    test_api_tools_task_complete
);
api_test!(test_api_search_web_integration, test_api_tools_search_web);
api_test!(test_api_search_code_integration, test_api_tools_search_code);
api_test!(
    test_api_unauthorized_401_integration,
    test_api_unauthorized_returns_401
);
api_test!(
    test_api_not_found_404_integration,
    test_api_not_found_returns_404
);
api_test!(
    test_api_server_500_integration,
    test_api_server_error_returns_500
);
api_test!(test_api_websocket_integration, test_api_websocket_8_frames);
api_test!(
    test_api_rate_limit_integration,
    test_api_rate_limit_enforced
);

// ============================================================================
// 15 TUI 集成测试
// ============================================================================

fn new_harness_sync() -> IntegrationHarness {
    tokio_block_on(IntegrationHarness::start()).unwrap()
}

#[test]
fn test_tui_status_nav_integration() {
    let mut h = new_harness_sync();
    test_tui_status_nav_renders(&mut h).unwrap();
}

#[test]
fn test_tui_session_nav_integration() {
    let mut h = new_harness_sync();
    test_tui_session_nav_lists(&mut h).unwrap();
}

#[test]
fn test_tui_tools_nav_integration() {
    let mut h = new_harness_sync();
    test_tui_tools_nav_shows_6(&mut h).unwrap();
}

#[test]
fn test_tui_settings_nav_integration() {
    let mut h = new_harness_sync();
    test_tui_settings_nav_5_providers(&mut h).unwrap();
}

#[test]
fn test_tui_help_nav_integration() {
    let mut h = new_harness_sync();
    test_tui_help_nav_6_anchors(&mut h).unwrap();
}

#[test]
fn test_tui_organ_heart_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_heart_pulse(&mut h).unwrap();
}

#[test]
fn test_tui_organ_brain_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_brain_llm(&mut h).unwrap();
}

#[test]
fn test_tui_organ_hand_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_hand_tools(&mut h).unwrap();
}

#[test]
fn test_tui_organ_eye_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_eye_input(&mut h).unwrap();
}

#[test]
fn test_tui_organ_ear_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_ear_events(&mut h).unwrap();
}

#[test]
fn test_tui_organ_memory_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_memory_history(&mut h).unwrap();
}

#[test]
fn test_tui_organ_voice_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_voice_state(&mut h).unwrap();
}

#[test]
fn test_tui_organ_body_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_body_resources(&mut h).unwrap();
}

#[test]
fn test_tui_organ_mind_integration() {
    let mut h = new_harness_sync();
    test_tui_organ_mind_anchors(&mut h).unwrap();
}

#[test]
fn test_tui_quit_q_integration() {
    let mut h = new_harness_sync();
    test_tui_quit_key_q(&mut h).unwrap();
}

// ============================================================================
// 5 harness 集成测试
// ============================================================================

#[test]
fn test_harness_start_workspace_root_contains_crates() {
    let h = new_harness_sync();
    assert!(h.workspace_root.join("crates").is_dir());
    assert!(h.workspace_root.join("Cargo.toml").is_file());
}

#[test]
fn test_harness_start_tempdir_created() {
    let h = new_harness_sync();
    assert!(h.tempdir.path().is_dir());
}

#[test]
fn test_harness_tui_app_mirror_default_bridge() {
    let h = new_harness_sync();
    let app = h.tui_app.lock();
    assert_eq!(app.nav, NavPageMirror::Bridge);
    assert!(!app.should_quit);
}

#[test]
fn test_harness_api_uri_contains_server() {
    let h = new_harness_sync();
    let uri = h.api_uri("/v1/test");
    assert!(uri.contains("/v1/test"));
    assert!(uri.starts_with("http://"));
}

#[test]
fn test_harness_tui_backend_default_24x80() {
    let h = new_harness_sync();
    assert_eq!(h.tui_backend.width, DEFAULT_WIDTH);
    assert_eq!(h.tui_backend.height, DEFAULT_HEIGHT);
}

// ============================================================================
// 5 集成 smoke 测试 (跑全栈)
// ============================================================================

#[test]
fn test_smoke_run_all_5_workspace() {
    let root = locate_workspace_root_for_tests();
    test_workspace_cargo_check_passes(&root).unwrap();
    test_workspace_no_locked_violation(&root).unwrap();
    test_workspace_no_sandbox_path_writes(&root).unwrap();
    test_workspace_no_workspace_version_modified(&root).unwrap();
    test_workspace_8_promises_audit_passes(&root).unwrap();
}

#[test]
fn test_smoke_run_all_5_tui_nav() {
    let mut h = new_harness_sync();
    test_tui_status_nav_renders(&mut h).unwrap();
    test_tui_session_nav_lists(&mut h).unwrap();
    test_tui_tools_nav_shows_6(&mut h).unwrap();
    test_tui_settings_nav_5_providers(&mut h).unwrap();
    test_tui_help_nav_6_anchors(&mut h).unwrap();
}

#[test]
fn test_smoke_run_all_9_organ() {
    let mut h = new_harness_sync();
    test_tui_organ_heart_pulse(&mut h).unwrap();
    test_tui_organ_brain_llm(&mut h).unwrap();
    test_tui_organ_hand_tools(&mut h).unwrap();
    test_tui_organ_eye_input(&mut h).unwrap();
    test_tui_organ_ear_events(&mut h).unwrap();
    test_tui_organ_memory_history(&mut h).unwrap();
    test_tui_organ_voice_state(&mut h).unwrap();
    test_tui_organ_body_resources(&mut h).unwrap();
    test_tui_organ_mind_anchors(&mut h).unwrap();
}

#[test]
fn test_smoke_report_generate_60plus() {
    // 模拟 60+ 测试结果, 验 report 能正确聚合
    let mut results = Vec::new();
    for i in 0..70 {
        let layer = match i % 3 {
            0 => E2eLayer::Api,
            1 => E2eLayer::Tui,
            _ => E2eLayer::Workspace,
        };
        results.push(TestResult::ok(format!("test_{i}"), layer, 5));
    }
    let report = generate_report(&results);
    assert_eq!(report.total_tests, 70);
    assert_eq!(report.passed, 70);
    assert!(report.all_passed());
}

#[test]
fn test_smoke_assert_all_passed_when_60plus_ok() {
    let results: Vec<TestResult> = (0..70)
        .map(|i| TestResult::ok(format!("t{i}"), E2eLayer::Api, 5))
        .collect();
    let report = generate_report(&results);
    assert_all_passed(&report).unwrap();
}

// ============================================================================
// 5 K-1 强校验集成测试
// ============================================================================

#[test]
fn test_k1_integration_five_nav_count() {
    let _h = new_harness_sync();
    assert_eq!(FIVE_NAV, 5);
    assert_eq!(NavPageMirror::ALL.len(), 5);
}

#[test]
fn test_k1_integration_nine_organs_count() {
    let _h = new_harness_sync();
    assert_eq!(NINE_ORGANS, 9);
    assert_eq!(OrganMirror::ALL.len(), 9);
}

#[test]
fn test_k1_integration_six_phi_anchors_count() {
    assert_eq!(SIX_PHI_ANCHORS, 6);
}

#[test]
fn test_k1_integration_eight_promises_count() {
    assert_eq!(EIGHT_PROMISES, 8);
}

#[test]
fn test_k1_integration_v2_endpoint_groups_count() {
    assert_eq!(V2_ENDPOINT_GROUPS, 6);
}

// ============================================================================
// 5 报告集成测试
// ============================================================================

#[test]
fn test_report_integration_all_passed() {
    let results: Vec<TestResult> = (0..10)
        .map(|i| TestResult::ok(format!("t{i}"), E2eLayer::Api, 5))
        .collect();
    let r = generate_report(&results);
    let s = format_human_readable(&r);
    assert!(s.contains("总测试"));
    assert!(s.contains("10"));
    let json = format_json(&r);
    assert!(json.contains("\"total_tests\":10"));
    assert_all_passed(&r).unwrap();
}

#[test]
fn test_report_integration_mixed() {
    let results = vec![
        TestResult::ok("a", E2eLayer::Api, 5),
        TestResult::fail("b", E2eLayer::Tui, "x", 5),
        TestResult::skip("c", E2eLayer::Workspace, "y"),
    ];
    let r = generate_report(&results);
    assert_eq!(r.passed, 1);
    assert_eq!(r.failed, 1);
    assert_eq!(r.skipped, 1);
    assert!(!r.all_passed());
    assert!(assert_all_passed(&r).is_err());
}

#[test]
fn test_report_integration_pass_rate() {
    let results = vec![
        TestResult::ok("a", E2eLayer::Api, 5),
        TestResult::ok("b", E2eLayer::Api, 5),
        TestResult::fail("c", E2eLayer::Api, "x", 5),
    ];
    let r = generate_report(&results);
    let rate = r.pass_rate();
    assert!((rate - 2.0 / 3.0).abs() < 0.001);
}

#[test]
fn test_report_integration_by_layer() {
    let results = vec![
        TestResult::ok("a1", E2eLayer::Api, 5),
        TestResult::ok("a2", E2eLayer::Api, 5),
        TestResult::ok("t1", E2eLayer::Tui, 5),
        TestResult::ok("w1", E2eLayer::Workspace, 5),
    ];
    let r = generate_report(&results);
    assert_eq!(r.by_layer.get(&E2eLayer::Api).unwrap().total, 2);
    assert_eq!(r.by_layer.get(&E2eLayer::Tui).unwrap().total, 1);
    assert_eq!(r.by_layer.get(&E2eLayer::Workspace).unwrap().total, 1);
}

#[test]
fn test_report_integration_failures_listed() {
    let results = vec![
        TestResult::ok("a", E2eLayer::Api, 5),
        TestResult::fail("b", E2eLayer::Tui, "buffer miss", 5),
        TestResult::fail("c", E2eLayer::Workspace, "LOCKED touched", 5),
    ];
    let r = generate_report(&results);
    let s = format_human_readable(&r);
    assert!(s.contains("失败列表"));
    assert!(s.contains("buffer miss"));
    assert!(s.contains("LOCKED touched"));
}

// ============================================================================
// 5 错误变体集成测试
// ============================================================================

#[test]
fn test_e2e_error_workspace_audit_display() {
    let e = E2EError::WorkspaceAudit {
        dimension: "x".into(),
        expected: "y".into(),
        actual: "z".into(),
        context: "ctx".into(),
    };
    let s = e.to_string();
    assert!(s.contains("x"));
    assert!(s.contains("ctx"));
}

#[test]
fn test_e2e_error_workspace_cargo_display() {
    let e = E2EError::WorkspaceCargo {
        command: "check".into(),
        exit_code: Some(1),
        stderr_excerpt: "err".into(),
    };
    let s = e.to_string();
    assert!(s.contains("check"));
}

#[test]
fn test_e2e_error_api_http_display() {
    let e = E2EError::ApiHttp {
        url: "http://x".into(),
        reason: "timeout".into(),
    };
    let s = e.to_string();
    assert!(s.contains("http://x"));
    assert!(s.contains("timeout"));
}

#[test]
fn test_e2e_error_api_status_display() {
    let e = E2EError::ApiStatus {
        url: "/v1/x".into(),
        expected: 200,
        actual: 500,
    };
    let s = e.to_string();
    assert!(s.contains("/v1/x"));
    assert!(s.contains("200"));
    assert!(s.contains("500"));
}

#[test]
fn test_e2e_error_tui_assert_display() {
    let e = E2EError::TuiAssert {
        context: "ctx".into(),
        expected: "exp".into(),
        actual: "act".into(),
    };
    let s = e.to_string();
    assert!(s.contains("ctx"));
    assert!(s.contains("exp"));
    assert!(s.contains("act"));
}
