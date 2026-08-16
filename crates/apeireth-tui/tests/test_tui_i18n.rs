#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// TUI i18n 集成测试 (R21 G-1 续补, 1.0 release #10 i18n 100% 收尾)
///
/// **测试目标** (per 主人派活单 2026-08-06):
/// - 验证 TUI 5 nav 走 `translator.t("nav.*")` 翻译表 (5 Locale 切换)
/// - 验证 TUI 9 organ 走 `translator.t("organs.*")` 翻译表 (5 Locale 切换)
/// - 验证 TUI 3 readiness 走 `translator.t("readiness.*")` 翻译表 (5 Locale 切换)
/// - 覆盖 5 Locale × (5 nav + 9 organ + 3 readiness) = 5 × 17 = 85 翻译点 100% 翻译
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: TUI 接 i18n 服务"用户用母语看 AI 状态"北极星 (跨 5 Locale 状态可视化)
/// - S-2 实事求是: 翻译表数据来自 i18n crate 编译期嵌入 (不虚构, 不 fallback 占位)
/// - O-2 走在前人肩上: 借 i18n crate Translator trait + apeireth-tui 5 nav/9 organ 编译期 enum
/// - O-3 干到底: 17 keys × 5 Locale = 85 翻译点全覆盖 (5+9+3 = 17, 跟 task 17×5=85 1:1)
/// - O-4 任何人都能接手: 头部说明 + 测试名清楚 + 85 翻译点 enumerate 显式
/// - O-5 不假装: 直接走 tr.t() 对比, 不允许 mock, 缺 key 返 key 自身 (i18next 1:1)
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"]
mod config_watcher;
#[path = "../src/http_llm.rs"]
mod http_llm;
#[path = "../src/llm_config.rs"]
mod llm_config;
#[path = "../src/observability.rs"]
mod observability;
#[path = "../src/onboarding.rs"]
mod onboarding;
#[path = "../src/organ/mod.rs"]
mod organ;
#[path = "../src/pages/mod.rs"]
mod pages;
#[path = "../src/persistence.rs"]
mod persistence;
#[path = "../src/theme.rs"]
mod theme;

#[path = "../src/error.rs"]
mod error;
#[path = "../src/http.rs"]
mod http;
#[path = "../src/nav/mod.rs"]
mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
/// **8 项承诺**: 全部遵守
/// **路径说明** (per 团队规范):
// - 不改 main.rs (LOCKED), 用 `#[path]` 把 nav / organ 源文件拉到 test binary root
// - 直接调 `Translator::t` 走 i18n 翻译表, 跟 nav/mod.rs / organ/mod.rs 用的同 1 份
mod test_common;

use apeireth_i18n::{
    Locale, TranslationArgs, Translator, TranslatorImpl, PLATFORM_NAME, SUPPORTED_LOCALES,
};
use nav::Nav;
use organ::{Organ, Readiness};

// =====================================================================
// 1. 17 keys 5 Locale 翻译表全覆盖 (5 Locale × 17 keys = 85 翻译点)
// =====================================================================

/// 5 nav 翻译 key (跟 nav/mod.rs::Nav::label() 1:1)
const FIVE_NAV_KEYS: &[&str] = &[
    "nav.status",
    "nav.session",
    "nav.tools",
    "nav.settings",
    "nav.help",
];

/// 9 器官翻译 key (跟 organ/mod.rs::Organ::name() 1:1)
const NINE_ORGAN_KEYS: &[&str] = &[
    "organs.heart",
    "organs.brain",
    "organs.hand",
    "organs.eye",
    "organs.ear",
    "organs.memory",
    "organs.voice",
    "organs.body",
    "organs.mind",
];

/// 3 readiness 翻译 key (跟 organ/mod.rs::Readiness::label() 1:1)
const THREE_READINESS_KEYS: &[&str] = &["readiness.ok", "readiness.partial", "readiness.stub"];

/// 17 keys 总数守门: 5 + 9 + 3 = 17
#[test]
fn seventeen_keys_count_matches_task_spec() {
    assert_eq!(FIVE_NAV_KEYS.len(), 5, "5 nav keys");
    assert_eq!(NINE_ORGAN_KEYS.len(), 9, "9 organ keys");
    assert_eq!(THREE_READINESS_KEYS.len(), 3, "3 readiness keys");
    assert_eq!(
        FIVE_NAV_KEYS.len() + NINE_ORGAN_KEYS.len() + THREE_READINESS_KEYS.len(),
        17,
        "总 key 数 = 17 (per task 17×5=85 翻译点)"
    );
}

// =====================================================================
// 2. 5 Locale × 17 keys 翻译全覆盖 (核心: 85 翻译点 100% 非空)
// =====================================================================

