//! 综合 Dashboard API endpoints (v2 reconstruction).
//!
//! **职责**: 把多个 Apeireth 后端模块的状态汇总显示 (cognition / perception /
//! consciousness / life-force / motivation / value).
//!
//! **v2 适配 vs v1**:
//! - v1 直接调 6 个 v1 器官 crate (`apeireth_cognition::run_cycle`,
//!   `apeireth_perception::ChannelKind`, `apeireth_consciousness::CognitiveDreamStateMachine`,
//!   `apeireth_life_force::LifeForce`, `apeireth_motivation::motivation_score`,
//!   `apeireth_value::ValueDimension::ALL`) — 这些 v1 crate 不在 v2 workspace.
//! - v2 没有这 6 个器官 crate (v2 用 apeireth-cognition / apeireth-perception 重写, 但
//!   是不同 crate name). 本模块生成 demo snapshot (诚实地标 "demo"), 保留 v1 同款
//!   pub API (DashboardPanel / DashboardState / collect_dashboard_state /
//!   render_dashboard_page / dashboard_handler / mount) — 后续 R19+ 接真器官后只改内部实现.
//!
//! **架构位置**:
//! - `GET /dashboard` — axum handler, SSR 渲染综合 dashboard HTML 页
//!
//! **诚实登记**: 6 panel 当前均为 demo snapshot (基于当前 minute_of_hour 派生数字),
//! 不接 LLM / 不持久化 state. 真实运行系统应从 global state 读 (R19+ 接入 supervisor).

#[cfg(feature = "ssr")]
use axum::{response::Html, routing::get, Router};

#[cfg(feature = "ssr")]
use crate::templates::html_escape;

// ============================================================
// DashboardState — 6 器官状态汇总
// ============================================================

/// 单个器官 panel 的标签 + 数据 + 描述
#[derive(Debug, Clone)]
pub struct DashboardPanel {
    /// 模块名 (cognition / perception / ...)
    pub module: &'static str,
    /// 中文显示名 (认知 / 感知 / ...)
    pub display_name: &'static str,
    /// emoji 图标
    pub icon: &'static str,
    /// 一句话状态摘要
    pub status_line: String,
    /// 关键数字 1
    pub key_metric_label: String,
    pub key_metric_value: String,
    /// 关键数字 2
    pub secondary_label: String,
    pub secondary_value: String,
    /// 关键数字 3 (可选, 副副指标)
    pub tertiary_label: String,
    pub tertiary_value: String,
    /// 备注 (诚实登记: real / demo / partial)
    pub data_source: &'static str,
}

/// 6 器官汇总
#[derive(Debug, Clone)]
pub struct DashboardState {
    pub cognition: DashboardPanel,
    pub perception: DashboardPanel,
    pub consciousness: DashboardPanel,
    pub life_force: DashboardPanel,
    pub motivation: DashboardPanel,
    pub value: DashboardPanel,
    /// snapshot 时间戳 (Unix seconds)
    pub snapshot_at: i64,
    /// continuity_id (跨 6 器官, 用于 audit)
    pub continuity_id: String,
}

// ============================================================
// 状态收集 — v2 demo snapshot (基于 now 派生确定性数字)
// ============================================================

fn snapshot_cognition(continuity_id: &str, now: i64) -> DashboardPanel {
    let _ = continuity_id;
    // 5 阶段: validate / score_v05 / score_v1136 / verdicts / decide (demo: 完成度派生)
    let minute_of_hour = (now / 60) % 60;
    let completion = (minute_of_hour % 5) + 1;  // 1..5
    let status = format!(
        "5 阶段认知循环, 完成度 {}/5 (v2 demo, minute_of_hour={})",
        completion, minute_of_hour
    );
    DashboardPanel {
        module: "cognition",
        display_name: "认知",
        icon: "🧠",
        status_line: status,
        key_metric_label: "current_stage".to_string(),
        key_metric_value: format!("{} / 5", completion),
        secondary_label: "next_stage".to_string(),
        secondary_value: format!("{} / 5", completion % 5 + 1),
        tertiary_label: "cycle_count".to_string(),
        tertiary_value: format!("{} 次/小时 (demo)", minute_of_hour / 5),
        data_source: "demo: 派生 minute_of_hour (v2 不接 apeireth_cognition)",
    }
}

