//! `apeireth-memory::hybrid_search` — BM25 词法与向量混合检索召回引擎 (R12-Storage 实施).
//!
//! **设计哲学 (存储与检索最优解)**:
//! - **① 双路召回 (Lexical + Semantic)**:
//!   - 词法路: 确定性 Okapi BM25 算法 (支持 ASCII 词元与 CJK 连续二字切词, 0 外部 NLP 库依赖)
//!   - 语义路: 确定性余弦相似度 [`VectorIndex`]
//! - **② 倒数排名融合 (Reciprocal Rank Fusion, RRF)**:
//!   - `RRF_score(d) = w_vec / (k + rank_vec(d)) + w_bm25 / (k + rank_bm25(d))` (标准 k = 60)
//!   - 抹平不同模型与词法评分量纲差异, 保证鲁棒排序
//! - **③ 加权分值融合 (Weighted Score Fusion)**:
//!   - `score(d) = α * norm(vec_score) + (1 - α) * norm(bm25_score)`
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决纯向量检索在精准关键词、专用术语和代码符号上召回不足的问题
//! 2. 系统: 放置在 `apeireth-memory`, 组合 `canonical::VectorIndex`
//! 3. 架构: 纯数学与确定性数据结构, 0 unsafe, 0 网络模型依赖

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::canonical::domain::MemoryId;
use crate::canonical::vector::VectorIndex;
use crate::MemoryError;

/// BM25 超参数配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Config {
    /// 词频饱和度参数 k1 (默认 1.2).
    pub k1: f32,
    /// 文档长度归一化参数 b (默认 0.75).
    pub b: f32,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self { k1: 1.2, b: 0.75 }
    }
}

impl Bm25Config {
    /// Returns parameters that are safe for floating-point scoring.
    ///
    /// `Bm25Config` remains a plain public configuration struct for backwards
    /// compatibility.  A non-finite `k1` or `b` is therefore normalized to the
    /// documented defaults at scoring time, and a finite `b` is constrained to
    /// the conventional `[0.0, 1.0]` range.  This prevents a caller-provided
    /// NaN from escaping into a ranking comparator.
    fn normalized_for_scoring(&self) -> Self {
        let defaults = Self::default();
        Self {
            k1: if self.k1.is_finite() && self.k1 >= 0.0 {
                self.k1
            } else {
                defaults.k1
            },
            b: if self.b.is_finite() {
                self.b.clamp(0.0, 1.0)
            } else {
                defaults.b
            },
        }
    }
}

/// Orders scored results by score descending, followed by stable identity
/// ascending.  All search paths use this rule so replay ordering does not
/// depend on hash-map iteration order.
fn compare_score_desc_then_id(
    left_score: f32,
    left_id: &str,
    right_score: f32,
    right_id: &str,
) -> std::cmp::Ordering {
    right_score
        .total_cmp(&left_score)
        .then_with(|| left_id.cmp(right_id))
}

/// Keeps ranking scores finite even when a caller supplies extreme but finite
/// floating-point configuration values.  BM25 and RRF are non-negative in
/// their supported parameter ranges, so positive overflow is saturated and
/// invalid/negative intermediate values become the least relevant score.
fn finite_non_negative_score(score: f32) -> f32 {
    if score.is_finite() && score >= 0.0 {
        score
    } else if score.is_infinite() && score.is_sign_positive() {
        f32::MAX
    } else {
        0.0
    }
}

/// 确定性分词器: 提取 ASCII 小写单词 + CJK 连续二字 (char-bigram).
/// 0 外部依赖, 纯确定性.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_ascii = String::new();
    let mut last_cjk: Option<char> = None;

    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            current_ascii.push(c.to_ascii_lowercase());
            last_cjk = None;
        } else if c.is_alphabetic() && (c as u32) > 0x2E80 {
            if !current_ascii.is_empty() {
                tokens.push(std::mem::take(&mut current_ascii));
            }
            if let Some(prev) = last_cjk {
                tokens.push(format!("{prev}{c}"));
            } else {
                tokens.push(c.to_string());
            }
            last_cjk = Some(c);
        } else {
            if !current_ascii.is_empty() {
                tokens.push(std::mem::take(&mut current_ascii));
            }
            last_cjk = None;
        }
    }
    if !current_ascii.is_empty() {
        tokens.push(current_ascii);
    }
    tokens
}