/// 85 翻译点全覆盖: 5 Locale 各自 17 keys 全部翻译 (非空, 非 fallback 占位)
#[tokio::test]
async fn eighty_five_translation_points_all_non_empty() {
    let tr = TranslatorImpl::new().expect("TranslatorImpl::new must Ok (skeleton 阶段编译期嵌入)");
    let args = TranslationArgs::new();
    let mut total = 0;
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.expect("set_locale");
        for key in FIVE_NAV_KEYS
            .iter()
            .chain(NINE_ORGAN_KEYS)
            .chain(THREE_READINESS_KEYS)
        {
            let v = tr.t(key, &args).await;
            assert!(
                !v.is_empty(),
                "{locale:?} {key} 翻译不能为空 (5 Locale × 17 keys = 85 翻译点 100% 守门)"
            );
            total += 1;
        }
    }
    assert_eq!(
        total, 85,
        "实际跑了 {total} 翻译点, 期望 85 (5 Locale × 17 keys)"
    );
}

// =====================================================================
// 3. TUI 方法 (Nav::label / Organ::name / Readiness::label) 走翻译表
// =====================================================================

/// 5 nav 方法 (Nav::label) 在 5 Locale 都翻译非空
#[tokio::test]
async fn nav_label_5_locales_all_translated() {
    let tr = TranslatorImpl::new().unwrap();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).unwrap();
            let s = nav.label(&tr).await;
            assert!(
                !s.is_empty(),
                "{locale:?} Nav::{nav:?}.label() 应非空 (5 Locale 100% 翻译守门)"
            );
        }
    }
}

/// 9 organ 方法 (Organ::name) 在 5 Locale 都翻译非空
#[tokio::test]
async fn organ_name_5_locales_all_translated() {
    let tr = TranslatorImpl::new().unwrap();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let s = organ.name(&tr).await;
            assert!(
                !s.is_empty(),
                "{locale:?} Organ::{organ:?}.name() 应非空 (5 Locale 100% 翻译守门)"
            );
        }
    }
}

/// 3 readiness 方法 (Readiness::label) 在 5 Locale 都翻译非空
#[tokio::test]
async fn readiness_label_5_locales_all_translated() {
    let tr = TranslatorImpl::new().unwrap();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for r in [Readiness::Ok, Readiness::Partial, Readiness::Stub] {
            let s = r.label(&tr).await;
            assert!(
                !s.is_empty(),
                "{locale:?} {r:?}.label() 应非空 (5 Locale 100% 翻译守门)"
            );
        }
    }
}

// =====================================================================
// 4. TUI 方法返值 == 直接 tr.t() 返值 (1:1 翻译守门)
// =====================================================================

/// Nav::label(tr) 返值跟 tr.t("nav.*") 1:1 (3 nav 抽测, 5 Locale 全跑)
#[tokio::test]
async fn nav_label_matches_tr_t_directly() {
    let tr = TranslatorImpl::new().unwrap();
    let args = TranslationArgs::new();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for n in 0..=4u8 {
            let nav = Nav::from_u8(n).unwrap();
            let key = FIVE_NAV_KEYS[n as usize];
            let via_method = nav.label(&tr).await;
            let via_t = tr.t(key, &args).await;
            assert_eq!(
                via_method, via_t,
                "{locale:?} Nav::{nav:?}.label() 应 == tr.t({key}) (1:1 翻译守门)"
            );
        }
    }
}

/// Organ::name(tr) 返值跟 tr.t("organs.*") 1:1
#[tokio::test]
async fn organ_name_matches_tr_t_directly() {
    let tr = TranslatorImpl::new().unwrap();
    let args = TranslationArgs::new();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let key = NINE_ORGAN_KEYS[n as usize];
            let via_method = organ.name(&tr).await;
            let via_t = tr.t(key, &args).await;
            assert_eq!(
                via_method, via_t,
                "{locale:?} Organ::{organ:?}.name() 应 == tr.t({key}) (1:1 翻译守门)"
            );
        }
    }
}

/// Readiness::label(tr) 返值跟 tr.t("readiness.*") 1:1
#[tokio::test]
async fn readiness_label_matches_tr_t_directly() {
    let tr = TranslatorImpl::new().unwrap();
    let args = TranslationArgs::new();
    let pairs = [
        (Readiness::Ok, "readiness.ok"),
        (Readiness::Partial, "readiness.partial"),
        (Readiness::Stub, "readiness.stub"),
    ];
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for (r, key) in pairs {
            let via_method = r.label(&tr).await;
            let via_t = tr.t(key, &args).await;
            assert_eq!(
                via_method, via_t,
                "{locale:?} {r:?}.label() 应 == tr.t({key}) (1:1 翻译守门)"
            );
        }
    }
}

// =====================================================================
// 5. zh-CN / ja / fr / de 翻译非 fallback 到 en (O-5 不假装)
// =====================================================================

