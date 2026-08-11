//! # apeireth-image-prompt benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Image Prompt Library 关键 API 性能:
//! - `validate_tool_call`: 8 工具白名单
//! - `TemplateRenderer::new(body).render(&vars)`: 模板渲染
//! - `DedupIndex::new(1000).insert/contains`: sha256 LRU dedup
//! - SHA256 计算 (1KB / 64KB payload, 1:1 翻译 v0.9.21 dedup)
//! - `PromptEntry` struct 序列化
//!
//! **基线** (1.0.0): target/criterion/apeireth-image-prompt/bench/

use std::collections::HashMap;

use apeireth_image_prompt::{
    DedupIndex, PromptEntry, PromptCategory, TemplateRenderer, TOOL_WHITELIST, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_image_prompt_add");
            let args = black_box(serde_json::json!({"text": "test"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_template_render(c: &mut Criterion) {
    c.bench_function("template_render", |b| {
        let renderer = TemplateRenderer::new("Photorealistic {{subject}} in {{style}} lighting");
        let mut vars = HashMap::new();
        vars.insert("subject".into(), "a red rose".into());
        vars.insert("style".into(), "soft".into());
        b.iter(|| {
            let _ = renderer.render(black_box(&vars)).unwrap();
        });
    });
}

fn bench_dedup_index_insert(c: &mut Criterion) {
    c.bench_function("dedup_index_insert", |b| {
        b.iter(|| {
            let mut idx = DedupIndex::new(1000);
            for i in 0..10 {
                let sha = format!("sha256-{:04x}", i);
                let id = format!("prompt-{i}");
                idx.insert(black_box(sha), black_box(id));
            }
        });
    });
}

fn bench_sha256_1kb(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("sha256_1kb", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(&data));
            let _ = hasher.finalize();
        });
    });
}

fn bench_prompt_entry_serialize(c: &mut Criterion) {
    c.bench_function("prompt_entry_serialize", |b| {
        let entry = PromptEntry::new("test prompt body", "Photorealistic", PromptCategory::Photorealistic);
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&entry)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_template_render,
    bench_dedup_index_insert,
    bench_sha256_1kb,
    bench_prompt_entry_serialize
);
criterion_main!(benches);