/// 词法 BM25 检索命中条目.
#[derive(Debug, Clone, PartialEq)]
pub struct Bm25Hit {
    pub id: String,
    pub score: f32,
}

/// 纯内存确定性 Okapi BM25 索引.
#[derive(Debug, Clone)]
pub struct Bm25Index {
    config: Bm25Config,
    /// 文档 ID -> 文档总词数
    doc_lengths: HashMap<String, usize>,
    /// 文档 ID -> (词项 -> 词频)
    doc_term_freqs: HashMap<String, HashMap<String, usize>>,
    /// 词项 -> 包含该词项的文档总数 (Document Frequency)
    term_doc_freqs: HashMap<String, usize>,
    /// 总文档数
    total_docs: usize,
    /// 所有文档总词长之和
    total_tokens: usize,
}

impl Bm25Index {
    /// 构造新的 BM25 索引.
    pub fn new(config: Bm25Config) -> Self {
        Self {
            config,
            doc_lengths: HashMap::new(),
            doc_term_freqs: HashMap::new(),
            term_doc_freqs: HashMap::new(),
            total_docs: 0,
            total_tokens: 0,
        }
    }

    /// 索引中的文档总数.
    pub fn len(&self) -> usize {
        self.total_docs
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.total_docs == 0
    }

    /// 平均文档长度.
    pub fn avg_doc_length(&self) -> f32 {
        if self.total_docs == 0 {
            0.0
        } else {
            (self.total_tokens as f32) / (self.total_docs as f32)
        }
    }

    /// 插入或更新文档.
    pub fn insert(&mut self, id: impl Into<String>, text: &str) {
        let id = id.into();
        self.remove(&id);

        let tokens = tokenize(text);
        let doc_len = tokens.len();

        let mut tf_map: HashMap<String, usize> = HashMap::new();
        for t in &tokens {
            *tf_map.entry(t.clone()).or_default() += 1;
        }

        for term in tf_map.keys() {
            *self.term_doc_freqs.entry(term.clone()).or_default() += 1;
        }

        self.total_docs += 1;
        self.total_tokens += doc_len;
        self.doc_lengths.insert(id.clone(), doc_len);
        self.doc_term_freqs.insert(id, tf_map);
    }

    /// 移除文档.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(old_tf) = self.doc_term_freqs.remove(id) {
            let old_len = self.doc_lengths.remove(id).unwrap_or(0);
            self.total_docs = self.total_docs.saturating_sub(1);
            self.total_tokens = self.total_tokens.saturating_sub(old_len);

            for term in old_tf.keys() {
                if let Some(df) = self.term_doc_freqs.get_mut(term) {
                    *df = df.saturating_sub(1);
                    if *df == 0 {
                        self.term_doc_freqs.remove(term);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Executes a BM25 query, ordered by score descending and then document ID
    /// ascending.  The explicit ID tie-break makes equal-score results stable
    /// across process runs and hash-map layouts.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Hit> {
        if self.total_docs == 0 || top_k == 0 {
            return Vec::new();
        }

        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        let config = self.config.normalized_for_scoring();
        let avg_dl = self.avg_doc_length().max(1.0);
        let n = self.total_docs as f32;
        let mut scores: HashMap<String, f32> = HashMap::new();

        for term in &query_tokens {
            let df = self.term_doc_freqs.get(term).copied().unwrap_or(0) as f32;
            if df == 0.0 {
                continue;
            }

            // 标准平滑 Okapi IDF 公式: ln(1.0 + (N - df + 0.5) / (df + 0.5))
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            for (doc_id, tf_map) in &self.doc_term_freqs {
                if let Some(&tf) = tf_map.get(term) {
                    let doc_len = self.doc_lengths.get(doc_id).copied().unwrap_or(1) as f32;
                    let tf_f = tf as f32;
                    let num = tf_f * (config.k1 + 1.0);
                    let denom = tf_f + config.k1 * (1.0 - config.b + config.b * (doc_len / avg_dl));
                    let term_score = finite_non_negative_score(idf * (num / denom));
                    let entry = scores.entry(doc_id.clone()).or_default();
                    *entry = finite_non_negative_score(*entry + term_score);
                }
            }
        }

        let mut hits: Vec<Bm25Hit> = scores
            .into_iter()
            .map(|(id, score)| Bm25Hit { id, score })
            .collect();

        hits.sort_by(|a, b| compare_score_desc_then_id(a.score, &a.id, b.score, &b.id));
        hits.truncate(top_k);
        hits
    }
}

/// 混合检索命中条目.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridHit {
    /// 文档/记忆 ID
    pub id: String,
    /// 最终融合分值 (越高越相关)
    pub score: f32,
    /// 语义向量路排名 (从 1 开始, None 表示该路未命中)
    pub vector_rank: Option<usize>,
    /// 词法 BM25 路排名 (从 1 开始, None 表示该路未命中)
    pub bm25_rank: Option<usize>,
}

/// 混合检索召回引擎 (结合 VectorIndex 与 Bm25Index).
pub struct HybridSearchEngine {
    pub vector_index: VectorIndex,
    pub bm25_index: Bm25Index,
}

impl HybridSearchEngine {
    /// 构造新的混合检索引擎 (指定向量维度).
    pub fn new(vector_dimension: usize) -> Result<Self, MemoryError> {
        let vec_idx =
            VectorIndex::new(vector_dimension).map_err(|e| MemoryError::Invalid(e.to_string()))?;
        Ok(Self {
            vector_index: vec_idx,
            bm25_index: Bm25Index::new(Bm25Config::default()),
        })
    }

