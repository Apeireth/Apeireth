//! P1-9: Episode Dedup (借鉴 mempalace `dedup.py`).
//!
//! ## 目的
//! 防止同一 session 多次写入近重复 (e.g. 用户重复说同一件事, 或 pipeline retry
//! 产生重复 episode). 用 embedding cosine distance 检测, 保留最长版本, 报告重复.
//!
//! ## 借鉴 mempalace `mempalace/dedup.py`
//! - **group by session_id** (mempalace: `source_file`)
//! - **greedy**: 按 content 长度降序遍历, 跟已 keep 的对比
//! - **threshold**: cosine DISTANCE (非 similarity), 默认 0.15
//!   (= ~85% cosine similarity = 近相同)
//! - **keep longest, delete shorter**
//!
//! ## apeireth 适配
//! - append-only 架构 → 不能真 DELETE (per `append_only.rs`)
//! - 默认 `dry_run=true` (跟 mempalace 同语义) — 只生成 `DedupReport`
//! - caller 拿 report 决定怎么落地 (e.g. 加 dedup_log 表, 或在 index_episode 时跳过)
//!
//! ## 用法
//! ```rust,no_run
//! use apeireth_memory::dedup::{dedup_session, default_threshold};
//! use apeireth_memory::semantic::{EmbedFn, HashEmbedder};
//! use apeireth_memory::SqliteMemoryStore;
//! use std::sync::Arc;
//!
//! let store = Arc::new(SqliteMemoryStore::open_in_memory()?);
//! let embedder: Arc<dyn EmbedFn> = Arc::new(HashEmbedder::new(64));
//! let report = dedup_session(&store, "session-1", default_threshold(), embedder)?;
//! println!("kept {} deleted {}", report.kept.len(), report.deleted.len());
//! # Ok::<(), apeireth_memory::MemoryError>(())
//! ```

use std::sync::Arc;

use apeireth_core::Episode;

use crate::episode::EpisodeStore;
use crate::semantic::EmbedFn;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 默认 cosine DISTANCE threshold (跟 mempalace `DEFAULT_THRESHOLD = 0.15` 一致).
///
/// 含义: cosine distance < 0.15 = cosine similarity > ~85% = 近重复.
pub fn default_threshold() -> f32 {
    0.15
}

/// 单条 episode 在 dedup 中的处置.
#[derive(Debug, Clone, PartialEq)]
pub enum DedupAction {
    /// 保留 (在当前 group 里是最长或最不重复的).
    Kept,
    /// 标记为重复 (`of_id` 是被保留下来的那条).
    Duplicate {
        /// 被保留的 episode ID.
        of_id: String,
        /// cosine distance (0.0 = 相同, 2.0 = 完全相反, 1.0 = 正交).
        distance: f32,
    },
}

/// Dedup 结果报告.
#[derive(Debug, Clone)]
pub struct DedupReport {
    /// 处理的 session_id.
    pub session_id: String,
    /// 保留的 episode IDs (含 action=Kept).
    pub kept: Vec<(String, DedupAction)>,
    /// 被标记为重复的 episode IDs.
    pub deleted: Vec<(String, DedupAction)>,
    /// 实际使用的 threshold (cosine distance).
    pub threshold: f32,
    /// 扫描的 episode 总数.
    pub scanned: usize,
}

/// cosine distance = 1 - cosine_similarity.
///
/// 输入向量应 L2 归一化 (HashEmbedder 已经归一化).
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 2.0; // 完全不匹配视为最大距离
    }
    if a.is_empty() {
        return 1.0;
    }
    let mut dot = 0.0_f64;
    let mut na = 0.0_f64;
    let mut nb = 0.0_f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let x = *x as f64;
        let y = *y as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 1.0;
    }
    let sim = dot / (na.sqrt() * nb.sqrt());
    let sim = sim.clamp(-1.0, 1.0);
    1.0 - sim as f32
}

