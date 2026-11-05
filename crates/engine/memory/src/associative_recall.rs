//! **联想召回通道**（2026-11-05 接线）—— 记忆引擎的**第三条**召回通道。
//!
//! # 为什么需要第三条通道
//!
//! 现有两条通道：
//! - **词法**（BM25）：捞「**字面像**」的记忆；
//! - **向量**（语义嵌入）：捞「**意思像**」的记忆。
//!
//! ⇒ ⇒ 两者都落在「**相似性**」这个轴上 ⇒ ⇒ **捞不到「该在场但不像」的那部分**。
//!
//! **联想网络**（`AssociationStore`）记的是「**实体对反复共现**」⇒ ⇒ 它捞的是
//! 「**和查询实体一起出现过**」的记忆 ⇒ ⇒ ⇒ **与前两条通道正交**。
//!
//! ⭐ **这是「融合」在召回面的真正含义**：不是「把更多算法塞进一个打分器」，
//! 而是「**多路召回各自负责一个正交的面**」⇒ ⇒ 三条通道的候选并集再统一打分。
//!
//! # 接线前它是什么样子
//!
//! `MemoryCoordinator::associations` 字段在、`with_experience` 会装上
//! ⇒ ⇒ **但整个 crate 没有一处调用 `top_associations`** ⇒ ⇒ **已接线未通电**
//! ⇒ ⇒ 联想能力「算得出来但从不上场」。
//!
//! # 安全边界（**不可调**）
//!
//! 本通道**只放大召回面，不放大权限面**：
//! - 每一联想候选都走**与其它层同一套**治理过滤（fail-closed）；
//! - 候选的 `scope` 取自**真实 episode 的 session**（不伪造）；
//! ⇒ ⇒ ⇒ **不引入新的绕过面**。
//!
//! # 零回归
//!
//! **没装 `AssociationStore`** ⇒ ⇒ 本通道不参与 ⇒ ⇒ 行为**逐字节**不变。

use std::sync::Arc;

use crate::retrieval_pipeline::MemoryCandidateSource;
use crate::scope::{MemoryCandidate, MemoryScope, ScoreComponents};
use apeireth_core::Episode;
use apeireth_plugin::experience::AssociationStore;
use apeireth_plugin::memory_backend::MemoryBackend;

/// 联想强度归一化的**共现次数上限**（超过按 100 算 ⇒ ⇒ 0..1 饱和）。
const ASSOC_CO_OCCURRENCE_CAP: u32 = 100;

/// **查表键的数量上限**（2026-11-05）：问句实词最多试这么多个键
/// ⇒ ⇒ 避免「长问句 ⇒ 几十次查库」。
const ASSOC_LOOKUP_KEY_LIMIT: usize = 4;

/// **联想召回源**：把「**和查询实体反复共现**」的记忆作为独立通道供出来。
///
/// ⚠️ 它实现的是 [`MemoryCandidateSource`]（**通道**）而不是「往候选列表里塞」
/// ⇒ ⇒ 因为召回链的唯一入口就是 `sources`（词法 / 向量 / **联想**）。
pub struct AssociativeCandidateSource {
    inner: Inner,
}

enum Inner {
    /// **活源**：每次召回现查（联想网络 + 后端 + 治理）。
    Live {
        associations: Arc<dyn AssociationStore>,
        backend: Arc<dyn MemoryBackend>,
        governance: Arc<dyn crate::MemoryGovernanceStore>,
    },
    /// **预计算**（召回入口已算过一次 ⇒ ⇒ 不重复查库）。
    Precomputed(Vec<MemoryCandidate>),
}

impl AssociativeCandidateSource {
    /// 构造联想召回源。
    pub fn new(
        associations: Arc<dyn AssociationStore>,
        backend: Arc<dyn MemoryBackend>,
        governance: Arc<dyn crate::MemoryGovernanceStore>,
    ) -> Self {
        Self {
            inner: Inner::Live {
                associations,
                backend,
                governance,
            },
        }
    }

    /// **预计算**构造：召回入口已算过一次 ⇒ ⇒ 直接用（**不重复查库**）。
    pub fn from_candidates(candidates: Vec<MemoryCandidate>) -> Self {
        Self {
            inner: Inner::Precomputed(candidates),
        }
    }

    /// 联想强度：共现次数 ⇒ ⇒ 0..1（开方 ⇒ ⇒ 增长放缓，避免高频实体独占）。
    fn strength(co_occurrence_count: u32) -> f64 {
        let capped = co_occurrence_count.min(ASSOC_CO_OCCURRENCE_CAP);
        (f64::from(capped) / f64::from(ASSOC_CO_OCCURRENCE_CAP)).sqrt()
    }
}

