//! # apeireth-tui benches — render_5_nav (R21 — 1.0 release #7 perf 续 D-P2)
//!
//! 5 个 bench 测 5 nav 页面 (Bridge / Dialogue / Growth / History / Settings) 渲染时间:
//! - `render_nav_bridge`: 0 舰桥 (ΣΚΟΠΗ, 默认首页)
//! - `render_nav_dialogue`: 1 对话 (ΔΙΑΛΟΓΟΣ)
//! - `render_nav_growth`: 2 生长 (ΑΥΞΗΣΙΣ)
//! - `render_nav_history`: 3 历史 (ΙΣΤΟΡΙΑ)
//! - `render_nav_settings`: 4 设置 (ΤΑΞΙΣ)
//!
//! **关键设计**:
//! - R121 续 (V2-6 战区 2.5): TUI 加 lib.rs (Cargo.toml [lib] 段), bench 改用 `apeireth_tui::*` 公开 API
//! - 修复 8 bench errors (binary crate 0 lib 路径错, 1:1 镜像 main.rs mod 声明)
//! - 用 `ratatui::backend::TestBackend` + `ratatui::Terminal` 在内存里渲染, 0 TTY 依赖
//! - 0 改 src/ 下任何文件 (mod 声明 lib.rs 1:1 镜像 main.rs, 0 业务逻辑变化)
//! - 0 触碰 24 LOCKED (tui 不在 24 LOCKED 名单)
//!
//! **基线** (1.0.0): target/criterion/apeireth-tui/render_5_nav/
//! **target P95**: 5 nav < 1 ms / page (per 1.0 release 12 项 #8)

use apeireth_tui::{bridge, dialogue, growth, history, settings, App, NavPage, Theme, ThemeStyle};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// 构造 TestBackend + Terminal + App (固定 80x24, 跟 e2e 一致)
fn setup_terminal_app(nav: NavPage) -> (Terminal<TestBackend>, App, ThemeStyle) {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("create TestBackend terminal");
    let mut app = App::new();
    app.nav = nav;
    let style = ThemeStyle::of(Theme::Archaic);
    // 强制 initial draw (e2e `TuiHarness::start_with_size` 同模式)
    let _ = terminal.draw(|f| {
        let area = f.area();
        // 渲染对应 nav (warm-up: 让 lazy field 初始化)
        // R121 续 (V2-6 战区 2.5): 用 apeireth_tui::pages::* 公开 API 替代 #[path] hack
        match app.nav {
            NavPage::Bridge => bridge::render(f, area, &app, &style),
            NavPage::Dialogue => dialogue::render(f, area, &mut app, &style),
            NavPage::Growth => growth::render(f, area, &app, &style),
            NavPage::History => history::render(f, area, &app, &style),
            NavPage::Settings => settings::render(f, area, &app, &style),
        }
    });
    (terminal, app, style)
}

fn bench_render_nav_bridge(c: &mut Criterion) {
    let (mut terminal, app, style) = setup_terminal_app(NavPage::Bridge);
    c.bench_function("render_nav_bridge", |b| {
        b.iter(|| {
            let _ = terminal.draw(|f| {
                let area = f.area();
                bridge::render(f, area, black_box(&app), black_box(&style));
            });
        });
    });
}

fn bench_render_nav_dialogue(c: &mut Criterion) {
    let (mut terminal, mut app, style) = setup_terminal_app(NavPage::Dialogue);
    c.bench_function("render_nav_dialogue", |b| {
        b.iter(|| {
            let _ = terminal.draw(|f| {
                let area = f.area();
                // dialogue::render 需要 &mut App (R121 续 1:1 跟 pages/dialogue.rs:35 签名)
                dialogue::render(f, area, black_box(&mut app), black_box(&style));
            });
        });
    });
}

fn bench_render_nav_growth(c: &mut Criterion) {
    let (mut terminal, app, style) = setup_terminal_app(NavPage::Growth);
    c.bench_function("render_nav_growth", |b| {
        b.iter(|| {
            let _ = terminal.draw(|f| {
                let area = f.area();
                growth::render(f, area, black_box(&app), black_box(&style));
            });
        });
    });
}

fn bench_render_nav_history(c: &mut Criterion) {
    let (mut terminal, app, style) = setup_terminal_app(NavPage::History);
    c.bench_function("render_nav_history", |b| {
        b.iter(|| {
            let _ = terminal.draw(|f| {
                let area = f.area();
                history::render(f, area, black_box(&app), black_box(&style));
            });
        });
    });
}

fn bench_render_nav_settings(c: &mut Criterion) {
    let (mut terminal, app, style) = setup_terminal_app(NavPage::Settings);
    c.bench_function("render_nav_settings", |b| {
        b.iter(|| {
            let _ = terminal.draw(|f| {
                let area = f.area();
                settings::render(f, area, black_box(&app), black_box(&style));
            });
        });
    });
}

criterion_group!(
    benches,
    bench_render_nav_bridge,
    bench_render_nav_dialogue,
    bench_render_nav_growth,
    bench_render_nav_history,
    bench_render_nav_settings
);
criterion_main!(benches);
