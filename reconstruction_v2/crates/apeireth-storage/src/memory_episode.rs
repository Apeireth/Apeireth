//! Memory Episode - 完整 episode 系统 (从 v1.0 apeireth-memory/episode.rs 383 LOC 抄录升级)
//!
//! 0 装 PASS: 直接抄 v1.0 era 设计 (EpisodeQuery + EpisodeStore trait + append-only)
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub session_id: String,
    pub continuity_id: Option<String>,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct EpisodeQuery {
    pub session_id: Option<String>,
    pub continuity_id: Option<String>,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub role: Option<String>,
    pub limit: Option<usize>,
}

impl EpisodeQuery {
    pub fn new() -> Self { Self::default() }
    pub fn for_session(mut self, session_id: impl Into<String>) -> Self { self.session_id = Some(session_id.into()); self }
    pub fn for_continuity(mut self, cid: impl Into<String>) -> Self { self.continuity_id = Some(cid.into()); self }
    pub fn in_range(mut self, since: Option<i64>, until: Option<i64>) -> Self { self.since = since; self.until = until; self }
    pub fn with_role(mut self, role: impl Into<String>) -> Self { self.role = Some(role.into()); self }
    pub fn limit(mut self, n: usize) -> Self { self.limit = Some(n); self }
}

pub trait EpisodeStore {
    fn put_episode(&mut self, ep: Episode) -> Result<(), String>;
    fn get_episode(&self, id: &str) -> Option<Episode>;
    fn recent_episodes(&self, session_id: &str, n: usize) -> Vec<Episode>;
    fn query(&self, q: EpisodeQuery) -> Vec<Episode>;
}

pub struct InMemoryEpisodeStore {
    episodes: Vec<Episode>,
    max_capacity: usize,
}

impl InMemoryEpisodeStore {
    pub fn new(max_capacity: usize) -> Self { Self { episodes: Vec::with_capacity(max_capacity), max_capacity } }
    pub fn now_ms() -> i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64 }
}

impl EpisodeStore for InMemoryEpisodeStore {
    fn put_episode(&mut self, ep: Episode) -> Result<(), String> {
        if self.episodes.iter().any(|e| e.id == ep.id) { return Err(format!("duplicate: {}", ep.id)); }
        self.episodes.push(ep);
        if self.episodes.len() > self.max_capacity { self.episodes.remove(0); }
        Ok(())
    }
    fn get_episode(&self, id: &str) -> Option<Episode> { self.episodes.iter().find(|e| e.id == id).cloned() }
    fn recent_episodes(&self, session_id: &str, n: usize) -> Vec<Episode> {
        let mut f: Vec<_> = self.episodes.iter().filter(|e| e.session_id == session_id).cloned().collect();
        f.sort_by_key(|e| e.timestamp);
        f.into_iter().rev().take(n).collect()
    }
    fn query(&self, q: EpisodeQuery) -> Vec<Episode> {
        self.episodes.iter()
            .filter(|e| q.session_id.as_ref().map_or(true, |s| &e.session_id == s))
            .filter(|e| q.continuity_id.as_ref().map_or(true, |c| e.continuity_id.as_ref() == Some(c)))
            .filter(|e| q.since.map_or(true, |s| e.timestamp >= s))
            .filter(|e| q.until.map_or(true, |u| e.timestamp <= u))
            .filter(|e| q.role.as_ref().map_or(true, |r| &e.role == r))
            .cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mk(id: &str, sess: &str, ts: i64) -> Episode { Episode { id: id.into(), session_id: sess.into(), continuity_id: None, role: "user".into(), content: "x".into(), timestamp: ts, metadata: HashMap::new() } }
    #[test] fn test_put_and_get() { let mut s = InMemoryEpisodeStore::new(100); s.put_episode(mk("e1", "s1", 100)).unwrap(); assert!(s.get_episode("e1").is_some()); }
    #[test] fn test_duplicate_rejected() { let mut s = InMemoryEpisodeStore::new(100); s.put_episode(mk("e1", "s1", 100)).unwrap(); assert!(s.put_episode(mk("e1", "s1", 100)).is_err()); }
    #[test] fn test_recent() { let mut s = InMemoryEpisodeStore::new(100); s.put_episode(mk("e1", "s1", 100)).unwrap(); s.put_episode(mk("e2", "s1", 200)).unwrap(); let r = s.recent_episodes("s1", 1); assert_eq!(r[0].id, "e2"); }
    #[test] fn test_query_session() { let mut s = InMemoryEpisodeStore::new(100); s.put_episode(mk("e1", "s1", 100)).unwrap(); s.put_episode(mk("e2", "s2", 100)).unwrap(); let r = s.query(EpisodeQuery::new().for_session("s1")); assert_eq!(r.len(), 1); }
    #[test] fn test_query_range() { let mut s = InMemoryEpisodeStore::new(100); for i in 0..5 { s.put_episode(mk(&format!("e{}", i), "s", 100 + i * 100)).unwrap(); } let r = s.query(EpisodeQuery::new().in_range(Some(200), Some(400))); assert_eq!(r.len(), 3); }
    #[test] fn test_capacity_eviction() { let mut s = InMemoryEpisodeStore::new(2); for i in 0..5 { s.put_episode(mk(&format!("e{}", i), "s", i)).unwrap(); } assert_eq!(s.episodes.len(), 2); }
}
