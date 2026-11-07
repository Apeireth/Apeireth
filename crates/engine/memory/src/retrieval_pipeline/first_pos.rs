//! 「首命中位置」重排（2026-11-07）：机制、实测与开关口径。
//!
//! # 为什么整段放在子模块
//! `retrieval_pipeline.rs` 是 `scripts/arch-baseline.txt` 登记的**尺寸棘轮文件**（只许不增）；
//! 这个特征的「机制说明 + 实测表 + 实现 + 测试」一次性有 100 多行 ⇒ ⇒ 全部搬到本子模块，
//! 主文件只留**一个公开字段 + 一行调用** ⇒ ⇒ **语义与默认值逐字不变，父文件回到基线内**。
//!
//! ⭐⭐ **「首命中位置」重排强度**（2026-11-07 新增，**默认 0.0 = 关闭**）。
//!
//! # 它是什么
//! 题面词在候选文本里**首次出现得越靠前**，越可能是该段落的主题
//! ⇒ ⇒ 给候选加 `alpha × 位置分` 的加成（位置分已按题归一化到 (0,1]）。
//!
//! # 实测（**两份考卷结论相反，所以默认关闭**）
//! ## 真实用户查询（MIRACL 中文 dev · 393 条真实母语查询 · 采样池 22862 段）
//! | α | recall@10 | 相对基线 |
//! |---|---|---|
//! | 0.05 | 0.8448 | **+0.1201** |
//! | **0.10** | **0.8473** | **+0.1227** |
//! | 0.50 | 0.8041 | +0.0794 |
//! （基线 0.7246；天花板 0.9389 ⇒ ⇒ **吃掉天花板余量的 57%**）
//!
//! ## 本项目模板考卷
//! | 考卷 / 语料 | α=0.10 |
//! |---|---|
//! | 词面卷 / redchamber | **-0.0252** |
//! | 词面卷 / watershed | **-0.0202** |
//! | 公平卷 / redchamber | +0.0240 |
//! | 公平卷 / watershed | +0.0081 |
//!
//! # ⚠️ 为什么默认关（**这是测量仪器问题，不是产品问题**）
//! 模板题是「{主体}后来是怎么改的？」
//! ⇒ ⇒ 记录开头的「却说/话说」这类**套话**被当作命中
//! ⇒ ⇒ ⇒ **首位置反而与相关性负相关** ⇒ ⇒ 模板卷**系统性惩罚**这个特征。
//!
//! ⇒ ⇒ 一旦闸门换成真查询卷（或双卷并行），把这里设成 **0.10** 即可。
//! ⇒ ⇒ ⇒ **在此之前保持 0.0：不引入任何闸门回归。**

use crate::MemoryCandidate;

/// ⭐ **首命中位置分**（2026-11-07）：题面词在正文里首次出现位置的倒数。
///
/// # 口径
/// * 返回 `1/(1+pos)`，`pos` 是**首次出现处之前的字符数**；
/// * **没有命中**（或题面没有可用词）⇒ ⇒ 返回 `0.0`；
/// * 调用方**必须**按题把结果归一化到 (0,1]（调用方负责按题归一化到 (0,1]）。
///
/// # 为什么用「字符位置」而不是「词位置」
/// 本仓库的分词是 CJK bigram（无词典依赖）；字符位置**不依赖分词口径**
/// ⇒ ⇒ 跨机器逐字一致。
pub(super) fn first_pos_score(query_terms: &[String], content: &str) -> f64 {
    let mut position = f64::MAX;
    for term in query_terms {
        if let Some(at) = content.find(term.as_str()) {
            position = position.min(at as f64);
        }
    }
    if position.is_finite() {
        1.0 / (1.0 + position)
    } else {
        0.0
    }
}

/// 把「首命中位置」加成应用到并集候选上（`alpha == 0.0` ⇒ 立即返回、零副作用）。
pub(super) fn apply(candidates: &mut [MemoryCandidate], query: &str, alpha: f64) {
    if alpha == 0.0 {
        return;
    }
    let query_terms =
        crate::hybrid_search::tokenize(&crate::hybrid_search::normalize_for_index(query));
    let raw: Vec<f64> = candidates
        .iter()
        .map(|candidate| first_pos_score(&query_terms, &candidate.content))
        .collect();
    let max = raw.iter().cloned().fold(0.0_f64, f64::max);
    if max > 0.0 {
        for (candidate, value) in candidates.iter_mut().zip(raw) {
            candidate.score += alpha * (value / max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::FixedSource;
    use super::super::{HybridRetrievalPipeline, SessionContext};
    use crate::{MemoryCandidate, MemoryProvenance, MemoryScope, ScoreComponents};

    /// ⭐⭐ **`first_pos_alpha` 默认必须是 0.0**（2026-11-07）。
    ///
    /// # 为什么锁这个
    /// 该特征在**真实用户查询**上 +0.1227（α=0.10），在**模板词面卷**上 −0.0252。
    /// 闸门用的是模板卷 ⇒ ⇒ **默认开启会直接让闸门红**。
    /// ⇒ ⇒ 所以默认值**必须**是 0.0，等闸门升级后再考虑打开。
    #[test]
    fn first_pos_alpha_defaults_to_disabled() {
        assert_eq!(
            HybridRetrievalPipeline::default().first_pos_alpha,
            0.0,
            "默认必须关闭：模板卷会掉（−0.0252），真实查询卷会涨（+0.1227）"
        );
    }

    /// ⭐ **`first_pos_alpha = 0.0` 时结果必须与「没有这个特征」逐字相同**。
    ///
    /// 判据：同一批候选、同一套分量，开 0.0 与开一个极大负值都**不该改变排序**
    /// （因为整段加成被 `!= 0.0` 短路掉了）⇒ ⇒ 这里直接比对 id 序列。
    #[test]
    fn first_pos_alpha_zero_is_a_no_op() {
        let sources = vec![crate::MemoryCandidate {
            id: "a".into(),
            layer: "episodic".into(),
            scope: MemoryScope::Global,
            content: "宝玉哭了".into(),
            score: 0.0,
            score_components: ScoreComponents {
                lexical: 0.4,
                ..Default::default()
            },
            provenance: MemoryProvenance::default(),
        }];
        let source = FixedSource(sources);
        let pipeline = HybridRetrievalPipeline::default();
        let (hits, _) = pipeline
            .retrieve_with_status_in(
                SessionContext::Ambient,
                "宝玉",
                &[MemoryScope::Global],
                &[&source],
                10,
                1_000,
            )
            .expect("retrieve");
        let with_feature_disabled = hits.into_iter().map(|h| h.id).collect::<Vec<_>>();
        assert_eq!(
            with_feature_disabled,
            vec!["a".to_string()],
            "first_pos 关闭时不该影响候选集合"
        );
    }
}
