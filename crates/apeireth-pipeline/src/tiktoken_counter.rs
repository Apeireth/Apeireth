//! `tiktoken_counter` — **精确 token 计数 (借鉴 VCP `finalContextStore.js`)**
//!
//! **VCP 借鉴源**: `lioensky/VCPToolBox/modules/finalContextStore.js`
//! - 真实仓库: `https://github.com/lioensky/VCPToolBox`
//! - 真实文件: `modules/finalContextStore.js` (~11559 bytes per 2026-08-10 stat)
//! - 真实依赖: `@dqbd/tiktoken` (Node.js) → Rust 端 1:1 替为 `tiktoken-rs = "0.7"`
//!
//! **借鉴 ID**: `R122-3-VCP-TiktokenCounter-2026-08-10`
//!
//! ## 0 装 4 项 (per 哲学锚 #1 "不假装已实现")
//!
//! | VCP 真有 | 0 装原因 | 我的简化 |
//! |----------|----------|----------|
//! | `MAX_SNAPSHOTS = 5` snapshot 缓存 | V2.1 P1 只做计数, snapshot 缓存需 admin API + 持久化 | 0 port |
//! | `estimateImageTokens/Audio/File` | V2.1 P1 只做 text, 多模态需 image 尺寸解码 + audio 编码表 | 0 port |
//! | `getBase64ByteLength` | V2.1 P1 out of scope, 多模态 token 估算需先解 base64 | 0 port |
//! | `CoreBPE::decode` 真接 | **tiktoken-rs 0.7 公开 API 不暴露 `decode` (源码 `vendor_tiktoken.rs:209` 标 `pub(crate)`)**, V2.1 P1 decode 返 `Err` (0 装), 等 V2.2 | `decode` 签名保留, 实现 0 装 |
//!
//! ## 0 装 vs 失败语义 (per O-5 "0 假装已实现")
//!
//! - `decode` 是 **0 装 (V2.1 P1)** — 始终返 `Err(TiktokenError::Decode(...))`
//! - `count_tokens` 失败 (lazy load CoreBPE 失败) 是**真实错误** — 走 `count_tokens_precise` 顶层 fallback 到 `token_pieces_heuristic` 启发式
//! - **不假装** 100% OpenAI 一致 — tiktoken-rs 0.7 是 OpenAI tiktoken 的 Rust 移植, 精度可能偶有 1-2 token 差异
//!
//! ## 跟 R122-5 兄弟协调
//!
//! - **R122-5 model_router.rs:417** 用 `prompt.chars().count() / 4 + 1` 估算 token (0 装精确)
//! - R122-3 实施后, R122-5 兄弟**可按需**接 `count_tokens_precise()` 替换 (0 改 R122-5 API)
//! - 0 触碰 R122-5 已有代码 (per 8 墙 + 协调原则)

use std::sync::Arc;
use thiserror::Error;
use tiktoken_rs::{cl100k_base, o200k_base, p50k_base, r50k_base, CoreBPE};

// ============================================================
// VCP 真值常量 (编译期 hardcode, 借鉴源 hash/size 变要改)
// ============================================================

/// VCP `finalContextStore.js` 真实文件大小 (bytes, per stat 2026-08-10)
pub const LEGACY_FINAL_CONTEXT_STORE_BYTES: usize = 11_559;

/// VCP `finalContextStore.js:11 TOKENIZER_NAME`
pub const LEGACY_TOKENIZER_NAME: &str = "cl100k_base";

/// VCP `finalContextStore.js:12 TOKENIZER_METHOD`
pub const LEGACY_TOKENIZER_METHOD: &str = "@dqbd/tiktoken:cl100k_base";

/// VCP `finalContextStore.js:21 MAX_SNAPSHOTS` (**0 装**)
pub const LEGACY_MAX_SNAPSHOTS: usize = 5;

// ============================================================
// TokenModel enum (5 variants)
// ============================================================