fn snapshot_perception(_continuity_id: &str, now: i64) -> DashboardPanel {
    // 5 通道 (跟 v1 同款): text / vision / voice / tactile / command
    let channels = ["text", "vision", "voice", "tactile", "command"];
    let minute_of_hour = (now / 60) % 60;
    let active = minute_of_hour % 5;  // 当前活跃通道索引
    let status = format!(
        "5 通道感知, 当前活跃 = #{} {} (v2 demo)",
        active + 1, channels[active as usize]
    );
    DashboardPanel {
        module: "perception",
        display_name: "感知",
        icon: "👁️",
        status_line: status,
        key_metric_label: "active_channel".to_string(),
        key_metric_value: channels[active as usize].to_string(),
        secondary_label: "attention_threshold".to_string(),
        secondary_value: "0.50 (v1 default, demo)".to_string(),
        tertiary_label: "top_k".to_string(),
        tertiary_value: "10 (v1 default, demo)".to_string(),
        data_source: "demo: 5 通道硬编码 (v2 不接 apeireth_perception)",
    }
}

fn snapshot_consciousness(_continuity_id: &str, now: i64) -> DashboardPanel {
    // 5 状态: waking / focus / drift / dream / deep_sleep (demo 简化)
    let states = ["waking", "focus", "drift", "dream", "deep_sleep"];
    let minute_of_hour = (now / 60) % 60;
    let current = states[(minute_of_hour / 12) as usize % 5];
    let status = format!("意识状态机, 当前 = {} (v2 demo)", current);
    DashboardPanel {
        module: "consciousness",
        display_name: "意识",
        icon: "💭",
        status_line: status,
        key_metric_label: "current_state".to_string(),
        key_metric_value: current.to_string(),
        secondary_label: "transition_count".to_string(),
        secondary_value: format!("{} 次 (demo)", minute_of_hour / 12),
        tertiary_label: "dream_progress".to_string(),
        tertiary_value: format!("{}% (demo)", minute_of_hour % 100),
        data_source: "demo: 状态机派生 (v2 不接 apeireth_consciousness)",
    }
}

fn snapshot_life_force(_continuity_id: &str, now: i64) -> DashboardPanel {
    // endurance ∈ [0, 1], sgi (self-growth indicator) ∈ [0, 1]
    let minute_of_hour = (now / 60) % 60;
    let endurance = 0.7 + (minute_of_hour as f64 / 60.0) * 0.3;
    let sgi = 0.6 + ((minute_of_hour as f64).sin() * 0.2 + 0.2);
    let status = format!(
        "生命力 endurance={:.2} sgi={:.2} (v2 demo)",
        endurance, sgi
    );
    DashboardPanel {
        module: "life-force",
        display_name: "生命力",
        icon: "💪",
        status_line: status,
        key_metric_label: "endurance".to_string(),
        key_metric_value: format!("{:.2} / 1.00", endurance),
        secondary_label: "sgi (self-growth)".to_string(),
        secondary_value: format!("{:.2}", sgi),
        tertiary_label: "exhaustion_check".to_string(),
        tertiary_value: "✓ pass (demo)".to_string(),
        data_source: "demo: 派生 minute_of_hour (v2 不接 apeireth_life_force)",
    }
}

fn snapshot_motivation(_continuity_id: &str, now: i64) -> DashboardPanel {
    // motivation_score = weighted(autonomy, value, intrinsic) ∈ [0, 1]
    let minute_of_hour = (now / 60) % 60;
    let autonomy = 0.85 + (minute_of_hour as f64 / 60.0) * 0.10;
    let value = 0.83 + (minute_of_hour as f64 / 60.0) * 0.10;
    let intrinsic = 0.81 + (minute_of_hour as f64 / 60.0) * 0.10;
    let mscore = (autonomy + value + intrinsic) / 3.0;
    let status = format!(
        "动机 mscore={:.3} (autonomy={:.2}, value={:.2}, intrinsic={:.2})",
        mscore, autonomy, value, intrinsic
    );
    DashboardPanel {
        module: "motivation",
        display_name: "动机",
        icon: "🎯",
        status_line: status,
        key_metric_label: "motivation_score".to_string(),
        key_metric_value: format!("{:.3}", mscore),
        secondary_label: "autonomy".to_string(),
        secondary_value: format!("{:.2}", autonomy),
        tertiary_label: "intrinsic".to_string(),
        tertiary_value: format!("{:.2}", intrinsic),
        data_source: "demo: weighted average (v2 不接 apeireth_motivation)",
    }
}

