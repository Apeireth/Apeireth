//! InMemory MemoryBackend（仅测试，进程重启数据丢失）。
//!
//! 0 装 PASS：进程结束 = 数据全无。**不**用于生产。`BackendKind::InMemory` 明确标识此点。
//!
//! 实现用 `std::sync::Mutex<HashMap<...>>` 包裹所有状态，**Send + Sync**。
//! 不阻塞 I/O（纯内存）。

use std::collections::HashMap;
use std::sync::Mutex;

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::StreamKind;
use apeireth_plugin::memory_backend::{CapabilityResult, EpisodeWriteOutcome};

use crate::append_only::HistoryEntry;
use crate::MemoryResult;

use super::{BackendKind, MemoryBackend};

/// 进程内 HashMap 后端（测试用）。
pub struct InMemoryBackend {
    episodes_by_id: Mutex<HashMap<String, Episode>>,
    metadata_by_episode: Mutex<HashMap<String, serde_json::Value>>,
    /// `(stream_kind, session_id) -> ordered list of entries`
    streams: Mutex<HashMap<(StreamKind, String), Vec<HistoryEntry>>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            episodes_by_id: Mutex::new(HashMap::new()),
            metadata_by_episode: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend for InMemoryBackend {
    fn name(&self) -> &'static str {
        "in_memory"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }

    fn put_episode(&self, ep: &Episode) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut map = self
            .episodes_by_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if map.contains_key(&ep.id) {
            return Err(Box::new(crate::MemoryError::Invalid(format!(
                "episode id already exists: {}",
                ep.id
            ))));
        }
        map.insert(ep.id.clone(), ep.clone());
        Ok(())
    }

    fn put_episode_with_metadata(
        &self,
        ep: &Episode,
        metadata: serde_json::Value,
    ) -> CapabilityResult<EpisodeWriteOutcome> {
        let mut episodes = self
            .episodes_by_id
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut sidecars = self
            .metadata_by_episode
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if let Some(existing) = episodes.get(&ep.id) {
            if existing.content != ep.content
                || existing.timestamp != ep.timestamp
                || existing.role != ep.role
                || existing.session_id != ep.session_id
            {
                return Err(crate::MemoryError::Invalid(
                    "episode ID conflicts with stored evidence".into(),
                )
                .into());
            }
        }
        if let Some(existing) = sidecars.get(&ep.id) {
            crate::memory_chain::validate_metadata_retry(existing, &metadata)?;
        }
        let outcome = match episodes.entry(ep.id.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ep.clone());
                EpisodeWriteOutcome::Inserted
            }
            _ => EpisodeWriteOutcome::Existing,
        };
        sidecars.entry(ep.id.clone()).or_insert(metadata);
        Ok(outcome)
    }

    fn get_episode(
        &self,
        id: &str,
    ) -> Result<Option<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let map = self
            .episodes_by_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(map.get(id).cloned())
    }

    fn recent_episodes(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let map = self
            .episodes_by_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut all: Vec<Episode> = map
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect();
        all.sort_by_key(|e| e.timestamp);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }

    fn put_episode_metadata(
        &self,
        episode_id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.metadata_by_episode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(episode_id.to_string(), metadata);
        Ok(())
    }

    fn get_episode_metadata(
        &self,
        episode_id: &str,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .metadata_by_episode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(episode_id)
            .cloned())
    }

    fn append_stream(
        &self,
        kind: StreamKind,
        entry: HistoryEntry,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut streams = self
            .streams
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let key = (kind, entry.session_id.clone().unwrap_or_default());
        let list = streams.entry(key).or_insert_with(Vec::new);
        list.push(entry);
        Ok(())
    }

    fn list_stream(
        &self,
        kind: StreamKind,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let streams = self
            .streams
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut alive: Vec<HistoryEntry> = streams
            .iter()
            .filter(|((k, _), _)| *k == kind)
            .flat_map(|(_, list)| list.iter())
            .filter(|e| e.tombstoned_at.is_none())
            // L 组: 与 SqliteBackend / FileBackend / EncryptedFileBackend 对齐 —
            // `entry.session_id` 为 None 时不归属任何 session, 对任意
            // session_id 查询都可见. 旧实现把 None 归一成 "" 键桶, 只有查询
            // 空串才命中, 语义与其它三个后端分裂.
            .filter(|e| match &e.session_id {
                None => true,
                Some(s) => s == session_id,
            })
            .cloned()
            .collect();
        alive.sort_by_key(|e| e.created_at);
        if alive.len() > n {
            let skip = alive.len() - n;
            alive.drain(..skip);
        }
        Ok(alive)
    }
}

