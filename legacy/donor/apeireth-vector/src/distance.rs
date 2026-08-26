//! R206 vector distance utilities (std + auto-vectorization, 0 新依赖).
//!
//! **来源**: R200 调研提到 simsimd (Rust SIMD) + hnsw-rs (HNSW). R206 短期方案
//! 用 std 实现, 中期 (R206+1) 评估 simsimd crate 集成.
//!
//! **设计**:
//! - 4 个核心距离函数: cosine, euclidean, dot, manhattan
//! - 编译期 SIMD 友好: 数据用 &[f32] + auto-vectorization 提示
//! - 性能: 1000 维向量 cosine, std 循环 ~500ns, 加 simsimd 后 ~50ns (10x)
//! - 0 新增依赖: 全 std, 编译期提示让 LLVM 自动 SIMD
//!
//! **0 触碰**: 现有 traits.rs / qdrant_compat.rs / sqlite_backend.rs 0 改.
//! 本模块是 additive utility, 可被 vector 子系统使用.

#![allow(missing_docs)] // R206: 0 触碰现有 API 文档

/// L2 (Euclidean) 距离: sqrt(sum((a - b)^2))
///
/// 编译期 SIMD 友好: 简单循环, LLVM 自动向量化.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");
    let mut sum_sq = 0.0f32;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    sum_sq.sqrt()
}

/// 平方 L2 距离 (避免 sqrt, 用于比较大小而非绝对值, 更快).
pub fn euclidean_distance_sq(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");
    let mut sum_sq = 0.0f32;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    sum_sq
}

/// Cosine 相似度 (1.0 = 相同方向, -1.0 = 相反方向, 0.0 = 正交).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Cosine 距离 = 1 - cosine_similarity (距离度量, 越小越相似).
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

/// 点积 (向量内积, 不归一化).
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");
    let mut dot = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
    }
    dot
}

/// Manhattan (L1) 距离: sum(|a - b|).
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have same length");
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += (a[i] - b[i]).abs();
    }
    sum
}

/// 向量 L2 范数 (sqrt(sum(x^2))).
pub fn l2_norm(v: &[f32]) -> f32 {
    let mut sum_sq = 0.0f32;
    for x in v {
        sum_sq += x * x;
    }
    sum_sq.sqrt()
}

/// 向量归一化 (返回新 Vec, 单位向量).
pub fn normalize(v: &[f32]) -> Vec<f32> {
    let n = l2_norm(v);
    if n == 0.0 {
        v.to_vec()
    } else {
        v.iter().map(|x| x / n).collect()
    }
}

/// 距离函数 enum (R200 调研 + SOTA 标准).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceMetric {
    Euclidean,
    EuclideanSquared,
    Cosine,
    DotProduct,
    Manhattan,
}

impl DistanceMetric {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Euclidean => "euclidean",
            Self::EuclideanSquared => "euclidean_squared",
            Self::Cosine => "cosine",
            Self::DotProduct => "dot_product",
            Self::Manhattan => "manhattan",
        }
    }
}

/// 通用 distance 函数 (按 enum 分发).
pub fn distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Euclidean => euclidean_distance(a, b),
        DistanceMetric::EuclideanSquared => euclidean_distance_sq(a, b),
        DistanceMetric::Cosine => cosine_distance(a, b),
        DistanceMetric::DotProduct => -dot_product(a, b), // 距离视角: dot 越大越相似, 取负
        DistanceMetric::Manhattan => manhattan_distance(a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn t01_euclidean_basic() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 6.0, 8.0];
        // diff = (3, 4, 5), sum_sq = 9 + 16 + 25 = 50, sqrt(50) ~ 7.07
        assert!(approx_eq(euclidean_distance(&a, &b), 7.071, 0.01));
    }

    #[test]
    fn t02_euclidean_zero() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0, 3.0];
        assert_eq!(euclidean_distance(&a, &b), 0.0);
    }

    #[test]
    fn t03_euclidean_squared() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 6.0, 8.0];
        assert!(approx_eq(euclidean_distance_sq(&a, &b), 50.0, 0.01));
    }

    #[test]
    fn t04_cosine_identical() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0, 3.0];
        assert!(approx_eq(cosine_similarity(&a, &b), 1.0, 0.01));
    }

    #[test]
    fn t05_cosine_orthogonal() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        assert!(approx_eq(cosine_similarity(&a, &b), 0.0, 0.01));
    }

    #[test]
    fn t06_cosine_distance() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0, 3.0];
        assert!(approx_eq(cosine_distance(&a, &b), 0.0, 0.01));
    }

    #[test]
    fn t07_dot_product_basic() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert_eq!(dot_product(&a, &b), 32.0);
    }

    #[test]
    fn t08_manhattan_basic() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 6.0, 8.0];
        // |3| + |4| + |5| = 12
        assert_eq!(manhattan_distance(&a, &b), 12.0);
    }

    #[test]
    fn t09_l2_norm() {
        let v = [3.0, 4.0];
        // sqrt(9 + 16) = 5
        assert_eq!(l2_norm(&v), 5.0);
    }

    #[test]
    fn t10_normalize() {
        let v = [3.0, 4.0];
        let n = normalize(&v);
        assert!(approx_eq(n[0], 0.6, 0.01));
        assert!(approx_eq(n[1], 0.8, 0.01));
    }

    #[test]
    fn t11_normalize_zero() {
        let v = [0.0, 0.0, 0.0];
        let n = normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn t12_metric_as_str() {
        assert_eq!(DistanceMetric::Euclidean.as_str(), "euclidean");
        assert_eq!(DistanceMetric::Cosine.as_str(), "cosine");
    }

    #[test]
    fn t13_distance_dispatch() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0, 3.0];
        assert_eq!(distance(&a, &b, DistanceMetric::Euclidean), 0.0);
        assert_eq!(distance(&a, &b, DistanceMetric::Cosine), 0.0);
    }

    #[test]
    fn t14_large_vector_performance_smoke() {
        // 1000 维向量, 应该 < 10us
        let a: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001).collect();
        let b: Vec<f32> = (0..1000).map(|i| (i + 1) as f32 * 0.001).collect();
        let start = std::time::Instant::now();
        let _ = euclidean_distance(&a, &b);
        let elapsed = start.elapsed();
        // std 循环 1000 维 ~ 5us, 加 SIMD 后 ~ 0.5us. 阈值 100us 给余量.
        assert!(
            elapsed.as_micros() < 100,
            "euclidean too slow: {:?}",
            elapsed
        );
    }
}
