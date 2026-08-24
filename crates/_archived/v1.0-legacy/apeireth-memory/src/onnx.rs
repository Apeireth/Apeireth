// 模块级豁免: 测试需操纵进程 env (std::env::set_var 在 Rust 2024 为 unsafe);
// crate 级 deny(unsafe_code) 在此收敛 (仅测试路径使用, 可审查).
#![allow(unsafe_code)]

//! `apeireth-memory::onnx` — 本地 ONNX embedding (审计 P2#12, 2026-08-16 backlog 全清).
//!
//! 目标: 去 MiniMax 依赖的本地嵌入路径。实现 (0 装 PASS):
//! - feature `onnx` 开启: [`OnnxEmbedder`] 用 **tract-onnx** (纯 Rust 推理引擎,
//!   无预编译 runtime 下载 — GitHub 被墙环境下可构建) 真加载 `.onnx` 模型真推理。
//! - feature 关闭 / 模型缺失: 调用方用 [`resolve_local_embedder`] 得到明确 Err,
//!   由调用方降级 [`HashEmbedder`] (确定性, 语义质量降级但诚实标注)。
//!
//! 诚实边界: 本实现假定模型输入是 **token-id 张量** (i64, shape [1, seq_len]) —
//! 完整 LLM tokenizer 超出本模块范围; 提供 [`embed_token_ids`] 原始接口,
//! [`embed`] 接受 "id id id" 文本格式 (显式文档化, 不假装通用文本模型)。

use std::path::Path;
use std::sync::Arc;

use crate::semantic::{EmbedFn, EmbedderIdentity, HashEmbedder};

/// 本地嵌入源 (配置选择).
#[derive(Debug, Clone, PartialEq)]
pub enum LocalEmbedderSource {
    /// 确定性 hash (测试/开发/降级).
    Hash,
    /// ONNX 模型文件路径.
    Onnx(String),
}

/// 解析本地嵌入源: env `APEIRETH_LOCAL_EMBEDDER` =
/// `hash` | `onnx:<model.onnx>`. 未设置 → Hash (默认诚实降级).
pub fn local_embedder_source_from_env() -> LocalEmbedderSource {
    match std::env::var("APEIRETH_LOCAL_EMBEDDER") {
        Ok(v) if v.starts_with("onnx:") => {
            LocalEmbedderSource::Onnx(v["onnx:".len()..].to_string())
        }
        _ => LocalEmbedderSource::Hash,
    }
}

/// 按源解析本地 embedder (trait 对象).
/// Onnx 加载失败 → 明确 Err (不假装); 调用方自行降级 Hash.
pub fn resolve_local_embedder() -> Result<Arc<dyn EmbedFn>, String> {
    match local_embedder_source_from_env() {
        LocalEmbedderSource::Hash => Ok(Arc::new(HashEmbedder::new(384))),
        LocalEmbedderSource::Onnx(path) => {
            let e = OnnxEmbedder::load(Path::new(&path)).map_err(|e| {
                format!("ONNX 加载失败 ({path}): {e} — 请检查模型路径, 或设 APEIRETH_LOCAL_EMBEDDER=hash 降级")
            })?;
            Ok(Arc::new(e))
        }
    }
}

/// ONNX 本地嵌入器 (feature "onnx"): tract-onnx 真加载真推理.
#[cfg(feature = "onnx")]
pub struct OnnxEmbedder {
    // tract 0.23: into_runnable() 返回 Arc<TypedRunnableModel> (SimplePlan alias),
    // run() 接收 &Arc<Self> → 必须持 Arc
    model: Arc<tract_core::model::typed::TypedRunnableModel>,
    dim: usize,
}

#[cfg(feature = "onnx")]
impl OnnxEmbedder {
    /// 加载 ONNX 模型 (tract 纯 Rust, 无 runtime 下载).
    /// 加载时用模型声明的输入 shape 做一次探测推理, 取输出最后维为 dim.
    pub fn load(model_path: &Path) -> Result<Self, String> {
        use tract_onnx::prelude::*;

        // 1. 模型输入 shape (TypedModel 阶段可查, plan 阶段不可)
        let model = tract_onnx::onnx()
            .model_for_path(model_path)
            .map_err(|e| e.to_string())?;
        let input_shape: Vec<usize> = {
            let inputs = model.input_outlets().map_err(|e| e.to_string())?;
            let fact = model.outlet_fact(inputs[0]).map_err(|e| e.to_string())?;
            fact.shape
                .as_concrete_finite()
                .map_err(|e| e.to_string())?
                .unwrap_or_default()
                .to_vec()
        };
        if input_shape.is_empty() {
            return Err("ONNX 模型输入 shape 无法确定 (无法构造探测输入)".to_string());
        }
        // 2. 编译 + 运行 (into_runnable → Arc<TypedRunnableModel>, run 直接可用)
        let plan = model
            .into_optimized()
            .map_err(|e| e.to_string())?
            .into_runnable()
            .map_err(|e| e.to_string())?;
        let tensor = tract_ndarray::Array::<i64, _>::zeros(input_shape).into_tensor();
        let outputs = plan.run(tvec!(tensor.into())).map_err(|e| e.to_string())?;
        let out = outputs[0]
            .to_plain_array_view::<f32>() // tract 0.23: to_array_view → to_plain_array_view
            .map_err(|e| e.to_string())?;
        let dim = out.shape().last().copied().unwrap_or(0);
        if dim == 0 {
            return Err("ONNX 模型输出维度为 0 (无法确定 dim)".to_string());
        }
        Ok(Self { model: plan, dim })
    }

