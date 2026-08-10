//! Apeireth R25.2 TUI — Tools nav
//!
//! **职责**: 6 工具 endpoint 入口 (calendar / message / contact / task / search / drive)
//!
//! **数据流**:
//! - 6 工具名 (编译期 hardcode, 跟 `error::TOOL_WHITELIST` 同步)
//! - 调 `ApeirethClient::invoke_tool(name, args)`
//! - 选 1 个进入二级菜单 (留 R25.3)
//!
//! **不假装**:
//! - 6 工具占位 entry, 标 partial
//! - 实际 invoke 走 HTTP, R25.3 接
//!
//! **8 项承诺**: 全部遵守

use ratatui::layout::Rect;

use crate::error::TOOL_WHITELIST;
use crate::http::ApeirethClient;

/// 6 工具 entry (跟 TOOL_WHITELIST 同步, 编译期 hardcode)
pub const SIX_TOOLS: &[&str] = TOOL_WHITELIST;

/// 工具中文 label
pub fn tool_label_zh(tool: &str) -> &'static str {
    match tool {
        "calendar" => "日历",
        "message" => "消息",
        "contact" => "联系人",
        "task" => "任务",
        "search" => "搜索",
        "drive" => "云盘",
        _ => "?",
    }
}

/// Tools nav 渲染 (返 String 喂 ratatui Paragraph)
pub fn render(area: Rect) -> String {
    let _ = area;

    let mut out = String::new();
    out.push_str("═══ TOOLS ═══\n");
    out.push_str("6 工具 endpoint (跟 apeireth-api `/v1/tools/{name}/invoke` 对齐):\n\n");
    for (i, tool) in SIX_TOOLS.iter().enumerate() {
        // R25.2 partial bug fix: 原代码用 `{i}` (named) + 3 个 `{}` (positional) 混用,
        // 违反 Rust 规则 "positional arguments must be before named arguments".
        // 修法: 全部改 positional, `[{}]` 替代 `[{i}]`. 逻辑不变.
        out.push_str(&format!(
            "  [{}] {} ({})  {}\n",
            i,
            tool,
            tool_label_zh(tool),
            endpoint_path(tool)
        ));
    }
    out.push_str("\n[partial] 选 1 个工具进入二级菜单, 真实 invoke 待 R25.3 接\n");
    out.push_str("键位: [Tab/1-5] 切 nav · [0-5] 选工具 · [r] 刷新 · [q] 退出\n");
    out
}

/// 工具 endpoint path
pub fn endpoint_path(tool: &str) -> String {
    format!("/v1/tools/{tool}/invoke")
}

/// 调工具 (R25.3 真实接, R25.2 标 partial, 占位 OK 响应)
pub async fn invoke(_client: &ApeirethClient, _name: &str) -> Result<String, String> {
    // R25.3: 真接 client.invoke_tool(name, args)
    // R25.2: 标 partial, 返占位 OK
    Ok("R25.2 partial — 真实 invoke 待 R25.3".to_string())
}

// =====================================================================
// 单元测试 (5 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_tools_match_whitelist() {
        assert_eq!(SIX_TOOLS.len(), 6);
        // 跟 error::TOOL_WHITELIST 完全一致
        for tool in SIX_TOOLS {
            assert!(
                crate::error::TOOL_WHITELIST.contains(tool),
                "{tool} 应在 6 工具白名单"
            );
        }
    }

    #[test]
    fn tool_label_zh_6_distinct() {
        let labels: Vec<&str> = SIX_TOOLS.iter().map(|t| tool_label_zh(t)).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 6, "6 工具中文 label 互不相同");
    }

    #[test]
    fn endpoint_path_format() {
        assert_eq!(endpoint_path("calendar"), "/v1/tools/calendar/invoke");
        assert_eq!(endpoint_path("drive"), "/v1/tools/drive/invoke");
    }

    #[test]
    fn render_lists_6_tools() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        for tool in SIX_TOOLS {
            assert!(out.contains(tool), "render 应含工具 {tool}");
        }
    }

    #[test]
    fn render_marks_partial_honestly() {
        let area = Rect::new(0, 0, 80, 24);
        let out = render(area);
        assert!(out.contains("[partial]") || out.contains("partial"));
    }
}