/// 对一个 session 做近重复检测.
///
/// ## 算法 (per mempalace `dedup_source_group`)
/// 1. 拉该 session 全部 episodes
/// 2. 按 content 长度降序排序 (长 = 信息多 = 优先保留)
/// 3. greedy: 遍历排序后, 若跟某已 keep 的 cosine distance < threshold,
///    标记为 Duplicate; 否则 Kept
/// 4. 返回 DedupReport (默认 dry_run=true, 不修改 store)
///
/// ## Append-only 守诚信
/// 当前实现只生成报告, **不真删**任何 episode. 想落地需要 caller 自己加
/// `dedup_log` 表或 tombstone 字段 (per `append_only.rs` 守则).
pub fn dedup_session(
    store: &SqliteMemoryStore,
    session_id: &str,
    threshold: f32,
    embedder: Arc<dyn EmbedFn>,
) -> MemoryResult<DedupReport> {
    if !(0.0..=2.0).contains(&threshold) {
        return Err(MemoryError::Invalid(format!(
            "threshold {} out of range [0.0, 2.0]",
            threshold
        )));
    }

    let mut episodes = <SqliteMemoryStore as EpisodeStore>::query(
        store,
        &crate::episode::EpisodeQuery::new()
            .for_session(session_id)
            .limit(100_000),
    )?;

    let scanned = episodes.len();
    if episodes.is_empty() {
        return Ok(DedupReport {
            session_id: session_id.to_string(),
            kept: Vec::new(),
            deleted: Vec::new(),
            threshold,
            scanned: 0,
        });
    }

    // 长 -> 短, 保留信息最多的
    episodes.sort_by(|a, b| b.content.len().cmp(&a.content.len()));

    let mut kept: Vec<(Episode, Vec<f32>)> = Vec::new();
    let mut kept_report: Vec<(String, DedupAction)> = Vec::new();
    let mut deleted_report: Vec<(String, DedupAction)> = Vec::new();

    for ep in episodes {
        if ep.content.is_empty() || ep.content.len() < 20 {
            // 太短没价值, 直接标 duplicate (per mempalace)
            deleted_report.push((
                ep.id.clone(),
                DedupAction::Duplicate {
                    of_id: String::new(),
                    distance: 0.0,
                },
            ));
            continue;
        }
        let vec = embedder.embed(&ep.content);
        let mut duplicate_of: Option<(String, f32)> = None;
        for (kept_ep, kept_vec) in &kept {
            let dist = cosine_distance(&vec, kept_vec);
            if dist < threshold {
                duplicate_of = Some((kept_ep.id.clone(), dist));
                break;
            }
        }
        match duplicate_of {
            Some((of_id, distance)) => {
                deleted_report.push((ep.id.clone(), DedupAction::Duplicate { of_id, distance }));
            }
            None => {
                kept_report.push((ep.id.clone(), DedupAction::Kept));
                kept.push((ep, vec));
            }
        }
    }

    Ok(DedupReport {
        session_id: session_id.to_string(),
        kept: kept_report,
        deleted: deleted_report,
        threshold,
        scanned,
    })
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::HashEmbedder;

    fn make_episode(id: &str, session: &str, ts: i64, content: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: "user".into(),
            content: content.into(),
            session_id: session.into(),
        }
    }

    fn fresh_store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("open memory")
    }

    fn embedder_64() -> Arc<dyn EmbedFn> {
        Arc::new(HashEmbedder::new(64))
    }

    /// 1. default_threshold 返回 0.15
    #[test]
    fn default_threshold_is_0_15() {
        assert!((default_threshold() - 0.15).abs() < 1e-6);
    }

    /// 2. cosine_distance: 完全相同 → 0.0
    #[test]
    fn cosine_distance_identical_is_zero() {
        let v = vec![1.0, 0.0, 0.0];
        let d = cosine_distance(&v, &v);
        assert!(d.abs() < 1e-6);
    }

    /// 3. cosine_distance: 正交 → 1.0
    #[test]
    fn cosine_distance_orthogonal_is_one() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    /// 4. cosine_distance: 相反 → 2.0
    #[test]
    fn cosine_distance_opposite_is_two() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_distance(&a, &b) - 2.0).abs() < 1e-6);
    }

    /// 5. cosine_distance: 不同长度 → 2.0 (invalid signal)
    #[test]
    fn cosine_distance_different_len_is_two() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_distance(&a, &b) - 2.0).abs() < 1e-6);
    }

    /// 6. dedup_session 空 session → 全空 report
    #[test]
    fn dedup_empty_session_returns_empty_report() {
        let store = fresh_store();
        let r = dedup_session(&store, "no-such-session", 0.15, embedder_64()).unwrap();
        assert_eq!(r.scanned, 0);
        assert!(r.kept.is_empty());
        assert!(r.deleted.is_empty());
    }

    /// 7. dedup_session: 唯一一条 episode → 不删
    #[test]
    fn dedup_single_episode_kept() {
        let store = fresh_store();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, "this is a unique episode with some content"),
        )
        .unwrap();
        let r = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        assert_eq!(r.scanned, 1);
        assert_eq!(r.kept.len(), 1);
        assert!(r.deleted.is_empty());
    }

    /// 8. dedup_session: 完全相同的 2 条 → 1 删 (长的留, 短的删)
    #[test]
    fn dedup_identical_pair_deletes_shorter() {
        let store = fresh_store();
        let content = "the quick brown fox jumps over the lazy dog and runs into the forest";
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, content),
        )
        .unwrap();
        // 重复内容
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "s1", 2, content),
        )
        .unwrap();
        let r = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        assert_eq!(r.scanned, 2);
        assert_eq!(r.kept.len(), 1);
        assert_eq!(r.deleted.len(), 1);
        // kept 是 e1 (先 put, 同长度 → 字典序先)
        match &r.kept[0].1 {
            DedupAction::Kept => {}
            _ => panic!("expected Kept"),
        }
    }

    /// 9. dedup_session: 完全不同的 2 条 → 都留
    #[test]
    fn dedup_different_content_both_kept() {
        let store = fresh_store();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, "alpha beta gamma delta epsilon zeta eta theta"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "s1", 2, "rust python go javascript typescript cpp java kotlin"),
        )
        .unwrap();
        let r = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        assert_eq!(r.scanned, 2);
        assert_eq!(r.kept.len(), 2);
        assert!(r.deleted.is_empty());
    }

    /// 10. dedup_session: append-only 守诚信 — 真删 = 0 行
    #[test]
    fn dedup_does_not_actually_delete() {
        let store = fresh_store();
        let content = "this is a test episode with some unique content for testing";
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, content),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "s1", 2, content),
        )
        .unwrap();
        let _r = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        // 真查 episode 数, 应仍是 2 (没真删)
        let count = <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, "s1").unwrap();
        assert_eq!(count, 2, "append-only: dedup 不应真删");
    }

    /// 11. dedup_session: 短 episode (< 20 字符) 直接标 duplicate
    #[test]
    fn dedup_too_short_episodes_marked_duplicate() {
        let store = fresh_store();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, "short"),
        )
        .unwrap();
        let r = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        assert_eq!(r.scanned, 1);
        assert_eq!(r.deleted.len(), 1);
        assert!(r.kept.is_empty());
    }

    /// 12. dedup_session: threshold 越严格 → 删越少 (高 threshold = 易删)
    #[test]
    fn dedup_threshold_lower_means_stricter() {
        let store = fresh_store();
        let c1 = "this is a long content for the first episode with many words to embed";
        let c2 = "this is a long content for the second episode with many words to embed";
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "s1", 1, c1),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "s1", 2, c2),
        )
        .unwrap();
        // 默认 0.15: 大概率判重复
        let r_strict = dedup_session(&store, "s1", 0.15, embedder_64()).unwrap();
        // threshold = 0.0001: 极严, 大概率都留
        let r_loose = dedup_session(&store, "s1", 0.0001, embedder_64()).unwrap();
        assert!(
            r_loose.kept.len() >= r_strict.kept.len(),
            "looser threshold should keep >= (got loose={}, strict={})",
            r_loose.kept.len(),
            r_strict.kept.len()
        );
    }

    /// 13. dedup_session: threshold 校验 ([0.0, 2.0])
    #[test]
    fn dedup_invalid_threshold_errors() {
        let store = fresh_store();
        let r = dedup_session(&store, "s1", 3.0, embedder_64());
        assert!(r.is_err());
        let r = dedup_session(&store, "s1", -0.1, embedder_64());
        assert!(r.is_err());
    }
}