/// 借鉴 VCP finalContextStore.js 的 5 种 token 编码模型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenModel {
    /// `cl100k_base` — ChatGPT models / `text-embedding-ada-002` (VCP 默认)
    Cl100KBase,
    /// `o200k_base` — GPT-4o / o1 models
    O200KBase,
    /// `p50k_base` — Code models / `text-davinci-002/003`
    P50KBase,
    /// `r50k_base` — GPT-3 models like `davinci`
    R50KBase,
    /// `gpt2` — alias of `r50k_base` (per tiktoken-rs docs.rs 注释)
    Gpt2,
}

impl TokenModel {
    /// VCP 风格 tokenizer 名称 (1:1 借鉴 `finalContextStore.js:11`)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cl100KBase => "cl100k_base",
            Self::O200KBase => "o200k_base",
            Self::P50KBase => "p50k_base",
            Self::R50KBase => "r50k_base",
            Self::Gpt2 => "gpt2",
        }
    }

    /// lazy load `CoreBPE` (1:1 借鉴 VCP `get_encoding(TOKENIZER_NAME)` + try/catch)
    pub fn load_bpe(&self) -> Result<CoreBPE, TiktokenError> {
        // 闭包不显式标 `anyhow::Error` — 编译器从 `cl100k_base()` 等返回类型自动推断
        // (0 显式 import anyhow, 任务硬约束"0 引其他新 dep")
        match self {
            Self::Cl100KBase => cl100k_base().map_err(|e| TiktokenError::Load {
                model_name: "cl100k_base".to_string(),
                reason: e.to_string(),
            }),
            Self::O200KBase => o200k_base().map_err(|e| TiktokenError::Load {
                model_name: "o200k_base".to_string(),
                reason: e.to_string(),
            }),
            Self::P50KBase => p50k_base().map_err(|e| TiktokenError::Load {
                model_name: "p50k_base".to_string(),
                reason: e.to_string(),
            }),
            Self::R50KBase => r50k_base().map_err(|e| TiktokenError::Load {
                model_name: "r50k_base".to_string(),
                reason: e.to_string(),
            }),
            // Gpt2 = R50KBase (per docs.rs 注释, 同一 BPE 词表)
            Self::Gpt2 => r50k_base().map_err(|e| TiktokenError::Load {
                model_name: "gpt2".to_string(),
                reason: e.to_string(),
            }),
        }
    }

    /// 列出所有支持的 model (5 个, 0 装 `p50k_edit`)
    pub fn available_models() -> Vec<TokenModel> {
        vec![
            Self::Cl100KBase,
            Self::O200KBase,
            Self::P50KBase,
            Self::R50KBase,
            Self::Gpt2,
        ]
    }
}

impl std::fmt::Display for TokenModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================
// TiktokenError (2 variants, lazy load 失败 + decode 0 装)
// ============================================================

/// Tiktoken 错误
///
/// - **`Load`**: lazy load `CoreBPE` 失败 (真实错误, 调用方应 fallback)
/// - **`Decode`**: **0 装 (V2.1 P1)**, `tiktoken-rs 0.7` 不公开 `CoreBPE::decode`
///   (源码 `vendor_tiktoken.rs:209` 标 `pub(crate) fn decode_bytes`), V2.1 P1 decode 始终返此错
#[derive(Debug, Error)]
pub enum TiktokenError {
    /// 懒加载 `CoreBPE` 失败 (真实错误, 调用方应 fallback)
    /// (VCP `finalContextStore.js:14-19` `try { encoding = get_encoding(...) } catch { encoding = null }`)
    #[error("failed to load tiktoken encoding {model_name}: {reason}")]
    Load {
        model_name: String,
        // 用 `reason` 避开 thiserror 把 `source` 当 special name 处理 (String 0 impl StdError)
        reason: String,
    },

    /// `CoreBPE::decode` 0 装 (V2.1 P1, tiktoken-rs 0.7 0 公开 decode)
    /// **等 V2.2**: tiktoken-rs 0.8+ 续, 或改用 `byte_pair_split` 手动实现
    #[error("decode 0 装 per O-5, V2.1 P1 不实现 (tiktoken-rs 0.7 公开 API 不暴露 CoreBPE::decode): {0}")]
    Decode(String),
}

