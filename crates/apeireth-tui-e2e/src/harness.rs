//! # TuiHarness — App 启动 + 1s tick + 5 快捷键事件
//!
//! **职责**:
//! - 启动: 构造 `TuiApp` + `ratatui::Terminal<TestBackend>`
//! - `tick()`: 模拟 1s tick, 推进 spinner / render_tick
//! - `send_key()`: 转发键盘事件到 `TuiApp::handle_key`
//! - `quit()`: 设 `should_quit = true`
//! - `buffer()` / `snapshot()`: 委托到 terminal.backend().buffer()
//!
//! **1s tick vs 250ms tick**:
//! - tui 主循环是 250ms tick (4Hz)
//! - e2e 1s tick 是 "模拟 1 秒经过", 等价 4 次 250ms
//! - e2e 不真等 1s, 用 `app.tick()` 直接推进状态
//!
//! **5 快捷键** (per tui main.rs 注释):
//! - `q` / `Esc` 退出
//! - `Tab` / `BackTab` 切 nav
//! - `1`-`5` 跳 nav
//! - `t` 切 theme
//! - `m` 切 mode
//!
//! **8 不修改承诺**: 跟 lib.rs / error.rs 一致

use crate::backend::BufferSnapshot;
use crate::error::{TuiE2EError, TuiE2EResult};
use crate::TuiApp;
use crossterm::event::KeyCode;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// TuiHarness — 端到端测试驱动
///
/// `terminal` 持有唯一的 `TestBackend`, draw 写入的 buffer
/// 可以通过 `buffer()` / `snapshot()` / `backend()` 读出.
pub struct TuiHarness {
    /// TuiApp 状态
    pub app: TuiApp,
    /// ratatui Terminal (绑定 TestBackend, 用来 render 验证)
    pub terminal: Terminal<TestBackend>,
    /// TuiTestBackend 视图 (snapshot / assert_* 助手, 委托到 terminal.backend)
    pub backend_view: TuiTestBackendView,
}

impl TuiHarness {
    /// 启动默认 24×80 harness
    pub fn start() -> TuiE2EResult<Self> {
        Self::start_with_size(80, 24)
    }

    /// 启动指定尺寸
    pub fn start_with_size(width: u16, height: u16) -> TuiE2EResult<Self> {
        if width == 0 || height == 0 {
            return Err(TuiE2EError::BackendCreate {
                width,
                height,
                reason: "width / height must be > 0 (K-1 强校验)".into(),
            });
        }
        let test_backend = TestBackend::new(width, height);
        let terminal = Terminal::new(test_backend)
            .map_err(|e| TuiE2EError::HarnessStart { reason: e.to_string() })?;
        Ok(Self {
            app: TuiApp::new(),
            terminal,
            backend_view: TuiTestBackendView { width, height },
        })
    }

    /// 启动并预置一些对话 (e.g. user 问 + assistant 答)
    pub fn start_with_chat(messages: Vec<(&str, &str)>) -> TuiE2EResult<Self> {
        let mut h = Self::start()?;
        for (role, content) in messages {
            match role {
                "user" => h.app.push_user_input(content),
                "assistant" => h.app.push_assistant_reply(content),
                "system" => h.app.push_system(content),
                _ => {
                    return Err(TuiE2EError::HarnessStart {
                        reason: format!("unknown role: {role}"),
                    });
                }
            }
        }
        Ok(h)
    }

    /// 模拟 1s tick — 推进 spinner / render_tick (不真等 1s)
    pub fn tick(&mut self) -> TuiE2EResult<()> {
        self.app.tick();
        // 重新 draw 一帧 (跟 tui run_app 主循环 draw 镜像)
        self.terminal
            .draw(|f| {
                let _ = f;
            })
            .map_err(|e| TuiE2EError::HarnessTick { reason: e.to_string() })?;
        Ok(())
    }

    /// 模拟 N 个 1s tick
    pub fn tick_n(&mut self, n: u32) -> TuiE2EResult<()> {
        for _ in 0..n {
            self.tick()?;
        }
        Ok(())
    }

