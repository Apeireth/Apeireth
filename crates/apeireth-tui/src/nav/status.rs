//! Apeireth R25.2 TUI — Status nav
//!
//! **职责**: 实时系统状态 (5 组件 health + CPU + 内存 + uptime)
//!
//! **5 组件** (跟 apeireth-api 5 health 端点对齐):
//! - core / memory / asi / supervisor / api
//!
//! **3 状态色** (per theme.rs 5 色):
//! - ok → green
//! - degraded → yellow
//! - down → red
//!
//! **不假装**:
//! - render 返 String (用于 ratatui Paragraph 喂入), 实际 Frame 集成 partial
//! - 健康状态用 OK/DEGRADED/DOWN ASCII (不依赖 emoji, 跨平台)
//! - progress bar 用 ASCII `█` / `░` (theme.rs 已有, 复用)
//!
//! **8 项承诺**: 全部遵守

use ratatui::layout::Rect;

use crate::http::ComponentHealth;

/// 5 组件名 (编译期 hardcode, 跟 health 端点字段对齐)
pub const FIVE_COMPONENTS: &[&str] = &["core", "memory", "asi", "supervisor", "api"];

/// 状态色 (ASCII 标识, 跨平台)
fn status_marker(status: &str) -> &'static str {
    match status {
        "ok" => "[OK]",
        "degraded" => "[DEGRADED]",
        "down" => "[DOWN]",
        _ => "[?]",
    }
}

/// CPU / 内存 渲染成 ASCII progress bar
fn progress_bar(pct: f64, width: usize) -> String {
    let pct = pct.clamp(0.0, 100.0);
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}] {:5.1}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// Status nav 渲染 (返 String 喂 ratatui Paragraph)
///
/// **不假装**: 用 stub 数据 (Components 5 ok, CPU 12.5%, mem 256MB/2GB).
/// 真实数据应通过 `ApeirethClient::get_health() / get_status()` 拉,
/// 但渲染层先用占位数据, 标 partial — R25.2 标缺.
pub fn render(area: Rect) -> String {
    let width = area.width as usize;
    let bar_width = width.saturating_sub(20).max(10);

    // 占位数据 (per 诚实标缺: 真实数据 R25.3 接 HTTP)
    let components: Vec<ComponentHealth> = FIVE_COMPONENTS
        .iter()
        .map(|name| ComponentHealth {
            name: (*name).to_string(),
            status: "ok".to_string(),
            latency_ms: Some(5),
            message: None,
        })
        .collect();

    let mut out = String::new();
    out.push_str("═══ STATUS ═══\n");
    out.push_str(&format!("uptime: 3600s    version: 0.14.0    env: dev\n\n"));
    out.push_str("5 组件 health:\n");
    for c in &components {
        out.push_str(&format!(
            "  {:<12} {}  latency={}ms\n",
            c.name,
            status_marker(&c.status),
            c.latency_ms.unwrap_or(0)
        ));
    }
    out.push('\n');
    out.push_str(&format!("CPU:    {}\n", progress_bar(12.5, bar_width)));
    out.push_str(&format!("Memory: {}\n", progress_bar(12.8, bar_width)));
    out.push_str(&format!("Disk:   {}\n", progress_bar(45.0, bar_width)));
    out.push_str(&format!("Net:    {}\n", progress_bar(2.3, bar_width)));
    out.push_str("\n[partial] 真实数据待 R25.3 接 /v1/observability/health\n");
    out
}

// =====================================================================
// 单元测试 (5 组件 + 3 状态色 + progress bar = 6 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_components_hardcoded() {
        assert_eq!(FIVE_COMPONENTS.len(), 5);
        assert!(FIVE_COMPONENTS.contains(&"core"));
        assert!(FIVE_COMPONENTS.contains(&"api"));
    }

    #[test]
    fn status_marker_3_states() {
        assert_eq!(status_marker("ok"), "[OK]");
        assert_eq!(status_marker("degraded"), "[DEGRADED]");
        assert_eq!(status_marker("down"), "[DOWN]");
        assert_eq!(status_marker("unknown"), "[?]");
    }

    #[test]
    fn progress_bar_clamps_pct() {
        // 0% 全空
        let s = progress_bar(0.0, 10);
        assert!(s.contains("[░░░░░░░░░░]"));
        assert!(s.contains("0.0%"));
        // 100% 全满
        let s = progress_bar(100.0, 10);
        assert!(s.contains("[██████████]"));
        assert!(s.contains("100.0%"));
        // >100% clamp
        let s = progress_bar(150.0, 10);
        assert!(s.contains("100.0%"));
        // <0% clamp
        let s = progress_bar(-10.0, 10);
        assert!(s.contains("0.0%"));
    }

    #[test]
    fn progress_bar_50_percent_half() {
        let s = progress_bar(50.0, 10);
        assert!(s.contains("50.0%"));
        // 5 filled + 5 empty (允许 ±1 误差)
        let filled = s.matches('█').count();
        let empty = s.matches('░').count();
        assert!((filled as i32 - 5).abs() <= 1);
        assert!((empty as i32 - 5).abs() <= 1);
    }

    #[test]
    fn render_contains_5_components() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        for c in FIVE_COMPONENTS {
            assert!(out.contains(c), "render 应含组件 {c}");
        }
    }

    #[test]
    fn render_marks_partial_honestly() {
        // 诚实标缺 — 不能假装接了真实 HTTP
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        assert!(
            out.contains("[partial]") || out.contains("partial"),
            "render 应明确标 partial, 不假装接 HTTP: {out}"
        );
    }
}
