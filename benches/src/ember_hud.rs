//! Ember HUD 渲染帧基准: 呼吸曲线 + 微光 uniforms 合成 (驱动侧)。
//!
//! 口径 (与 README「Ember HUD Render Tick」行的可测部分对应):
//! 每帧 tick = `EmberHudDriver::synthesize_uniforms` 单次: 4.0s 三次正弦呼吸
//! 曲线求值 + 认知姿态映射 + 色温→RGB 换算 + vignette 参数合成, 产出前端
//! 渲染用 uniforms。每样本 = batch 帧的平均。
//!
//! 诚实边界: 本基准测**驱动侧合成**; 浏览器/CSS/WebGL 实际绘制 (GPU 合成、
//! 合成器提交) 不在计时区间内, 前端绘制链路的端到端帧预算需浏览器侧测量。

use std::hint::black_box;

use apeireth_gateway::{EmberCognitiveStance, EmberHudDriver};

use crate::support::{measure_per_op, BenchConfig, Outcome, Target};

/// 每样本帧数 (batch)。
const BATCH: usize = 256;

/// 运行 Ember HUD 渲染帧基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let driver = EmberHudDriver::default();
    let stances = [
        EmberCognitiveStance::DeepCodingFocus,
        EmberCognitiveStance::AttentivePresence,
        EmberCognitiveStance::DreamingConsolidation,
        EmberCognitiveStance::EmpatheticCare,
    ];

    // 60 fps 帧序: 时间按 1/60 s 递进, 姿态轮换。
    let frame = std::cell::Cell::new(0u64);
    let samples_ns = measure_per_op(cfg.samples(3), cfg.samples(80), BATCH, || {
        let n = frame.get();
        frame.set(n + 1);
        let time_secs = (n as f32) / 60.0;
        let stance = stances[(n as usize) % stances.len()];
        let uniforms = driver.synthesize_uniforms(time_secs, stance);
        black_box(uniforms);
    });

    vec![Outcome::from_ns(
        "ember-hud-tick",
        "Ember HUD 渲染帧 (呼吸曲线 + 微光 uniforms 合成)",
        "EmberHudDriver::synthesize_uniforms 单帧 (呼吸曲线求值 + 姿态映射 + 色温换算 + vignette 合成)",
        "us",
        BATCH,
        cfg.samples(3),
        samples_ns,
        Some(Target {
            label: "< 0.5 ms".to_string(),
            value: 500.0,
        }),
        "仅驱动侧合成 (帧 uniforms 产出); 浏览器/CSS/GPU 实际绘制不在计时区间, 端到端渲染帧需浏览器侧测量 (如实标注边界)",
    )]
}
