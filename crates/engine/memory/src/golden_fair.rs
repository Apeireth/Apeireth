//! `apeireth-memory::golden_fair` —— **通道公平考卷**（零人工标注）。
//!
//! # 为什么有这个模块
//!
//! 「不许不涨」的尺子（[`crate::golden_run::run_with_extra_sources`] + 融合通道闸门）
//! 让**同一份考卷跑两遍**（单路 BM25 vs 多路）⇒ ⇒ 量出**每个通道的收益**。
//!
//! ⚠️ **但内部卷测不出联想通道**：内部卷的题是「**从记录里挖出来的**」
//! （主体 = 该记录的最高频实体 ⇒ [`crate::golden_mining::Prepared::subject`]）
//! ⇒ ⇒ **主体必在正文里、且是正文里最突出的那个** ⇒ ⇒ **词面通道天然占优**
//! ⇒ ⇒ 联想通道（「**该在场但词面弱**」那一路）在内部卷上**测不出收益**。
//!
//! ⇒ 本模块造**第二份卷子**：**通道公平卷**（`fair exam`）——
//! **同一套题面模板、同一批记录、同一个 runner**，**只把主体换掉**。
//!
//! # 公平卷的主体是什么（**这是全模块的核心**）
//!
//! 对一条记录 `A`：
//!
//! 1. **内部卷会问的主体** = `A` 的最高频/最长实体（记作 `s(A)`）——**词面最强**；
//! 2. **联想实体集** `E(A)` = `A` 正文里**最先出现的 N 个实词**（`N = 4`）
//!    ⇒ ⇒ **与联想通道的查表键同源**（通道就是按「记录开头的实体」建边的）；
//! 3. **公平卷的主体** = `E(A)` 里**那个不是 `s(A)` 的实体** `t`
//!    ⇒ ⇒ **内部卷绝不会问它**（内部卷只问 `s(A)`）⇒ ⇒ **两卷问的是不同的词**；
//! 4. **`t` 必须是「跨记录桥」**：`t` 同时出现在 **≥2 条记录**的实体集里
//!    ⇒ ⇒ **这道题问的实体真的连着两条记录**（不是只在一条记录里冒个泡）。
//!
//! ⇒ ⇒ **两卷的唯一差别 = 主体是谁** ⇒ ⇒ **同一个 runner、同一把尺子、同一个 k**
//! ⇒ ⇒ **可直接对比**（这正是「判据单一」的含义）。
//!
//! # ⭐ 为什么这叫「公平」
//!
//! | | 内部卷（词面卷） | 公平卷 |
//! |---|---|---|
//! | 主体 | `s(A)` = 正文里**最突出**的实体 | `t` = `E(A)` 里**次级**的实体 |
//! | 词面信号 | **强**（主体高频 ⇒ BM25 一猜中） | **弱**（次级实体 ⇒ 稀释 ⇒ BM25 排序下沉） |
//! | 联想信号 | 主体多半**不在** `E(A)` ⇒ 通道够不着 | 主体**就在** `E(A)` ⇒ 通道够得着 |
//!
//! ⇒ ⇒ 公平卷**不是**给通道「放水」：它只是把**词面那道题的偏置拆掉**
//! ⇒ ⇒ **两卷都跑、两个 delta 都报**（词面卷管「不许有害」，公平卷管「到底有没有用」）。
//!
//! # ⚠️ **口径限制（必须随数字一起读）**
//!
//! 1. **「词面完全不像但该在场」对联想通道结构上不可得**：
//!    联想通道返回的是 `edge.last_seen_episode_id`（**边发生的那个 episode**）
//!    ⇒ ⇒ 该 episode **必然含边的两个端点** ⇒ ⇒ **必然含查询词**
//!    （见 [`crate::associative_recall`]）⇒ ⇒ 公平卷的形态只能是
//!    **「词面**弱**但该在场」**，不是「词面**不像**」。
//! 2. **语料级联想边是「替身」**：生产的真来源是内容里的 `associate:<实体> <实体>` 标记
//!    （[`apeireth_plugin::experience`]）⇒ ⇒ 小说语料没有这种标记
//!    ⇒ ⇒ 这里用**同记录内实词共现**当替身 ⇒ ⇒ **替身比生产宽松**
//!    （生产一个实体对只留「最后一次出现的 episode」，替身留**全部**出现过的记录）
//!    ⇒ ⇒ ⇒ **公平卷上的 delta 是生产收益的**上界**，不是生产收益本身**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-11-06）
//!
//! - **IMPLEMENTED** ✔：类型、生成器、替身联想网、测试齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产 0 调用点（只有评估闸门在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：尚未在真实卷子数据上跑过（待闸门首跑）。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::experience::{AssociationEdge, AssociationStore};
use crate::golden_mining::MineRecord;
use crate::golden_run::IndexRecord;
use crate::golden_set::GoldenCase;
use crate::retrieval_pipeline::MemoryCandidateSource;
use crate::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

