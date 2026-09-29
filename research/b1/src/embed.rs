//! 嵌入抽象层: 骨架阶段不引入 fastembed, 用 trait 抽象 + 注入实现。
//!
//! 协议 §7.3: 嵌入模型必须钉死并写进 config_hash。默认 fastembed 本地 ONNX
//! `all-MiniLM-L6-v2` (384-d, 确定性、零 API 成本、离线可复现), 与 `DS_API_KEY` 无关。
//! 骨架阶段 (0 LLM) 不实现真实嵌入, 只提供:
//!   - `EmbeddingProvider` trait (运行时/未来 fastembed 的接口面);
//!   - `InjectedEmbeddingProvider` (从外部传入向量, 供测试与离线构图);
//!   - `l2_normalize` (协议 §2.2: 无论哪种嵌入, 一律 L2 归一化后再算距离)。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 批量嵌入接口: 输入文本, 输出同序嵌入向量 (未归一化)。
pub trait EmbeddingProvider {
    fn embed(&self, texts: &[String]) -> Vec<Vec<f32>>;
}

/// 从外部注入向量的嵌入提供者 (测试与离线构图用)。
/// 向量顺序必须与 `texts` 顺序一一对应。
pub struct InjectedEmbeddingProvider {
    pub vectors: Vec<Vec<f32>>,
}

impl EmbeddingProvider for InjectedEmbeddingProvider {
    fn embed(&self, texts: &[String]) -> Vec<Vec<f32>> {
        assert_eq!(
            self.vectors.len(),
            texts.len(),
            "InjectedEmbeddingProvider: 向量数 {} != 文本数 {}",
            self.vectors.len(),
            texts.len()
        );
        self.vectors.clone()
    }
}

// TODO (协议 §7.3, 骨架阶段留占位, 不写假实现):
//   实现 `FastembedEmbeddingProvider`:
//     - 依赖 fastembed (本地 ONNX), 模型钉死 `all-MiniLM-L6-v2` (384-d);
//     - 首次运行联网下载模型权重 (一次), 与 `DS_API_KEY` 无关;
//     - `embed(texts)` 返回 384 维向量, 调用方统一走 `l2_normalize`;
//     - 把 config_hash 的 `embed_model` 从 "injected" 切换为 "all-MiniLM-L6-v2";
//     - 缓存写 `research/b1/.cache/embeddings.bin` (二进制), 替代本骨架的 JSON 缓存。
//   注意: 不得伪造确定性的伪嵌入来跑真实验 —— 那等于篡改输入。

/// L2 归一化 (原地)。协议 §2.2: 一律归一化, 使欧氏距离 ∈ [0,2]。
pub fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-12 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// 缓存文件格式 (骨架用 JSON, 便于人工审计; fastembed 落地后换成 `.bin` 二进制)。
/// ```json
/// {
///   "format": 1,
///   "embed_model": "injected",
///   "dim": 384,
///   "vectors": { "<dia_id>": [0.0, ...] }
/// }
/// ```
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EmbeddingCache {
    #[serde(default)]
    pub format: u32,
    #[serde(default)]
    pub embed_model: String,
    #[serde(default)]
    pub dim: usize,
    pub vectors: HashMap<String, Vec<f32>>,
}

/// 从缓存文件读取 dia_id → 嵌入向量。
pub fn load_cache(path: &Path) -> Result<EmbeddingCache, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("读嵌入缓存 {} 失败: {e}", path.display()))?;
    let cache: EmbeddingCache =
        serde_json::from_str(&text).map_err(|e| format!("解析嵌入缓存失败: {e}"))?;
    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_provider_returns_vectors_in_order() {
        let p = InjectedEmbeddingProvider {
            vectors: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        let out = p.embed(&["a".into(), "b".into()]);
        assert_eq!(out, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    #[should_panic]
    fn injected_provider_mismatch_panics() {
        let p = InjectedEmbeddingProvider {
            vectors: vec![vec![1.0]],
        };
        let _ = p.embed(&["a".into(), "b".into()]);
    }

    #[test]
    fn l2_normalize_unit_and_zero() {
        let mut v = vec![3.0, 4.0];
        l2_normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);

        let mut z = vec![0.0, 0.0];
        l2_normalize(&mut z); // 零向量保持不变 (不除零)
        assert_eq!(z, vec![0.0, 0.0]);
    }
}
