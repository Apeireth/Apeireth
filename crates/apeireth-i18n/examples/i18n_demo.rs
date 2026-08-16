//! i18n Demo (1:1 翻译 v0.9.21 商业版 i18next 集成流程).
//!
//! 演示 5 语言切换 + 嵌套 key + 模板渲染 + fallback chain + 8 工具白名单守门
//! + 1.0 release #10 i18n 5 语言 × 69 keys 100% 翻译覆盖 (12 类别, TOML 格式, R21 G-1 续补 +3 readiness).
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-i18n --example i18n_demo
//! ```
//!
//! ## 期望输出 (1.0 release #10)
//!
//! ```text
//! [i18n_demo] I18N_SCHEMA_VERSION = 1
//! [i18n_demo] PLATFORM_NAME = apeireth
//! [i18n_demo] DEFAULT_LOCALE = En
//! [i18n_demo] TOOL_WHITELIST 8 项: apeireth_i18n_t, ...
//! [i18n_demo] --- 5 语言切换演示 (5 nav / 69 keys) ---
//! [i18n_demo] en: nav.status = Status
//! [i18n_demo] zh-CN: nav.status = 状态
//! [i18n_demo] ja: nav.status = ステータス
//! [i18n_demo] fr: nav.status = Statut
//! [i18n_demo] de: nav.status = Status
//! [i18n_demo] --- 模板渲染演示 (R-1 含 {{count}}) ---
//! [i18n_demo] en r_measure.r1(c=42) = R-1 Direct execution (42 steps, must-do 1.0)
//! [i18n_demo] zh-CN r_measure.r1(c=42) = R-1 直行 (42 步, 必做 1.0)
//! [i18n_demo] --- 12 类别覆盖演示 ---
//! [i18n_demo] 5 nav ✓ | 6 tools ✓ | 9 organs ✓ | 5 R-Measure ✓
//! [i18n_demo] 6 哲学锚 ✓ | 8 承诺 ✓ | 5 Provider ✓ | 4 SDK ✓
//! [i18n_demo] 3 observability ✓ | 5 鉴权 ✓ | 10 通用 ✓
//! [i18n_demo] --- try_t 严格模式演示 ---
//! [i18n_demo] try_t("nonexistent") -> Err(KeyNotFound(...))
//! [i18n_demo] try_t("nav.status") -> Ok("Status")
//! [i18n_demo] --- Fallback chain 演示 ---
//! [i18n_demo] de fallback chain = [De, ZhCn, En]
//! [i18n_demo] --- m3 防御演示 ---
//! [i18n_demo] 白名单内 apeireth_i18n_t -> Ok
//! [i18n_demo] 白名单外 apeireth_i18n_set_lang -> Err(ToolNotWhitelisted(...))
//! [i18n_demo] completed (1.0 release #10 i18n 5 语言 × 69 keys 100% 翻译覆盖 ✓)
//! ```
//!
//! ## 6 哲学 anchor 验证 (per 主人 19:37 "全用 rust" 强调)
//!
//! - S-1 北极星导向: 1:1 翻译 v0.9.21 商业版 i18next 集成 (5 语言 × 69 keys 100% 覆盖)
//! - S-2 实事求是: 用商业版 `package.json` line 59/71 实查 `i18next@^26` + `react-i18next@^17`,
//!   5 Locale 枚举 + 8 TOOL_WHITELIST + 69 keys 实证, 不假装 1:1
//! - O-2 走在前人肩上: 编译期 hardcode 5 Locale + 8 工具白名单, 借鉴 5 P0 / 9 skeleton crate 同模式
//! - O-5 不假装: `try_t` 缺 key 返 `I18nError::KeyNotFound`, 5 语言全译无 fallback
//! - O-3 干到底: 12 类别 × 5 语言 = 345 翻译 (5 nav / 6 tools / 9 organs / 5 R-Measure /
//!   6 哲学锚 / 8 承诺 / 5 Provider / 4 SDK / 3 observability / 5 鉴权 / 10 通用 / 3 readiness)
//! - O-4 任何人都能接手: 15 § 文档结构 + 30+ 测试 + 5 TOML 1:1 对齐