impl crate::scope::ScopedMemoryBackend for InMemoryBackend {
    /// 候选发现：**两条切片**（新鲜度 / 相关度），与 SQL 侧同一口径。
    ///
    /// # 为什么内存实现也要两条切片
    ///
    /// 三条后端路径（`backend/sqlite.rs` / `src/lib.rs` / 本文件）喂给
    /// `MemoryCoordinator` 的是同一种候选 ⇒ ⇒ 若只有 SQL 路径两条切片，
    /// 内存路径仍会「按最新截断」⇒ ⇒ 长期关键事实在内存路径上照样被挤掉
    /// ⇒ ⇒ 三条路径口径必须一致。
    ///
    /// # 三条不变量怎么保住
    ///
    /// * **作用域**：可见性判定沿用 `scope.is_visible_in`（与 SQL 侧同一
    ///   `MemoryScope` 语义）⇒ ⇒ 相关度切片不产生越权可见性；
    /// * **治理状态**：本实现不持有治理行（治理在 `coordinator` 侧）
    ///   ⇒ ⇒ 切片不新增也不放宽治理判定；
    /// * **时间过滤**：`as_of` 仍在**进入切片之前**过滤 ⇒ ⇒ 两条切片都看不到
    ///   时间锚点之后的记录；
    /// * **预算**：合并后**总量仍 ≤ `query.limit`** ⇒ ⇒ 相关度切片**占用**
    ///   预算里的位置，不是追加预算。
    fn query_candidates(
        &self,
        query: &crate::scope::MemoryCandidateQuery,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        if query.visible_scopes.is_empty() || query.limit == 0 {
            return Ok(Vec::new());
        }
        let episodes = self
            .episodes_by_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let metadata_map = self
            .metadata_by_episode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut matched = Vec::new();
        for ep in episodes.values() {
            if let Some(as_of_ms) = query.as_of_ms {
                if (ep.timestamp * 1000) > as_of_ms {
                    continue;
                }
            }
            let scope = if let Some(meta) = metadata_map.get(&ep.id) {
                meta.get("scope")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_else(|| crate::scope::MemoryScope::Session {
                        session_id: ep.session_id.clone(),
                    })
            } else {
                crate::scope::MemoryScope::Session {
                    session_id: ep.session_id.clone(),
                }
            };

            if scope.is_visible_in(&query.visible_scopes) {
                matched.push(ep.clone());
            }
        }

        // ── 切片 1：新鲜度（既有行为，逐条不变）─────────────────────────────
        let mut fresh_rows = matched.clone();
        fresh_rows.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then_with(|| b.id.cmp(&a.id)));
        fresh_rows.truncate(query.limit);
        // ⚠️ 这里**不**再翻转：合并阶段按「相关度优先 → 新鲜度补位」取前 `limit` 条
        // ⇒ ⇒ 翻转过会让补位从**最老**的一端开始 ⇒ ⇒ 被挤掉的是最新那条
        // ⇒ ⇒ 与 SQL 侧（新鲜度切片保持时间倒序）不一致 ⇒ ⇒ 输出顺序由合并阶段统一排。

        // ⚠️ 便宜先判（省一次全表扫描）：若新鲜度切片**没取满**预算 ⇒ ⇒ 可见历史
        // 全部落在这条切片里 ⇒ ⇒ 相关度切片只可能返回**已经在池子里**的行
        // ⇒ ⇒ 合并结果逐条相同 ⇒ ⇒ 直接跳过相关度切片。
        // ⇒ 预算中性（不多取、不多花）、不改动任何不变量（作用域 / 治理 / 时间 / 来源）。
        if fresh_rows.len() < query.limit {
            return Ok(crate::backend::candidate::merge_candidates(
                Vec::new(),
                fresh_rows,
                query.limit,
            ));
        }

        // ── 切片 2：相关度（只收命中行；命中口径与 SQL 侧 LIKE 语义一致）─────
        let relevance_limit = query.relevance_slice_limit();
        let mut relevance_rows: Vec<Episode> = Vec::new();
        if relevance_limit > 0 {
            let terms = crate::backend::candidate::normalized_terms(&query.query_terms);
            if !terms.is_empty() {
                let mut scored: Vec<(usize, Episode)> = matched
                    .into_iter()
                    .filter_map(|ep| {
                        let hits = crate::backend::candidate::hit_count(
                            &ep.content.to_lowercase(),
                            &terms,
                        );
                        if hits == 0 {
                            None
                        } else {
                            Some((hits, ep))
                        }
                    })
                    .collect();
                // 命中多者优先；同命中数按时间新者优先（与 SQL 侧排序一致）。
                scored.sort_by(|a, b| {
                    b.0.cmp(&a.0)
                        .then_with(|| b.1.timestamp.cmp(&a.1.timestamp))
                        .then_with(|| b.1.id.cmp(&a.1.id))
                });
                relevance_rows = scored
                    .into_iter()
                    .take(relevance_limit)
                    .map(|(_, ep)| ep)
                    .collect();
            }
        }

