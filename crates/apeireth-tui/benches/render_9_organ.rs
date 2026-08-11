//! # apeireth-tui benches — render_9_organ (R21 — 1.0 release #7 perf 续 D-P2)
//!
//! 10 个 bench 测 9 器官 widget 渲染 + 1 dispatch 性能:
//! - 9 organ widget: heart / brain / hand / eye / ear / memory / voice / body / mind
//! - 1 organ dispatch: `render_organ_widget(organ, state)` (按 enum 分发)
//!
//! **关键设计**:
//! - 用 sister `apeireth-observability::tui_dashboard` 9 widget 函数 (公共 API, dev-dep 接入)
//! - 0 改 src/ (24 LOCKED 严守), 0 `#[path]` 引用 (避免 `#[cfg(test)]` mod tests 污染编译)
//! - 用 `ratatui::backend::TestBackend` + `Paragraph` 渲染 (String -> ratatui pipeline)
//!
//! **基线** (1.0.0): target/criterion/apeireth-tui/render_9_organ/
//! **target P95**: 9 organ < 1 ms / widget (per 1.0 release 12 项 #8)

use apeireth_telemetry::observability::tui_dashboard::{
    OrganKind, OrganReadiness, TuiOrganState, render_body_widget, render_brain_widget,
    render_ear_widget, render_eye_widget, render_hand_widget, render_heart_widget,
    render_memory_widget, render_mind_widget, render_organ_widget, render_voice_widget,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

/// 构造 TestBackend + Terminal + 1 个 organ state (固定 80x24, 跟 e2e 一致)
fn setup_terminal_organ_state(organ: OrganKind) -> (Terminal<TestBackend>, TuiOrganState) {
    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).expect("create TestBackend terminal");
    let state = TuiOrganState::new(
        organ,
        OrganReadiness::Ok,
        100.0,
        format!("{:?} bench (R21+ 1.0 release #7 perf)", organ),
    );
    (terminal, state)
}

/// 把 organ widget String 渲染到 TestBackend
fn render_organ_string_to_backend(terminal: &mut Terminal<TestBackend>, text: &str) {
    let _ = terminal.draw(|f| {
        let area = f.area();
        let p = Paragraph::new(Line::from(text))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    });
}

fn bench_render_heart_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Heart);
    c.bench_function("render_organ_heart", |b| {
        b.iter(|| {
            let s = render_heart_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_brain_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Brain);
    c.bench_function("render_organ_brain", |b| {
        b.iter(|| {
            let s = render_brain_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_hand_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Hand);
    c.bench_function("render_organ_hand", |b| {
        b.iter(|| {
            let s = render_hand_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_eye_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Eye);
    c.bench_function("render_organ_eye", |b| {
        b.iter(|| {
            let s = render_eye_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_ear_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Ear);
    c.bench_function("render_organ_ear", |b| {
        b.iter(|| {
            let s = render_ear_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_memory_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Memory);
    c.bench_function("render_organ_memory", |b| {
        b.iter(|| {
            let s = render_memory_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_voice_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Voice);
    c.bench_function("render_organ_voice", |b| {
        b.iter(|| {
            let s = render_voice_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_body_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Body);
    c.bench_function("render_organ_body", |b| {
        b.iter(|| {
            let s = render_body_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_mind_widget(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Mind);
    c.bench_function("render_organ_mind", |b| {
        b.iter(|| {
            let s = render_mind_widget(black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

fn bench_render_organ_widget_dispatch(c: &mut Criterion) {
    let (mut terminal, state) = setup_terminal_organ_state(OrganKind::Heart);
    c.bench_function("render_organ_widget_dispatch", |b| {
        b.iter(|| {
            let s = render_organ_widget(black_box(OrganKind::Heart), black_box(&state));
            render_organ_string_to_backend(&mut terminal, black_box(&s));
        });
    });
}

criterion_group!(
    benches,
    bench_render_heart_widget,
    bench_render_brain_widget,
    bench_render_hand_widget,
    bench_render_eye_widget,
    bench_render_ear_widget,
    bench_render_memory_widget,
    bench_render_voice_widget,
    bench_render_body_widget,
    bench_render_mind_widget,
    bench_render_organ_widget_dispatch
);
criterion_main!(benches);
