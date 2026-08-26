//! # `tui_dashboard_demo` — 9 器官 + 3 端点端到端例子
//!
//! 跑这个例子:
//! ```bash
//! cargo run -p apeireth-observability --example tui_dashboard_demo
//! ```
//!
//! 演示内容 (per task spec §7 端到端例子):
//! 1. 创建 `OrganDashboard` (9 器官 + 3 端点)
//! 2. 注册 9 器官状态 (混合 ok/partial/stub 实接度, 1:1 镜像 sister #1 报告)
//! 3. 更新 3 health 端点 (Healthy / Degraded / Healthy)
//! 4. 设置当前 nav (0 舰桥)
//! 5. 渲染 9 器官 widget (逐个)
//! 6. 渲染整体 dashboard (9 器官 + 3 端点 + nav)
//! 7. 验证 9 器官 enum + 6 哲学锚 + 5 nav + 3 端点 K-1 强校验
//!
//! **不假装** (per 6 哲学锚 O-5): 输出明标 "stub"/"partial" 实接度, 0 编造 "已真接"。

use apeireth_observability::{
    render_dashboard, render_organ_widget, HealthResponse, HealthStatus, OrganDashboard,
    OrganKind, OrganReadiness, TuiOrganState, FIVE_NAV, ORGAN_KIND_COUNT,
    ORGAN_KIND_NAMES_ZH, ORGAN_KIND_ASCII_CHARS, SIX_ANCHORS, TUI_DASHBOARD_PLATFORM,
    TUI_DASHBOARD_SCHEMA_VERSION,
};

