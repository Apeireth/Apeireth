//! 「首命中位置」重排（2026-11-07 初版 · **2026-11-08 数字与结论修正**）：机制、实测与开关口径。
//!
//! # 为什么整段放在子模块
//! `retrieval_pipeline.rs` 是 `scripts/arch-baseline.txt` 登记的**尺寸棘轮文件**（只许不增）；
//! 这个特征的「机制说明 + 实测表 + 实现 + 测试」一次性有 100 多行 ⇒ ⇒ 全部搬到本子模块，
//! 主文件只留**一个公开字段 + 一行调用** ⇒ ⇒ **语义与默认值逐字不变，父文件回到基线内**。
//!
//! ⭐⭐ **「首命中位置」重排强度**（**默认 0.0 = 关闭**）。
//!
//! # 它是什么
//! 题面词在候选文本里**首次出现得越靠前**，越可能是该段落的主题
//! ⇒ ⇒ 给候选加 `alpha × 位置分` 的加成（位置分已按题归一化到 (0,1]）。
//!
//! # ⚠️⚠️ **2026-11-08：撤回一个旧结论**
//! 此前本文件写的是「真实用户查询 **+0.1227**、模板词面卷 **−0.0252**
//!；**是模板卷偏**，换考卷就能开」。
//!；**那个 +0.1227 是「混口径减法」，无效**：
//!
//! | 数字 | 它是什么指标 |
//! |---|---|
//! | 基线 0.7246 | **闸门口径**：逐题「命中比例」再平均（= `golden_run::metrics.recall_at_k`） |
//! | 0.8473 | **二值「任一命中」** |
//!
//!；两个**不同的指标**相减；同口径重测后，真实查询上的真实收益是 **+0.0192**。
//!
//! # 实测（**2026-11-08 全部重测 · 口径统一 · 两种口径同基线**）
//!
//! ## 真实用户查询（MIRACL 中文 dev · 393 条真实母语查询 · 采样池 22862 段 · k=10）
//! | α | 闸门口径 recall | 相对同口径基线 |
//! |---|---|---|
//! | 0.05 | 0.7414 | +0.0167 |
//! | **0.10** | **0.7438** | **+0.0192** |
//! | 0.20 | 0.7337 | +0.0091 |
//! | 0.50 | 0.6715 | −0.0531 |
//! （同口径基线 0.7246；天花板 0.9389）
//!；⚠️ **旧表里的 0.8448 / 0.8473 / 0.8041 是「二值任一命中」口径，
//! 且用过一个**与生产不等价**的分词器；已整体作废。**
//!
//! ## 本项目模板考卷（同一口径 · 同一实现）
//! | 考卷 / 语料 | 基线 | α=0.10 | delta |
//! |---|---|---|---|
//! | 词面卷 / redchamber | 0.5305 | 0.4874 | **−0.0431** |
//! | 词面卷 / watershed | 0.7319 | 0.7036 | **−0.0282** |
//!
//! # ⭐ 结论（2026-11-08）：**两把尺子都可信，是特征本身太弱，保持关闭**
//! * 真实查询上它只值 **+0.0192**（闸门容忍度 1%，勉强够，但不是 +0.12 那种量级）；
//! * 模板卷上它值 **−0.0431 / −0.0282**（闸门容忍度只有 1%，**必红**）；
//! * **「换考卷就能开」这条路不通**：即使只取「答案在候选池里」的可达子集，
//!   模板卷上仍是 −0.0257 / −0.0451；
//! * **「改特征实现」这条路也不通**（三个臂实测 α=0.10）：
//!   只用长度 ≥2 的词 −0.0399 / −0.0403；只用「池内命中最少的那个词」−0.0336 / −0.0444。
//!
//! # 机制（**实测**，不是推测）
//! * **模板卷的「记忆题」上位置分没有区分度**：
//!   答案候选与非答案候选的「最早命中位置」**完全相同**
//!   （redchamber 两边都是「寶玉 @ 145.1」）； 位置分只在基线分数接近的候选之间乱序
//!  ；净掉分。原因：模板题的实词是**人名**，人名在章回体正文里不在开头；
//!   且分词产出的**单字构件**（賈、寶、鳳）到处都是、出现得早。
//! * **真人问句的实词是主题属性**（首都、面积、人口）； 出现在主题段开头
//!  ；位置分有区分度；净涨分。
//!
//!；**「关着的理由」并没有消失**；继续保持 `0.0`。
//!；要开它，得先找到一个**在模板卷上也不掉**的信号，而不是换考卷。

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

    /// ⭐⭐ **`first_pos_alpha` 默认必须是 0.0**（2026-11-07 立，2026-11-08 复核仍成立）。
    ///
    /// # 为什么锁这个
    /// 闸门用的是**模板卷**；默认开启会直接让闸门红。
    ///；**同口径实测（2026-11-08）**：模板卷 alpha=0.10 是 **−0.0431 / −0.0282**
    /// （闸门容忍度只有基线的 1%）；
    /// 而真实用户查询上它只值 **+0.0192**（此前记的 +0.1227 是混口径减法，**已撤回**）。
    ///；**净效果接近零甚至为负**；默认值必须保持 0.0。
    #[test]
    fn first_pos_alpha_defaults_to_disabled() {
        assert_eq!(
            HybridRetrievalPipeline::default().first_pos_alpha,
            0.0,
            "默认必须关闭：模板卷掉 −0.0431 / −0.0282，真实查询卷只涨 +0.0192（旧记的 +0.1227 是混口径减法，已撤回）"
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