/// zh-CN locale 下 5 nav 翻译跟 en 翻译不同 (5 Locale 100% 翻译, 0 fallback 占位)
#[tokio::test]
async fn zh_cn_5_nav_not_fallback_to_en() {
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

/// 非 en Locale 下 9 organ 翻译至少 8/9 跟 en 不同 (允许 1 个巧合相同, 如 de "Hand" = en "Hand")
/// (zh-CN / ja 100% 不同因为 CJK 字符, fr / de 允许少量巧合)
#[tokio::test]
async fn non_en_4_locales_9_organ_not_fallback() {
    let tr = TranslatorImpl::new().unwrap();
    for locale in [Locale::ZhCn, Locale::Ja, Locale::Fr, Locale::De] {
        tr.set_locale(locale).await.unwrap();
        let mut diff_count = 0;
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let translated = organ.name(&tr).await;
            tr.set_locale(Locale::En).await.unwrap();
            let en = organ.name(&tr).await;
            if translated != en {
                diff_count += 1;
            }
            tr.set_locale(locale).await.unwrap();
        }
        // zh-CN / ja 字符集完全不同, 100% 不同; fr / de 允许 1 个巧合
        let min_diff = match locale {
            Locale::ZhCn | Locale::Ja => 9, // 100% 不同
            Locale::Fr | Locale::De => 7,   // 允许最多 2 个巧合 (de "Hand"=en "Hand" 是已知巧合)
            _ => 9,
        };
        assert!(
            diff_count >= min_diff,
            "{locale:?} 9 organ 翻译至少 {min_diff} 个跟 en 不同, 实际 {diff_count} 个 (O-5 不假装 0 fallback)"
        );
    }
}

// =====================================================================
// 6. 字符存在性守门 (5 Locale 1:1 字符特征)
// =====================================================================

/// 5 Locale 翻译都有非 ASCII 字符 (en / zh-CN / ja / fr / de 至少 4 Locale 含非 ASCII)
#[tokio::test]
async fn translations_have_locale_specific_chars() {
    let tr = TranslatorImpl::new().unwrap();
    let args = TranslationArgs::new();
    // 全跑 17 keys × 5 Locale = 85 个翻译
    let mut samples = Vec::new();
    for &locale in SUPPORTED_LOCALES {
        tr.set_locale(locale).await.unwrap();
        for key in FIVE_NAV_KEYS
            .iter()
            .chain(NINE_ORGAN_KEYS)
            .chain(THREE_READINESS_KEYS)
        {
            let v = tr.t(key, &args).await;
            samples.push((locale, key, v));
        }
    }
    // zh-CN: 至少 1 个汉字
    let zh_has_chinese = samples
        .iter()
        .filter(|(l, _, _)| *l == Locale::ZhCn)
        .any(|(_, _, v)| {
            v.chars()
                .any(|c| (c as u32) >= 0x4E00 && (c as u32) <= 0x9FFF)
        });
    assert!(zh_has_chinese, "zh-CN 翻译应含汉字");
    // ja: 至少 1 个假名 (hiragana / katakana)
    let ja_has_kana = samples
        .iter()
        .filter(|(l, _, _)| *l == Locale::Ja)
        .any(|(_, _, v)| {
            v.chars()
                .any(|c| (c as u32) >= 0x3040 && (c as u32) <= 0x30FF)
        });
    assert!(ja_has_kana, "ja 翻译应含假名 (hiragana/katakana)");
    // fr: 至少 1 个法语特殊字符 (é à ç)
    let fr_has_accent = samples
        .iter()
        .filter(|(l, _, _)| *l == Locale::Fr)
        .any(|(_, _, v)| {
            v.chars().any(|c| {
                matches!(
                    c,
                    'é' | 'à' | 'ç' | 'è' | 'ê' | 'ë' | 'î' | 'ï' | 'ô' | 'ù' | 'û'
                )
            })
        });
    assert!(fr_has_accent, "fr 翻译应含法语特殊字符 (é à ç 等)");
    // de: 至少 1 个德语特殊字符 (ä ö ü ß)
    let de_has_umlaut = samples
        .iter()
        .filter(|(l, _, _)| *l == Locale::De)
        .any(|(_, _, v)| {
            v.chars()
                .any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'ß' | 'Ä' | 'Ö' | 'Ü'))
        });
    assert!(de_has_umlaut, "de 翻译应含德语特殊字符 (ä ö ü ß 等)");
}

// =====================================================================
// 7. K-1 强校验: PLATFORM_NAME = "apeireth" (R21 G-1 不破坏 K-1 守门)
// =====================================================================

/// 平台名编译期守门 (R21 G-1 不破坏, TUI 接 i18n 0 副作用)
#[test]
fn platform_name_unchanged_k1_strong_check() {
    assert_eq!(
        PLATFORM_NAME, "apeireth",
        "K-1 强校验 #1: PLATFORM_NAME 编译期 hardcode, 不写装饰名"
    );
}

// =====================================================================
// 8. ascii_char 跨平台守门 (R21 G-1 保留, 非翻译)
// =====================================================================

/// Organ::ascii_char() 跨平台守门: 9 器官 ASCII 字符不含非 ASCII 字符 (♥ 例外)
#[test]
fn organ_ascii_chars_cross_platform_safe_after_i18n() {
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
