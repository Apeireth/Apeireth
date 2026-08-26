//! # `integration_e2e_demo` — 跑全部 e2e 测试 + 生成报告
//!
//! **职责**: 启动 IntegrationHarness, 跑全部 41 测试 (5 workspace + 21 API + 15 TUI), 输出 human-readable + JSON 报告.
//!
//! **跑法**:
//! ```bash
//! cd crates/apeireth-integration-e2e
//! cargo run --example integration_e2e_demo
//! ```
//!
//! **8 不修改承诺**: 跟 src/ 一致

use std::time::Instant;

use apeireth_integration_e2e::prelude::*;
use apeireth_integration_e2e::*;

// 同步 helper: 在 sync 上下文跑 async
fn tokio_block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn main() {
    println!("=== Apeireth 集成测试 e2e 演示 ===\n");

    let mut results: Vec<TestResult> = Vec::new();

    // 1. workspace e2e (5 测试)
    println!("--- 1. Workspace e2e (5 测试) ---");
    let root = locate_workspace_root();
    let workspace_results: Vec<(&str, bool, String, u64)> = vec![
        run_workspace(
            "test_workspace_cargo_check_passes",
            test_workspace_cargo_check_passes(&root),
        ),
        run_workspace(
            "test_workspace_no_locked_violation",
            test_workspace_no_locked_violation(&root),
        ),
        run_workspace(
            "test_workspace_no_sandbox_path_writes",
            test_workspace_no_sandbox_path_writes(&root),
        ),
        run_workspace(
            "test_workspace_no_workspace_version_modified",
            test_workspace_no_workspace_version_modified(&root),
        ),
        run_workspace(
            "test_workspace_8_promises_audit_passes",
            test_workspace_8_promises_audit_passes(&root),
        ),
    ];
    for (name, ok, msg, elapsed) in &workspace_results {
        if *ok {
            println!("  ✓ {name} ({elapsed} ms)");
        } else {
            println!("  ✗ {name}: {msg}");
        }
    }
    for (name, ok, msg, elapsed) in &workspace_results {
        if *ok {
            results.push(TestResult::ok(*name, E2eLayer::Workspace, *elapsed));
        } else {
            results.push(TestResult::fail(
                *name,
                E2eLayer::Workspace,
                msg.clone(),
                *elapsed,
            ));
        }
    }

    // 2. API e2e (21 测试)
    println!("\n--- 2. API e2e (21 测试) ---");
    tokio_block_on(async {
        let mut h = IntegrationHarness::start().await.unwrap();
        let api_results: Vec<(&str, bool, String, u64)> = vec![
            run_api(
                "test_api_metrics_endpoint_returns_prometheus",
                test_api_metrics_endpoint_returns_prometheus(&mut h).await,
            ),
            run_api(
                "test_api_health_endpoint_5_components",
                test_api_health_endpoint_5_components(&mut h).await,
            ),
            run_api(
                "test_api_status_endpoint_uptime",
                test_api_status_endpoint_uptime(&mut h).await,
            ),
            run_api(
                "test_api_tools_calendar_list",
                test_api_tools_calendar_list(&mut h).await,
            ),
            run_api(
                "test_api_tools_calendar_create",
                test_api_tools_calendar_create(&mut h).await,
            ),
            run_api(
                "test_api_tools_calendar_get",
                test_api_tools_calendar_get(&mut h).await,
            ),
            run_api(
                "test_api_tools_calendar_update",
                test_api_tools_calendar_update(&mut h).await,
            ),
            run_api(
                "test_api_tools_calendar_delete",
                test_api_tools_calendar_delete(&mut h).await,
            ),
            run_api(
                "test_api_tools_message_list",
                test_api_tools_message_list(&mut h).await,
            ),
            run_api(
                "test_api_tools_message_send",
                test_api_tools_message_send(&mut h).await,
            ),
            run_api(
                "test_api_tools_contact_list",
                test_api_tools_contact_list(&mut h).await,
            ),
            run_api(
                "test_api_tools_contact_create",
                test_api_tools_contact_create(&mut h).await,
            ),
            run_api(
                "test_api_tools_task_list",
                test_api_tools_task_list(&mut h).await,
            ),
            run_api(
                "test_api_tools_task_complete",
                test_api_tools_task_complete(&mut h).await,
            ),
            run_api(
                "test_api_tools_search_web",
                test_api_tools_search_web(&mut h).await,
            ),
            run_api(
                "test_api_tools_search_code",
                test_api_tools_search_code(&mut h).await,
            ),
            run_api(
                "test_api_unauthorized_returns_401",
                test_api_unauthorized_returns_401(&mut h).await,
            ),
            run_api(
                "test_api_not_found_returns_404",
                test_api_not_found_returns_404(&mut h).await,
            ),
            run_api(
                "test_api_server_error_returns_500",
                test_api_server_error_returns_500(&mut h).await,
            ),
            run_api(
                "test_api_websocket_8_frames",
                test_api_websocket_8_frames(&mut h).await,
            ),
            run_api(
                "test_api_rate_limit_enforced",
                test_api_rate_limit_enforced(&mut h).await,
            ),
        ];
        for (name, ok, msg, elapsed) in &api_results {
            if *ok {
                println!("  ✓ {name} ({elapsed} ms)");
            } else {
                println!("  ✗ {name}: {msg}");
            }
        }
        for (name, ok, msg, elapsed) in &api_results {
            if *ok {
                results.push(TestResult::ok(*name, E2eLayer::Api, *elapsed));
            } else {
                results.push(TestResult::fail(
                    *name,
                    E2eLayer::Api,
                    msg.clone(),
                    *elapsed,
                ));
            }
        }
        h.shutdown().await.unwrap();
    });

    // 3. TUI e2e (15 测试)
    println!("\n--- 3. TUI e2e (15 测试) ---");
    let mut h = tokio_block_on(IntegrationHarness::start()).unwrap();
    let tui_results: Vec<(&str, bool, String, u64)> = vec![
        run_tui(
            "test_tui_status_nav_renders",
            test_tui_status_nav_renders(&mut h),
        ),
        run_tui(
            "test_tui_session_nav_lists",
            test_tui_session_nav_lists(&mut h),
        ),
        run_tui(
            "test_tui_tools_nav_shows_6",
            test_tui_tools_nav_shows_6(&mut h),
        ),
        run_tui(
            "test_tui_settings_nav_5_providers",
            test_tui_settings_nav_5_providers(&mut h),
        ),
        run_tui(
            "test_tui_help_nav_6_anchors",
            test_tui_help_nav_6_anchors(&mut h),
        ),
        run_tui(
            "test_tui_organ_heart_pulse",
            test_tui_organ_heart_pulse(&mut h),
        ),
        run_tui("test_tui_organ_brain_llm", test_tui_organ_brain_llm(&mut h)),
        run_tui(
            "test_tui_organ_hand_tools",
            test_tui_organ_hand_tools(&mut h),
        ),
        run_tui("test_tui_organ_eye_input", test_tui_organ_eye_input(&mut h)),
        run_tui(
            "test_tui_organ_ear_events",
            test_tui_organ_ear_events(&mut h),
        ),
        run_tui(
            "test_tui_organ_memory_history",
            test_tui_organ_memory_history(&mut h),
        ),
        run_tui(
            "test_tui_organ_voice_state",
            test_tui_organ_voice_state(&mut h),
        ),
        run_tui(
            "test_tui_organ_body_resources",
            test_tui_organ_body_resources(&mut h),
        ),
        run_tui(
            "test_tui_organ_mind_anchors",
            test_tui_organ_mind_anchors(&mut h),
        ),
        run_tui("test_tui_quit_key_q", test_tui_quit_key_q(&mut h)),
    ];
    for (name, ok, msg, elapsed) in &tui_results {
        if *ok {
            println!("  ✓ {name} ({elapsed} ms)");
        } else {
            println!("  ✗ {name}: {msg}");
        }
    }
    for (name, ok, msg, elapsed) in &tui_results {
        if *ok {
            results.push(TestResult::ok(*name, E2eLayer::Tui, *elapsed));
        } else {
            results.push(TestResult::fail(
                *name,
                E2eLayer::Tui,
                msg.clone(),
                *elapsed,
            ));
        }
    }

    // 4. 报告
    println!("\n--- 4. 报告 ---");
    let report = generate_report(&results);
    println!("{}", format_human_readable(&report));
    println!("\n--- 5. JSON 报告 ---");
    println!("{}", format_json(&report));

    // 6. 守门
    match assert_all_passed(&report) {
        Ok(()) => {
            println!("\n✓ 全部 {} 测试通过, e2e 守门 OK", report.total_tests);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\n✗ e2e 守门失败: {e}");
            std::process::exit(1);
        }
    }
}