    /// 插入文档同时更新向量索引与 BM25 词法索引.
    pub fn insert(
        &mut self,
        id: &str,
        text: &str,
        vector: Option<Vec<f32>>,
    ) -> Result<(), MemoryError> {
        self.bm25_index.insert(id, text);
        if let Some(vec) = vector {
            let memory_id = MemoryId::new(id).map_err(|e| MemoryError::Invalid(e.to_string()))?;
            let _ = self.vector_index.remove(&memory_id);
            self.vector_index
                .insert(memory_id, vec)
                .map_err(|e| MemoryError::Other(e.to_string()))?;
        }
        Ok(())
    }

    /// Executes reciprocal-rank-fusion (RRF) hybrid retrieval.
    ///
    /// 公式: `score(d) = (w_vec / (k + rank_vec)) + (w_bm25 / (k + rank_bm25))`
    /// Parameters `rrf_k`, `w_vector`, and `w_bm25` must be finite and
    /// non-negative.  Results are ordered by fused score descending and then
    /// document ID ascending.
    ///
    /// `query_vector = None` intentionally disables the semantic channel and
    /// preserves lexical-only retrieval.  Supplying `Some(...)` selects the
    /// semantic channel; invalid vector input (including a dimension mismatch
    /// or NaN) is returned to the caller instead of being silently ignored.
    pub fn search_rrf(
        &self,
        query_text: &str,
        query_vector: Option<&[f32]>,
        top_k: usize,
        rrf_k: f32,
        w_vector: f32,
        w_bm25: f32,
    ) -> Result<Vec<HybridHit>, MemoryError> {
        validate_rrf_parameters(rrf_k, w_vector, w_bm25)?;

        let recall_limit = top_k.saturating_mul(2);
        let bm25_hits = self.bm25_index.search(query_text, recall_limit);
        let mut bm25_ranks: HashMap<String, usize> = HashMap::new();
        for (idx, hit) in bm25_hits.iter().enumerate() {
            bm25_ranks.insert(hit.id.clone(), idx + 1);
        }

        let mut vector_ranks: HashMap<String, usize> = HashMap::new();
        if let Some(vec) = query_vector {
            let vec_hits = self
                .vector_index
                .query(vec, recall_limit)
                .map_err(|error| {
                    MemoryError::Invalid(format!("semantic vector query rejected: {error}"))
                })?;
            for (idx, hit) in vec_hits.iter().enumerate() {
                vector_ranks.insert(hit.id.to_string(), idx + 1);
            }
        }

        // A BTreeSet is deterministic even before the final score sort.  This
        // matters for exact replay when all fusion inputs have identical score.
        let mut all_ids: BTreeSet<String> = BTreeSet::new();
        all_ids.extend(bm25_ranks.keys().cloned());
        all_ids.extend(vector_ranks.keys().cloned());

        let mut hybrid_hits = Vec::new();
        for id in all_ids {
            let v_rank = vector_ranks.get(&id).copied();
            let b_rank = bm25_ranks.get(&id).copied();

            let mut score = 0.0f32;
            if let Some(r) = v_rank {
                score = finite_non_negative_score(score + w_vector / (rrf_k + r as f32));
            }
            if let Some(r) = b_rank {
                score = finite_non_negative_score(score + w_bm25 / (rrf_k + r as f32));
            }

            hybrid_hits.push(HybridHit {
                id,
                score,
                vector_rank: v_rank,
                bm25_rank: b_rank,
            });
        }

        hybrid_hits.sort_by(|a, b| compare_score_desc_then_id(a.score, &a.id, b.score, &b.id));
        hybrid_hits.truncate(top_k);
        Ok(hybrid_hits)
    }