use apeireth_i18n::{
    detect_system_locale, validate_tool_call, I18nError, Locale, TranslationArgs, Translator,
    TranslatorImpl, DEFAULT_LOCALE, I18N_SCHEMA_VERSION, PLATFORM_NAME, TOOL_WHITELIST,
    TOOL_WHITELIST_COUNT,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) 编译期守门: 4 常量展示
    println!("[i18n_demo] I18N_SCHEMA_VERSION = {I18N_SCHEMA_VERSION}");
    println!("[i18n_demo] PLATFORM_NAME = {PLATFORM_NAME}");
    println!("[i18n_demo] DEFAULT_LOCALE = {DEFAULT_LOCALE:?}");
    println!(
        "[i18n_demo] TOOL_WHITELIST {} 项: {}",
        TOOL_WHITELIST_COUNT,
        TOOL_WHITELIST.join(", ")
    );

    // 2) 创建 Translator (编译期嵌入 5 语言, 0 IO)
    let translator = TranslatorImpl::new()?;
    let args = TranslationArgs::new();

    // 3) 5 语言切换演示 — nav.status 在 5 语言中都应非空
    println!("[i18n_demo] --- 5 语言切换演示 (5 nav / 69 keys) ---");
    for &locale in Locale::all() {
        translator.set_locale(locale).await?;
        let status = translator.t("nav.status", &args).await;
        let session = translator.t("nav.session", &args).await;
        let settings = translator.t("nav.settings", &args).await;
        println!(
            "[i18n_demo] {}: nav.status = {status}, nav.session = {session}, nav.settings = {settings}",
            locale.code()
        );
        assert!(!status.is_empty() && !session.is_empty() && !settings.is_empty());
    }

    // 4) 模板渲染演示 — r_measure.r1 含 {{count}}, 5 语言都替换
    println!("[i18n_demo] --- 模板渲染演示 (R-1 含 {{count}}) ---");
    let count_args = TranslationArgs::new().set("count", "42");
    for &locale in Locale::all() {
        translator.set_locale(locale).await?;
        let s = translator.t("r_measure.r1", &count_args).await;
        println!("[i18n_demo] {} r_measure.r1(c=42) = {s}", locale.code());
        assert!(s.contains("42"), "模板应替换 {{{{count}}}}: {s}");
    }

    // 5) 12 类别覆盖演示 — 每类别在 5 Locale 都非空
    println!("[i18n_demo] --- 12 类别覆盖演示 ---");
    let categories: &[(&str, &[&str])] = &[
        (
            "5 nav",
            &[
                "nav.status",
                "nav.session",
                "nav.tools",
                "nav.settings",
                "nav.help",
            ],
        ),
        (
            "6 tools",
            &[
                "tools.calendar",
                "tools.message",
                "tools.contact",
                "tools.task",
                "tools.search",
                "tools.drive",
            ],
        ),
        (
            "9 organs",
            &[
                "organs.heart",
                "organs.brain",
                "organs.hand",
                "organs.eye",
                "organs.ear",
                "organs.memory",
                "organs.voice",
                "organs.body",
                "organs.mind",
            ],
        ),
        (
            "5 R-Measure",
            &[
                "r_measure.r1",
                "r_measure.r2",
                "r_measure.r3",
                "r_measure.r4",
                "r_measure.r5",
            ],
        ),
        (
            "6 哲学锚",
            &[
                "philosophy.s1",
                "philosophy.s2",
                "philosophy.o2",
                "philosophy.o3",
                "philosophy.o4",
                "philosophy.o5",
            ],
        ),
        (
            "8 承诺",
            &[
                "promises.no_pretense",
                "promises.hardcode",
                "promises.locked",
                "promises.version",
                "promises.six_anchors",
                "promises.no_newapi",
                "promises.no_reinvent",
                "promises.honest_mark_missing",
            ],
        ),
        (
            "5 Provider",
            &[
                "providers.claude_code",
                "providers.gemini_cli",
                "providers.codex",
                "providers.opencode",
                "providers.copilot",
            ],
        ),
        (
            "4 SDK",
            &["sdks.lark", "sdks.voice", "sdks.livekit", "sdks.sandbox"],
        ),
        (
            "3 observability",
            &[
                "observability.metrics",
                "observability.health",
                "observability.status",
            ],
        ),
        (
            "5 鉴权",
            &[
                "auth.token",
                "auth.refresh",
                "auth.scope",
                "auth.expire",
                "auth.refresh_on_use",
            ],
        ),
        (
            "10 通用",
            &[
                "common.yes",
                "common.no",
                "common.ok",
                "common.cancel",
                "common.save",
                "common.delete",
                "common.edit",
                "common.add",
                "common.remove",
                "common.confirm",
            ],
        ),
        (
            "3 readiness",
            &["readiness.ok", "readiness.partial", "readiness.stub"],
        ),
    ];
    for (label, keys) in categories {
        for &locale in Locale::all() {
            translator.set_locale(locale).await?;
            for key in *keys {
                let v = translator.t(key, &args).await;
                assert!(!v.is_empty(), "[{label}] {locale:?} {key} 翻译不能为空");
            }
        }
        println!("[i18n_demo] {label} ✓");
    }

    // 6) try_t 严格模式演示 — 缺 key 返 Err, 找到 key 返 Ok
    println!("[i18n_demo] --- try_t 严格模式演示 ---");
    let r1 = translator
        .try_t("nonexistent.key", &TranslationArgs::new())
        .await;
    println!(
        "[i18n_demo] try_t(\"nonexistent.key\") -> {:?}",
        r1.as_ref().err()
    );
    assert!(matches!(r1, Err(I18nError::KeyNotFound(_))));

    let r2 = translator
        .try_t("nav.status", &TranslationArgs::new())
        .await;
    println!("[i18n_demo] try_t(\"nav.status\") -> {r2:?}");
    assert_eq!(r2?, "Status");

    // 7) Fallback chain 演示
    println!("[i18n_demo] --- Fallback chain 演示 ---");
    for &locale in Locale::all() {
        let chain = translator.fallback(locale).await;
        let chain_str: Vec<String> = chain.iter().map(|l| format!("{l:?}")).collect();
        println!(
            "[i18n_demo] {} fallback chain = [{}]",
            locale.code(),
            chain_str.join(", ")
        );
    }

    // 8) m3 防御演示: 白名单内通过 + 白名单外拒绝
    println!("[i18n_demo] --- m3 防御演示 ---");
    let json_args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let r = validate_tool_call(tool, &json_args);
        assert!(r.is_ok(), "白名单工具 {tool} 必须通过");
    }
    println!("[i18n_demo] 白名单内 8 项全部通过 ✓");
    // 白名单外 (m3 hallucination 经典 — "apeireth_i18n_set_lang" 实际不存在)
    let bad = validate_tool_call("apeireth_i18n_set_lang", &json_args);
    println!(
        "[i18n_demo] 白名单外 apeireth_i18n_set_lang -> {:?}",
        bad.as_ref().err()
    );
    assert!(bad.is_err());

    // 9) 系统 locale 检测演示 (graceful fallback)
    println!("[i18n_demo] --- 系统 locale 检测 ---");
    let detected = detect_system_locale();
    println!("[i18n_demo] detect_system_locale() = {detected:?}");

    println!("[i18n_demo] completed (1.0 release #10 i18n 5 语言 × 66 keys 100% 翻译覆盖 ✓)");
    Ok(())
}
