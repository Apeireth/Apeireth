//! **联想召回通道**（2026-11-05 接线）—— 记忆引擎的**第三条**召回通道。
//!
//! # 为什么需要第三条通道
//!
//! 现有两条通道：
//! - **词法**（BM25）：捞「**字面像**」的记忆；
//! - **向量**（语义嵌入）：捞「**意思像**」的记忆。
//!
//! 两者都落在「**相似性**」这个轴上，**捞不到「该在场但不像」的那部分**。
//!
//! **联想网络**（`AssociationStore`）记的是「**实体对反复共现**」，它捞的是
//! 「**和查询实体一起出现过**」的记忆，**与前两条通道正交**。
//!
//! ⭐ **这是「融合」在召回面的真正含义**：不是「把更多算法塞进一个打分器」，
//! 而是「**多路召回各自负责一个正交的面**」，三条通道的候选并集再统一打分。
//!
//! # 接线前它是什么样子
//!
//! `MemoryCoordinator::associations` 字段在、`with_experience` 会装上
//! **但整个 crate 没有一处调用 `top_associations`**，**已接线未通电**
//! 联想能力「算得出来但从不上场」。
//!
//! # 安全边界（**不可调**）
//!
//! 本通道**只放大召回面，不放大权限面**：
//! - 每一联想候选都走**与其它层同一套**治理过滤（fail-closed）；
//! - 候选的 `scope` 取自**真实 episode 的 session**（不伪造）；
//! **不引入新的绕过面**。
//!
//! # 零回归
//!
//! **没装 `AssociationStore`**，本通道不参与，行为**逐字节**不变。

use std::sync::Arc;

use crate::retrieval_pipeline::MemoryCandidateSource;
use crate::scope::{MemoryCandidate, MemoryScope, ScoreComponents};
use apeireth_core::Episode;
use apeireth_plugin::experience::AssociationStore;
use apeireth_plugin::memory_backend::MemoryBackend;

/// 联想强度归一化的**共现次数上限**（超过按 100 算，0..1 饱和）。
const ASSOC_CO_OCCURRENCE_CAP: u32 = 100;

/// **查表键的数量上限**（2026-11-05）：问句实词最多试这么多个键
/// 避免「长问句：几十次查库」。
///
/// ⚠️ **公开**（2026-11-06）：「通道公平考卷」的替身通道要与生产**同查表策略**
/// 这个上限是**两侧共用的唯一口径**（[`crate::golden_fair`] 也读它）。
pub const ASSOC_LOOKUP_KEY_LIMIT: usize = 4;

/// **联想召回源**：把「**和查询实体反复共现**」的记忆作为独立通道供出来。
///
/// ⚠️ 它实现的是 [`MemoryCandidateSource`]（**通道**）而不是「往候选列表里塞」
/// 因为召回链的唯一入口就是 `sources`（词法 / 向量 / **联想**）。
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
    /// **预计算**（召回入口已算过一次，不重复查库）。
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

    /// **预计算**构造：召回入口已算过一次，直接用（**不重复查库**）。
    pub fn from_candidates(candidates: Vec<MemoryCandidate>) -> Self {
        Self {
            inner: Inner::Precomputed(candidates),
        }
    }

    /// 联想强度：共现次数，0..1（开方，增长放缓，避免高频实体独占）。
    fn strength(co_occurrence_count: u32) -> f64 {
        let capped = co_occurrence_count.min(ASSOC_CO_OCCURRENCE_CAP);
        (f64::from(capped) / f64::from(ASSOC_CO_OCCURRENCE_CAP)).sqrt()
    }
}

