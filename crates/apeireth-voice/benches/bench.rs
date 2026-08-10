//! # apeireth-voice benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Voice SDK 关键 API 性能:
//! - `validate_tool_call`: 9 工具白名单
//! - `VoiceConfig::default()`: 默认配置构造
//! - `WakeWordType` enum 序列化 (5 wake words)
//! - `AudioFrame::new(samples)`: 构造音频帧
//! - `is_stub_mode()`: 编译期常量
//!
//! **基线** (1.0.0): target/criterion/apeireth-voice/bench/

use apeireth_voice::{
    AudioFrame, VoiceConfig, WakeWordType, TOOL_WHITELIST, is_stub_mode, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_voice_wake_word_detect");
            let args = black_box(serde_json::json!({"audio": "base64data"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_voice_config_default(c: &mut Criterion) {
    c.bench_function("voice_config_default", |b| {
        b.iter(|| {
            let _ = VoiceConfig::default();
        });
    });
}

fn bench_wake_word_type_serialize(c: &mut Criterion) {
    c.bench_function("wake_word_type_serialize", |b| {
        let w = WakeWordType::Apeireth;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&w)).unwrap();
        });
    });
}

fn bench_audio_frame_construct(c: &mut Criterion) {
    c.bench_function("audio_frame_construct", |b| {
        // 512 样本 (per VOICE_FRAME_LENGTH 1:1 翻译 v0.9.21 pvrecorder frame size)
        let samples = vec![0i16; 512];
        b.iter(|| {
            let _ = AudioFrame::new(black_box(samples.clone()));
        });
    });
}

fn bench_is_stub_mode(c: &mut Criterion) {
    c.bench_function("is_stub_mode", |b| {
        b.iter(|| {
            let _ = black_box(is_stub_mode());
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_voice_config_default,
    bench_wake_word_type_serialize,
    bench_audio_frame_construct,
    bench_is_stub_mode
);
criterion_main!(benches);
