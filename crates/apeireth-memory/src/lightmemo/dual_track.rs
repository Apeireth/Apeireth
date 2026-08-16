//! `lightmemo::dual_track` — 双轨语义决策 (审计 P2#8, 2026-08-16 backlog 全清).
//!
//! 双轨: **episodes = 事实源 (权威)**, **L1-L4 分层索引 = 快速路径 (补充)**。
//! 查询时两条轨道各自产出候选, 本模块做合并决策:
//!   1. 同 id 去重 — episodes 优先 (事实源权威, 索引不覆盖事实)
//!   2. 索引独有命中 (episodes 无) → 补充进结果, 但诚实标记来源 `Index`
//!   3. 打分: 双轨命中 > 单轨; fusion 多模式命中加分; episodes 轨道自带分保留
//!
//! 纯函数机制 (0 假装): 不做"看似聪明"的语义融合, 只做可验证的
//! 去重/优先级/打分决策; 测试覆盖全部规则。

use std::collections::HashMap;

use super::search::SearchHit;

/// 命中来源 (诚实标注, 不合并糊弄).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitSource {
    /// 事实源 (episodes) 独有.
    Episode,
    /// 分层索引独有 (补充, 可信度低于事实源).
    Index,
    /// 双轨都有 (最可信).
    Both,
}

/// 双轨合并后的命中.
#[derive(Debug, Clone)]
pub struct DualTrackHit {
    pub id: String,
    pub content: String,
    pub source: HitSource,
    pub score: f32,
}

/// 双轨语义决策: episodes (事实源) + 索引命中 → 合并排序.
///
/// - `episodes`: 事实源候选 (id, content), 调用方已按自身相关性排序 (如排名注入)
/// - `index_hits`: lightmemo 索引命中 (SearchPipeline 产物)
/// - `index_content`: 索引 id → content (取内容用)
/// - `top_k`: 输出上限
pub fn dual_track_merge(
    episodes: &[(String, String)],
    index_hits: &[SearchHit],
    index_content: &HashMap<String, String>,
    top_k: usize,
) -> Vec<DualTrackHit> {
    // episodes 轨道: 全进 (事实源权威)
    let mut out: Vec<DualTrackHit> = episodes
        .iter()
        .map(|(id, content)| DualTrackHit {
            id: id.clone(),
            content: content.clone(),
            source: HitSource::Episode,
            score: 1.0,
        })
        .collect();
    let episode_ids: std::collections::HashSet<&String> =
        out.iter().map(|h| &h.id).collect();

    // 索引轨道: 去重后补充 (标记 Index; 双轨都有 → 升级 Both + 加分)
    for hit in index_hits {
        let Some(content) = index_content.get(&hit.id).cloned() else {
            continue;
        };
        let extra = (hit.matched_in.len() as f32) * 0.25; // 多模式命中加分
        if let Some(existing) = out.iter_mut().find(|h| h.id == hit.id) {
            existing.source = HitSource::Both;
            existing.score = (existing.score + 0.5 + extra).min(2.0);
        } else {
            out.push(DualTrackHit {
                id: hit.id.clone(),
                content,
                source: HitSource::Index,
                score: 0.5 + extra,
            });
        }
    }

    // 排序: 分高在前, 同分 episodes 优先 (事实源权威)
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
    });
    out.truncate(top_k);
    out
}

fn source_rank(s: HitSource) -> u8 {
    match s {
        HitSource::Both => 0,
        HitSource::Episode => 1,
        HitSource::Index => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lightmemo::search::SearchMode;

    #[test]
    fn episodes_win_over_index_on_same_id() {
        let episodes = vec![("e1".to_string(), "主人喜欢深蓝夜空".to_string())];
        let hits = vec![SearchHit {
            id: "e1".to_string(),
            score: 1.0,
            matched_in: vec![SearchMode::Keyword],
        }];
        let mut content = HashMap::new();
        content.insert("e1".to_string(), "索引里的旧内容".to_string());
        let out = dual_track_merge(&episodes, &hits, &content, 10);
        assert_eq!(out.len(), 1, "同 id 应去重");
        assert_eq!(out[0].content, "主人喜欢深蓝夜空", "episodes 事实源内容优先");
        assert_eq!(out[0].source, HitSource::Both, "双轨都有 → Both");
        assert!(out[0].score > 1.0, "双轨命中应加分");
    }

    #[test]
    fn index_only_hits_are_appended_as_index_source() {
        let episodes: Vec<(String, String)> = vec![("e1".to_string(), "事实".to_string())];
        let hits = vec![SearchHit {
            id: "idx1".to_string(),
            score: 0.8,
            matched_in: vec![SearchMode::Keyword],
        }];
        let mut content = HashMap::new();
        content.insert("idx1".to_string(), "索引补充内容".to_string());
        let out = dual_track_merge(&episodes, &hits, &content, 10);
        assert_eq!(out.len(), 2);
        let idx = out.iter().find(|h| h.id == "idx1").expect("索引命中应补充");
        assert_eq!(idx.source, HitSource::Index, "诚实标记来源");
        assert_eq!(idx.content, "索引补充内容");
        // episodes 优先: e1 应排前面 (同分时事实源优先)
        assert_eq!(out[0].id, "e1");
    }

    #[test]
    fn multi_mode_hits_score_higher() {
        let episodes: Vec<(String, String)> = Vec::new();
        let hits = vec![
            SearchHit { id: "a".to_string(), score: 1.0, matched_in: vec![SearchMode::Keyword] },
            SearchHit { id: "b".to_string(), score: 1.0, matched_in: vec![SearchMode::Keyword, SearchMode::Tag] },
        ];
        let mut content = HashMap::new();
        content.insert("a".to_string(), "单模式".to_string());
        content.insert("b".to_string(), "多模式".to_string());
        let out = dual_track_merge(&episodes, &hits, &content, 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "b", "fusion 多模式命中应排前");
    }

    #[test]
    fn top_k_respected() {
        let episodes: Vec<(String, String)> = (0..5).map(|i| (format!("e{i}"), format!("内容{i}"))).collect();
        let out = dual_track_merge(&episodes, &[], &HashMap::new(), 3);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn empty_both_returns_empty() {
        let out = dual_track_merge(&[], &[], &HashMap::new(), 10);
        assert!(out.is_empty());
    }

    #[test]
    fn index_hit_without_content_is_skipped() {
        let episodes: Vec<(String, String)> = Vec::new();
        let hits = vec![SearchHit { id: "ghost".to_string(), score: 1.0, matched_in: vec![SearchMode::Keyword] }];
        let out = dual_track_merge(&episodes, &hits, &HashMap::new(), 10);
        assert!(out.is_empty(), "索引有 id 无内容 → 跳过 (不假装)");
    }
}