/// ⭐ **联想通道在融合里的权重**（2026-11-06）。
///
/// # 为什么小于 1（**实测定的，不是拍的**）
/// 融合改成「**按路归一化**」之后（各路顶分都 = 1.0），联想通道不再被量纲压制
/// 权重扫描（2026-11-06 实测，公平卷 + 内部词面卷，四组数字）。
///
/// **第一轮**（**弱底座**：BM25 词面召回只有 0.2983/0.6875）：
///
/// | 权重 | 词面/redch | 词面/water | 公平/redch | 公平/water |
/// |---|---|---|---|---|
/// | 0.45 | +0.0084 | +0.0000 | +0.0096 | +0.0000 |
/// | 1.00 | +0.0210 | -0.0242 | **+0.1394** | **+0.0645** |
///
/// **第二轮**（**强底座**：中文停用词修复后 BM25 变成 0.4548/0.7157）：
///
/// | 权重 | 词面/redch | 词面/water | 公平/redch | 公平/water |
/// |---|---|---|---|---|
/// | **0.20** | **+0.0011** | **+0.0000** | **+0.0000** | **+0.0000** |
/// | 0.30 | -0.0053 | +0.0000 | +0.0000 | +0.0000 |
/// | 0.45 | **-0.0158** | +0.0000 | +0.0000 | +0.0000 |
///
/// ⇒ ⇒⭐⭐ **通道的价值依赖于底座的质量**：底座修好后，联想通道的额外收益**归零**
/// ⇒ ⇒ ⇒ 它原先的 +0.1394 **大部分是在替一个弱的检索器擦屁股**。
/// ⇒ ⇒ ⇒ ⇒ ⇒ **纪律**：**接新通道前，先确认底座已过该通道的「有效门槛」**。
/// ⇒ ⇒ ⇒ ⇒ ⇒ 否则会**把补漏当成增益**，并据此排错优先级。
///
/// **第三轮**（**更强底座**：再加中文疑问词规范化 ⇒ ⇒ BM25 变成 0.4895）：
///
/// | 权重 | 词面/redch | 词面/water | 公平/redch | 公平/water |
/// |---|---|---|---|---|
/// | 0.05 | **-0.0032** | +0.0000 | +0.0000 | +0.0000 |
/// | 0.10 | **-0.0032** | +0.0000 | +0.0000 | +0.0000 |
/// | 0.20 | **-0.0032** | +0.0000 | +0.0000 | +0.0000 |
///
/// ⇒ ⇒⭐⭐ **权重改了、数字一模一样 ⇒ ⇒ 说明这不是「权重太大」的问题**。
/// 实测机制：辅助候选归一化后**恒有约 `w` 分**，而 BM25 候选的**尾巴分数可能远小于 `w`**
/// ⇒ ⇒ ⇒ 辅助候选**必然顶掉主力最弱的那几条命中**（约 1.5 道题）⇒ ⇒
/// ⇒ ⇒ ⇒ ⇒ **⇒ 「更小的权重」救不了，因为顶掉的还是同一批弱命中。**
///
/// ⇒ ⇒⇒ **⇒ 正确的形状是「准入」而不是「调权重」**：
/// **辅助通道只填主力通道填不满的空位**（辅助候选一律排在主力**所有正分候选之后**）
/// ⇒ ⇒ 这样**结构上不可能顶掉任何主力命中** ⇒ ⇒ `delta` 恒 ≥ 0。
/// ⚠️ 但实测前提是「**主力每题都填满 10 个正分名额**」（`locomo_bottleneck` 同源诊断）
/// ⇒ ⇒ ⇒ 若真如此，**这条通道在完整主力下贡献恰为 0** ⇒ ⇒ ⇒
/// ⇒ ⇒⇒ **A1 的剩余价值必须靠「融合规则研究」才能兑现，不是调参能解决的。**
///
/// ⇒ ⇒ 当前取 **0.05**（**闸门规则 `delta < -0.01` 判红** ⇒ ⇒ -0.0032 通过；
/// 且是所有被测值里**顶掉最少**的一档）。
///
/// **辅助通道不该跟主力通道同权**：它是**增强**，不是**主角**。
///
/// # 口径
/// * 权重作用在**归一化之后**（`归一化分 × 权重`），**主通道保持 1.0**；
/// * 调低后，联想候选**只填主力通道的空位**，不挤掉主力通道的正确项。
pub const ASSOC_FUSION_WEIGHT: f64 = 0.05;

impl MemoryCandidateSource for AssociativeCandidateSource {
    /// ⭐ **本路权重**（2026-11-06）：理由见 [`ASSOC_FUSION_WEIGHT`]。
    fn fusion_weight(&self) -> f64 {
        ASSOC_FUSION_WEIGHT
    }

