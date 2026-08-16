//! # TuiTestBackend — ratatui TestBackend 包装 + 断言助手
//!
//! **职责**:
//! - 包装 `ratatui::backend::TestBackend` (无 TTY, 内存里跑终端)
//! - 提供 `assert_contains` / `assert_not_contains` / `assert_color` 等断言
//! - 提供 `BufferSnapshot` 不可变快照, 跨测试 boundary 安全传递
//!
//! **为什么不用真终端**:
//! - e2e 跑 CI 时没 TTY
//! - 真终端需要 raw mode, 不可重入
//! - TestBackend 跟 ratatui 渲染管线 1:1 等价, 0 偏差
//!
//! **8 不修改承诺**:
//! - 错误能装到实现 ✓ (`TuiE2EError::BackendCreate` / `BufferEmpty` / `BufferAssert`)
//! - 错误数 hardcode ✓ (3 跟 backend 相关的变体)
//! - 0 改 LOCKED ✓
//! - 0 改 workspace version ✓
//! - 6 哲学锚透传 ✓
//! - 0 依赖 NewAPI ✓
//! - 0 重复造轮子 ✓ (TestBackend 本身是 ratatui 现成)
//! - 0 假装实缺 ✓ (0 尺寸 backend 显式拒绝, K-1 强校验)

use crate::error::{TuiE2EError, TuiE2EResult};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// TuiTestBackend — ratatui `TestBackend` 薄包装
///
/// 字段都是 pub, 方便 e2e 测试直接读 (e.g. 算 row count)
pub struct TuiTestBackend {
    /// ratatui TestBackend
    pub backend: TestBackend,
    /// 终端宽
    pub width: u16,
    /// 终端高
    pub height: u16,
}

impl TuiTestBackend {
    /// 构造 24×80 默认 (跟 tui `setup_terminal` 默认一致)
    pub fn new(width: u16, height: u16) -> TuiE2EResult<Self> {
        Self::new_with(width, height)
    }

    /// 构造自定义尺寸 (内部用, 暴露 `new` 是 alias)
    pub fn new_with(width: u16, height: u16) -> TuiE2EResult<Self> {
        if width == 0 || height == 0 {
            return Err(TuiE2EError::BackendCreate {
                width,
                height,
                reason: "width / height must be > 0 (K-1 强校验, 跟 tui 0 尺寸拒绝一致)".into(),
            });
        }
        let backend = TestBackend::new(width, height);
        Ok(Self {
            backend,
            width,
            height,
        })
    }

    /// 24×80 默认 (跟 `DEFAULT_WIDTH` / `DEFAULT_HEIGHT` 一致)
    pub fn default_24x80() -> TuiE2EResult<Self> {
        Self::new(crate::DEFAULT_WIDTH, crate::DEFAULT_HEIGHT)
    }

    /// 120×40 宽屏
    pub fn wide_120x40() -> TuiE2EResult<Self> {
        Self::new(120, 40)
    }

    /// 取 buffer 引用 (用于渲染验证)
    pub fn buffer(&self) -> &Buffer {
        self.backend.buffer()
    }

