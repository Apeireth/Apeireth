//! Memory Episode - episode 存储 (抄 v1 apeireth-memory/episode.rs)
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct EpisodeQuery {
    pub session_id: Option<String>,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub role: Option<String>,
    pub limit: Option<usize>,
}

impl EpisodeQuery {
    pub fn new() -> Self { Self::default() }

    pub fn for_session(mut self, s: impl Into<String>) -> Self {
        self.session_id = Some(s.into());
        self
    }

    pub fn in_range(mut self, since: Option<i64>, until: Option<i64>) -> Self {
        self.since = since;
        self.until = until;
        self
    }

    pub fn with_role(mut self, r: impl Into<String>) -> Self {
        self.role = Some(r.into());
        self
    }

    pub fn limit(mut self, l: usize) -> Self {
        self.limit = Some(l);
        self
    }
}

/// Alias for clarity — many call sites (incl. apeireth-web) use this name.
pub type InMemoryEpisodeStore = EpisodeStore;

#[derive(Debug, Clone)]
pub struct EpisodeStore {
    pub episodes: Vec<Episode>,
    pub max_capacity: usize,
}

impl EpisodeStore {
    pub fn new(max_capacity: usize) -> Self {
        Self { episodes: Vec::new(), max_capacity }
    }

    pub fn append(&mut self, e: Episode) {
        if self.episodes.iter().any(|x| x.id == e.id) {
            return;
        }
        self.episodes.push(e);
        if self.episodes.len() > self.max_capacity {
            self.episodes.remove(0);
        }
    }

    pub fn by_session(&self, s: &str) -> Vec<&Episode> {
        self.episodes.iter().filter(|e| e.session_id == s).collect()
    }

    pub fn query(&self, q: &EpisodeQuery) -> Vec<&Episode> {
        let mut r: Vec<&Episode> = self.episodes.iter()
            .filter(|e| q.session_id.as_ref().map_or(true, |s| &e.session_id == s))
            .filter(|e| q.since.map_or(true, |t| e.timestamp >= t))
            .filter(|e| q.until.map_or(true, |t| e.timestamp <= t))
            .filter(|e| q.role.as_ref().map_or(true, |r| &e.role == r))
            .collect();
        if let Some(l) = q.limit {
            r.truncate(l);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_unique() {
        let mut s = EpisodeStore::new(10);
        s.append(Episode { id: "e1".into(), session_id: "s1".into(), role: "user".into(), content: "x".into(), timestamp: 0, metadata: HashMap::new() });
        s.append(Episode { id: "e1".into(), session_id: "s1".into(), role: "user".into(), content: "x".into(), timestamp: 0, metadata: HashMap::new() });
        assert_eq!(s.episodes.len(), 1);
    }

    #[test]
    fn test_query_by_session() {
        let mut s = EpisodeStore::new(10);
        s.append(Episode { id: "e1".into(), session_id: "s1".into(), role: "user".into(), content: "x".into(), timestamp: 0, metadata: HashMap::new() });
        let r = s.query(&EpisodeQuery::new().for_session("s1"));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_query_with_role() {
        let mut s = EpisodeStore::new(10);
        s.append(Episode { id: "e1".into(), session_id: "s1".into(), role: "user".into(), content: "x".into(), timestamp: 0, metadata: HashMap::new() });
        s.append(Episode { id: "e2".into(), session_id: "s1".into(), role: "assistant".into(), content: "y".into(), timestamp: 0, metadata: HashMap::new() });
        let r = s.query(&EpisodeQuery::new().for_session("s1").with_role("user"));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "e1");
    }

    #[test]
    fn test_query_with_limit() {
        let mut s = EpisodeStore::new(10);
        for i in 0..5 {
            s.append(Episode { id: format!("e{i}"), session_id: "s".into(), role: "user".into(), content: "x".into(), timestamp: i, metadata: HashMap::new() });
        }
        let r = s.query(&EpisodeQuery::new().for_session("s").limit(2));
        assert_eq!(r.len(), 2);
    }
}