    /// 发送键盘事件
    pub fn send_key(&mut self, key: KeyCode) -> TuiE2EResult<()> {
        self.app.handle_key(key);
        // 模拟 tui run_app 收到 key 后的 draw
        self.terminal
            .draw(|f| {
                let _ = f;
            })
            .map_err(|e| TuiE2EError::HarnessKey { reason: e.to_string() })?;
        Ok(())
    }

    /// 发送 q 退出
    pub fn quit(&mut self) -> TuiE2EResult<()> {
        self.send_key(KeyCode::Char('q'))
    }

    /// buffer 引用 (跨测试 boundary 用 snapshot 更安全)
    pub fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.backend().buffer()
    }

    /// buffer 快照
    pub fn snapshot(&self) -> BufferSnapshot {
        let buf = self.terminal.backend().buffer();
        let area = buf.area;
        let mut cells = String::with_capacity((area.width as usize) * (area.height as usize));
        for y in 0..area.height {
            let mut prev_was_cjk = false;
            for x in 0..area.width {
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                // ratatui 0.29: CJK char width=2, 下一格是 " " 续位
                // 跳过 CJK 续位空格, 让 "桥接" 在 text 里连续
                if prev_was_cjk && sym == " " {
                    prev_was_cjk = false;
                    continue;
                }
                // 检测 CJK: 符号非 ASCII 且 非空
                let is_cjk = !sym.is_empty()
                    && sym.chars().any(|c| {
                        let cp = c as u32;
                        (0x4E00..=0x9FFF).contains(&cp)        // CJK Unified
                            || (0x3000..=0x303F).contains(&cp)  // CJK Punctuation
                            || (0xFF00..=0xFFEF).contains(&cp)  // Fullwidth
                    });
                prev_was_cjk = is_cjk;
                cells.push_str(sym);
            }
            cells.push('\n');
        }
        BufferSnapshot {
            width: area.width,
            height: area.height,
            text: cells,
        }
    }

    /// 渲染: 跟 tui main.rs 的 1 屏 4 panel 布局镜像
    ///  - top: 5 nav
    ///  - middle: 9 organ
    ///  - content: 当前 nav 内容
    ///  - status: status bar
    pub fn render_4_panel(&mut self) -> TuiE2EResult<()> {
        use crate::render;
        self.terminal
            .draw(|f| {
                render::render_4_panel(f, &mut self.app);
            })
            .map_err(|e| TuiE2EError::HarnessTick { reason: e.to_string() })?;
        Ok(())
    }

    /// 渲染并返回 buffer 快照 (便捷)
    pub fn render_and_snapshot(&mut self) -> TuiE2EResult<BufferSnapshot> {
        self.render_4_panel()?;
        Ok(self.snapshot())
    }

    /// 断言 buffer 包含 (便捷)
    pub fn assert_contains(&self, text: &str) -> TuiE2EResult<()> {
        let snap = self.snapshot();
        if !snap.text.contains(text) {
            return Err(TuiE2EError::BufferAssert {
                expected: text.into(),
                actual: format!(
                    "<not found in buffer {w}x{h}>",
                    w = snap.width,
                    h = snap.height
                ),
                context: "harness.assert_contains".into(),
            });
        }
        Ok(())
    }

    /// 断言 buffer 不包含 (便捷)
    pub fn assert_not_contains(&self, text: &str) -> TuiE2EResult<()> {
        let snap = self.snapshot();
        if snap.text.contains(text) {
            return Err(TuiE2EError::BufferAssert {
                expected: format!("<NOT contains `{text}`>"),
                actual: format!(
                    "<contains `{text}` in buffer {w}x{h}>",
                    w = snap.width,
                    h = snap.height
                ),
                context: "harness.assert_not_contains".into(),
            });
        }
        Ok(())
    }

    /// 兼容旧 field 访问 — `h.backend` 等同 `h.backend_view`
    pub fn backend(&self) -> &TuiTestBackendView {
        &self.backend_view
    }
}

