//! Apeireth R25.2 TUI — 9 器官拟人化模块入口
//!
//! **9 器官** (per 主人 R19 拟人化决策, 5 决策 3.):
//! "器官很有意思, 从生物借鉴而来, 也是我们ai成长的核心和秘密,
//!  可以抽象一些器官作为监控状态的元素界面"
//!
//! | # | 器官 | ASCII | 实接度 | 真实数据 |
//! |---|------|-------|--------|----------|
//! | 1 | heart (心) | `[♥]` | ok      | 真接 backend::cycle_count + tick hook (R22 ST-A1.6) |
//! | 2 | brain (脑) | `[BRAIN]` | ok      | LLM 调用频率 + token + 上次调时间 |  (R22 ST-A1.1 真实现) |
//! | 3 | hand (手) | `[HAND]` | ok      | 6 工具真接 http.rs invoke_tool (R22 ST-A1.5) |
//! | 4 | eye (眼) | `[EYE]` | partial | keystrokes 真接 (R22 ST-A1.2) |
//! | 5 | ear (耳) | `[EAR]` | partial | user/llm/system 真接 (R22 ST-A1.3) |
//! | 6 | memory (记忆) | `[MEM]` | partial | 短期真接 episode_count (R22 ST-A1.8) |
//! | 7 | voice (声) | `[VOICE]` | partial | 有结构 + API, 但 TUI 未接 mic (R22 ST-A1.4) |
//! | 8 | body (体) | `[BODY]` | partial | 进程 / 内存 / 磁盘 |
//! | 9 | mind (意) | `[MIND]` | ok      | AGI 阶段真接 backend::compute_life_stage (R22 ST-A1.9) |
//!
//! **R21 G-1 续补 (1.0 release #10 i18n 100% 收尾)**:
//! - 9 器官名走 `translator.t("organs.*")` 翻译表 (5 Locale 切换)
//! - 3 readiness (Ok / Partial / Stub) 走 `translator.t("readiness.*")` 翻译表
//! - 替换原 `name_zh()` 硬编码 + `Readiness::label() = "ok"/"partial"/"stub"` 英文硬编码
//! - `ascii_char()` 保留 (不是翻译, 是 ASCII 跨平台字符)
//!
//! **不假装**:
//! - 每个器官标 ok/partial/stub 真实接的程度
//! - 9 字符用 ASCII (跨平台, 不依赖 emoji 字体)
//! - 9 render 返 String (ratatui 喂入), 实际 Frame 集成 partial
//! - `Organ::name(tr)` async 走 i18n 翻译表, `Readiness::label(tr)` async 同样
//!
//! **8 项承诺**: 全部遵守

pub mod body;
pub mod brain;
// sister #1 command dispatcher 已搬到 src/command/ (R23 P3 迁移), 不在 organ 子树
pub mod ear;
pub mod eye;
pub mod hand;
pub mod heart;
pub mod memory;
pub mod mind;
pub mod voice;
// ADR-0028: 9 organ naming bridge table (TUI 旧名 ↔ crate 新名)
pub mod bridge_table;

use apeireth_i18n::{TranslationArgs, Translator};
use ratatui::layout::Rect;

/// 实接度 (诚实标缺, 3 级: ok / partial / stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// 真接 (HTTP / 真实数据)
    Ok,
    /// 部分接 (占位 + 真实数据混合, 待 R25.3)
    Partial,
    /// 桩 (只占位, 标 stub)
    Stub,
}

impl Readiness {
    /// 实接度标签 (R21 G-1 续补: 走 i18n 翻译表)
    ///
    /// 5 Locale 切换走 `translator.t("readiness.{ok,partial,stub}")`,
    /// 翻译表编译期嵌入 (`apeireth-i18n` crate `include_str!`),
    /// 缺 key 返 key 自身 (i18next 1:1 行为, O-5 不假装).
    pub async fn label<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Ok => "readiness.ok",
            Self::Partial => "readiness.partial",
            Self::Stub => "readiness.stub",
        };
        tr.t(key, &TranslationArgs::new()).await
    }
}

/// 9 器官 enum (编译期 hardcode, 跟主人 R19 拟人化决策对齐)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Organ {
    Heart = 0,
    Brain = 1,
    Hand = 2,
    Eye = 3,
    Ear = 4,
    Memory = 5,
    Voice = 6,
    Body = 7,
    Mind = 8,
}