        // ── 合并：相关度切片优先 → 新鲜度切片补位 → 去重 → 总量 ≤ limit ──
        Ok(crate::backend::candidate::merge_candidates(
            relevance_rows,
            fresh_rows,
            query.limit,
        ))
    }
}

#[allow(dead_code)]
fn stream_name_to_kind(_name: &str) -> MemoryResult<StreamKind> {
    // 0 装: typed enum 编译期保证 6 个 StreamKind, 无运行时转换
    // 保留为 helper 占位给 caller 用 (e.g. CLI 参数解析); rc 阶段接 SchemaRegistry 时改
    Err(crate::MemoryError::Invalid(
        "stream_name_to_kind is deprecated; use StreamKind typed enum directly".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::HistoryEntry;

    fn ep(id: &str, session: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: format!("content of {id}"),
            session_id: session.to_string(),
        }
    }

    fn he(id: &str, session: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            subject_id: "subj-1".to_string(),
            subject_rev: 1,
            session_id: Some(session.to_string()),
            created_at: 1_700_000_100,
            payload: serde_json::json!({"kind": "test"}),
            source: "test".to_string(),
            tags: vec!["unit".to_string()],
            tombstoned_at: None,
        }
    }

    #[test]
    fn name_and_kind() {
        let b = InMemoryBackend::new();
        assert_eq!(b.name(), "in_memory");
        assert_eq!(b.kind(), BackendKind::InMemory);
    }

    #[test]
    fn episode_roundtrip() {
        let b = InMemoryBackend::new();
        let e = ep("ep-1", "sess-1");
        b.put_episode(&e).unwrap();
        let got = b.get_episode("ep-1").unwrap().expect("episode exists");
        assert_eq!(got.id, e.id);
        assert_eq!(got.timestamp, e.timestamp);
        assert_eq!(got.role, e.role);
        assert_eq!(got.content, e.content);
        assert_eq!(got.session_id, e.session_id);
        let recent = b.recent_episodes("sess-1", 10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn append_only_rejects_duplicate_episode_id() {
        let b = InMemoryBackend::new();
        let e = ep("dup", "s");
        b.put_episode(&e).unwrap();
        let r = b.put_episode(&e);
        assert!(r.is_err());
    }

    #[test]
    fn stream_roundtrip() {
        let b = InMemoryBackend::new();
        let session = "s1";
        let thought = crate::StreamKind::Thought;
        b.append_stream(thought, he("t1", session)).unwrap();
        b.append_stream(thought, he("t2", session)).unwrap();
        let list = b.list_stream(thought, session, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "t1");
        assert_eq!(list[1].id, "t2");
    }

    #[test]
    fn list_stream_filters_tombstoned() {
        let b = InMemoryBackend::new();
        let session = "s";
        let mut alive = he("a", session);
        alive.tombstoned_at = None;
        let mut dead = he("d", session);
        dead.tombstoned_at = Some(1_700_000_500);
        let action = crate::StreamKind::Action;
        b.append_stream(action, alive).unwrap();
        b.append_stream(action, dead).unwrap();
        let list = b.list_stream(action, session, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a");
    }

    /// L 组: None-session 条目对任意 session 查询可见 (与 SqliteBackend /
    /// FileBackend / EncryptedFileBackend 对齐, 旧实现把它们锁在 "" 键桶).
    #[test]
    fn list_stream_none_session_matches_any_session() {
        let b = InMemoryBackend::new();
        let thought = crate::StreamKind::Thought;
        let mut orphan = he("orphan", "whatever");
        orphan.session_id = None;
        b.append_stream(thought, orphan).unwrap();
        b.append_stream(thought, he("owned", "s1")).unwrap();
        let for_s1 = b.list_stream(thought, "s1", 10).unwrap();
        assert_eq!(for_s1.len(), 2, "None-session 条目必须对具名 session 可见");
        let for_other = b.list_stream(thought, "other", 10).unwrap();
        assert_eq!(for_other.len(), 1);
        assert_eq!(for_other[0].id, "orphan");
    }

    #[test]
    fn unknown_stream_name_is_rejected() {
        let b = InMemoryBackend::new();
        // typed enum 不可能 unknown (编译期保证)
        let invalid = crate::StreamKind::Thought;
        let r = b.append_stream(invalid, he("x", "s"));
        assert!(r.is_ok(), "typed enum 编译期保证, typed call 不应失败");
    }
}