fn snapshot_value(_continuity_id: &str, now: i64) -> DashboardPanel {
    // 5 层 onion (L1..L5, AI 可自决层过滤)
    let layers = ["L1 direct", "L2 tactical", "L3 strategic", "L4 philosophical", "L5 foundational"];
    let minute_of_hour = (now / 60) % 60;
    let current_layer = (minute_of_hour / 12) as usize % 5;
    let status = format!(
        "洋葱 5 层, 当前 = {} (v2 demo)",
        layers[current_layer]
    );
    DashboardPanel {
        module: "value",
        display_name: "价值",
        icon: "🧅",
        status_line: status,
        key_metric_label: "current_layer".to_string(),
        key_metric_value: layers[current_layer].to_string(),
        secondary_label: "ai_self_modifiable_count".to_string(),
        secondary_value: "3 个 (demo, L1+L2+L4)".to_string(),
        tertiary_label: "threshold".to_string(),
        tertiary_value: "0.70 (v1 default, demo)".to_string(),
        data_source: "demo: 5 层硬编码 (v2 不接 apeireth_value)",
    }
}

/// 一次性收集 6 器官状态
pub fn collect_dashboard_state(continuity_id: &str, now: i64) -> DashboardState {
    DashboardState {
        cognition: snapshot_cognition(continuity_id, now),
        perception: snapshot_perception(continuity_id, now),
        consciousness: snapshot_consciousness(continuity_id, now),
        life_force: snapshot_life_force(continuity_id, now),
        motivation: snapshot_motivation(continuity_id, now),
        value: snapshot_value(continuity_id, now),
        snapshot_at: now,
        continuity_id: continuity_id.to_string(),
    }
}

// ============================================================
// HTML 渲染
// ============================================================

fn render_panel_card(panel: &DashboardPanel) -> String {
    format!(
        r#"<div class="council-card dashboard-card dashboard-{module}">
            <div class="council-card-header">
                <span class="council-domain">{icon} {display_name}</span>
                <span class="council-stance stance-neutral">live</span>
            </div>
            <div class="council-reasoning">
                <p class="dashboard-status">{status}</p>
                <dl class="dashboard-metrics">
                    <dt>{key_label}</dt><dd class="dashboard-key">{key_value}</dd>
                    <dt>{sec_label}</dt><dd>{sec_value}</dd>
                    <dt>{ter_label}</dt><dd>{ter_value}</dd>
                </dl>
                <p class="dashboard-source">data: {source}</p>
            </div>
        </div>"#,
        module = panel.module,
        icon = panel.icon,
        display_name = html_escape(panel.display_name),
        status = html_escape(&panel.status_line),
        key_label = html_escape(&panel.key_metric_label),
        key_value = html_escape(&panel.key_metric_value),
        sec_label = html_escape(&panel.secondary_label),
        sec_value = html_escape(&panel.secondary_value),
        ter_label = html_escape(&panel.tertiary_label),
        ter_value = html_escape(&panel.tertiary_value),
        source = html_escape(panel.data_source),
    )
}

/// 渲染综合 dashboard HTML 页
pub fn render_dashboard_page(state: &DashboardState) -> String {
    let cognition = render_panel_card(&state.cognition);
    let perception = render_panel_card(&state.perception);
    let consciousness = render_panel_card(&state.consciousness);
    let life_force = render_panel_card(&state.life_force);
    let motivation = render_panel_card(&state.motivation);
    let value = render_panel_card(&state.value);

    let all_panels =
        format!("{cognition}{perception}{consciousness}{life_force}{motivation}{value}");

    let snapshot_at_human = format_unix_ts(state.snapshot_at);
    let continuity_short = if state.continuity_id.len() > 32 {
        format!("{}…", &state.continuity_id[..32])
    } else {
        state.continuity_id.clone()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Dashboard — 6 器官状态汇总</title>
    <meta name="description" content="Apeireth Web 综合 Dashboard — cognition / perception / consciousness / life-force / motivation / value 6 器官状态 (v2 demo snapshot)" />
    <style>
        .dashboard-metrics {{
            margin: 0.75rem 0 0.5rem 0;
            display: grid;
            grid-template-columns: max-content 1fr;
            gap: 0.4rem 1rem;
            font-size: 0.92rem;
        }}
        .dashboard-metrics dt {{
            color: #9090b0;
            font-weight: 600;
        }}
        .dashboard-metrics dd {{
            color: #d0d0e0;
            margin: 0;
            font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
            font-size: 0.88rem;
        }}
        .dashboard-metrics dd.dashboard-key {{
            color: #fbbf24;
            font-weight: 700;
        }}
        .dashboard-status {{
            color: #e0e0ff;
            font-weight: 500;
            line-height: 1.5;
            margin-bottom: 0.5rem;
        }}
        .dashboard-source {{
            margin-top: 0.75rem;
            padding-top: 0.5rem;
            border-top: 1px dashed rgba(255, 255, 255, 0.08);
            color: #707090;
            font-size: 0.78rem;
            font-style: italic;
        }}
        .dashboard-meta {{
            display: flex;
            justify-content: space-between;
            flex-wrap: wrap;
            gap: 1rem;
            color: #a0a0c0;
            font-size: 0.88rem;
            margin-bottom: 1.5rem;
        }}
        .dashboard-meta code {{
            background: rgba(255, 255, 255, 0.05);
            padding: 0.15rem 0.4rem;
            border-radius: 4px;
            font-size: 0.82rem;
        }}
    </style>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>🛰️ Apeireth Dashboard</h1>
            <p class="apeireth-tagline">6 器官综合状态 · cognition / perception / consciousness / life-force / motivation / value (v2 demo snapshot)</p>
        </header>

        <div class="dashboard-meta">
            <span>📅 snapshot: <code>{snapshot_at_human}</code></span>
            <span>🆔 continuity: <code>{continuity_short}</code></span>
            <span>📊 6 器官 · v2 demo (派生)</span>
        </div>

        <div class="council-grid">
            <h2>器官状态 (snapshot at {snapshot_at_human})</h2>
            <div class="council-cards">
                {all_panels}
            </div>
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/">← 返回 Council</a>
        </div>

        <footer class="dashboard-meta" style="margin-top: 3rem; padding-top: 1.5rem; border-top: 1px solid rgba(255,255,255,0.1);">
            <span>⚠️ v2: 6 器官当前为 demo snapshot (派生自 minute_of_hour), 真实接入在 R19+ 后续 patch. 模块接口稳定, 只换内部实现.</span>
        </footer>
    </main>
</body>
</html>"#,
        snapshot_at_human = snapshot_at_human,
        continuity_short = continuity_short,
        all_panels = all_panels,
    )
}