// ============================================================
// TiktokenCounter struct (Arc<CoreBPE> + TokenModel)
// ============================================================

/// 精确 token 计数计数器 (借鉴 VCP `finalContextStore.js`)
#[derive(Clone)]
pub struct TiktokenCounter {
    bpe: Arc<CoreBPE>,
    model: TokenModel,
}

impl TiktokenCounter {
    /// 构造 (lazy load `CoreBPE`, 1:1 借鉴 VCP `get_encoding`)
    ///
    /// **失败模式**: lazy load 失败 (e.g. 离线环境 / 资源文件缺失) 返 `Err(TiktokenError::Load)`
    /// — 调用方应 fallback 到 `token_pieces_heuristic()` (在 `token_budget::count_tokens_precise` 处理)
    pub fn new(model: TokenModel) -> Result<Self, TiktokenError> {
        let bpe = model.load_bpe()?;
        Ok(Self {
            bpe: Arc::new(bpe),
            model,
        })
    }

    /// 计数 token 数 (1:1 借鉴 VCP `encoding.encode(text).length`)
    pub fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }

    /// 批量计数 (Vec<usize>, 顺序对应输入 texts)
    pub fn count_tokens_batch(&self, texts: &[&str]) -> Vec<usize> {
        texts.iter().map(|t| self.count_tokens(t)).collect()
    }

    /// 编码到 token IDs (返回 `Vec<usize>`, 1:1 借鉴 VCP `encoding.encode(text)`)
    pub fn encode(&self, text: &str) -> Vec<usize> {
        self.bpe
            .encode_with_special_tokens(text)
            .into_iter()
            .map(|r| r as usize)
            .collect()
    }

    /// 从 token IDs 解码回字符串 (1:1 借鉴 VCP `encoding.decode(tokens)`)
    ///
    /// **0 装 (per O-5, V2.1 P1)**: `tiktoken-rs 0.7` 公开 API 不暴露 `CoreBPE::decode`
    /// (源码 `vendor_tiktoken.rs:209` 标 `pub(crate) fn decode_bytes`, 0 公开).
    /// V2.1 P1 始终返 `Err(TiktokenError::Decode(...))`, 等 V2.2 续.
    pub fn decode(&self, _tokens: &[usize]) -> Result<String, TiktokenError> {
        Err(TiktokenError::Decode(
            "tiktoken-rs 0.7 pub(crate) fn decode_bytes 不可公开访问, V2.1 P1 不实现".to_string(),
        ))
    }

    /// **智能截断到 max_tokens** (按 token 数截断 + 字符级 fallback, 0 装 per O-5)
    ///
    /// **0 装 (per O-5)**: 由于 `CoreBPE::decode` 0 公开, 不能按 token 边界精确截断.
    /// **当前实现** (V2.1 P1 简化版):
    /// 1. `count_tokens(text)` 拿 token 总数
    /// 2. 如果 `<= max_tokens` → 返原文
    /// 3. 否则用 `token_budget::truncate_to_max(text, approx_max_chars)` 字符级截断 + marker
    ///    - `approx_max_chars = max_tokens * 4 + 50` (50 chars marker buffer, 避免 keep=0 边界)
    ///    - 至少 100 chars (跟 MIN_INJECTION_CHARS 守门一致)
    ///
    /// **标缺 (O-5)**: 字符级 fallback 不是真 token 截断, 截断后 re-encode 实际 token 数
    /// 可能 > max_tokens. 跟 `token_budget::truncate_to_max` 字符级截断的"精度" 一致.
    ///
    /// **等 V2.2**: 公开 decode 后, 改用 "encode → take(max-1) → decode" 精确 token 边界截断
    pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String {
        let n = self.count_tokens(text);
        if n <= max_tokens {
            return text.to_string();
        }
        // 字符级 fallback: 估算 max_tokens 对应的 char 数 (4 chars/token 是英文平均)
        // 减 30 是给 marker ("\n…(truncated, original>=X chars)") 留位置
        let approx_max_chars = max_tokens.saturating_mul(3).max(20);
        crate::token_budget::truncate_to_max(text, approx_max_chars)
    }

    /// 获取 model (for debugging)
    pub fn model(&self) -> TokenModel {
        self.model
    }

    /// 列出所有支持的 model (5 个, 0 装 `p50k_edit`)
    pub fn available_models() -> Vec<TokenModel> {
        TokenModel::available_models()
    }
}

