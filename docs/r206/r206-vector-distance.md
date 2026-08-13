# R206 vector distance utilities 进 apeireth-vector (R200 调研推荐)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R206
> **日期**: 2026-08-13
> **来源**: R200 调研提到 simsimd (Rust SIMD)
> **状态**: 实施完成, 14/14 单测全过 (累计 43/43)

---

## 0. 背景

apeireth-vector 当前 5 个 src 文件, 已有 sqlite-vec 真接 + qdrant HTTP 兼容. 缺通用 distance utilities.

R200 调研提到 simsimd (Rust SIMD). R206 短期方案: std + auto-vectorization, 0 新依赖, 中期 (R206+1) 评估 simsimd 集成.

---

## 1. 设计

### 1.1 5 个距离函数

`
ust
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32;       // sqrt(sum((a-b)^2))
pub fn euclidean_distance_sq(a: &[f32], b: &[f32]) -> f32;    // 避免 sqrt, 比较用
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;       // 1=同方向, 0=正交, -1=反
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32;          // 1 - cosine
pub fn dot_product(a: &[f32], b: &[f32]) -> f32;             // 内积
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32;      // sum(|a-b|)
pub fn l2_norm(v: &[f32]) -> f32;
pub fn normalize(v: &[f32]) -> Vec<f32>;
`

### 1.2 DistanceMetric enum

`
ust
pub enum DistanceMetric {
    Euclidean, EuclideanSquared, Cosine, DotProduct, Manhattan
}

pub fn distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32;
`

### 1.3 SIMD 友好

- 简单循环, LLVM 自动向量化
- 1000 维向量 < 100us (实测 ~5us)
- 中期 (R206+1): 评估 simsimd crate (~10x 加速)

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 现有 traits / qdrant_compat / sqlite_backend: 0 改
- lib.rs 改 1 行: pub mod distance

---

## 3. 测试 (14/14 pass, 累计 43/43)

- t01-t03: euclidean basic / zero / squared
- t04-t06: cosine identical / orthogonal / distance
- t07: dot product
- t08: manhattan
- t09-t11: l2_norm / normalize / normalize_zero
- t12-t13: metric as_str / distance dispatch
- t14: large vector performance smoke (1000 维 < 100us)

---

## 4. 中期路径 (R206+1 候选)

- simsimd crate 集成 (~10x 加速)
- HNSW 索引 (hnsw-rs)
- batch distance (SIMD 一次算 N 对)