/// **联想实体键的数量上限**（每条记录取正文里最先出现的这么多个实词当「实体」）。
///
/// # 为什么是 4
///
/// ⚠️ **必须与联想通道的替身建边口径一致** ⇒ ⇒ 否则「卷子问的实体」与「通道查的键」不是同一套
/// ⇒ ⇒ 公平卷就白造了（通道够不着答案记录）。
/// ⇒ ⇒ 这个数是**两侧共用的唯一常量**（本模块 + 替身建边都读它）。
pub const ASSOC_ENTITY_LIMIT: usize = 4;

/// **联想强度的共现次数上限**（超过按 100 算 ⇒ ⇒ 0..1 饱和）。
///
/// ⚠️ 与生产 [`crate::associative_recall::AssociativeCandidateSource`] **逐字同值**
/// ⇒ ⇒ 替身通道算出来的分数与生产**同一个量纲**。
const ASSOC_CO_OCCURRENCE_CAP: u32 = 100;

/// **联想强度**：共现次数 ⇒ ⇒ 0..1（开方 ⇒ ⇒ 增长放缓，避免高频实体独占）。
///
/// ⚠️ 与生产**逐字同公式**（`sqrt(count / cap)`）。
fn assoc_strength(co_occurrence_count: u32) -> f64 {
    let capped = co_occurrence_count.min(ASSOC_CO_OCCURRENCE_CAP);
    (f64::from(capped) / f64::from(ASSOC_CO_OCCURRENCE_CAP)).sqrt()
}

/// 一条记录的**联想实体键**（= 正文里最先出现的 [`ASSOC_ENTITY_LIMIT`] 个实词）。
///
/// # 口径
///
/// * 实词取 [`crate::query_signals::content_terms_of`]（与联想通道的**查表键同源**）；
/// * **按出现顺序**（不是按频次）⇒ ⇒ 这是「**这条记录开篇在讲谁**」的代理；
/// * 去重保序 ⇒ ⇒ 同一个词不会占两个名额。
pub fn association_entities(content: &str) -> Vec<String> {
    crate::query_signals::content_terms_of(content)
        .into_iter()
        .take(ASSOC_ENTITY_LIMIT)
        .collect()
}

/// **语料级联想网**（**替身**）：实体对 → 共现次数 + 出现过的记录。
///
/// # 与生产的差别（**必须随数字一起读**）
///
/// | | 生产（[`AssociationStore`]） | 本替身 |
/// |---|---|---|
/// | 边的来源 | 内容里的 `associate:<实体> <实体>` 标记 | 同一条记录内的实词共现 |
/// | 一个实体对的边数 | **1 条**（聚合计数 + 最近一次 episode） | **N 条**（每个出现过的记录一条） |
/// | `co_occurrence_count` | 该实体对**共现过几次** | **同**（语料级共现次数） |
///
/// ⇒ ⇒ **替身的可达面比生产宽**（生产一个实体对只留「最后一次出现的 episode」）
/// ⇒ ⇒ ⇒ **在替身上量到的 delta 是生产收益的**上界**。
///
/// # 确定性
///
/// 边的顺序 = `(共现次数降序, from, to, episode_id)` ⇒ 同输入逐字相等。
#[derive(Debug, Default)]
pub struct CorpusAssociations {
    inner: Mutex<Inner>,
}

/// 联想网的内部状态（**一把锁** ⇒ 与生产的 SQLite 写入同语义：原子累加）。
#[derive(Debug, Default)]
struct Inner {
    /// 边：`(from, to, episode_id)` → 记录次数（替身里恒为 1）。
    edges: BTreeMap<(String, String, String), u32>,
    /// 实体对：`(from, to)` → **语料级共现次数**（= 有多少条记录里这对实体一起出现）。
    pair_counts: BTreeMap<(String, String), u32>,
    /// 实体 → 出现过它的记录 id 集合（建桥判定用）。
    entity_records: BTreeMap<String, BTreeSet<String>>,
}

