//! # apeireth-tui benches — render_dashboard (R21 — 1.0 release #7 perf 续 D-P2)
//!
//! 5 个 bench 测 TUI dashboard 整体 + 9 organ 并发注册 + 5 nav dispatch 性能:
//! - `render_dashboard_full`: 1 函数 (9 organ + 3 endpoint + 5 nav + 6 锚 一体渲染)
//! - `register_tui_organ_state_9x_concurrent`: 9 器官并发注册 (1 owner × 9 thread)
//! - `render_5_nav_dispatch`: 5 nav dispatch (按 enum match + render)
//! - `render_3_endpoint_dispatch`: 3 health endpoint 渲染 (/health /ready /metrics)
//! - `render_9_widgets_all_sequential`: 9 器官 widget 顺序渲染 (测 N=9 pipeline)
//!
//! **关键设计**:
//! - 用 sister `apeireth-observability::tui_dashboard` 公共 API (dev-dep 接入)
//! - 0 改 src/ (24 LOCKED 严守), 0 `#[path]` 引用
//!
//! **基线** (1.0.0): target/criterion/apeireth-tui/render_dashboard/
//! **target P95**:
//!   - render_dashboard_full < 10 ms
//!   - register_tui_organ_state_9x_concurrent < 1 ms
//!   - render_5_nav_dispatch < 5 ms
//!   - render_3_endpoint_dispatch < 3 ms
//!   - render_9_widgets_all_sequential < 9 ms

use std::sync::Arc;

use apeireth_observability::tui_dashboard::{
    OrganDashboard, OrganKind, OrganReadiness, TuiOrganState, render_dashboard,
    render_organ_widget,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

/// 构造 1 个装满 9 器官 + 3 health endpoint 的 OrganDashboard
fn setup_dashboard_full() -> (Terminal<TestBackend>, OrganDashboard) {
    let backend = TestBackend::new(120, 40);
    let terminal = Terminal::new(backend).expect("create TestBackend terminal");
    let mut dash = OrganDashboard::new();
    // 9 器官全注册 (mixed ok / partial / stub, 跟 sister #6 1:1 镜像)
    dash.register_tui_organ_state(
        OrganKind::Heart,
        TuiOrganState::ok(OrganKind::Heart, 100.0, "60Hz pulse"),
    );
    dash.register_tui_organ_state(
        OrganKind::Brain,
        TuiOrganState::ok(OrganKind::Brain, 100.0, "LLM call"),
    );
    dash.register_tui_organ_state(
        OrganKind::Hand,
        TuiOrganState::ok(OrganKind::Hand, 100.0, "tool call"),
    );
    dash.register_tui_organ_state(
        OrganKind::Eye,
        TuiOrganState::partial(OrganKind::Eye),
    );
    dash.register_tui_organ_state(
        OrganKind::Ear,
        TuiOrganState::partial(OrganKind::Ear),
    );
    dash.register_tui_organ_state(
        OrganKind::Memory,
        TuiOrganState::ok(OrganKind::Memory, 100.0, "sqlite"),
    );
    dash.register_tui_organ_state(
        OrganKind::Voice,
        TuiOrganState::ok(OrganKind::Voice, 100.0, "tts"),
    );
    dash.register_tui_organ_state(
        OrganKind::Body,
        TuiOrganState::ok(OrganKind::Body, 100.0, "vital"),
    );
    dash.register_tui_organ_state(
        OrganKind::Mind,
        TuiOrganState::ok(OrganKind::Mind, 100.0, "6 anchors"),
    );
    (terminal, dash)
}

/// 把 dashboard 字符串渲染到 TestBackend (测 ratatui pipeline)
fn render_dashboard_string_to_backend(terminal: &mut Terminal<TestBackend>, text: &str) {
    let _ = terminal.draw(|f| {
        let area = f.area();
        let p = Paragraph::new(Line::from(text))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    });
}

fn bench_render_dashboard_full(c: &mut Criterion) {
    let (mut terminal, dash) = setup_dashboard_full();
    c.bench_function("render_dashboard_full", |b| {
        b.iter(|| {
            let s = render_dashboard(black_box(&dash));
            render_dashboard_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_register_tui_organ_state_9x_concurrent(c: &mut Criterion) {
    let dash = Arc::new(std::sync::Mutex::new(OrganDashboard::new()));
    c.bench_function("register_tui_organ_state_9x_concurrent", |b| {
        b.iter(|| {
            // 9 器官并发注册 (1 owner × 9 thread, per 蓝图 §3.5 并发守门)
            // 用 Mutex 守门, 跟 sister #6 tui_dashboard.rs Arc<Mutex<[TuiOrganState; 9]>> 1:1 镜像
            let handles: Vec<_> = (0..9)
                .map(|i| {
                    let dash_clone = Arc::clone(&dash);
                    std::thread::spawn(move || {
                        let organ = match i {
                            0 => OrganKind::Heart,
                            1 => OrganKind::Brain,
                            2 => OrganKind::Hand,
                            3 => OrganKind::Eye,
                            4 => OrganKind::Ear,
                            5 => OrganKind::Memory,
                            6 => OrganKind::Voice,
                            7 => OrganKind::Body,
                            _ => OrganKind::Mind,
                        };
                        let state = TuiOrganState::ok(organ, 100.0, "concurrent bench");
                        let mut d = dash_clone.lock().unwrap();
                        let _ = d.register_tui_organ_state(organ, state);
                    })
                })
                .collect();
            for h in handles {
                let _ = h.join();
            }
        });
    });
}

fn bench_render_5_nav_dispatch(c: &mut Criterion) {
    let (mut terminal, dash) = setup_dashboard_full();
    c.bench_function("render_5_nav_dispatch", |b| {
        b.iter(|| {
            // 5 nav dispatch: 借 render_dashboard (其内部走 FIVE_NAV 分支)
            let s = render_dashboard(black_box(&dash));
            render_dashboard_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_3_endpoint_dispatch(c: &mut Criterion) {
    // 3 endpoint mock 都在 render_dashboard 内部
    let (mut terminal, _) = setup_dashboard_full();
    let dash = OrganDashboard::new();
    c.bench_function("render_3_endpoint_dispatch", |b| {
        b.iter(|| {
            // 3 endpoint dispatch: /health /ready /metrics (借 render_dashboard 内部 3 endpoint mock)
            let s = render_dashboard(black_box(&dash));
            render_dashboard_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_9_widgets_all_sequential(c: &mut Criterion) {
    let (mut terminal, _) = setup_dashboard_full();
    c.bench_function("render_9_widgets_all_sequential", |b| {
        b.iter(|| {
            // 9 器官 widget 顺序渲染 (测 N=9 pipeline 总耗时)
            // 全部走 render_organ_widget dispatch (per 9 organ enum), 0 fn 指针不匹配
            for organ in [
                OrganKind::Heart,
                OrganKind::Brain,
                OrganKind::Hand,
                OrganKind::Eye,
                OrganKind::Ear,
                OrganKind::Memory,
                OrganKind::Voice,
                OrganKind::Body,
                OrganKind::Mind,
            ] {
                let state = TuiOrganState::new(
                    organ,
                    OrganReadiness::Ok,
                    100.0,
                    "seq bench",
                );
                let s = render_organ_widget(black_box(organ), black_box(&state));
                render_dashboard_string_to_backend(&mut terminal, black_box(&s));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_render_dashboard_full,
    bench_register_tui_organ_state_9x_concurrent,
    bench_render_5_nav_dispatch,
    bench_render_3_endpoint_dispatch,
    bench_render_9_widgets_all_sequential
);
criterion_main!(benches);