    /// Weighted score fusion (documented in this module as ③, previously
    /// unimplemented).
    ///
    /// `score(d) = α * norm(vec_score) + (1 - α) * norm(bm25_score)`
    ///
    /// Each channel is min-max normalized over the recalled set. A document
    /// missing from a channel contributes `0` for that channel. `alpha` must
    /// be finite and in `[0, 1]`. `query_vector = None` disables the semantic
    /// channel (same contract as [`Self::search_rrf`]).
    pub fn search_weighted(
        &self,
        query_text: &str,
        query_vector: Option<&[f32]>,
        top_k: usize,
        alpha: f32,
    ) -> Result<Vec<HybridHit>, MemoryError> {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(MemoryError::Invalid(
                "alpha must be finite and in [0, 1]".into(),
            ));
        }

        let recall_limit = top_k.saturating_mul(2);
        let bm25_hits = self.bm25_index.search(query_text, recall_limit);
        let mut bm25_scores: HashMap<String, (usize, f32)> = HashMap::new();
        for (idx, hit) in bm25_hits.iter().enumerate() {
            bm25_scores.insert(hit.id.clone(), (idx + 1, hit.score));
        }

        let mut vector_scores: HashMap<String, (usize, f32)> = HashMap::new();
        if let Some(vec) = query_vector {
            let vec_hits = self
                .vector_index
                .query(vec, recall_limit)
                .map_err(|error| {
                    MemoryError::Invalid(format!("semantic vector query rejected: {error}"))
                })?;
            for (idx, hit) in vec_hits.iter().enumerate() {
                vector_scores.insert(hit.id.to_string(), (idx + 1, hit.score));
            }
        }

        let mut all_ids: BTreeSet<String> = BTreeSet::new();
        all_ids.extend(bm25_scores.keys().cloned());
        all_ids.extend(vector_scores.keys().cloned());

        let bm25_min_max = min_max(bm25_scores.values().map(|(_, s)| *s));
        let vec_min_max = min_max(vector_scores.values().map(|(_, s)| *s));

        let mut hybrid_hits = Vec::new();
        for id in all_ids {
            let v = vector_scores.get(&id).copied();
            let b = bm25_scores.get(&id).copied();
            let vec_norm = v.map(|(_, s)| min_max_norm(s, vec_min_max)).unwrap_or(0.0);
            let bm25_norm = b.map(|(_, s)| min_max_norm(s, bm25_min_max)).unwrap_or(0.0);
            let score = finite_non_negative_score(alpha * vec_norm + (1.0 - alpha) * bm25_norm);
            hybrid_hits.push(HybridHit {
                id,
                score,
                vector_rank: v.map(|(r, _)| r),
                bm25_rank: b.map(|(r, _)| r),
            });
        }

        hybrid_hits.sort_by(|a, b| compare_score_desc_then_id(a.score, &a.id, b.score, &b.id));
        hybrid_hits.truncate(top_k);
        Ok(hybrid_hits)
    }
}

fn min_max(scores: impl Iterator<Item = f32>) -> Option<(f32, f32)> {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut any = false;
    for score in scores {
        if !score.is_finite() {
            continue;
        }
        any = true;
        min = min.min(score);
        max = max.max(score);
    }
    any.then_some((min, max))
}

fn min_max_norm(score: f32, bounds: Option<(f32, f32)>) -> f32 {
    let Some((min, max)) = bounds else {
        return 0.0;
    };
    if !score.is_finite() {
        return 0.0;
    }
    if (max - min).abs() < f32::EPSILON {
        // A degenerate range (one hit, or all equal) is uninformative for
        // discrimination but must not zero a channel that actually fired.
        return 1.0;
    }
    finite_non_negative_score(((score - min) / (max - min)).clamp(0.0, 1.0))
}

