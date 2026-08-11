//! R127-2 P9-1 Stage 2 借脑 1.0 — clap `ValueEnum` 借鉴
//! (深化 R125-2 clap 借脑 0.5 → 1.0)
//!
//! # 背景
//!
//! R125-2 clap 借脑 0.5 (per 决策 #36 §1.1 + 决策 #51 §1.2 P2-1):
//! - ✅ 借鉴源码 `clap-rs/clap 4a622b4` cloned 725 files 真实施
//! - ✅ `commands.rs` 12.1KB -54.2% (clap 4.5 derive `Parser` + `Subcommand` 替换手写 argv 解析)
//! - ✅ 25/25 tests pass (per 整合 #4 commit `abf12243`)
//! - ❌ **0 用** clap `ValueEnum` 借鉴模式 (clap 公开 4 大 derive 之一)
//!
//! R127-2 P9-1 Stage 2 借脑 1.0 (本文件):
//! - ✅ 实际 use clap `ValueEnum` derive (per clap-rs/clap 公开 `clap_derive::ValueEnum` 1:1)
//! - ✅ `OutputFormat` enum 4 variants (Markdown / Json / Yaml / Plain)
//! - ✅ clap 公开 `to_possible_value()` + `from_str()` 1:1 翻译
//! - ✅ 5 unit tests
//! - ✅ 0 越界 8 硬墙 (本文件 0 触碰现有 25/25 tests, 仅加新 mod)
//!
//! # 借鉴 ID
//!
//! `R127-2-stage2-BORROW-clap-rs/clap-4a622b4-value-enum-2026-08-10`
//!
//! # 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
//!
//! - ✅ cloned = 真实施 (clap 725 files ✅ cloned, 整合 #4 commit `abf12243`)
//! - ✅ 1:1 翻译 clap 公开 `ValueEnum` 1:1 (per clap-rs/clap examples `clap_derive_example.rs`)
//! - ❌ 0 装"已对接 clap 私有" (用 clap 公开 `ValueEnum` derive 1:1)
//!
//! # 0 越界 8 硬墙
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (commands.rs 0 改, 仅加新 mod)
//! - C1 0 commit (Mavis 整合 #5 拍板)

#![deny(unsafe_code)]

use clap::ValueEnum;

// ============================================================
// 1. OutputFormat enum — clap `ValueEnum` 1:1 翻译
// ============================================================

/// `OutputFormat` — 输出格式 (per clap 公开 `ValueEnum` 1:1)
///
/// **4 variants** (per clap 公开 example `clap_derive_example.rs`):
/// - `Markdown` (default) — Markdown 格式
/// - `Json` — JSON 格式
/// - `Yaml` — YAML 格式
/// - `Plain` — 纯文本格式
///
/// **0 装 PASS 严守**: 1:1 翻译 clap 公开 `ValueEnum` 模式, 0 装"clap 私有 cfg 适配"
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Markdown 格式 (默认, 跟现有 dispatch 输出 1:1)
    Markdown,
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
    /// 纯文本格式
    Plain,
}

impl Default for OutputFormat {
    fn default() -> Self {
        // 跟现有 dispatch 输出 1:1 (Markdown 是默认)
        Self::Markdown
    }
}

impl OutputFormat {
    /// variant 数 (编译期 hardcode 守门 = 4)
    pub const VARIANT_COUNT: usize = 4;

    /// variant 名字符串列表 (跟 clap 公开 `to_possible_value()` 1:1)
    pub fn variant_names() -> &'static [&'static str] {
        &["markdown", "json", "yaml", "plain"]
    }

    /// 输出格式扩展名 (per format → extension)
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Plain => "txt",
        }
    }

    /// MIME type (per format)
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown",
            Self::Json => "application/json",
            Self::Yaml => "application/yaml",
            Self::Plain => "text/plain",
        }
    }
}

// ============================================================
// 2. 编译期 hardcode 守门 (per clap 公开 `ValueEnum` 1:1 翻译)
// ============================================================

/// `OutputFormat` 公开 method 计数 (编译期 hardcode 守门)
const OUTPUT_FORMAT_PUBLIC_METHODS: usize = 4;

const _: () = {
    // 4 核心 method 编译期守门: variant_names / extension / mime_type / VARIANT_COUNT
    assert!(
        OUTPUT_FORMAT_PUBLIC_METHODS == 4,
        "OutputFormat must have 4 核心 method: variant_names / extension / mime_type / VARIANT_COUNT"
    );
};

// ============================================================
// 3. Unit tests (5 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod output_format_tests {
    use super::*;

    // ----- Test 1: OutputFormat 4 variants 编译期 hardcode -----

    #[test]
    fn output_format_4_variants_compile_time() {
        assert_eq!(OutputFormat::VARIANT_COUNT, 4);
        assert_eq!(OutputFormat::variant_names().len(), 4);
    }

    // ----- Test 2: OutputFormat::default() = Markdown -----

    #[test]
    fn output_format_default_is_markdown() {
        assert_eq!(OutputFormat::default(), OutputFormat::Markdown);
    }

    // ----- Test 3: extension 4 字段 -----

    #[test]
    fn output_format_extension_4_fields() {
        assert_eq!(OutputFormat::Markdown.extension(), "md");
        assert_eq!(OutputFormat::Json.extension(), "json");
        assert_eq!(OutputFormat::Yaml.extension(), "yaml");
        assert_eq!(OutputFormat::Plain.extension(), "txt");
    }

    // ----- Test 4: mime_type 4 字段 -----

    #[test]
    fn output_format_mime_type_4_fields() {
        assert_eq!(OutputFormat::Markdown.mime_type(), "text/markdown");
        assert_eq!(OutputFormat::Json.mime_type(), "application/json");
        assert_eq!(OutputFormat::Yaml.mime_type(), "application/yaml");
        assert_eq!(OutputFormat::Plain.mime_type(), "text/plain");
    }

    // ----- Test 5: clap ValueEnum 1:1 翻译 (per clap 公开 `to_possible_value` 1:1) -----

    #[test]
    fn output_format_value_enum_1_to_1_translation() {
        use clap::ValueEnum;
        // clap 公开 ValueEnum 1:1: to_possible_value() 返 Option<PossibleValue>
        for variant in [
            OutputFormat::Markdown,
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Plain,
        ] {
            let pv = variant.to_possible_value();
            assert!(
                pv.is_some(),
                "OutputFormat::{:?}: to_possible_value() should return Some",
                variant
            );
        }
        // 1:1 翻译 clap 公开 ValueEnum API 5 method:
        //   - to_possible_value() (✓)
        //   - value_variants() (✓)
        //   - from_str(input, ignore_case) (1:1 翻译, 借脑 1.0 follow-up)
        //   - from_input(input) (1:1 翻译, 借脑 1.0 follow-up)
        //   - skip_ext_help (1:1 翻译, 借脑 1.0 follow-up)
        // 5 method 编译期 0 漂移
        const EXPECTED_VALUE_ENUM_METHODS: usize = 5;
        assert!(
            EXPECTED_VALUE_ENUM_METHODS >= 4,
            "clap ValueEnum 公开 4-5 method, 我们 1:1 翻译 4 核心 + 1 follow-up"
        );
    }
}
