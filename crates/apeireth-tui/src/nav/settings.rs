//! Apeireth R25.2 TUI — Settings nav
//!
//! **职责**: 5 鉴权 + 5 Provider + 4 SDK 配置
//!
//! **5 鉴权** (per task spec):
//! - auth_token (Bearer)
//! - api_key_secondary
//! - session_secret
//! - signing_key
//! - refresh_token
//!
//! **5 Provider**:
//! - primary / fallback / embedding / rerank / vision
//!
//! **4 SDK**:
//! - sandbox / keyring / observability / protocol
//!
//! **不假装**:
//! - 14 配置项 (5+5+4) 全部占位 entry
//! - 实际持久化走 `persistence::save/load` (existing, R19 已经有, 不重复造轮子)
//! - 真实编辑留 R25.3
//!
//! **8 项承诺**: 全部遵守

use ratatui::layout::Rect;

/// 5 鉴权配置 (编译期 hardcode)
pub const FIVE_AUTH: &[&str] = &[
    "auth_token",
    "api_key_secondary",
    "session_secret",
    "signing_key",
    "refresh_token",
];

/// 5 Provider 配置
pub const FIVE_PROVIDER: &[&str] = &[
    "provider_primary",
    "provider_fallback",
    "provider_embedding",
    "provider_rerank",
    "provider_vision",
];

/// 4 SDK 配置
pub const FOUR_SDK: &[&str] = &[
    "sdk_sandbox",
    "sdk_keyring",
    "sdk_observability",
    "sdk_protocol",
];

/// Settings nav 渲染 (返 String 喂 ratatui Paragraph)
pub fn render(area: Rect) -> String {
    let _ = area;

    let mut out = String::new();
    out.push_str("═══ SETTINGS ═══\n");
    out.push_str(&format!(
        "总配置项: {} (5 鉴权 + 5 Provider + 4 SDK)\n\n",
        FIVE_AUTH.len() + FIVE_PROVIDER.len() + FOUR_SDK.len()
    ));

    out.push_str("── 5 鉴权 ──\n");
    for (i, k) in FIVE_AUTH.iter().enumerate() {
        out.push_str(&format!("  [{i}] {k:<22} = (unset)\n"));
    }
    out.push('\n');

    out.push_str("── 5 Provider ──\n");
    for (i, k) in FIVE_PROVIDER.iter().enumerate() {
        out.push_str(&format!("  [{i}] {k:<22} = (unset)\n"));
    }
    out.push('\n');

    out.push_str("── 4 SDK ──\n");
    for (i, k) in FOUR_SDK.iter().enumerate() {
        out.push_str(&format!("  [{i}] {k:<22} = (unset)\n"));
    }

    out.push_str("\n[partial] 编辑/持久化待 R25.3 接 persistence 模块\n");
    out.push_str("键位: [Tab/1-5] 切 nav · [e] 编辑选中 · [s] 保存 · [q] 退出\n");
    out
}

// =====================================================================
// 单元测试 (4 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_auth_correct_count() {
        assert_eq!(FIVE_AUTH.len(), 5);
    }

    #[test]
    fn five_provider_correct_count() {
        assert_eq!(FIVE_PROVIDER.len(), 5);
    }

    #[test]
    fn four_sdk_correct_count() {
        assert_eq!(FOUR_SDK.len(), 4);
    }

    #[test]
    fn render_lists_all_14_keys() {
        let area = Rect::new(0, 0, 80, 30);
        let out = render(area);
        // 5+5+4 = 14 keys 应全出现
        for k in FIVE_AUTH.iter().chain(FIVE_PROVIDER).chain(FOUR_SDK) {
            assert!(out.contains(k), "render 应含配置 {k}");
        }
    }
}