    /// buffer 不可变快照 (跨测试 boundary 安全)
    pub fn snapshot(&self) -> BufferSnapshot {
        let buf = self.backend.buffer();
        let area = buf.area;
        // 把每个 cell 的 (symbol, fg, bg) 序列化成 String
        // CJK char width=2, 下一格是 " " 续位 — 跳过续位让文本连续
        let mut cells = String::with_capacity((area.width as usize) * (area.height as usize));
        for y in 0..area.height {
            let mut prev_was_cjk = false;
            for x in 0..area.width {
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                if prev_was_cjk && sym == " " {
                    prev_was_cjk = false;
                    continue;
                }
                let is_cjk = !sym.is_empty()
                    && sym.chars().any(|c| {
                        let cp = c as u32;
                        (0x4E00..=0x9FFF).contains(&cp)
                            || (0x3000..=0x303F).contains(&cp)
                            || (0xFF00..=0xFFEF).contains(&cp)
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

    /// 断言 buffer 包含某段文本
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
                context: "assert_contains".into(),
            });
        }
        Ok(())
    }

    /// 断言 buffer 不包含某段文本
    pub fn assert_not_contains(&self, text: &str) -> TuiE2EResult<()> {
        let snap = self.snapshot();
        if snap.text.contains(text) {
            Err(TuiE2EError::BufferAssert {
                expected: format!("<NOT contains `{text}`>"),
                actual: format!(
                    "<contains `{text}` in buffer {w}x{h}>",
                    w = snap.width,
                    h = snap.height
                ),
                context: "assert_not_contains".into(),
            })
        } else {
            Ok(())
        }
    }

    /// 断言 (x, y) 位置的 fg / bg 颜色
    pub fn assert_color(
        &self,
        x: u16,
        y: u16,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> TuiE2EResult<()> {
        let buf = self.backend.buffer();
        if x >= buf.area.width || y >= buf.area.height {
            return Err(TuiE2EError::BufferAssert {
                expected: format!("({x},{y}) in {}x{}", buf.area.width, buf.area.height),
                actual: "out of range".into(),
                context: "assert_color".into(),
            });
        }
        let cell = &buf[(x, y)];
        if let Some(want_fg) = fg {
            if cell.fg != want_fg {
                return Err(TuiE2EError::BufferAssert {
                    expected: format!("fg={want_fg:?}"),
                    actual: format!("fg={:?}", cell.fg),
                    context: format!("assert_color at ({x},{y})"),
                });
            }
        }
        if let Some(want_bg) = bg {
            if cell.bg != want_bg {
                return Err(TuiE2EError::BufferAssert {
                    expected: format!("bg={want_bg:?}"),
                    actual: format!("bg={:?}", cell.bg),
                    context: format!("assert_color at ({x},{y})"),
                });
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for TuiTestBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiTestBackend")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

/// 不可变 buffer 快照 (跨 boundary 安全)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSnapshot {
    /// 宽
    pub width: u16,
    /// 高
    pub height: u16,
    /// 全部 cell symbol 拼成 text (按行)
    pub text: String,
}

impl BufferSnapshot {
    /// 包含某段
    pub fn contains(&self, needle: &str) -> bool {
        self.text.contains(needle)
    }

    /// 行数 (跟 height 近似, 但会去掉尾部全空行)
    pub fn non_empty_lines(&self) -> usize {
        self.text.lines().filter(|l| !l.trim().is_empty()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_24x80_ok() {
        let b = TuiTestBackend::new(24, 80).unwrap();
        assert_eq!(b.width, 24);
        assert_eq!(b.height, 80);
    }

    #[test]
    fn new_80x24_ok() {
        let b = TuiTestBackend::new(80, 24).unwrap();
        assert_eq!(b.width, 80);
        assert_eq!(b.height, 24);
    }

    #[test]
    fn new_zero_width_rejected() {
        let r = TuiTestBackend::new(0, 24);
        assert!(matches!(r, Err(TuiE2EError::BackendCreate { .. })));
    }

    #[test]
    fn new_zero_height_rejected() {
        let r = TuiTestBackend::new(24, 0);
        assert!(matches!(r, Err(TuiE2EError::BackendCreate { .. })));
    }

    #[test]
    fn default_24x80_ok() {
        let b = TuiTestBackend::default_24x80().unwrap();
        assert_eq!(b.width, 80);
        assert_eq!(b.height, 24);
    }

    #[test]
    fn wide_120x40_ok() {
        let b = TuiTestBackend::wide_120x40().unwrap();
        assert_eq!(b.width, 120);
        assert_eq!(b.height, 40);
    }

    #[test]
    fn snapshot_dimensions() {
        let b = TuiTestBackend::new(40, 10).unwrap();
        let s = b.snapshot();
        assert_eq!(s.width, 40);
        assert_eq!(s.height, 10);
    }

    #[test]
    fn assert_contains_finds_text() {
        // 用 harness 渲染 4 panel, buffer 必有内容
        let mut h = crate::harness::TuiHarness::start_with_size(80, 24).unwrap();
        h.render_4_panel().unwrap();
        // 调试: 看 row0 前 20 cell 的 (symbol, width)
        let buf = h.buffer();
        for x in 0..20 {
            let cell = &buf[(x, 0)];
            eprintln!(
                "DEBUG cell[{x}]: symbol={:?} width={}",
                cell.symbol(),
                std::str::from_utf8(cell.symbol().as_bytes()).map_or("?", |_| "utf8")
            );
        }
        // top nav 必有 "桥接" (NavPage::Bridge label_zh)
        h.assert_contains("桥接").unwrap();
    }

    #[test]
    fn assert_contains_fails_on_missing() {
        let mut h = crate::harness::TuiHarness::start_with_size(80, 24).unwrap();
        h.render_4_panel().unwrap();
        let r = h.assert_contains("xyzzy_missing");
        assert!(matches!(r, Err(TuiE2EError::BufferAssert { .. })));
    }

    #[test]
    fn assert_not_contains_passes() {
        // 空 backend 全是空白, 不含任意文本
        let b = TuiTestBackend::new(20, 5).unwrap();
        b.assert_not_contains("absent").unwrap();
    }

    #[test]
    fn assert_not_contains_fails_on_present() {
        // 渲染后 buffer 必有 "桥接", 验 "不包含桥接" 失败
        let mut h = crate::harness::TuiHarness::start_with_size(80, 24).unwrap();
        h.render_4_panel().unwrap();
        let r = h.assert_not_contains("桥接");
        assert!(matches!(r, Err(TuiE2EError::BufferAssert { .. })));
    }

    #[test]
    fn assert_color_oob_fails() {
        let b = TuiTestBackend::new(10, 5).unwrap();
        let r = b.assert_color(99, 99, Some(Color::Red), None);
        assert!(matches!(r, Err(TuiE2EError::BufferAssert { .. })));
    }

    #[test]
    fn snapshot_non_empty_lines_empty_backend() {
        let b = TuiTestBackend::new(20, 5).unwrap();
        let s = b.snapshot();
        // 空 backend 全是空白, 非空行 = 0
        assert_eq!(s.non_empty_lines(), 0);
    }
}
