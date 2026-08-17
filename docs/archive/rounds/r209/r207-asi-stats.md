# R207 ASI 高级统计 utilities (R200 调研推荐)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R207
> **日期**: 2026-08-13
> **来源**: R200 调研提到 statrs (Rust 统计库)
> **状态**: 实施完成, 14/14 单测全过 (累计 99/99)

---

## 0. 背景

apeireth-asi 现有 10 个子模块 (calibration/dim_enhance/drift/history/llm_judge/measurement/render/scheduler/tokenizer). 缺通用 stats utilities.

R200 调研提到 statrs. R207 短期方案: std + auto-vectorization, 0 新依赖.

---

## 1. 设计

### 1.1 描述统计

`
ust
pub fn mean(values: &[f64]) -> f64;
pub fn variance_pop(values: &[f64]) -> f64;       // 除以 N
pub fn variance_sample(values: &[f64]) -> f64;    // 除以 N-1, Bessel 校正
pub fn stddev_pop(values: &[f64]) -> f64;
pub fn stddev_sample(values: &[f64]) -> f64;
pub fn median(values: &mut [f64]) -> f64;          // 排序
pub fn percentile(values: &mut [f64], p: f64) -> f64;  // R type 7
`

### 1.2 标准化

`
ust
pub fn z_score(values: &[f64]) -> Vec<f64>;       // mean=0, std=1
pub fn min_max_scale(values: &[f64]) -> Vec<f64>; // [0, 1]
`

### 1.3 Welford streaming

`
ust
pub struct Welford { count, mean, m2 }
impl Welford {
    pub fn new() -> Self;
    pub fn update(&mut self, value: f64);  // 数值稳定 online
    pub fn count(&self) -> u64;
    pub fn mean(&self) -> f64;
    pub fn variance(&self) -> f64;
    pub fn stddev(&self) -> f64;
}
`

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- V0.5 24 维 / V1136 9 子测度 baseline: 0 改
- 现有 10 个子模块: 0 改
- lib.rs 改 1 行: pub mod stats

---

## 3. 测试 (14/14 pass, 累计 99/99)

- t01-t02: mean basic / empty
- t03-t04: variance pop / sample
- t05: stddev pop
- t06-t07: median odd / even
- t08-t09: percentile P50 / P90
- t10: z_score
- t11: min_max_scale
- t12-t14: Welford streaming / default / matches offline

---

## 4. 中期路径 (R207+1 候选)

- statrs crate 集成 (gamma / beta / t-distribution)
- Welford 用 SIMD 加速
- 集成进 drift.rs (drift detection 强化)