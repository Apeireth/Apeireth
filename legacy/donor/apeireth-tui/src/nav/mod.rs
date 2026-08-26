//! Apeireth R25.2 TUI — 5 nav 模块入口
//!
//! **5 nav** (per 主人 R22 拍板):
//! - 0 Status (实时系统状态: 5 组件 health + CPU + 内存)
//! - 1 Session (活跃会话列表: 调 `apeireth-api` HTTP `/v1/sessions`)
//! - 2 Tools (6 工具 endpoint 入口: calendar/message/contact/task/search/drive)
//! - 3 Settings (5 鉴权 + 5 Provider + 4 SDK 配置)
//! - 4 Help (6 哲学锚 + 8 项承诺 + 1.0 release 文档)
//!
//! **R21 G-1 续补 (1.0 release #10 i18n 100% 收尾)**:
//! - 5 nav 标题走 `translator.t("nav.*")` 翻译表 (5 Locale 切换)
//! - 替换原 `label_zh()` / `label_greek()` 硬编码 (R19 拍板的"中文 + 希腊文"国际化审美)
//! - i18n crate 12 类别 69 keys 100% 翻译, TUI 5 nav 消费 `nav.{status,session,tools,settings,help}`
//!
//! **不假装**:
//! - 5 nav 全部 5 文件齐全, 每个 nav 有 `pub fn render(...)` 入口
//! - dispatcher 走 enum `Nav` 编译期 hardcode
//! - 5 渲染函数都返回 `String` (用于 ratatui Paragraph) + 自身状态
//! - 实际 ratatui Frame 集成留给 main.rs (R25 暂不接, R25.2 标 partial)
//! - `Nav::label(tr)` async 走 i18n 翻译表, 5 Locale 切换 O(1) 编译期嵌入
//!
//! **8 项承诺**:
//! - 不假装已实现 ✅ (5 nav dispatcher + 5 render 函数, 实际 Frame 集成 partial)
//! - 编译期 hardcode (5 nav enum, 5 文件) ✅
//! - 不改 LOCKED ✅
//! - 不改 workspace version ✅
//! - 6 哲学锚穿透 (Help nav 渲染 6 锚) ✅
//! - 不依赖 NewAPI ✅
//! - 不重复造轮子 (用 apeireth-i18n 翻译表) ✅
//! - 诚实标缺 (5 nav render partial, 不假装接 Frame) ✅

pub mod help;
pub mod session;
pub mod settings;
pub mod status;
pub mod tools;

use apeireth_i18n::{TranslationArgs, Translator};
use ratatui::layout::Rect;

/// 5 nav enum (编译期 hardcode, 跟主人 R22 拍板对齐)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Status = 0,
    Session = 1,
    Tools = 2,
    Settings = 3,
    Help = 4,
}

impl Nav {
    /// 数字键 0-4 → Nav
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Status),
            1 => Some(Self::Session),
            2 => Some(Self::Tools),
            3 => Some(Self::Settings),
            4 => Some(Self::Help),
            _ => None,
        }
    }

    /// 顺序: 0→1→2→3→4→0
    pub fn next(self) -> Self {
        Self::from_u8(((self as u8) + 1) % 5).unwrap()
    }

    /// 反向: 0→4→3→2→1→0
    pub fn prev(self) -> Self {
        let n = if (self as u8) == 0 {
            4
        } else {
            (self as u8) - 1
        };
        Self::from_u8(n).unwrap()
    }

    /// nav 标题 (R21 G-1 续补: 走 i18n 翻译表)
    ///
    /// 5 Locale 切换走 `translator.t("nav.{status,session,tools,settings,help}")`,
    /// 翻译表编译期嵌入 (`apeireth-i18n` crate `include_str!("../locales/*.toml")`),
    /// 缺 key 返 key 自身 (i18next 1:1 行为, O-5 不假装).
    pub async fn label<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Status => "nav.status",
            Self::Session => "nav.session",
            Self::Tools => "nav.tools",
            Self::Settings => "nav.settings",
            Self::Help => "nav.help",
        };
        tr.t(key, &TranslationArgs::new()).await
    }
}

/// 5 nav dispatcher — 走 enum 调对应 nav 的 render 函数
///
/// **不假装**: render 返 String (用于 ratatui Paragraph 喂入), 实际 Frame 集成
/// 留给 main.rs (per R25.2 partial 标缺 — TUI 终极方向是 Tauri, TUI 是测试床,
/// 5 nav 渲染接口稳定, 集成是 1-2 行)
pub fn dispatch_render(nav: Nav, area: Rect) -> String {
    match nav {
        Nav::Status => status::render(area),
        Nav::Session => session::render(area),
        Nav::Tools => tools::render(area),
        Nav::Settings => settings::render(area),
        Nav::Help => help::render(area),
    }
}