fn validate_rrf_parameters(rrf_k: f32, w_vector: f32, w_bm25: f32) -> Result<(), MemoryError> {
    for (name, value) in [("rrf_k", rrf_k), ("w_vector", w_vector), ("w_bm25", w_bm25)] {
        if !value.is_finite() || value < 0.0 {
            return Err(MemoryError::Invalid(format!(
                "{name} must be finite and non-negative"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizer_splits_ascii_and_cjk_bigrams() {
        let text = "Rust 语言很好用，Apeireth v2.0 架构！";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"v2".to_string()));
        assert!(tokens.contains(&"语言".to_string()));
        assert!(tokens.contains(&"言很".to_string()));
        assert!(tokens.contains(&"架构".to_string()));
    }

    #[test]
    fn bm25_exact_match_and_ranking() {
        let mut idx = Bm25Index::new(Bm25Config::default());
        idx.insert("doc1", "深度学习模型训练指南与 GPU 优化");
        idx.insert("doc2", "Rust 异步并发编程与系统架构设计");
        idx.insert("doc3", "Rust 深度内存管理与生命周期");

        let hits = idx.search("Rust 编程", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "doc2");

        let hits_dl = idx.search("GPU 优化", 5);
        assert_eq!(hits_dl[0].id, "doc1");
    }

    #[test]
    fn hybrid_rrf_combines_vector_and_bm25() {
        let mut engine = HybridSearchEngine::new(3).unwrap();
        // doc1: 强语义向量接近 [1.0, 0.0, 0.0]
        engine
            .insert("doc1", "普通文本一条", Some(vec![1.0, 0.0, 0.0]))
            .unwrap();
        // doc2: 精准关键词命中 "独特关键词"
        engine
            .insert("doc2", "包含独特关键词的内容", Some(vec![0.0, 1.0, 0.0]))
            .unwrap();

        let hits = engine
            .search_rrf("独特关键词", Some(&[1.0, 0.0, 0.0]), 5, 60.0, 1.0, 1.0)
            .unwrap();

        assert_eq!(hits.len(), 2);
        // 两者均有一路第一名，由于权重相同，均获得有效 RRF 评分
        assert!(hits[0].score > 0.0);
        assert!(hits[1].score > 0.0);
    }

    #[test]
    fn bm25_ties_are_id_sorted_for_ascii_cjk_and_repeated_queries() {
        let mut ascii = Bm25Index::new(Bm25Config::default());
        ascii.insert("zulu", "rust retrieval");
        ascii.insert("alpha", "rust retrieval");
        ascii.insert("empty", "");

        let expected_ascii = vec!["alpha", "zulu"];
        for _ in 0..100 {
            let actual: Vec<_> = ascii
                .search("rust", 10)
                .into_iter()
                .map(|hit| hit.id)
                .collect();
            assert_eq!(actual, expected_ascii);
        }

        let mut cjk = Bm25Index::new(Bm25Config::default());
        cjk.insert("zulu", "语言模型");
        cjk.insert("alpha", "语言模型");
        for _ in 0..100 {
            let actual: Vec<_> = cjk
                .search("语言", 10)
                .into_iter()
                .map(|hit| hit.id)
                .collect();
            assert_eq!(actual, expected_ascii);
        }
    }

    #[test]
    fn rrf_ties_are_id_sorted_and_replay_exactly() {
        let mut engine = HybridSearchEngine::new(2).unwrap();
        // Vector rank: alpha then beta.  BM25 rank: beta then alpha.
        // Equal weights make the two RRF sums exactly the same pair of terms.
        engine
            .insert("alpha", "needle", Some(vec![1.0, 0.0]))
            .unwrap();
        engine
            .insert("beta", "needle needle", Some(vec![0.0, 1.0]))
            .unwrap();

        let expected = vec!["alpha", "beta"];
        for _ in 0..100 {
            let hits = engine
                .search_rrf("needle", Some(&[1.0, 0.0]), 2, 60.0, 1.0, 1.0)
                .unwrap();
            let actual: Vec<_> = hits.iter().map(|hit| hit.id.as_str()).collect();
            assert_eq!(actual, expected);
            assert_eq!(hits[0].vector_rank, Some(1));
            assert_eq!(hits[0].bm25_rank, Some(2));
            assert_eq!(hits[1].vector_rank, Some(2));
            assert_eq!(hits[1].bm25_rank, Some(1));
            assert_eq!(hits[0].score, hits[1].score);
        }
    }

    #[test]
    fn absent_semantic_channel_is_lexical_only_but_invalid_vectors_are_errors() {
        let mut engine = HybridSearchEngine::new(2).unwrap();
        engine
            .insert("lexical", "needle", Some(vec![1.0, 0.0]))
            .unwrap();

        let lexical_only = engine
            .search_rrf("needle", None, 10, 60.0, 1.0, 1.0)
            .unwrap();
        assert_eq!(lexical_only.len(), 1);
        assert_eq!(lexical_only[0].id, "lexical");
        assert_eq!(lexical_only[0].vector_rank, None);

        let wrong_dimension = engine
            .search_rrf("needle", Some(&[1.0]), 10, 60.0, 1.0, 1.0)
            .unwrap_err();
        assert!(matches!(
            wrong_dimension,
            MemoryError::Invalid(message) if message.contains("dimension mismatch")
        ));

        let nan_vector = engine
            .search_rrf("needle", Some(&[f32::NAN, 0.0]), 10, 60.0, 1.0, 1.0)
            .unwrap_err();
        assert!(matches!(nan_vector, MemoryError::Invalid(_)));
        assert!(matches!(
            engine.search_rrf("needle", None, 10, f32::NAN, 1.0, 1.0),
            Err(MemoryError::Invalid(_))
        ));
    }

    #[test]
    fn zero_vectors_zero_length_documents_and_nonfinite_bm25_config_are_safe() {
        let mut engine = HybridSearchEngine::new(2).unwrap();
        engine.insert("zero", "", Some(vec![0.0, 0.0])).unwrap();
        let zero_query = engine
            .search_rrf("", Some(&[0.0, 0.0]), 1, 60.0, 1.0, 1.0)
            .unwrap();
        assert_eq!(zero_query.len(), 1);
        assert!(zero_query[0].score.is_finite());

        let mut bm25 = Bm25Index::new(Bm25Config {
            k1: f32::NAN,
            b: f32::NAN,
        });
        bm25.insert("zulu", "rust");
        bm25.insert("alpha", "rust");
        bm25.insert("empty", "");
        let hits = bm25.search("rust", 10);
        assert!(hits.iter().all(|hit| hit.score.is_finite()));
        let ids: Vec<_> = hits.into_iter().map(|hit| hit.id).collect();
        assert_eq!(ids, vec!["alpha", "zulu"]);
    }

    #[test]
    fn weighted_fusion_respects_alpha_and_rejects_invalid() {
        let mut engine = HybridSearchEngine::new(3).unwrap();
        engine
            .insert("doc1", "普通文本一条", Some(vec![1.0, 0.0, 0.0]))
            .unwrap();
        engine
            .insert("doc2", "包含独特关键词的内容", Some(vec![0.0, 1.0, 0.0]))
            .unwrap();

        let lexical = engine
            .search_weighted("独特关键词", Some(&[1.0, 0.0, 0.0]), 5, 0.0)
            .unwrap();
        assert_eq!(lexical[0].id, "doc2");

        let semantic = engine
            .search_weighted("独特关键词", Some(&[1.0, 0.0, 0.0]), 5, 1.0)
            .unwrap();
        assert_eq!(semantic[0].id, "doc1");

        assert!(matches!(
            engine.search_weighted("x", None, 5, 1.5),
            Err(MemoryError::Invalid(_))
        ));
        assert!(matches!(
            engine.search_weighted("x", None, 5, f32::NAN),
            Err(MemoryError::Invalid(_))
        ));
    }

    #[test]
    fn weighted_ties_are_id_sorted() {
        let mut engine = HybridSearchEngine::new(2).unwrap();
        engine
            .insert("alpha", "needle", Some(vec![1.0, 0.0]))
            .unwrap();
        engine
            .insert("beta", "needle", Some(vec![1.0, 0.0]))
            .unwrap();
        let hits = engine
            .search_weighted("needle", Some(&[1.0, 0.0]), 2, 0.5)
            .unwrap();
        assert_eq!(hits[0].id, "alpha");
        assert_eq!(hits[1].id, "beta");
        assert_eq!(hits[0].score, hits[1].score);
    }
}