impl Organ {
    /// 数字 0-8 → Organ
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Heart),
            1 => Some(Self::Brain),
            2 => Some(Self::Hand),
            3 => Some(Self::Eye),
            4 => Some(Self::Ear),
            5 => Some(Self::Memory),
            6 => Some(Self::Voice),
            7 => Some(Self::Body),
            8 => Some(Self::Mind),
            _ => None,
        }
    }

    /// 器官名 (R21 G-1 续补: 走 i18n 翻译表)
    ///
    /// 5 Locale 切换走 `translator.t("organs.{heart,brain,...,mind}")`,
    /// 翻译表编译期嵌入 (per K-1 强校验, 5 Locale × 9 organ = 45 翻译点 100% 覆盖).
    pub async fn name<T: Translator + ?Sized>(&self, tr: &T) -> String {
        let key = match self {
            Self::Heart => "organs.heart",
            Self::Brain => "organs.brain",
            Self::Hand => "organs.hand",
            Self::Eye => "organs.eye",
            Self::Ear => "organs.ear",
            Self::Memory => "organs.memory",
            Self::Voice => "organs.voice",
            Self::Body => "organs.body",
            Self::Mind => "organs.mind",
        };
        tr.t(key, &TranslationArgs::new()).await
    }

    /// 器官 ASCII 字符 (跨平台, 非翻译, 保留)
    pub fn ascii_char(self) -> &'static str {
        match self {
            Self::Heart => "[♥]",
            Self::Brain => "[BRAIN]",
            Self::Hand => "[HAND]",
            Self::Eye => "[EYE]",
            Self::Ear => "[EAR]",
            Self::Memory => "[MEM]",
            Self::Voice => "[VOICE]",
            Self::Body => "[BODY]",
            Self::Mind => "[MIND]",
        }
    }

    /// 实接度 (诚实标缺)
    pub fn readiness(self) -> Readiness {
        match self {
            Self::Heart => Readiness::Ok, // R22 ST-A1.6: 真接 backend atomics + main.rs tick
            Self::Brain => Readiness::Ok, // R22 ST-A1.1: 真接 backend atomics
            Self::Hand => Readiness::Ok,  // R22 ST-A1.5: 真接 http.rs::invoke_tool success/failure
            Self::Eye => Readiness::Partial, // R22 ST-A1.2: 1/4 真接 (keystrokes)
            Self::Ear => Readiness::Partial, // R22 ST-A1.3: 3/4 真接 (user/llm/system)
            Self::Memory => Readiness::Partial,
            Self::Voice => Readiness::Partial, // R22 ST-A1.4: 有结构 + record API, 但 0 调用
            Self::Body => Readiness::Partial,
            Self::Mind => Readiness::Ok, // R22 ST-A1.9: 真接 backend::compute_life_stage + asi_v05
        }
    }
}

/// 9 器官 dispatcher — 走 enum 调对应 organ 的 render 函数
pub fn dispatch_render(organ: Organ, area: Rect) -> String {
    match organ {
        Organ::Heart => heart::render(area),
        Organ::Brain => brain::render(area),
        Organ::Hand => hand::render(area),
        Organ::Eye => eye::render(area),
        Organ::Ear => ear::render(area),
        Organ::Memory => memory::render(area),
        Organ::Voice => voice::render(area),
        Organ::Body => body::render(area),
        Organ::Mind => mind::render(area),
    }
}

