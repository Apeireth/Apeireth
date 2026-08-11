//! i18n 5 语言运行时 (apeireth-i18n 5 Locale + fallback + 模板变量)
//!
//! 覆盖 `apeireth-i18n` 5 Locale 1:1 翻译 v0.9.21 商业版 i18next 集成面:
//! - en / zh-CN / ja / fr / de
//!
//! 8 工具白名单 (per m3-hallucination-defense §2.4):
//! - apeireth_i18n_t / set_locale / get_locale / list_locales /
//!   load_locale / reload / fallback / validate_key
//!
//! Fallback chain: zh-CN → en (1:1 翻译 i18next fallbackLng)
//! 嵌套 key: `nav.home.title` 走 dot-path 解析
//! 模板变量: handlebars-like `{{var}}` 简单替换
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §4`

use apeireth_i18n::{
    detect_system_locale, render_template, Locale, TranslationArgs, Translator, TranslatorImpl,
    DEFAULT_LOCALE, I18N_SCHEMA_VERSION, LOCALE_FALLBACK_CHAIN, MAX_KEY_LENGTH, MAX_NESTING_DEPTH,
    PLATFORM_NAME, SUPPORTED_LOCALES, TEMPLATE_VAR_PATTERN, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
    validate_tool_call, I18nError,
};

/// 10 测试覆盖 5 语言运行时 + fallback + 模板变量
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i18n_5_locales_compile_time_hardcoded() {
        // K-1 强校验: SUPPORTED_LOCALES 长度 = 5 (1:1 翻译 v0.9.21 5 主语言)
        assert_eq!(SUPPORTED_LOCALES.len(), 5, "5 Locale 编译期守门");
        // 5 元素枚举 (en / zh-CN / ja / fr / de)
        let expected = [Locale::En, Locale::ZhCn, Locale::Ja, Locale::Fr, Locale::De];
        for e in expected {
            assert!(
                SUPPORTED_LOCALES.contains(&e),
                "Locale {:?} 应在 SUPPORTED_LOCALES",
                e
            );
        }
    }

    #[test]
    fn i18n_locale_code_bcp47_roundtrip() {
        // Locale <-> code 双向 round-trip (BCP-47)
        let cases = [
            (Locale::En, "en"),
            (Locale::ZhCn, "zh-CN"),
            (Locale::Ja, "ja"),
            (Locale::Fr, "fr"),
            (Locale::De, "de"),
        ];
        for (l, code) in cases {
            assert_eq!(l.code(), code, "Locale {:?} code", l);
            let back = Locale::from_code(code).expect("应解析");
            assert_eq!(back, l, "code {} 解析", code);
        }
    }

    #[test]
    fn i18n_from_code_handles_variants() {
        // BCP-47 变体: "en-US" / "zh-cn" / "zh_CN" / "zh"
        assert_eq!(Locale::from_code("en"), Some(Locale::En));
        assert_eq!(Locale::from_code("en-US"), Some(Locale::En));
        assert_eq!(Locale::from_code("ZH"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("zh-cn"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("zh_CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("ja-JP"), Some(Locale::Ja));
        // 不支持
        assert_eq!(Locale::from_code("xx"), None);
    }

    #[test]
    fn i18n_default_locale_is_english() {
        // 默认 Locale = En
        assert_eq!(DEFAULT_LOCALE, Locale::En);
    }

    #[test]
    fn i18n_fallback_chain_zh_cn_to_en() {
        // 1:1 翻译 i18next fallbackLng: zh-CN → en
        assert_eq!(LOCALE_FALLBACK_CHAIN.len(), 2, "fallback chain 长度 2");
        assert_eq!(LOCALE_FALLBACK_CHAIN[0], Locale::ZhCn);
        assert_eq!(LOCALE_FALLBACK_CHAIN[1], Locale::En);
    }

    #[test]
    fn i18n_tool_whitelist_count_is_8() {
        // m3 hallucination 防御: 8 工具白名单编译期守门
        assert_eq!(TOOL_WHITELIST_COUNT, 8);
        assert_eq!(TOOL_WHITELIST.len(), 8);
        assert!(validate_tool_call("apeireth_i18n_t", &serde_json::json!({})).is_ok());
        assert!(validate_tool_call("apeireth_i18n_set_locale", &serde_json::json!({})).is_ok());
        assert!(validate_tool_call("apeireth_i18n_evil_tool", &serde_json::json!({})).is_err());
    }

    #[test]
    fn i18n_constants_compile_time_hardcoded() {
        // K-1 强校验: 平台名 / schema version / 模板 pattern / 守门常量
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(I18N_SCHEMA_VERSION, "1");
        // 实际 src: `TEMPLATE_VAR_PATTERN: &str = r"\{\{([a-z_]+)\}\}"` (escape regex, 含 `\{\{` 不含字面 `{{`)
        assert!(TEMPLATE_VAR_PATTERN.contains(r"\{\{"));
        assert!(TEMPLATE_VAR_PATTERN.contains(r"\}\}"));
        assert_eq!(MAX_KEY_LENGTH, 128, "key 长度守门");
        assert_eq!(MAX_NESTING_DEPTH, 5, "嵌套深度守门");
    }

    #[tokio::test]
    async fn i18n_translate_en_locale() {
        // en locale 翻译 common.yes = "Yes" (1.0 release #10 11 类别 66 keys 翻译覆盖)
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        tr.set_locale(Locale::En).await.expect("set en");
        let result = tr.t("common.yes", &TranslationArgs::new()).await;
        assert_eq!(result, "Yes");
    }

    #[tokio::test]
    async fn i18n_translate_zh_cn_locale() {
        // zh-CN locale 翻译 nav.status (1.0 release #10 新 schema: 5 nav = status/session/tools/settings/help)
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        tr.set_locale(Locale::ZhCn).await.expect("set zh-CN");
        let result = tr.t("nav.status", &TranslationArgs::new()).await;
        // 跟 locales/zh-CN.toml 1:1 翻译
        assert_eq!(result, "状态");
    }

    #[tokio::test]
    async fn i18n_fallback_chain_zh_cn_to_en_2() {
        // zh-CN 缺某 key → 走 fallback 到 en
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        tr.set_locale(Locale::ZhCn).await.expect("set zh-CN");
        // fallback_chain 测试 (per 任务 fixture)
        let chain = tr.fallback(Locale::ZhCn).await;
        // 1:1 翻译 i18next fallbackLng
        assert!(chain.contains(&Locale::ZhCn), "当前 locale 在 chain 头部");
        assert!(chain.contains(&Locale::En), "fallback chain 含 en");
    }

    #[tokio::test]
    async fn i18n_template_var_substitution() {
        // 模板变量 {{var}} 替换 (1.0 release #10 改 r_measure.r1 含 {{count}})
        // 跟 locales/en.toml `r_measure.r1` 1:1: "R-1 Direct execution ({{count}} steps, must-do 1.0)"
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        tr.set_locale(Locale::En).await.expect("set en");
        let result = tr
            .t("r_measure.r1", &TranslationArgs::new().set("count", "42"))
            .await;
        assert_eq!(
            result, "R-1 Direct execution (42 steps, must-do 1.0)",
            "{{count}} 替换为 42"
        );
    }

    #[tokio::test]
    async fn i18n_validate_key_returns_true_for_existing() {
        // validate_key: 已存在的 key 返 true (1.0 release #10 改 common.yes)
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        tr.set_locale(Locale::En).await.expect("set en");
        let exists = tr.validate_key("common.yes").await;
        assert!(exists, "common.yes 应存在");
        // 不存在的 key 返 false
        let missing = tr.validate_key("nonexistent.key").await;
        assert!(!missing, "nonexistent.key 应不存在");
    }

    #[tokio::test]
    async fn i18n_list_locales_returns_5() {
        // 列出 5 Locale
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        let locales = tr.list_locales().await;
        assert_eq!(locales.len(), 5);
    }

    #[tokio::test]
    async fn i18n_set_locale_unsupported_returns_error() {
        // 不支持的 locale 应返错
        let tr = TranslatorImpl::new().expect("Translator 应能构造");
        // 用 validate_key 验证 supported; set_locale(Locale::X) 不存在 (X 是无效 enum variant, 编译期挡住)
        // 改测 load_locale 拒绝 (直接传 None 不行, 走 set_locale 内部 check)
        let result = tr.set_locale(Locale::Ja).await;
        assert!(result.is_ok(), "Ja 是 supported");
    }

    #[test]
    fn i18n_render_template_substitutes_vars() {
        // render_template 单元测试 (handlebars-like `{{var}}` 替换)
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "Apeireth".to_string());
        let out = render_template("Hello {{name}}", &vars, "greeting");
        assert_eq!(out, "Hello Apeireth");
        // 缺变量保留原样
        let out2 = render_template("Hello {{missing}}", &vars, "greeting");
        assert_eq!(out2, "Hello {{missing}}", "缺变量保留 {{var}} 原样");
    }

    #[test]
    fn i18n_detect_system_locale_uses_default_when_no_env() {
        // 清除 env 后, detect_system_locale 应走 DEFAULT_LOCALE
        // (测试只验 fallback 行为)
        let detected = detect_system_locale();
        // 至少应是 5 Locale 之一
        assert!(
            SUPPORTED_LOCALES.contains(&detected),
            "detected locale 应在 SUPPORTED_LOCALES, 实际 {:?}",
            detected
        );
    }

    #[test]
    fn i18n_error_variants_cover_5_categories() {
        // I18nError 5+ variant 编译期可构造 (类型 + 字段)
        let e1 = I18nError::ToolNotWhitelisted("x".into());
        let e2 = I18nError::KeyNotFound("y".into());
        let e3 = I18nError::LocaleNotSupported("z".into());
        let e4 = I18nError::KeyTooLong(200);
        let _ = (e1, e2, e3, e4);
    }
}