fn run_workspace(name: &str, result: E2EResult<()>) -> (&str, bool, String, u64) {
    let start = Instant::now();
    let r = result;
    let elapsed = start.elapsed().as_millis() as u64;
    match r {
        Ok(()) => (name, true, String::new(), elapsed),
        Err(e) => (name, false, e.to_string(), elapsed),
    }
}

fn run_api(name: &str, result: E2EResult<()>) -> (&str, bool, String, u64) {
    let start = Instant::now();
    let r = result;
    let elapsed = start.elapsed().as_millis() as u64;
    match r {
        Ok(()) => (name, true, String::new(), elapsed),
        Err(e) => (name, false, e.to_string(), elapsed),
    }
}

fn run_tui(name: &str, result: E2EResult<()>) -> (&str, bool, String, u64) {
    let start = Instant::now();
    let r = result;
    let elapsed = start.elapsed().as_millis() as u64;
    match r {
        Ok(()) => (name, true, String::new(), elapsed),
        Err(e) => (name, false, e.to_string(), elapsed),
    }
}

fn locate_workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = `crates/apeireth-integration-e2e/`, 需上 2 级到主仓根
    if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest);
        if let Some(grand) = p.parent().and_then(|x| x.parent()) {
            return grand.to_path_buf();
        }
    }
    std::env::current_dir().unwrap()
}
