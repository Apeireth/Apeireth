//! # `test_tui_dashboard` — 9 器官 + 3 端点集成测试
//!
//! 跟 sister #1 (借鉴 #1 9 器官 command 模式) + sister #6 (借鉴 #6 SharedState 模式)
//! + observability 3 端点 三方集成验证.
//!
//! **9 器官状态写入 + 读取 + 仪表盘渲染 mock** (per task spec §6 集成测试).
//!
//! ## 测试矩阵
//!
//! | 类别 | 数量 | 描述 |
//! |---|---:|------|
//! | K-1 强校验 | 5 | platform / organ count / 6 anchor / 5 nav / 3 endpoint |
//! | OrganKind enum | 4 | 9 器官 roundtrip / names_zh / ascii_chars / sister 同步 |
//! | TuiOrganState | 3 | stub / partial / ok 三种 readiness |
//! | OrganDashboard | 7 | new / register / read / register 9 / health / set_nav / unknown |
//! | Widget render | 3 | 9 器官覆盖 / mind 含 6 锚 / format 一致 |
//! | Dashboard render | 2 | 9 器官 + 3 端点 / mind 含 6 锚 |
//! | 跨模块 mock | 2 | 9 organ 注册 + dashboard 渲染端到端 |
//!
//! **总计 26 集成测试**.

use apeireth_observability::{
    render_dashboard, render_organ_widget, HealthResponse, HealthStatus, OrganDashboard,
    OrganKind, OrganReadiness, TuiOrganState, DASHBOARD_HEALTH_ENDPOINTS, FIVE_NAV,
    ORGAN_KIND_ASCII_CHARS, ORGAN_KIND_COUNT, ORGAN_KIND_NAMES_ZH, SIX_ANCHORS,
    TUI_DASHBOARD_PLATFORM, TUI_DASHBOARD_SCHEMA_VERSION,
};

// ============================================================================
// K-1 强校验 (5 项, per task spec §6)
// ============================================================================

#[test]
fn k1_platform_name_apeireth() {
    assert_eq!(TUI_DASHBOARD_PLATFORM, "apeireth", "K-1 强校验: 平台名");
}

#[test]
fn k1_organ_count_is_9() {
    assert_eq!(ORGAN_KIND_COUNT, 9, "K-1 强校验: 9 器官");
    assert_eq!(OrganKind::all().len(), 9, "OrganKind::all() 返回 9");
}

#[test]
fn k1_six_anchors_hardcoded() {
    assert_eq!(SIX_ANCHORS.len(), 6, "K-1 强校验: 6 哲学锚");
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("S-1")));
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("S-2")));
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("O-2")));
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("O-3")));
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("O-4")));
    assert!(SIX_ANCHORS.iter().any(|a| a.contains("O-5")));
}

#[test]
fn k1_five_nav_hardcoded() {
    assert_eq!(FIVE_NAV.len(), 5, "K-1 强校验: 5 nav (主人 R19 决定)");
    assert!(FIVE_NAV.iter().any(|n| n.contains("舰桥")));
    assert!(FIVE_NAV.iter().any(|n| n.contains("对话")));
    assert!(FIVE_NAV.iter().any(|n| n.contains("生长")));
    assert!(FIVE_NAV.iter().any(|n| n.contains("历史")));
    assert!(FIVE_NAV.iter().any(|n| n.contains("设置")));
}

#[test]
fn k1_3_health_endpoints_hardcoded() {
    assert_eq!(DASHBOARD_HEALTH_ENDPOINTS.len(), 3, "K-1 强校验: 3 端点");
    assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/health"));
    assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/ready"));
    assert!(DASHBOARD_HEALTH_ENDPOINTS.contains(&"/metrics"));
}

// ============================================================================
// OrganKind enum (4 项)
// ============================================================================

#[test]
fn organ_kind_from_u8_all_9_roundtrip() {
    for v in 0u8..=8 {
        let organ = OrganKind::from_u8(v).expect("0-8 必 valid");
        assert_eq!(organ.as_u8(), v, "roundtrip 0-8");
    }
    assert!(OrganKind::from_u8(9).is_none());
    assert!(OrganKind::from_u8(100).is_none());
    assert!(OrganKind::from_u8(255).is_none());
}