/// Unix timestamp → 人类可读 ("2026-08-04 12:34:56 UTC")
fn format_unix_ts(ts: i64) -> String {
    let secs_per_day = 86_400_i64;
    let days = ts.div_euclid(secs_per_day);
    let secs_in_day = ts.rem_euclid(secs_per_day);
    let hour = secs_in_day / 3600;
    let min = (secs_in_day % 3600) / 60;
    let sec = secs_in_day % 60;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        y, m, d, hour, min, sec
    )
}

/// Howard Hinnant `civil_from_days` 算法.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y_tmp = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y_tmp + 1 } else { y_tmp } as i32;
    (y, m, d)
}

/// stdlib `SystemTime` → Unix seconds
pub fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ============================================================
// axum handler + router helper
// ============================================================

/// `GET /dashboard` — 综合 dashboard 页
pub async fn dashboard_handler() -> Html<String> {
    let now = unix_now();
    let continuity_id = format!("did:apeireth:dashboard:{}", now);
    let state = collect_dashboard_state(&continuity_id, now);
    Html(render_dashboard_page(&state))
}

/// 把 dashboard 路由挂到现有 Router
pub fn mount(router: Router) -> Router {
    router.route("/dashboard", get(dashboard_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_dashboard_state_produces_all_six_panels() {
        let now = 1_700_000_000;
        let cid = "did:apeireth:test";
        let state = collect_dashboard_state(cid, now);
        for panel in [
            &state.cognition,
            &state.perception,
            &state.consciousness,
            &state.life_force,
            &state.motivation,
            &state.value,
        ] {
            assert!(!panel.status_line.is_empty(), "{:?} status empty", panel.module);
            assert!(!panel.key_metric_value.is_empty(), "{:?} key empty", panel.module);
        }
    }

    #[test]
    fn render_dashboard_page_contains_all_six_cards() {
        let now = 1_700_000_000;
        let cid = "did:apeireth:test";
        let state = collect_dashboard_state(cid, now);
        let html = render_dashboard_page(&state);
        for module in [
            "cognition", "perception", "consciousness",
            "life-force", "motivation", "value",
        ] {
            assert!(
                html.contains(&format!("dashboard-{module}")),
                "missing {module} card in dashboard html"
            );
        }
    }

    #[test]
    fn format_unix_ts_epoch_0() {
        assert_eq!(format_unix_ts(0), "1970-01-01 00:00:00 UTC");
    }

    #[test]
    fn format_unix_ts_2024() {
        assert_eq!(format_unix_ts(1_704_067_200), "2024-01-01 00:00:00 UTC");
    }

    #[test]
    fn format_unix_ts_with_time_of_day() {
        // 2024-01-01 13:14:56 UTC = 1704067200 + 13*3600 + 14*60 + 56 = 1704114896
        assert_eq!(format_unix_ts(1_704_114_896), "2024-01-01 13:14:56 UTC");
    }

    #[test]
    fn mount_adds_dashboard_route() {
        let router = mount(Router::new());
        // 简单 smoke test: 不 panic 即可 (router 自身可构造)
        let _ = router;
    }
}