impl MemoryCandidateSource for AssociativeCandidateSource {
    fn candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, crate::MemoryError> {
        // 预计算 ⇒ ⇒ 直接给出（**不重复查库**）。
        if let Inner::Precomputed(candidates) = &self.inner {
            return Ok(candidates.clone());
        }
        let (associations, backend, governance) = match &self.inner {
            Inner::Live {
                associations,
                backend,
                governance,
            } => (associations, backend, governance),
            Inner::Precomputed(_) => unreachable!("上面已返回"),
        };
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        // ⭐ **查表键 = 问句里的「实词」**（2026-11-05 修正）
        //
        // **修正前**：拿**整个问句**当键去查联想网络 ⇒ ⇒ **键对不上**
        // ⇒ ⇒ 联想边的键是**实体名**（内容里 `associate:<实体> <实体>` 标记里的那个）
        // ⇒ ⇒ 问句 ≠ 实体名 ⇒ ⇒ ⇒ **通道几乎永不触发**（接了等于没接）。
        //
        // **修正后**：用**问句实词**（与写入侧 marker 里的词**同源同口径**）当键
        // ⇒ ⇒ `query_signals::content_terms_of` ⇒ ⇒ **最长实词优先**、其余依次
        // ⇒ ⇒ **多键查表**（上限 [`ASSOC_LOOKUP_KEY_LIMIT`]）⇒ ⇒ 命中面更全。
        let mut lookup_keys: Vec<String> = Vec::new();
        if let Some(subject) = crate::query_signals::subject_of_query(query) {
            lookup_keys.push(subject);
        }
        for term in crate::query_signals::content_terms_of(query) {
            if lookup_keys.len() >= ASSOC_LOOKUP_KEY_LIMIT {
                break;
            }
            if !lookup_keys.iter().any(|k| k == &term) {
                lookup_keys.push(term);
            }
        }
        if lookup_keys.is_empty() {
            return Ok(Vec::new());
        }
        // 联想条数 ⇒ ⇒ 取候选的 2 倍（多取一些，让统一打分去挑）。
        let assoc_limit = (limit as u32).saturating_mul(2).max(4).min(32);
        // ⚠️ 联想面**失败不许拖垮整条召回**（它是**增强**，不是**依赖**）
        // ⇒ ⇒ 任一键查失败 ⇒ ⇒ 本通道**静默跳过** ⇒ ⇒ 其它通道照常。
        let mut edges: Vec<apeireth_plugin::experience::AssociationEdge> = Vec::new();
        for key in &lookup_keys {
            match associations.top_associations(key, assoc_limit) {
                Ok(found) => edges.extend(found),
                Err(_) => continue,
            }
        }

        let mut out: Vec<MemoryCandidate> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for edge in edges {
            let episode_id = edge.last_seen_episode_id.trim();
            if episode_id.is_empty() || seen.iter().any(|s| s == episode_id) {
                continue;
            }
            seen.push(episode_id.to_string());
            let episode: Option<Episode> = match backend.get_episode(episode_id) {
                Ok(episode) => episode,
                Err(_) => continue,
            };
            let Some(episode) = episode else { continue };

            // ⚠️ **治理过滤：与其它层同一套三态口径**
            // （`Err` = 不可判定 ⇒ ⇒ **不放行** fail-closed；`Ok(None)` = 查无治理记录
            //   ⇒ ⇒ **放行**（存量行默认 active）；`Forgotten` ⇒ ⇒ **不放行**）。
            match governance.get_governed(&episode.id) {
                Err(_) => continue,
                Ok(Some(governed)) => {
                    if governed.status != crate::MemoryGovernanceStatus::Active {
                        continue;
                    }
                }
                Ok(None) => {}
            }

            let strength = Self::strength(edge.co_occurrence_count);
            let session_id = episode.session_id.clone();
            out.push(MemoryCandidate {
                id: format!("assoc:{episode_id}"),
                layer: "relational".to_string(),
                scope: MemoryScope::Session {
                    session_id: session_id.clone(),
                },
                content: episode.content.clone(),
                score: 0.0,
                score_components: ScoreComponents {
                    recency: 1.0,
                    importance: 0.5,
                    confidence: 0.5,
                    activation: 0.5,
                    // 联想强度 ⇒ ⇒ 落在**图维度**（不凭空抬高别的维度）。
                    graph: strength,
                    ..Default::default()
                },
                provenance: crate::scope::MemoryProvenance {
                    source: "association".to_string(),
                    source_session: Some(session_id),
                    ..Default::default()
                },
            });
        }
        Ok(out)
    }
}