impl CorpusAssociations {
    /// 用一批记录建联想网（**替身建边**：同记录内的实词两两共现）。
    pub fn build(records: &[MineRecord]) -> Self {
        let store = Self::default();
        for record in records {
            let entities = association_entities(&record.content);
            for left in 0..entities.len() {
                for right in (left + 1)..entities.len() {
                    let _ =
                        store.record_cooccurrence(&entities[left], &entities[right], &record.id);
                }
            }
        }
        store
    }

    /// 这个实体出现在**几条记录**的实体集里（= 它是不是「跨记录桥」）。
    pub fn entity_record_count(&self, entity: &str) -> usize {
        self.inner
            .lock()
            .expect("联想网互斥锁")
            .entity_records
            .get(entity)
            .map_or(0, BTreeSet::len)
    }

    /// 这个实体桥接的记录 id 集合（**确定性**：按 id 升序）。
    pub fn entity_records(&self, entity: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("联想网互斥锁")
            .entity_records
            .get(entity)
            .map(|records| records.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl AssociationStore for CorpusAssociations {
    fn record_cooccurrence(
        &self,
        from: &str,
        to: &str,
        episode_id: &str,
    ) -> apeireth_plugin::CapabilityResult<()> {
        // ⚠️ **无向对规范化**（`from`/`to` 谁大谁小不重要 ⇒ 生产的查表是
        // `WHERE from_entity = ?1 OR to_entity = ?1` ⇒ 边是**双向**的）。
        let (left, right) = if from <= to {
            (from.to_string(), to.to_string())
        } else {
            (to.to_string(), from.to_string())
        };
        let mut inner = self.inner.lock().expect("联想网互斥锁");
        *inner
            .edges
            .entry((left.clone(), right.clone(), episode_id.to_string()))
            .or_insert(0) += 1;
        *inner
            .pair_counts
            .entry((left.clone(), right.clone()))
            .or_insert(0) += 1;
        inner
            .entity_records
            .entry(left)
            .or_default()
            .insert(episode_id.to_string());
        inner
            .entity_records
            .entry(right)
            .or_default()
            .insert(episode_id.to_string());
        Ok(())
    }

    fn top_associations(
        &self,
        entity: &str,
        limit: u32,
    ) -> apeireth_plugin::CapabilityResult<Vec<AssociationEdge>> {
        let inner = self.inner.lock().expect("联想网互斥锁");
        let mut out: Vec<AssociationEdge> = Vec::new();
        for ((left, right, episode_id), _times) in inner.edges.iter() {
            if left != entity && right != entity {
                continue;
            }
            // ⚠️ **边权 = 该实体对的语料级共现次数**（与生产的 `co_occurrence_count` 同义：
            // 「这两个实体一起出现过几次」）。
            let count = inner
                .pair_counts
                .get(&(left.clone(), right.clone()))
                .copied()
                .unwrap_or(0);
            out.push(AssociationEdge {
                from_entity: left.clone(),
                to_entity: right.clone(),
                co_occurrence_count: count,
                last_seen_episode_id: episode_id.clone(),
                last_seen_at: 0,
            });
        }
        // ⚠️ **排序口径**：共现次数降序 ⇒ 其余按 (from, to, episode_id) 升序 ⇒ 确定性。
        out.sort_by(|left, right| {
            right
                .co_occurrence_count
                .cmp(&left.co_occurrence_count)
                .then_with(|| left.from_entity.cmp(&right.from_entity))
                .then_with(|| left.to_entity.cmp(&right.to_entity))
                .then_with(|| left.last_seen_episode_id.cmp(&right.last_seen_episode_id))
        });
        out.truncate(limit as usize);
        Ok(out)
    }
}

/// **语料级联想召回源**（**替身通道**）：按问句实词查联想网 ⇒ ⇒ 拉出关联记录当候选。
///
/// # 口径（与生产 [`crate::associative_recall::AssociativeCandidateSource`] 对齐）
///
/// * **查表键** = 问句主体 + 问句实词（上限
///   [`crate::associative_recall::ASSOC_LOOKUP_KEY_LIMIT`] 个）⇒ ⇒ **与生产同策略**；
/// * **候选分数** = 联想强度 `sqrt(min(count, 100) / 100)` ⇒ ⇒ **与生产同公式**；
/// * **候选 id** = 记录 id（生产是 `assoc:<episode_id>`，语料侧没有 episode 层 ⇒ 直接用记录 id）。
///
/// ⚠️ **为什么分数必须 > 0**：融合时**零分候选会被当成「没检索到」丢掉**
/// ⇒ ⇒ 替身通道给正分，生产通道靠 `score_components.graph` 参与统一打分。
pub struct CorpusAssociativeSource {
    /// 联想网。
    pub associations: Arc<CorpusAssociations>,
    /// 可检索文本（按 id 查找）。
    pub records: Vec<IndexRecord>,
}

impl MemoryCandidateSource for CorpusAssociativeSource {
    /// ⭐ **本路权重**（2026-11-06）：与生产联想通道**同口径**
    /// ⇒ 替身量到的东西必须与生产一致 ⇒ 否则数字不能信。
    fn fusion_weight(&self) -> f64 {
        crate::associative_recall::ASSOC_FUSION_WEIGHT
    }

    /// ⭐ **与生产同口径**（2026-11-07）：替身也是**填充通道**
    /// ⇒ ⇒ 否则尺子量与生产不同的东西（今天已经踩过一次两套口径不一致的坑）。
    fn is_filler(&self) -> bool {
        true
    }

    fn candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, crate::MemoryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        // ⚠️ **查表键策略 = 生产口径**（主体优先 + 实词补足，上限固定）。
        let mut lookup_keys: Vec<String> = Vec::new();
        if let Some(subject) = crate::query_signals::subject_of_query(query) {
            lookup_keys.push(subject);
        }
        for term in crate::query_signals::content_terms_of(query) {
            if lookup_keys.len() >= crate::associative_recall::ASSOC_LOOKUP_KEY_LIMIT {
                break;
            }
            if !lookup_keys.iter().any(|key| key == &term) {
                lookup_keys.push(term);
            }
        }
        if lookup_keys.is_empty() {
            return Ok(Vec::new());
        }
        let assoc_limit = (limit as u32).saturating_mul(2).max(4).min(32);
        let mut best_by_record: BTreeMap<String, f64> = BTreeMap::new();
        for key in &lookup_keys {
            let Ok(edges) = self.associations.top_associations(key, assoc_limit) else {
                continue;
            };
            for edge in edges {
                let episode_id = edge.last_seen_episode_id.trim();
                if episode_id.is_empty() {
                    continue;
                }
                let strength = assoc_strength(edge.co_occurrence_count);
                let entry = best_by_record.entry(episode_id.to_string()).or_insert(0.0);
                if strength > *entry {
                    *entry = strength;
                }
            }
        }
        let mut out: Vec<MemoryCandidate> = Vec::new();
        for (id, score) in best_by_record {
            let Some(record) = self.records.iter().find(|record| record.id == id) else {
                continue;
            };
            out.push(MemoryCandidate {
                id: record.id.clone(),
                layer: "golden".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score,
                score_components: Default::default(),
                provenance: MemoryProvenance {
                    source: "golden_association".to_string(),
                    source_session: None,
                    source_trace: None,
                    source_request: None,
                },
            });
        }
        // ⭐ **按路归一化**（2026-11-06 实测修正）：**本路顶分 = 1.0**
        //
        // **为什么**：BM25 路把分数按**自己的 max** 归一化（**顶分恒为 1.0**），
        // 而联想路的原始强度只有 0.37~0.51 ⇒ ⇒ **两路量纲不对等**
        // ⇒ ⇒ ⇒ 不归一化 ⇒ ⇒ 联想候选**结构性沉底** ⇒ ⇒ ⇒ **通道贡献恒为 0**
        // ⇒ ⇒ ⇒ 实测：公平卷上通道收益 **delta = 0.0000**（**白接**）。
        //
        // ⚠️ **与生产通道同口径**：生产的联想通道也必须按自己的 max 归一化
        // ⇒ ⇒ 否则**替身量的是假东西** ⇒ ⇒ ⇒ 数字不能信。
        let max_score = out.iter().map(|c| c.score).fold(0.0_f64, f64::max);
        if max_score > 0.0 {
            for candidate in &mut out {
                candidate.score /= max_score;
            }
        }
        // ⚠️ **确定性**：按 (分数降序, id 升序) ⇒ 与生产的「按 id 并集 + 统一打分」可比。
        out.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(out)
    }
}

/// **公平卷生成配置**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FairExamConfig {
    /// 单条记录最多出几道公平卷的题（防刷题）。
    pub max_cases_per_record: usize,
    /// **跨记录桥**的最低记录数（≥2 ⇒ 实体真的连着两条记录）。
    pub min_bridge_records: usize,
}

impl Default for FairExamConfig {
    fn default() -> Self {
        Self {
            max_cases_per_record: 2,
            min_bridge_records: 2,
        }
    }
}

/// **通道公平卷**的题面模板（**与内部卷逐字同模板** ⇒ 两卷可比）。
///
/// ⚠️ 与内部卷 [`crate::golden_mining::mine_keyword`] 的模板**逐字一致**
/// （`关于{主体}，我们当时记了什么？`）⇒ ⇒ **两卷唯一的差别就是主体是谁**。
pub fn fair_query(subject: &str) -> String {
    format!("关于{subject}，我们当时记了什么？")
}

/// 造**通道公平卷**（零人工标注）。
///
/// # 口径（逐条钉死）
///
/// 1. **主体** = 记录的联想实体集 `E(A)` 里**不是内部卷主体 `s(A)`** 的那个词；
/// 2. **跨记录桥** = 该词至少出现在 [`FairExamConfig::min_bridge_records`] 条记录的实体集里；
/// 3. **题面** = [`fair_query`]（与内部卷**逐字同模板**）；
/// 4. **标准答案** = 这条记录（`expect_ids = [A]`）；
/// 5. **答案片段** = `A` 正文里**含该主体**的那一句（**一定是正文的子串**）；
/// 6. **产出确定性** = 按记录顺序（`(时间, id)` 升序）⇒ 主体顺序 ⇒ 同输入逐字相等。
///
/// # 为什么不给「内部卷主体」出题
///
/// 内部卷**已经**问过 `s(A)` 了 ⇒ ⇒ 再问一遍就是**同一道题**（重复 ⇒ 白占名额）
/// ⇒ ⇒ 公平卷**只问内部卷不会问的词** ⇒ ⇒ **两卷问的词不相交** ⇒ ⇒ 两卷是**互补**的。
pub fn build_fair_exam(records: &[MineRecord], config: FairExamConfig) -> Vec<GoldenCase> {
    let associations = CorpusAssociations::build(records);
    // 按 `(时间, id)` 归一 ⇒ 输入顺序不影响产出（与内部卷同纪律）。
    let mut ordered: Vec<&MineRecord> = records.iter().collect();
    ordered.sort_by(|left, right| {
        left.timestamp_ms
            .cmp(&right.timestamp_ms)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut cases: Vec<GoldenCase> = Vec::new();
    for record in ordered {
        let internal_subject = crate::golden_mining::question_subject(record);
        let entities = association_entities(&record.content);
        let mut made = 0usize;
        for entity in entities {
            if made >= config.max_cases_per_record {
                break;
            }
            // ① 内部卷已经问过这个词 ⇒ 跳过（两卷问的词必须不相交）。
            if internal_subject.as_deref() == Some(entity.as_str()) {
                continue;
            }
            // ② 跨记录桥：这个词得连着**不止一条**记录。
            if associations.entity_record_count(&entity) < config.min_bridge_records {
                continue;
            }
            // ③ 主体必须在正文里（否则答案片段无从谈起）。
            if !record.content.contains(&entity) {
                continue;
            }
            // ④ 答案片段：正文里含该主体的那一句（与内部卷同口径）。
            let Some(fragment) = crate::golden_mining::fragment_around(record, &entity) else {
                continue;
            };
            cases.push(GoldenCase {
                id: format!("gf-{}-{}", record.id, entity),
                query: fair_query(&entity),
                expect_ids: vec![record.id.clone()],
                expect_fragments: vec![fragment],
                tags: vec![
                    "fair".to_string(),
                    "kind:keyword".to_string(),
                    "bridge".to_string(),
                ],
            });
            made += 1;
        }
    }
    cases
}
