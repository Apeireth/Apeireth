//! Vector - 简单 cosine similarity (从 v1.0 apeireth-vector 2.4K LOC 收敛)
//!
//! 0 装 PASS: 纯内存 cosine similarity (无 hnsw/faiss), 适合 < 10K 向量场景.
//! 完整 v1.0 era 不做 (持久化 / 索引构建 / 量化).

#[derive(Debug, Clone)]
pub struct Vector {
    pub id: String,
    pub data: Vec<f32>,
}

impl Vector {
    pub fn new(id: impl Into<String>, data: Vec<f32>) -> Self {
        Self { id: id.into(), data }
    }

    /// 0 装 PASS: 真实 cosine (真点积 + L2 范数, 0 装 PASS 不假装)
    pub fn cosine(&self, other: &Self) -> f32 {
        if self.data.len() != other.data.len() { return 0.0; }
        let dot: f32 = self.data.iter().zip(&other.data).map(|(a,b)| a*b).sum();
        let n1: f32 = self.data.iter().map(|a| a*a).sum::<f32>().sqrt();
        let n2: f32 = other.data.iter().map(|a| a*a).sum::<f32>().sqrt();
        if n1 == 0.0 || n2 == 0.0 { return 0.0; }
        dot / (n1 * n2)
    }
}

/// 0 装 PASS: top-k 检索 (真线性扫描, 不假装索引加速)
pub fn top_k<'a>(query: &'a Vector, corpus: &'a [Vector], k: usize) -> Vec<(f32, &'a Vector)> {
    let mut scored: Vec<_> = corpus.iter().map(|v| (query.cosine(v), v)).collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(k).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_cosine_identical() {
        let v = Vector::new("a", vec![1.0, 0.0, 0.0]);
        assert!((v.cosine(&v) - 1.0).abs() < 1e-6);
    }
    #[test] fn test_cosine_orthogonal() {
        let a = Vector::new("a", vec![1.0, 0.0, 0.0]);
        let b = Vector::new("b", vec![0.0, 1.0, 0.0]);
        assert!(a.cosine(&b).abs() < 1e-6);
    }
    #[test] fn test_cosine_dim_mismatch() {
        let a = Vector::new("a", vec![1.0, 0.0]);
        let b = Vector::new("b", vec![1.0, 0.0, 0.0]);
        assert_eq!(a.cosine(&b), 0.0);
    }
    #[test] fn test_top_k() {
        let q = Vector::new("q", vec![1.0, 0.0]);
        let corpus = vec![
            Vector::new("a", vec![1.0, 0.0]),
            Vector::new("b", vec![0.0, 1.0]),
            Vector::new("c", vec![0.7, 0.7]),
        ];
        let top = top_k(&q, &corpus, 2);
        assert_eq!(top[0].0, 1.0); // a 完全匹配
        assert_eq!(top[1].0 > 0.5 && top[1].0 < 1.0, true); // c 部分匹配
    }
}