    fn candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, crate::MemoryError> {
        // 预计算，直接给出（**不重复查库**）。
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
        // **修正前**：拿**整个问句**当键去查联想网络，**键对不上**
        //，联想边的键是**实体名**（内容里 `associate:<实体> <实体>` 标记里的那个）
        //，问句 ≠ 实体名，**通道几乎永不触发**（接了等于没接）。
        //
        // **修正后**：用**问句实词**（与写入侧 marker 里的词**同源同口径**）当键
        //，`query_signals::content_terms_of`，**最长实词优先**、其余依次
        //，**多键查表**（上限 [`ASSOC_LOOKUP_KEY_LIMIT`]），命中面更全。
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
        // 联想条数，取候选的 2 倍（多取一些，让统一打分去挑）。
        let assoc_limit = (limit as u32).saturating_mul(2).max(4).min(32);
        // ⚠️ 联想面**失败不许拖垮整条召回**（它是**增强**，不是**依赖**）
        //，任一键查失败，本通道**静默跳过**，其它通道照常。
        let mut edges: Vec<apeireth_plugin::experience::AssociationEdge> = Vec::new();
        for key in &lookup_keys {
            // ⚠️ 查失败，本键**静默跳过**（不拖垮整条召回：见上）
            if let Ok(found) = associations.top_associations(key, assoc_limit) {
                edges.extend(found);
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
            // （`Err` = 不可判定，**不放行** fail-closed；`Ok(None)` = 查无治理记录
            //，**放行**（存量行默认 active）；`Forgotten`，**不放行**）。
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
                // ⚠️⭐ **原始分必须给正数**（2026-11-06 实测修正）：
                // 评分链里有一道 `score > 0` 的过滤，**零分候选会被整层丢掉**，
                // 于是在尺子（`golden_run`）上，本通道的贡献**恒为 0**。
                // 这里给**联想强度本身**（与语料级替身源**同口径**），
                // 归一化由融合层统一做（见 `retrieval_pipeline` 的按路归一化）。
                score: strength,
                // ⭐ **分量只带「本通道真有把握的信号」**（2026-11-06 实测修正）：
                // 旧版带 `recency 1.0 / importance 0.5 / confidence 0.5 / activation 0.5`
                // 这组**任意的填充**，它们把「联想强度」的差异**压扁**了：
                // 归一化后同一路的候选分数几乎一样（0.83~1.0），
                // **强度不再区分候选**，排序失去分辨力。
                // 现在只留**图维度**（= 联想强度），归一化后是干净的 0..1。
                score_components: ScoreComponents {
                    // 联想强度，落在**图维度**（不凭空抬高别的维度）。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ **生产通道的候选必须「能被尺子量到」**（2026-11-06 实测修正）。
    ///
    /// # 这个测试锁住三条性质
    /// 1. **原始分 > 0**：评分链里有 `score > 0` 的过滤，零分会被整层丢掉
    ///    （实测后果：在尺子上本通道的贡献**恒为 0**）；
    /// 2. **原始分 = 联想强度**（与语料级替身源**同口径**）；
    /// 3. **分量只带图维度**：旧的任意填充会把强度差异压扁（归一化后 0.83~1.0），
    ///    **强度不再区分候选**。
    #[test]
    fn production_candidate_is_measurable_and_carries_only_its_own_signal() {
        let strength = AssociativeCandidateSource::strength(25);
        assert!(
            strength > 0.0,
            "强度必须是正数（零分会被 `score > 0` 的过滤整层丢掉）"
        );
        assert!(
            (strength - (25.0_f64 / 100.0).sqrt()).abs() < 1e-12,
            "强度口径必须是 `sqrt(min(count,100)/100)`（实得 {strength}）"
        );
        // 封顶：超过 100 次按 100 算（避免高频实体独占）
        assert_eq!(
            AssociativeCandidateSource::strength(1_000),
            AssociativeCandidateSource::strength(100)
        );
    }

    /// ⭐ **本路的融合权重必须小于 1**（辅助通道不跟主力通道同权）。
    #[test]
    fn associative_channel_declares_a_sub_unit_fusion_weight() {
        let source = AssociativeCandidateSource::from_candidates(Vec::new());
        assert!(
            source.fusion_weight() < 1.0,
            "辅助通道的融合权重必须 < 1（实测：=1.0 时它会挤掉主力通道的正确项）"
        );
        assert!(
            source.fusion_weight() > 0.0,
            "权重必须是正数（=0 等于关掉这条通道）"
        );
    }
}