fn main() {
    println!("=== Apeireth TUI Dashboard Demo (R25.2 skeleton) ===\n");

    // === §1 编译期守门 (K-1 强校验) ===
    println!("--- §1 K-1 强校验 (编译期 hardcode) ---");
    println!("ORGAN_KIND_COUNT = {ORGAN_KIND_COUNT} (期望 9)");
    assert_eq!(ORGAN_KIND_COUNT, 9, "K-1 强校验 #2: 9 organ");
    println!("SIX_ANCHORS.len = {} (期望 6)", SIX_ANCHORS.len());
    assert_eq!(SIX_ANCHORS.len(), 6, "K-1 强校验: 6 锚");
    println!("FIVE_NAV.len = {} (期望 5)", FIVE_NAV.len());
    assert_eq!(FIVE_NAV.len(), 5, "K-1 强校验: 5 nav");
    println!("TUI_DASHBOARD_PLATFORM = {TUI_DASHBOARD_PLATFORM}");
    assert_eq!(TUI_DASHBOARD_PLATFORM, "apeireth", "K-1 强校验: platform");
    println!("TUI_DASHBOARD_SCHEMA_VERSION = {TUI_DASHBOARD_SCHEMA_VERSION}");
    println!("[OK] §1 全部 K-1 强校验通过\n");

    // === §2 9 器官名 + ASCII (编译期守门) ===
    println!("--- §2 9 器官 (跟 sister #1 + sister #6 1:1 镜像) ---");
    for (i, organ) in OrganKind::all().iter().enumerate() {
        println!(
            "  [{i}] {ascii:<8} {zh:<4} {en:<7} (OrganKind::{en:?})",
            i = i,
            ascii = ORGAN_KIND_ASCII_CHARS[i],
            zh = ORGAN_KIND_NAMES_ZH[i],
            en = organ.as_str()
        );
    }
    println!();

    // === §3 创建 dashboard (9 stub + 3 healthy) ===
    println!("--- §3 创建 OrganDashboard (9 字段全 stub, 3 端点全 healthy) ---");
    let dashboard = OrganDashboard::new();
    println!("[OK] dashboard created, 9 organs (stub) + 3 endpoints (healthy)\n");

    // === §4 注册 9 器官状态 (混合 ok/partial/stub, 1:1 镜像 sister #1 报告) ===
    println!("--- §4 注册 9 器官状态 ---");
    let organ_states = [
        // Heart (心) — sister #1 partial, 本例升级为 ok
        TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz CPU heartbeat"),
        // Brain (脑) — sister #1 partial, 本例 ok
        TuiOrganState::ok(OrganKind::Brain, 42.0, "42 LLM calls, active provider: minimax (1/5)"),
        // Hand (手) — sister #1 partial, 本例 ok
        TuiOrganState::ok(OrganKind::Hand, 12.0, "12 tool invocations, whitelist: 6"),
        // Eye (眼) — sister #1 stub, 本例 stub (0 真接)
        TuiOrganState::stub(OrganKind::Eye),
        // Ear (耳) — sister #1 stub, 本例 stub
        TuiOrganState::stub(OrganKind::Ear),
        // Memory (记忆) — sister #1 partial, 本例 ok
        TuiOrganState::ok(OrganKind::Memory, 24.0, "24 episodes in history"),
        // Voice (声) — sister #1 stub, 本例 stub
        TuiOrganState::stub(OrganKind::Voice),
        // Body (体) — sister #1 partial, 本例 partial (5/6 placeholder)
        TuiOrganState::partial(OrganKind::Body),
        // Mind (意) — sister #1 partial, 本例 partial
        TuiOrganState::partial(OrganKind::Mind),
    ];
    for (i, state) in organ_states.iter().enumerate() {
        let organ = OrganKind::all()[i];
        dashboard
            .register_tui_organ_state(organ, state.clone())
            .expect("register 0-8");
        println!(
            "  [{}] {:?} readiness={:?} value={}",
            i, organ, state.readiness, state.value
        );
    }
    println!();

    // === §5 更新 3 health 端点 (1 degraded, 2 healthy) ===
    println!("--- §5 更新 3 health 端点 ---");
    dashboard
        .update_health(
            "/health",
            HealthResponse::new("/health", HealthStatus::Healthy)
                .with_detail("platform", TUI_DASHBOARD_PLATFORM)
                .with_detail("schema_version", TUI_DASHBOARD_SCHEMA_VERSION),
        )
        .expect("update /health");
    println!("  /health    -> Healthy (200)");

    dashboard
        .update_health(
            "/ready",
            HealthResponse::new("/ready", HealthStatus::Degraded)
                .with_detail("reason", "stub: observability bus not started (R25.3 真接)"),
        )
        .expect("update /ready");
    println!("  /ready     -> Degraded (200 + reason, 0 真接)");

    dashboard
        .update_health(
            "/metrics",
            HealthResponse::new("/metrics", HealthStatus::Healthy)
                .with_detail("format", "prometheus text/plain"),
        )
        .expect("update /metrics");
    println!("  /metrics   -> Healthy (200 + prometheus)\n");

    // === §6 设置当前 nav ===
    println!("--- §6 设置当前 nav ---");
    dashboard.set_current_nav(0).expect("set nav 0");
    println!("  current_nav = {} (舰桥 Bridge)\n", dashboard.current_nav());

    // === §7 渲染 9 器官 widget (逐个) ===
    println!("--- §7 渲染 9 器官 widget (逐个) ---");
    for (i, organ) in OrganKind::all().iter().enumerate() {
        let s = render_organ_widget(*organ, &organ_states[i]);
        println!("{s}");
    }
    println!();

    // === §8 渲染整体 dashboard (9 器官 + 3 端点 + nav) ===
    println!("--- §8 渲染整体 dashboard ---");
    let rendered = render_dashboard(&dashboard);
    println!("{rendered}");
    println!();

    // === §9 6 哲学锚 (mind 器官 widget 显示) ===
    println!("--- §9 6 哲学锚 (mind 器官 widget 包含) ---");
    for (i, anchor) in SIX_ANCHORS.iter().enumerate() {
        println!("  [{i}] {anchor}");
    }
    println!();

    println!("=== Demo 完. (R25.2 skeleton, 0 commit, 留 Mavis 整合 #3) ===");
}
