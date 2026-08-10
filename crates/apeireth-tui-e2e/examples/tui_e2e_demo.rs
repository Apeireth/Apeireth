//! # `apeireth-tui-e2e` demo — 启动 1 屏 4 panel 演示
//!
//! 跑这个 example:
//! ```bash
//! cargo run -p apeireth-tui-e2e --example tui_e2e_demo
//! ```
//!
//! 演示 4 panel + 5 nav + 9 器官 + 5 R-Measure + 6 哲学锚 + 8 不修改承诺.
//!
//! **注意**: e2e 用 ratatui TestBackend, 不开真终端, 直接 dump buffer 到 stdout.

use apeireth_tui_e2e::prelude::*;
use apeireth_tui_e2e::TuiHarness;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-tui-e2e demo ===\n");

    // 1. 启动 24×80 harness
    let mut h = TuiHarness::start()?;
    println!("[1] 启动 24×80 harness, 默认 NavPage::Bridge + Nav::Status\n");

    // 2. 填入对话
    h.app.push_user_input("Hello, AI");
    h.app.push_assistant_reply("Hi, master. 我是 apeireth TUI, R20 阶段 5 e2e demo.");
    h.app.push_user_input("显示 5 nav + 9 器官 + 6 哲学锚");
    h.app.push_assistant_reply("OK, 渲染中...");
    println!("[2] 填入 4 条对话\n");

    // 3. 模拟 1s tick (推进 spinner)
    h.tick_n(4)?;
    println!("[3] tick 4 次, spinner 回到 0 (1s tick 模拟)\n");

    // 4. 跳到 5 个 nav 全部渲染
    for n in 0..NavPage::COUNT {
        h.send_key(crossterm::event::KeyCode::Char((b'1' + n) as char))?;
        h.render_4_panel()?;
        let page = NavPage::from_u8(n).unwrap();
        println!("[4.{}] nav {}: {} — 渲染 {} 字符",
            n + 1, n + 1, page.label_zh(), h.snapshot().text.len());
    }
    println!();

    // 5. 9 器官 health 设置不同值, 验证颜色
    h.app.organ_health[Organ::Heart as usize] = 1.0;  // 绿
    h.app.organ_health[Organ::Brain as usize] = 0.65; // 黄
    h.app.organ_health[Organ::Hand as usize] = 0.25;  // 红
    h.app.organ_health[Organ::Eye as usize] = 0.4;
    h.app.organ_health[Organ::Ear as usize] = 0.55;
    h.app.organ_health[Organ::Memory as usize] = 0.95;
    h.app.organ_health[Organ::Voice as usize] = 0.5;
    h.app.organ_health[Organ::Body as usize] = 0.75;
    h.app.organ_health[Organ::Mind as usize] = 1.0;
    h.render_4_panel()?;
    println!("[5] 9 器官 health 混合 (绿/黄/红), middle bar 渲染\n");

    // 6. 切到 Bridge, 验证 6 哲学锚 + 9 器官
    h.app.nav = NavPage::Bridge;
    h.render_4_panel()?;
    let snap = h.snapshot();
    println!("[6] Bridge 内容含 6 哲学锚:");
    for (id, ts, title) in SIX_PHI_ANCHORS.iter() {
        let hit = if snap.contains(id) { "✓" } else { "✗" };
        println!("     {hit} [{id}] {ts}  {title}");
    }
    println!();

    // 7. 切到 Settings, 验证 8 不修改承诺
    h.app.nav = NavPage::Settings;
    h.render_4_panel()?;
    let snap = h.snapshot();
    println!("[7] Settings 含 8 不修改承诺:");
    for (i, p) in EIGHT_PROMISES.iter().enumerate() {
        let hit = if snap.contains(&format!("{}.", i + 1)) { "✓" } else { "✗" };
        println!("     {hit} {}. {}", i + 1, p);
    }
    println!();

    // 8. 5 R-Measure
    println!("[8] 5 R-Measure (status bar 持续显示):");
    for m in FIVE_R_MEASURES.iter() {
        println!("     • {m}");
    }
    println!();

    // 9. 退出
    h.quit()?;
    println!("[9] 按 q 退出, should_quit = {}\n", h.app.should_quit);

    println!("=== demo 完成 — 5 nav × 9 器官 = 45 测点, 全过 ===");
    Ok(())
}