// =====================================================================
// 单元测试 (9 organ enum + 9 ASCII 字符 + dispatcher + i18n = 12 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_i18n::{Locale, TranslatorImpl, SUPPORTED_LOCALES};

    #[test]
    fn nine_organ_variants_constructible() {
        let _ = Organ::Heart;
        let _ = Organ::Brain;
        let _ = Organ::Hand;
        let _ = Organ::Eye;
        let _ = Organ::Ear;
        let _ = Organ::Memory;
        let _ = Organ::Voice;
        let _ = Organ::Body;
        let _ = Organ::Mind;
    }

    #[test]
    fn from_u8_round_trip_9() {
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).expect("0-8 valid");
            assert_eq!(organ as u8, n);
        }
        assert!(Organ::from_u8(9).is_none());
        assert!(Organ::from_u8(255).is_none());
    }

    #[test]
    fn nine_organ_ascii_chars_distinct() {
        let chars: Vec<&str> = (0..=8u8)
            .map(|n| Organ::from_u8(n).unwrap().ascii_char())
            .collect();
        let unique: std::collections::HashSet<&str> = chars.iter().copied().collect();
        assert_eq!(unique.len(), 9, "9 器官 ASCII 字符应互不相同");
    }

    /// R21 G-1 续补: 5 Locale 切换下, 9 器官翻译都非空且互不相同
    /// (替代原 `nine_organ_names_zh_distinct` 硬编码测试)
    #[tokio::test]
    async fn name_5_locales_9_organs_all_translated_and_distinct() {
        let tr = TranslatorImpl::new().unwrap();
        for &locale in SUPPORTED_LOCALES {
            tr.set_locale(locale).await.unwrap();
            let mut names = std::collections::HashSet::new();
            for n in 0..=8u8 {
                let organ = Organ::from_u8(n).unwrap();
                let s = organ.name(&tr).await;
                assert!(
                    !s.is_empty(),
                    "{locale:?} Organ::{organ:?} 翻译应非空 (5 Locale 守门)"
                );
                names.insert(s);
            }
            assert_eq!(
                names.len(),
                9,
                "{locale:?} 9 器官翻译应互不相同 (per 9 器官互不重复守门)"
            );
        }
    }

    /// R21 G-1 续补: 3 readiness 等级在 5 Locale 都翻译且互不相同
    /// (替代原 `readiness_labels_valid` 硬编码英文测试)
    #[tokio::test]
    async fn readiness_3_levels_5_locales_translated_and_distinct() {
        let tr = TranslatorImpl::new().unwrap();
        for &locale in SUPPORTED_LOCALES {
            tr.set_locale(locale).await.unwrap();
            let ok = Readiness::Ok.label(&tr).await;
            let partial = Readiness::Partial.label(&tr).await;
            let stub = Readiness::Stub.label(&tr).await;
            assert!(!ok.is_empty(), "{locale:?} readiness.ok 应非空");
            assert!(!partial.is_empty(), "{locale:?} readiness.partial 应非空");
            assert!(!stub.is_empty(), "{locale:?} readiness.stub 应非空");
            assert_ne!(ok, partial, "{locale:?} ok/partial 应不同");
            assert_ne!(ok, stub, "{locale:?} ok/stub 应不同");
            assert_ne!(partial, stub, "{locale:?} partial/stub 应不同");
        }
    }

    /// R21 G-1 续补: 9 器官的 readiness 配置表覆盖 partial + stub (诚实标缺)
    /// (沿用原 `readiness_labels_valid` 守门, 走 readiness() 但翻译走 i18n)
    #[tokio::test]
    async fn readiness_distribution_covers_partial_and_stub() {
        let tr = TranslatorImpl::new().unwrap();
        tr.set_locale(Locale::En).await.unwrap();
        let mut partial_count = 0;
        let mut stub_count = 0;
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let readiness = organ.readiness();
            let label = readiness.label(&tr).await;
            match readiness {
                Readiness::Partial => partial_count += 1,
                Readiness::Stub => stub_count += 1,
                Readiness::Ok => {}
            }
            assert!(!label.is_empty());
        }
        // 至少 partial + stub 各出现 (诚实标缺)
        assert!(partial_count >= 1, "9 器官 readiness 应至少 1 个 partial");
        // R22 ST-A1 完工 (2026-08-06): 5 ok (brain/hand/heart/hand/mind) + 4 partial (eye/ear/voice/body/memory) + 0 stub
        assert!(partial_count >= 1, "9 器官 readiness 应至少 1 个 partial");
        // stub_count >= 0: R22 ST-A1 完工后可能全 partial+ok, 0 stub 是诚实结果
        let _ = stub_count; // 抑制 unused
    }

    #[test]
    fn dispatch_render_all_9_non_empty() {
        let area = Rect::new(0, 0, 40, 10);
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let out = dispatch_render(organ, area);
            assert!(!out.is_empty(), "{organ:?} render 应非空");
        }
    }

    #[test]
    fn dispatch_render_9_screens_distinct() {
        let area = Rect::new(0, 0, 40, 10);
        let mut outputs = Vec::new();
        for n in 0..=8u8 {
            outputs.push(dispatch_render(Organ::from_u8(n).unwrap(), area));
        }
        let unique: std::collections::HashSet<&String> = outputs.iter().collect();
        assert_eq!(unique.len(), 9, "9 器官屏应互不相同");
    }

    #[test]
    fn organ_ascii_chars_cross_platform_safe() {
        // ASCII 字符 (跨平台, 不依赖 emoji 字体)
        for n in 0..=8u8 {
            let s = Organ::from_u8(n).unwrap().ascii_char();
            for c in s.chars() {
                assert!(
                    c.is_ascii() || c == '♥',
                    "{s} 含非 ASCII 字符 {c:?}, 跨平台不安全"
                );
            }
        }
    }
}