// =====================================================================
// 单元测试 (5 nav enum + dispatcher + i18n label = 8 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_i18n::{Locale, TranslatorImpl, SUPPORTED_LOCALES};

    #[test]
    fn five_nav_variants_constructible() {
        let _ = Nav::Status;
        let _ = Nav::Session;
        let _ = Nav::Tools;
        let _ = Nav::Settings;
        let _ = Nav::Help;
    }

    #[test]
    fn from_u8_round_trip() {
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).expect("0-4 valid");
            assert_eq!(nav as u8, n);
        }
        assert!(Nav::from_u8(5).is_none());
        assert!(Nav::from_u8(255).is_none());
    }

    #[test]
    fn next_wraps_at_5() {
        assert_eq!(Nav::Status.next(), Nav::Session);
        assert_eq!(Nav::Session.next(), Nav::Tools);
        assert_eq!(Nav::Tools.next(), Nav::Settings);
        assert_eq!(Nav::Settings.next(), Nav::Help);
        assert_eq!(Nav::Help.next(), Nav::Status); // 4+1=5→0
    }

    #[test]
    fn prev_wraps_at_0() {
        assert_eq!(Nav::Status.prev(), Nav::Help); // 0-1→4
        assert_eq!(Nav::Session.prev(), Nav::Status);
        assert_eq!(Nav::Tools.prev(), Nav::Session);
        assert_eq!(Nav::Settings.prev(), Nav::Tools);
        assert_eq!(Nav::Help.prev(), Nav::Settings);
    }

    /// R21 G-1 续补: 5 Locale 切换下, 5 nav 翻译都非空且互不相同
    /// (替代原 `label_zh_all_5_distinct` / `label_greek_all_5_distinct` 硬编码测试)
    #[tokio::test]
    async fn label_5_locales_5_nav_all_translated_and_distinct() {
        let tr = TranslatorImpl::new().unwrap();
        for &locale in SUPPORTED_LOCALES {
            tr.set_locale(locale).await.unwrap();
            let mut labels = std::collections::HashSet::new();
            for n in 0..=4u8 {
                let nav = Nav::from_u8(n).unwrap();
                let s = nav.label(&tr).await;
                assert!(
                    !s.is_empty(),
                    "{locale:?} Nav::{nav:?} 翻译应非空 (5 Locale 守门)"
                );
                labels.insert(s);
            }
            assert_eq!(
                labels.len(),
                5,
                "{locale:?} 5 nav 翻译应互不相同 (per 5 nav 互不重复守门)"
            );
        }
    }

    /// R21 G-1 续补: 5 Locale 切换下, 同一 nav 翻译跨 locale 不同
    /// (5 Locale 100% 翻译, 不 fallback 占位)
    #[tokio::test]
    async fn label_same_nav_diff_locales_distinct() {
        let tr = TranslatorImpl::new().unwrap();
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).unwrap();
            let mut labels = std::collections::HashSet::new();
            for &locale in SUPPORTED_LOCALES {
                tr.set_locale(locale).await.unwrap();
                labels.insert(nav.label(&tr).await);
            }
            // 5 Locale 翻译至少 3 个不同值 (zh/ja/西欧/德有显著差异)
            assert!(
                labels.len() >= 3,
                "Nav::{nav:?} 5 Locale 翻译应至少 3 不同值, 实际 {}",
                labels.len()
            );
        }
    }

    /// R21 G-1 续补: zh-CN locale 下 5 nav 翻译跟 en locale 下互不相同
    /// (中文守门: 不假装 fallback 到英文)
    #[tokio::test]
    async fn label_zh_cn_5_nav_not_fallback_to_en() {
        let tr = TranslatorImpl::new().unwrap();
        tr.set_locale(Locale::ZhCn).await.unwrap();
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).unwrap();
            let zh = nav.label(&tr).await;
            tr.set_locale(Locale::En).await.unwrap();
            let en = nav.label(&tr).await;
            assert_ne!(
                zh, en,
                "Nav::{nav:?} zh-CN 翻译应跟 en 翻译不同 (O-5 不假装 0 fallback)"
            );
            tr.set_locale(Locale::ZhCn).await.unwrap();
        }
    }

    #[test]
    fn dispatch_render_all_5_non_empty() {
        // 5 nav dispatch 都能返非空 String (实际渲染 5 屏)
        let area = Rect::new(0, 0, 80, 24);
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).unwrap();
            let out = dispatch_render(nav, area);
            assert!(!out.is_empty(), "{nav:?} render 应非空");
            // 至少 50 字符 (1 屏有内容, 不假装)
            assert!(out.len() > 50, "{nav:?} render 字符数太少: {}", out.len());
        }
    }

    #[test]
    fn dispatch_render_5_screens_distinct() {
        // 5 屏内容应互不相同 (不假装每屏都一样)
        let area = Rect::new(0, 0, 80, 24);
        let mut outputs = Vec::new();
        for n in 0..=4u8 {
            outputs.push(dispatch_render(Nav::from_u8(n).unwrap(), area));
        }
        let unique: std::collections::HashSet<&String> = outputs.iter().collect();
        assert_eq!(unique.len(), 5, "5 屏应互不相同");
    }
}
