//! # apeireth-i18n benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 i18n 关键 API 性能:
//! - `validate_tool_call`: 8 工具白名单
//! - `Locale::code()` + `from_code("en")`: 5 语言枚举 ↔ BCP-47 code 转换
//! - `render_template("Hello {{name}}", &vars)`: 模板渲染 (同步, 跟 image-prompt 一致)
//! - `Locale::all()`: 返回 SUPPORTED_LOCALES 引用
//! - `TranslatorImpl::new()`: 5 语言编译期嵌入初始化
//!
//! **基线** (1.0.0): target/criterion/apeireth-i18n/bench/

use std::collections::HashMap;

use apeireth_i18n::{
    Locale, SUPPORTED_LOCALES, TOOL_WHITELIST, TranslatorImpl, render_template, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_i18n_t");
            let args = black_box(serde_json::json!({"key": "app.name"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_locale_code(c: &mut Criterion) {
    c.bench_function("locale_code", |b| {
        b.iter(|| {
            for l in SUPPORTED_LOCALES {
                let _ = black_box(l.code());
            }
        });
    });
}

fn bench_locale_from_code(c: &mut Criterion) {
    c.bench_function("locale_from_code", |b| {
        b.iter(|| {
            for code in ["en", "zh-CN", "ja", "fr", "de", "en-US", "zh_CN", "ja-JP"] {
                let _ = Locale::from_code(black_box(code));
            }
        });
    });
}

fn bench_render_template(c: &mut Criterion) {
    c.bench_function("render_template", |b| {
        let mut vars = HashMap::new();
        vars.insert("name".into(), "World".into());
        vars.insert("version".into(), "1.0.0".into());
        b.iter(|| {
            let _ = render_template(
                black_box("Hello {{name}}, v{{version}}"),
                black_box(&vars),
                "greet",
            );
        });
    });
}

fn bench_translator_init(c: &mut Criterion) {
    c.bench_function("translator_init", |b| {
        b.iter(|| {
            // 编译期嵌入 5 语言 TOML + toml::from_str 反序列化
            // (1.0 release #10 i18n 改 TOML 格式, 替代原 JSON)
            let _ = TranslatorImpl::new().unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_locale_code,
    bench_locale_from_code,
    bench_render_template,
    bench_translator_init
);
criterion_main!(benches);
