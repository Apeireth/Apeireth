//! Apeireth R25.2 TUI — Session nav
//!
//! **职责**: 活跃会话列表, 调 `ApeirethClient::get_sessions()`
//!
//! **数据流**:
//! - HTTP: `GET /v1/sessions` → `Vec<Session>` (id / title / message_count / last_active_at)
//! - 渲染: list view, 每行 1 session
//! - 占位: R25.2 标 partial, 真实 HTTP 接 R25.3
//!
//! **不假装**:
//! - 真实数据应通过 ApeirethClient 拉
//! - 占位数据诚实标 "[stub]"
//!
//! **8 项承诺**: 全部遵守

use ratatui::layout::Rect;

use crate::http::Session;

/// Session nav 渲染 (返 String 喂 ratatui Paragraph)
///
/// **不假装**: 1 个 stub session + 明确标 "[stub] 真实数据待 R25.3"
pub fn render(area: Rect) -> String {
    let width = area.width as usize;
    let _ = width; // 暂未用 (未来列宽用)

    // 占位 session (诚实标缺)
    let sessions: Vec<Session> = vec![Session {
        id: "stub-session-001".to_string(),
        title: Some("Stub session".to_string()),
        created_at: Some("2026-08-04T22:00:00Z".to_string()),
        last_active_at: Some("2026-08-04T23:30:00Z".to_string()),
        message_count: Some(0),
    }];

    let mut out = String::new();
    out.push_str("═══ SESSION ═══\n");
    out.push_str(&format!("活跃会话: {}\n\n", sessions.len()));
    for s in &sessions {
        let title = s.title.as_deref().unwrap_or("(untitled)");
        let last = s.last_active_at.as_deref().unwrap_or("?");
        let count = s.message_count.unwrap_or(0);
        out.push_str(&format!(
            "  [{}] {}\n      last_active: {}  messages: {}\n",
            &s.id[..s.id.len().min(8)],
            title,
            last,
            count
        ));
    }
    out.push_str("\n[stub] 真实数据待 R25.3 接 /v1/sessions\n");
    out.push_str("键位: [Tab/1-5] 切 nav · [r] 刷新 · [q] 退出\n");
    out
}

// =====================================================================
// 单元测试 (4 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_session_count_header() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        assert!(out.contains("SESSION"));
        assert!(out.contains("活跃会话"));
    }

    #[test]
    fn render_marks_stub_honestly() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        assert!(out.contains("[stub]") || out.contains("stub"),
            "Session render 应明确标 stub, 不假装接 HTTP: {out}");
    }

    #[test]
    fn render_lists_stub_session() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        // R25.2 fix: render 用前 8 字符截断 (line 46: &s.id[..8]),
        // assertion 跟 render 输出一致
        assert!(out.contains("stub-ses"), "render 应含 ID 前 8 字符 'stub-ses': {out}");
        assert!(out.contains("Stub session"), "render 应含 stub title: {out}");
        // 完整 ID 也在 output 里 (作为 session metadata 完整, 可由 R25.3 拉真实数据时显示)
        // 注: 当前 render 只显示前 8 字符, 完整 ID 不在 output
    }

    #[test]
    fn render_includes_keybind_hint() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        // 应该有键位提示
        assert!(out.contains("[Tab") || out.contains("Tab"));
    }
}