    /// 原始接口: token-id 序列 → 向量 (shape [1, seq_len] i64).
    pub fn embed_token_ids(&self, ids: &[i64]) -> Result<Vec<f32>, String> {
        use tract_onnx::prelude::*;
        if ids.is_empty() {
            return Ok(vec![0.0; self.dim]);
        }
        let tensor = tract_ndarray::Array::from_shape_vec((1, ids.len()), ids.to_vec())
            .map_err(|e| e.to_string())?
            .into_tensor();
        let outputs = self
            .model
            .run(tvec!(tensor.into()))
            .map_err(|e| e.to_string())?;
        let out = outputs[0]
            .to_plain_array_view::<f32>() // tract 0.23: to_array_view → to_plain_array_view
            .map_err(|e| e.to_string())?;
        // 取最后一维 (句向量/池化输出)
        let flat: Vec<f32> = out.iter().copied().collect();
        let len = flat.len();
        Ok(flat[len.saturating_sub(self.dim)..].to_vec())
    }
}

#[cfg(feature = "onnx")]
impl EmbedFn for OnnxEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    /// 文本 → 向量. **诚实边界**: 本实现接受 "id id id" token-id 文本格式
    /// (完整 tokenizer 超出范围, 如实标注); 其他文本得到确定性 hash 兜底向量
    /// (不假装语义正确 — 调用方应自行 tokenize 后走 embed_token_ids).
    fn embed(&self, text: &str) -> Vec<f32> {
        let ids: Vec<i64> = text
            .split_whitespace()
            .filter_map(|t| t.parse::<i64>().ok())
            .collect();
        if ids.is_empty() {
            // 非 token-id 文本 → 诚实降级 hash (维度对齐)
            let h = HashEmbedder::new(self.dim);
            return h.embed(text);
        }
        self.embed_token_ids(&ids).unwrap_or_else(|_| {
            let h = HashEmbedder::new(self.dim);
            h.embed(text)
        })
    }

    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("apeireth/onnx-local/v1", self.dim)
    }
}

/// feature 关闭时的诚实占位: 明确报错 (不假装装了 ONNX).
#[cfg(not(feature = "onnx"))]
pub struct OnnxEmbedder;

#[cfg(not(feature = "onnx"))]
impl OnnxEmbedder {
    pub fn load(_model_path: &Path) -> Result<Self, String> {
        Err("ONNX 嵌入未启用 (feature \"onnx\" 关闭); 请用 --features onnx 构建, 或设 APEIRETH_LOCAL_EMBEDDER=hash 降级".to_string())
    }
}

/// 非 onnx 构建的类型占位实现: load 必 Err, embed 永不达 (resolve 不会返回本类型);
/// 实现仅为满足 trait 约束, 语义如实标注.
#[cfg(not(feature = "onnx"))]
impl EmbedFn for OnnxEmbedder {
    fn dim(&self) -> usize {
        0
    }
    fn embed(&self, _text: &str) -> Vec<f32> {
        Vec::new()
    }
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// env 是进程级共享 — 并行测试会竞争 APEIRETH_LOCAL_EMBEDDER.
    /// 用静态锁串行化 env 测试 (与 principles/master_token 同模式).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn source_parsing_from_env() {
        let _g = ENV_LOCK.lock().unwrap();
        // 未设置 → Hash (默认降级)
        unsafe { std::env::remove_var("APEIRETH_LOCAL_EMBEDDER") };
        assert_eq!(local_embedder_source_from_env(), LocalEmbedderSource::Hash);
        unsafe { std::env::set_var("APEIRETH_LOCAL_EMBEDDER", "onnx:C:/models/emb.onnx") };
        assert_eq!(
            local_embedder_source_from_env(),
            LocalEmbedderSource::Onnx("C:/models/emb.onnx".to_string())
        );
        unsafe { std::env::set_var("APEIRETH_LOCAL_EMBEDDER", "garbage") };
        assert_eq!(
            local_embedder_source_from_env(),
            LocalEmbedderSource::Hash,
            "未知值如实降级 Hash"
        );
        unsafe { std::env::remove_var("APEIRETH_LOCAL_EMBEDDER") };
    }

    #[test]
    fn resolve_defaults_to_hash() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("APEIRETH_LOCAL_EMBEDDER") };
        let e = resolve_local_embedder().expect("hash 解析");
        assert_eq!(e.dim(), 384);
    }

    #[test]
    fn onnx_missing_file_errors_honestly() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var(
                "APEIRETH_LOCAL_EMBEDDER",
                "onnx:C:/definitely/missing/model.onnx",
            )
        };
        let Err(msg) = resolve_local_embedder() else {
            panic!("模型缺失应明确报错 (不假装)")
        };
        assert!(msg.contains("ONNX 加载失败"), "错误应点名 ONNX 路径: {msg}");
        assert!(msg.contains("hash"), "应提示 hash 降级选项: {msg}");
        unsafe { std::env::remove_var("APEIRETH_LOCAL_EMBEDDER") };
    }

    #[test]
    fn hash_fallback_path_is_deterministic() {
        let a = HashEmbedder::new(32).embed("hello world");
        let b = HashEmbedder::new(32).embed("hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