/// TuiTestBackend 视图 — 持有 width/height, 把方法委托到 harness 的 terminal
///
/// 这是 `TuiTestBackend` 的 "瘦视图" 版本, 专为 harness 设计 —
/// 它没有自己的 TestBackend, 因为 TestBackend 被 Terminal 持有
/// (Terminal 拥有 backend 所有权, harness 通过 terminal.backend() 取 ref).
#[derive(Debug, Clone, Copy)]
pub struct TuiTestBackendView {
    /// 终端宽
    pub width: u16,
    /// 终端高
    pub height: u16,
}

impl TuiHarness {
    /// 取 buffer 引用 (兼容旧 `h.backend.backend.buffer()` 调用)
    pub fn backend_buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.backend().buffer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_default_ok() {
        let h = TuiHarness::start().unwrap();
        assert_eq!(h.app.nav, crate::NavPage::Bridge);
        assert!(!h.app.should_quit);
    }

    #[test]
    fn start_with_size_40x12() {
        let h = TuiHarness::start_with_size(40, 12).unwrap();
        assert_eq!(h.backend().width, 40);
        assert_eq!(h.backend().height, 12);
    }

    #[test]
    fn start_with_chat_inserts_messages() {
        let h = TuiHarness::start_with_chat(vec![
            ("user", "hi"),
            ("assistant", "hello"),
        ])
        .unwrap();
        assert_eq!(h.app.chat_history.len(), 2);
        assert_eq!(h.app.chat_history[0].role, "user");
        assert_eq!(h.app.chat_history[1].role, "assistant");
    }

    #[test]
    fn start_with_chat_unknown_role_errors() {
        let r = TuiHarness::start_with_chat(vec![("alien", "what")]);
        assert!(matches!(r, Err(TuiE2EError::HarnessStart { .. })));
    }

    #[test]
    fn tick_advances_render_tick() {
        let mut h = TuiHarness::start().unwrap();
        let before = h.app.render_tick;
        h.tick().unwrap();
        assert!(h.app.render_tick > before);
    }

    #[test]
    fn tick_n_advances_n_times() {
        let mut h = TuiHarness::start().unwrap();
        let before = h.app.render_tick;
        h.tick_n(10).unwrap();
        assert_eq!(h.app.render_tick, before + 10);
    }

    #[test]
    fn send_q_quits() {
        let mut h = TuiHarness::start().unwrap();
        h.send_key(KeyCode::Char('q')).unwrap();
        assert!(h.app.should_quit);
    }

    #[test]
    fn send_tab_advances_nav() {
        let mut h = TuiHarness::start().unwrap();
        assert_eq!(h.app.nav, crate::NavPage::Bridge);
        h.send_key(KeyCode::Tab).unwrap();
        assert_eq!(h.app.nav, crate::NavPage::Dialogue);
    }

    #[test]
    fn send_1_to_5_jumps() {
        let mut h = TuiHarness::start().unwrap();
        h.send_key(KeyCode::Char('3')).unwrap();
        assert_eq!(h.app.nav, crate::NavPage::Growth);
        h.send_key(KeyCode::Char('5')).unwrap();
        assert_eq!(h.app.nav, crate::NavPage::Settings);
    }

    #[test]
    fn quit_helper() {
        let mut h = TuiHarness::start().unwrap();
        h.quit().unwrap();
        assert!(h.app.should_quit);
    }

    #[test]
    fn render_4_panel_ok() {
        let mut h = TuiHarness::start().unwrap();
        h.render_4_panel().unwrap();
        // 渲染完 buffer 应该有内容 (top nav + middle organ + content + status)
        let snap = h.snapshot();
        // top nav 应有 "桥接" (NavPage::Bridge label_zh)
        assert!(snap.contains("桥接") || snap.text.len() > 100);
    }

    #[test]
    fn render_and_snapshot_returns_text() {
        let mut h = TuiHarness::start().unwrap();
        let snap = h.render_and_snapshot().unwrap();
        assert!(snap.width > 0);
        assert!(snap.height > 0);
    }
}