#[test]
fn organ_kind_names_zh_match_sister_reports_1_to_1() {
    // 跟 sister #1 (apeireth-tui/src/organ/mod.rs) + sister #6 (apeireth-state::organ)
    // 1:1 镜像 (LOCKED 边界同步, 编译期守门).
    assert_eq!(ORGAN_KIND_NAMES_ZH[0], "心");
    assert_eq!(ORGAN_KIND_NAMES_ZH[1], "脑");
    assert_eq!(ORGAN_KIND_NAMES_ZH[2], "手");
    assert_eq!(ORGAN_KIND_NAMES_ZH[3], "眼");
    assert_eq!(ORGAN_KIND_NAMES_ZH[4], "耳");
    assert_eq!(ORGAN_KIND_NAMES_ZH[5], "记忆");
    assert_eq!(ORGAN_KIND_NAMES_ZH[6], "声");
    assert_eq!(ORGAN_KIND_NAMES_ZH[7], "体");
    assert_eq!(ORGAN_KIND_NAMES_ZH[8], "意");
}

#[test]
fn organ_kind_ascii_chars_match_sister_reports_1_to_1() {
    assert_eq!(ORGAN_KIND_ASCII_CHARS[0], "[♥]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[1], "[BRAIN]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[2], "[HAND]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[3], "[EYE]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[4], "[EAR]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[5], "[MEM]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[6], "[VOICE]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[7], "[BODY]");
    assert_eq!(ORGAN_KIND_ASCII_CHARS[8], "[MIND]");
}

#[test]
fn organ_kind_as_str_lowercase() {
    assert_eq!(OrganKind::Heart.as_str(), "heart");
    assert_eq!(OrganKind::Brain.as_str(), "brain");
    assert_eq!(OrganKind::Hand.as_str(), "hand");
    assert_eq!(OrganKind::Eye.as_str(), "eye");
    assert_eq!(OrganKind::Ear.as_str(), "ear");
    assert_eq!(OrganKind::Memory.as_str(), "memory");
    assert_eq!(OrganKind::Voice.as_str(), "voice");
    assert_eq!(OrganKind::Body.as_str(), "body");
    assert_eq!(OrganKind::Mind.as_str(), "mind");
}

// ============================================================================
// TuiOrganState (3 项)
// ============================================================================

#[test]
fn tui_organ_state_stub_marks_stub() {
    let state = TuiOrganState::stub(OrganKind::Heart);
    assert_eq!(state.organ, OrganKind::Heart);
    assert_eq!(state.readiness, OrganReadiness::Stub);
    assert_eq!(state.value, 0.0);
    assert!(state.message.contains("stub"));
}

#[test]
fn tui_organ_state_partial_marks_partial() {
    let state = TuiOrganState::partial(OrganKind::Body);
    assert_eq!(state.readiness, OrganReadiness::Partial);
    assert!(state.message.contains("partial"));
}

#[test]
fn tui_organ_state_ok_marks_ok() {
    let state = TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz");
    assert_eq!(state.readiness, OrganReadiness::Ok);
    assert_eq!(state.value, 60.0);
    assert_eq!(state.message, "60Hz");
}

// ============================================================================
// OrganDashboard (7 项)
// ============================================================================

#[test]
fn organ_dashboard_new_has_9_stub_states_and_3_healthy_endpoints() {
    let dash = OrganDashboard::new();
    for organ in OrganKind::all() {
        let state = dash.read_organ_state(organ).expect("read");
        assert_eq!(state.readiness, OrganReadiness::Stub, "new() 全 stub");
    }
    for ep in DASHBOARD_HEALTH_ENDPOINTS {
        let h = dash.read_health(ep).expect("read health");
        assert_eq!(h.status, HealthStatus::Healthy, "new() 全 healthy");
    }
}

#[test]
fn organ_dashboard_register_and_read_single_organ() {
    let dash = OrganDashboard::new();
    let state = TuiOrganState::ok(OrganKind::Heart, 75.0, "75Hz");
    dash.register_tui_organ_state(OrganKind::Heart, state)
        .expect("register heart");
    let read = dash.read_organ_state(OrganKind::Heart).expect("read heart");
    assert_eq!(read.value, 75.0);
    assert_eq!(read.readiness, OrganReadiness::Ok);
    assert_eq!(read.message, "75Hz");
}

#[test]
fn organ_dashboard_register_all_9_organs_independently() {
    let dash = OrganDashboard::new();
    for organ in OrganKind::all() {
        let state = TuiOrganState::ok(organ, organ.as_u8() as f64, format!("{}_ok", organ.as_str()));
        dash.register_tui_organ_state(organ, state).expect("register");
    }
    for organ in OrganKind::all() {
        let read = dash.read_organ_state(organ).expect("read");
        assert_eq!(read.value, organ.as_u8() as f64, "{} value 独立", organ.as_str());
    }
}

#[test]
fn organ_dashboard_update_health_degraded() {
    let dash = OrganDashboard::new();
    let resp = HealthResponse::new("/ready", HealthStatus::Degraded)
        .with_detail("reason", "stub: bus not started");
    dash.update_health("/ready", resp).expect("update");
    let read = dash.read_health("/ready").expect("read");
    assert_eq!(read.status, HealthStatus::Degraded);
    assert_eq!(read.details.get("reason").map(|s| s.as_str()), Some("stub: bus not started"));
}

#[test]
fn organ_dashboard_rejects_unknown_health_endpoint() {
    let dash = OrganDashboard::new();
    let resp = HealthResponse::new("/unknown", HealthStatus::Healthy);
    let result = dash.update_health("/unknown", resp);
    assert!(result.is_err(), "未知端点必拒绝");
}

#[test]
fn organ_dashboard_set_and_read_nav_0_to_4() {
    let dash = OrganDashboard::new();
    for nav in 0u8..=4 {
        dash.set_current_nav(nav).expect("set 0-4");
        assert_eq!(dash.current_nav(), nav);
    }
    assert!(dash.set_current_nav(5).is_err());
    assert!(dash.set_current_nav(100).is_err());
}

#[test]
fn organ_dashboard_read_all_organ_states_returns_9() {
    let dash = OrganDashboard::new();
    let all = dash.read_all_organ_states();
    assert_eq!(all.len(), 9);
    for organ in OrganKind::all() {
        assert_eq!(all[organ.as_u8() as usize].organ, organ);
    }
}

// ============================================================================
// Widget render (3 项)
// ============================================================================

#[test]
fn render_9_organ_widgets_contain_organ_name() {
    let states = [
        TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz"),
        TuiOrganState::ok(OrganKind::Brain, 42.0, "calls=42"),
        TuiOrganState::ok(OrganKind::Hand, 12.0, "invokes=12"),
        TuiOrganState::stub(OrganKind::Eye),
        TuiOrganState::stub(OrganKind::Ear),
        TuiOrganState::ok(OrganKind::Memory, 24.0, "episodes=24"),
        TuiOrganState::stub(OrganKind::Voice),
        TuiOrganState::partial(OrganKind::Body),
        TuiOrganState::partial(OrganKind::Mind),
    ];
    for organ in OrganKind::all() {
        let s = render_organ_widget(organ, &states[organ.as_u8() as usize]);
        assert!(s.contains(organ.as_str()), "{} widget must contain organ name", organ.as_str());
        assert!(s.contains(organ.name_zh()), "{} widget must contain zh name", organ.as_str());
        assert!(s.contains(organ.ascii_char()), "{} widget must contain ascii char", organ.as_str());
    }
}

#[test]
fn render_mind_widget_includes_six_anchors() {
    let state = TuiOrganState::partial(OrganKind::Mind);
    let s = render_organ_widget(OrganKind::Mind, &state);
    for anchor in SIX_ANCHORS {
        assert!(s.contains(anchor), "mind widget must include anchor: {}", anchor);
    }
}

#[test]
fn render_widgets_use_consistent_format() {
    let state = TuiOrganState::ok(OrganKind::Heart, 60.0, "60Hz");
    let s = render_organ_widget(OrganKind::Heart, &state);
    // 格式: [♥] 心 heart    bpm= 60.0   ok        60Hz
    assert!(s.contains("[♥]"));
    assert!(s.contains("心"));
    assert!(s.contains("heart"));
    assert!(s.contains("bpm= 60.0"));
    assert!(s.contains("ok"));
    assert!(s.contains("60Hz"));
}

// ============================================================================
// Dashboard render (2 项)
// ============================================================================

#[test]
fn render_dashboard_includes_all_9_organs_and_3_endpoints_and_current_nav() {
    let dash = OrganDashboard::new();
    let s = render_dashboard(&dash);
    // 9 器官
    for organ in OrganKind::all() {
        assert!(s.contains(organ.as_str()), "dashboard must include {}", organ.as_str());
    }
    // 3 端点
    for ep in DASHBOARD_HEALTH_ENDPOINTS {
        assert!(s.contains(ep), "dashboard must include {}", ep);
    }
    // 5 nav (current 0 = 舰桥)
    assert!(s.contains("舰桥"), "dashboard must include current nav");
    // schema version
    assert!(s.contains(TUI_DASHBOARD_SCHEMA_VERSION));
}

#[test]
fn render_dashboard_mind_widget_includes_six_anchors() {
    let dash = OrganDashboard::new();
    let s = render_dashboard(&dash);
    for anchor in SIX_ANCHORS {
        assert!(s.contains(anchor), "dashboard must include anchor: {}", anchor);
    }
}

// ============================================================================
// 跨模块 mock (2 项, per task spec §6 集成测试)
// ============================================================================

#[test]
fn integration_end_to_end_9_organ_register_read_render() {
    // 1. 创建 dashboard
    let dash = OrganDashboard::new();

    // 2. 注册 9 器官 (混合 ok/partial/stub, 1:1 镜像 sister #1 实接度)
    let test_states = vec![
        (OrganKind::Heart, OrganReadiness::Ok, 60.0, "60Hz CPU"),
        (OrganKind::Brain, OrganReadiness::Ok, 42.0, "42 LLM calls"),
        (OrganKind::Hand, OrganReadiness::Ok, 12.0, "12 tool invocations"),
        (OrganKind::Eye, OrganReadiness::Stub, 0.0, "stub: 0 真接"),
        (OrganKind::Ear, OrganReadiness::Stub, 0.0, "stub: 0 真接"),
        (OrganKind::Memory, OrganReadiness::Ok, 24.0, "24 episodes"),
        (OrganKind::Voice, OrganReadiness::Stub, 0.0, "stub: 0 真接"),
        (OrganKind::Body, OrganReadiness::Partial, 0.0, "partial: 0/6"),
        (OrganKind::Mind, OrganReadiness::Partial, 0.85, "partial: seed"),
    ];
    for (organ, readiness, value, msg) in &test_states {
        let state = TuiOrganState::new(*organ, *readiness, *value, *msg);
        dash.register_tui_organ_state(*organ, state).expect("register");
    }

    // 3. 更新 3 health 端点
    dash.update_health(
        "/health",
        HealthResponse::new("/health", HealthStatus::Healthy),
    ).expect("update /health");
    dash.update_health(
        "/ready",
        HealthResponse::new("/ready", HealthStatus::Degraded)
            .with_detail("reason", "stub"),
    ).expect("update /ready");
    dash.update_health(
        "/metrics",
        HealthResponse::new("/metrics", HealthStatus::Healthy)
            .with_detail("format", "prometheus"),
    ).expect("update /metrics");

    // 4. 设置 nav
    dash.set_current_nav(2).expect("set nav 2 (生长)");
    assert_eq!(dash.current_nav(), 2);

    // 5. 验证 9 器官读取
    for (organ, expected_readiness, expected_value, expected_msg) in &test_states {
        let read = dash.read_organ_state(*organ).expect("read");
        assert_eq!(read.readiness, *expected_readiness, "{} readiness", organ.as_str());
        assert_eq!(read.value, *expected_value, "{} value", organ.as_str());
        assert_eq!(read.message, *expected_msg, "{} message", organ.as_str());
    }

    // 6. 渲染 dashboard mock
    let rendered = render_dashboard(&dash);
    assert!(rendered.contains("schema: 1"));
    assert!(rendered.contains("生长"), "current nav 2 = 生长");
    for organ in OrganKind::all() {
        assert!(rendered.contains(organ.as_str()));
    }
    for ep in DASHBOARD_HEALTH_ENDPOINTS {
        assert!(rendered.contains(ep));
    }
    assert!(rendered.contains("Degraded"), "/ready 是 Degraded");
}

#[test]
fn integration_thread_safety_register_concurrent() {
    use std::sync::Arc;
    use std::thread;

    // 验证 OrganDashboard 跨线程安全 (Arc<Mutex<...>> 守门).
    let dash = Arc::new(OrganDashboard::new());
    let mut handles = vec![];

    for i in 0..9 {
        let dash_clone = Arc::clone(&dash);
        let organ = OrganKind::from_u8(i as u8).expect("0-8");
        let handle = thread::spawn(move || {
            let state = TuiOrganState::ok(organ, i as f64, format!("thread_{i}"));
            dash_clone
                .register_tui_organ_state(organ, state)
                .expect("register in thread");
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread join");
    }

    // 验证 9 器官都注册成功
    for i in 0..9 {
        let organ = OrganKind::from_u8(i as u8).expect("0-8");
        let read = dash.read_organ_state(organ).expect("read");
        assert_eq!(read.value, i as f64, "thread {i} register");
        assert_eq!(read.message, format!("thread_{i}"));
    }
}
