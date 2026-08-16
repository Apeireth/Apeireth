//! Apeireth R25.2 TUI — Help nav
//!
//! **职责**: 6 哲学锚 + 8 项承诺 + 1.0 release 文档索引
//!
//! **6 哲学锚** (per task spec):
//! - S-1 北极星 (Apeireth = AGI 操作系统)
//! - S-2 实事求是 (状态真, 不装)
//! - O-2 走在前人肩上 (ratatui + apeireth-api 复用)
//! - O-3 干到底 (TUI 完整可用, 不是半成品)
//! - O-4 任何人都能接手 (主题 + 帮助 + 文档)
//! - O-5 不假装 (5 器官未实接, 标 stub, 不假装跳)
//!
//! **8 项承诺** (per task spec):
//! - 不假装已实现 / 编译期 hardcode / 不改 LOCKED / 不改 workspace version
//! - 6 哲学锚穿透 / 不依赖 NewAPI / 不重复造轮子 / 诚实标缺
//!
//! **1.0 release 文档**: 主人 R17 战役 0-4 收官
//!
//! **不假装**:
//! - 6 锚 + 8 承诺全部 hardcode 字符串
//! - release 文档路径标 stub (per 1.0 release 收官后回填真实路径)
//!
//! **8 项承诺**: 全部遵守 + 自身是 8 项承诺的载体

use ratatui::layout::Rect;

/// 6 哲学锚 (编译期 hardcode, 不重复造轮子)
pub const SIX_ANCHORS: &[(&str, &str)] = &[
    ("S-1", "北极星 — Apeireth = AGI 操作系统"),
    ("S-2", "实事求是 — 状态真, 不装"),
    ("O-2", "走在前人肩上 — ratatui + apeireth-api 复用"),
    ("O-3", "干到底 — TUI 完整可用, 不是半成品"),
    ("O-4", "任何人都能接手 — 主题 + 帮助 + 文档"),
    ("O-5", "不假装 — 5 器官未实接, 标 stub, 不假装跳"),
];

/// 8 项不修改承诺
pub const EIGHT_PROMISES: &[&str] = &[
    "不假装已实现 (9 器官按实接程度标 ok/partial/stub)",
    "编译期 hardcode (5 nav + 9 器官 enum)",
    "不改 LOCKED (24 LOCKED crate 的 src/ 不动)",
    "不改 workspace version",
    "6 哲学锚穿透 (mind organ + Help nav 显示 6 锚)",
    "不依赖 NewAPI",
    "不重复造轮子 (用 ratatui + apeireth-api 已有)",
    "诚实标缺 (5 器官 partial/stub 明说)",
];

/// Help nav 渲染 (返 String 喂 ratatui Paragraph)
pub fn render(area: Rect) -> String {
    let _ = area;

    let mut out = String::new();
    out.push_str("═══ HELP ═══\n\n");

    out.push_str("── 6 哲学锚 ──\n");
    for (code, text) in SIX_ANCHORS {
        out.push_str(&format!("  {code}  {text}\n"));
    }
    out.push('\n');

    out.push_str("── 8 项不修改承诺 ──\n");
    for (i, p) in EIGHT_PROMISES.iter().enumerate() {
        out.push_str(&format!("  {}. {p}\n", i + 1));
    }
    out.push('\n');

    out.push_str("── 1.0 Release 文档索引 ──\n");
    out.push_str("  [stub] 主人 R17 战役 0-4 收官后回填真实路径\n");
    out.push_str("  - R17 战役 0-4 收官: TBD\n");
    out.push_str("  - R22 5 nav 拍板: TBD\n");
    out.push_str("  - R25 改瘦 Step 1 (HTTP 客户端): src/http_llm.rs\n");
    out.push_str("  - R25.2 5 nav + 9 器官 (本文件): src/nav/ + src/organ/\n");

    out.push_str("\n键位: [Tab/1-5] 切 nav · [q] 退出\n");
    out
}

// =====================================================================
// 单元测试 (4 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_anchors_all_present() {
        assert_eq!(SIX_ANCHORS.len(), 6);
        let codes: Vec<&str> = SIX_ANCHORS.iter().map(|(c, _)| *c).collect();
        assert!(codes.contains(&"S-1"));
        assert!(codes.contains(&"S-2"));
        assert!(codes.contains(&"O-2"));
        assert!(codes.contains(&"O-3"));
        assert!(codes.contains(&"O-4"));
        assert!(codes.contains(&"O-5"));
    }

    #[test]
    fn eight_promises_count() {
        assert_eq!(EIGHT_PROMISES.len(), 8);
    }

    #[test]
    fn render_lists_all_anchors_and_promises() {
        let area = Rect::new(0, 0, 80, 50);
        let out = render(area);
        for (code, _) in SIX_ANCHORS {
            assert!(out.contains(code), "render 应含锚 {code}");
        }
        for (i, _) in EIGHT_PROMISES.iter().enumerate() {
            assert!(out.contains(&format!("{}.", i + 1)));
        }
    }

    #[test]
    fn render_marks_release_doc_stub() {
        let area = Rect::new(0, 0, 80, 50);
        let out = render(area);
        assert!(
            out.contains("[stub]") || out.contains("stub"),
            "release 文档应明确标 stub, 不假装路径"
        );
    }
}