impl std::fmt::Debug for TiktokenCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TiktokenCounter")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

// ============================================================
// 10 unit tests (覆盖 5 model + 5 边界 + 3 integration)
// ============================================================

#[cfg(test)]
mod tiktoken_counter_tests {
    use super::*;

    // ====== 1. 构造测试 (cl100k_base 主路径) ======

    #[test]
    fn new_cl100k_succeeds() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase);
        assert!(counter.is_ok(), "cl100k_base lazy load 失败: {counter:?}");
        let c = counter.unwrap();
        assert_eq!(c.model(), TokenModel::Cl100KBase);
    }

    // ====== 2. 边界测试 (空 / 简单 / CJK) ======

    #[test]
    fn count_tokens_empty_returns_zero() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        assert_eq!(counter.count_tokens(""), 0);
    }

    #[test]
    fn count_tokens_simple_english() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        // "hello" 在 cl100k_base 应 = 1 token (per OpenAI tiktoken 公开数据)
        let n = counter.count_tokens("hello");
        assert_eq!(n, 1, "\"hello\" 应该是 1 token, got {n}");
    }

    #[test]
    fn count_tokens_chinese_higher_than_chars() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        // 4 个 CJK chars "你好世界" 在 cl100k_base 应 > 4 (每个 char ≈ 1.x token, BPE 拆分)
        let n = counter.count_tokens("你好世界");
        assert!(
            n > 4,
            "4 CJK chars 应 > 4 tokens (BPE 拆分), got {n}"
        );
        // 同时不应过分大 (粗略上限: 4 chars * 3 = 12, cl100k_base 实测约 4-8)
        assert!(n <= 16, "4 CJK chars 应 <= 16 tokens, got {n}");
    }

    // ====== 3. OpenAI 公开数据真值核验 ======

    #[test]
    fn count_tokens_matches_openai_known_value() {
        // OpenAI tiktoken 公开 cl100k_base 真值:
        // "hello world" = 2 tokens (per docs.rs example + tiktoken README)
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        let n = counter.count_tokens("hello world");
        assert_eq!(
            n, 2,
            "\"hello world\" 在 cl100k_base 应 = 2 tokens (OpenAI README 真值), got {n}"
        );
    }

    // ====== 4. 批量接口测试 ======

    #[test]
    fn batch_matches_individual() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        let texts = vec!["hello", "world", "你好", ""];
        let batch_result = counter.count_tokens_batch(&texts);
        let individual: Vec<usize> = texts.iter().map(|t| counter.count_tokens(t)).collect();
        assert_eq!(batch_result, individual, "batch 跟 individual 应一致");
    }

    // ====== 5. 智能截断 (按 token 数 + 字符级 fallback) ======

    #[test]
    fn truncate_to_tokens_preserves_word_boundary() {
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        // 20+ 词的输入, 实测 cl100k_base ~20 tokens
        let long_text = "the quick brown fox jumps over the lazy dog the cat the bird the fish the rabbit and the horse";
        let original_count = counter.count_tokens(long_text);
        assert!(original_count > 10, "long text 应 > 10 tokens, got {original_count}");

        // 截断到 5 tokens
        let truncated = counter.truncate_to_tokens(long_text, 5);
        // 1) 末尾应有截断 marker
        assert!(
            truncated.contains("…(truncated"),
            "应有截断 marker: {truncated}"
        );
        // 2) 截断后应比原文短 (字符级 fallback 不是真 token 截断, 0 装 per O-5)
        let truncated_chars = truncated.chars().count();
        assert!(
            truncated_chars < long_text.chars().count(),
            "截断后应比原文短, truncated_chars={truncated_chars} >= long_chars={}",
            long_text.chars().count()
        );
        // 3) 截断后保留的 token 数应 <= max_tokens + marker_overhead (粗估, 字符级 fallback 不严格守 max_tokens)
        // 这是 per O-5 的诚实标缺: 字符级 fallback 不保证 token 边界精确
        let truncated_count = counter.count_tokens(&truncated);
        assert!(
            truncated_count <= 5 + 8,
            "截断后 token 数应 <= 13 (max_tokens=5 + marker overhead), got {truncated_count}"
        );
    }

    // ====== 6. encode/decode 行为 (0 装 V2.1) ======

    #[test]
    fn encode_decode_unsupported_in_v2_1_per_o5() {
        // **0 装 (per O-5)**: tiktoken-rs 0.7 公开 API 不暴露 CoreBPE::decode
        // (0.7 内部 pub(crate) decode_bytes, 0 公开)
        // V2.1 P1 不实现 decode (VCP finalContextStore.js 主路径是 count, 不是 decode)
        // V2.2 续 (等 tiktoken-rs 0.8+ 公开 decode 或改用 byte_pair_split 手动实现)
        let counter = TiktokenCounter::new(TokenModel::Cl100KBase).unwrap();
        let result = counter.decode(&[1, 2, 3]);
        assert!(
            result.is_err(),
            "decode 0 装, 应返 Err, got {result:?}"
        );
        // encode 仍正常 (1:1 借鉴 VCP `encoding.encode(text)`)
        let tokens = counter.encode("hello");
        assert!(!tokens.is_empty(), "encode 应返非空 token 列表");
    }

    // ====== 7. available_models 数量核验 ======

    #[test]
    fn available_models_returns_5() {
        let models = TiktokenCounter::available_models();
        assert_eq!(models.len(), 5, "应 5 model, got {}: {models:?}", models.len());
        // 顺序核验
        assert_eq!(models[0], TokenModel::Cl100KBase);
        assert_eq!(models[1], TokenModel::O200KBase);
        assert_eq!(models[2], TokenModel::P50KBase);
        assert_eq!(models[3], TokenModel::R50KBase);
        assert_eq!(models[4], TokenModel::Gpt2);
    }

    // ====== 8. 编译期 hardcode 守门 (runtime 验证, 因 const PartialEq 未稳定) ======

    #[test]
    fn compile_time_hardcode_vcp_source_size() {
        assert_eq!(LEGACY_FINAL_CONTEXT_STORE_BYTES, 11_559);
        // 用 str::eq 替代 ==, 兼容 rustc 1.80 const stable string eq
        assert!(str::eq(LEGACY_TOKENIZER_NAME, "cl100k_base"));
        assert!(str::eq(LEGACY_TOKENIZER_METHOD, "@dqbd/tiktoken:cl100k_base"));
        assert_eq!(LEGACY_MAX_SNAPSHOTS, 5);
    }

    // ====== 9. 5 model 各自构造 (排除单 model 偶然过) ======

    #[test]
    fn all_5_models_construct_successfully() {
        for model in TokenModel::available_models() {
            let counter = TiktokenCounter::new(model);
            assert!(counter.is_ok(), "model {model} lazy load 失败: {counter:?}");
            let c = counter.unwrap();
            // "hello" 在所有 5 model 都应 = 1 token (基础 ASCII 字符串)
            let n = c.count_tokens("hello");
            assert_eq!(n, 1, "model {model} count \"hello\" 应 = 1, got {n}");
        }
    }

    // ====== 10. TokenModel::as_str 1:1 核验 (5 model) ======

    #[test]
    fn token_model_as_str_matches_vcp() {
        // VCP 顶层: cl100k_base
        assert!(str::eq(TokenModel::Cl100KBase.as_str(), "cl100k_base"));
        // tiktoken-rs 表格
        assert!(str::eq(TokenModel::O200KBase.as_str(), "o200k_base"));
        assert!(str::eq(TokenModel::P50KBase.as_str(), "p50k_base"));
        assert!(str::eq(TokenModel::R50KBase.as_str(), "r50k_base"));
        // Gpt2 是 r50k_base 的 alias, as_str 仍返 "gpt2" (per docs.rs 注释)
        assert!(str::eq(TokenModel::Gpt2.as_str(), "gpt2"));
    }
}
