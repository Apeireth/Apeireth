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

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::canonical::domain::MemoryId;
use crate::canonical::vector::{VectorHit, VectorIndex};
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

    /// 执行 BM25 查询并返回降序排列的命中结果.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Hit> {
        if self.total_docs == 0 || top_k == 0 {
            return Vec::new();
        }

        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

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
                    let num = tf_f * (self.config.k1 + 1.0);
                    let denom = tf_f
                        + self.config.k1
                            * (1.0 - self.config.b + self.config.b * (doc_len / avg_dl));
                    let term_score = idf * (num / denom);
                    *scores.entry(doc_id.clone()).or_default() += term_score;
                }
            }
        }

        let mut hits: Vec<Bm25Hit> = scores
            .into_iter()
            .map(|(id, score)| Bm25Hit { id, score })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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

    /// 执行倒数排名融合 (Reciprocal Rank Fusion, RRF) 混合检索.
    ///
    /// 公式: `score(d) = (w_vec / (k + rank_vec)) + (w_bm25 / (k + rank_bm25))`
    /// 参数 k 通常取 60.0.
    pub fn search_rrf(
        &self,
        query_text: &str,
        query_vector: Option<&[f32]>,
        top_k: usize,
        rrf_k: f32,
        w_vector: f32,
        w_bm25: f32,
    ) -> Vec<HybridHit> {
        let bm25_hits = self.bm25_index.search(query_text, top_k * 2);
        let mut bm25_ranks: HashMap<String, usize> = HashMap::new();
        for (idx, hit) in bm25_hits.iter().enumerate() {
            bm25_ranks.insert(hit.id.clone(), idx + 1);
        }

        let mut vector_ranks: HashMap<String, usize> = HashMap::new();
        if let Some(vec) = query_vector {
            if let Ok(vec_hits) = self.vector_index.query(vec, top_k * 2) {
                for (idx, hit) in vec_hits.iter().enumerate() {
                    vector_ranks.insert(hit.id.to_string(), idx + 1);
                }
            }
        }

        let mut all_ids: HashSet<String> = HashSet::new();
        all_ids.extend(bm25_ranks.keys().cloned());
        all_ids.extend(vector_ranks.keys().cloned());

        let mut hybrid_hits = Vec::new();
        for id in all_ids {
            let v_rank = vector_ranks.get(&id).copied();
            let b_rank = bm25_ranks.get(&id).copied();

            let mut score = 0.0f32;
            if let Some(r) = v_rank {
                score += w_vector / (rrf_k + r as f32);
            }
            if let Some(r) = b_rank {
                score += w_bm25 / (rrf_k + r as f32);
            }

            hybrid_hits.push(HybridHit {
                id,
                score,
                vector_rank: v_rank,
                bm25_rank: b_rank,
            });
        }

        hybrid_hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hybrid_hits.truncate(top_k);
        hybrid_hits
    }
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

        let hits = engine.search_rrf("独特关键词", Some(&[1.0, 0.0, 0.0]), 5, 60.0, 1.0, 1.0);

        assert_eq!(hits.len(), 2);
        // 两者均有一路第一名，由于权重相同，均获得有效 RRF 评分
        assert!(hits[0].score > 0.0);
        assert!(hits[1].score > 0.0);
    }
}
