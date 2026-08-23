use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EpisodeKind {
    Thought,
    Action,
    Observation,
    Reflection,
    Evolution,
    Dream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub session_id: Uuid,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub hash_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub session_id: Uuid,
    pub kind: EpisodeKind,
    pub content: String,
    pub importance: f64,
    pub timestamp: DateTime<Utc>,
}

impl Episode {
    pub fn new(session_id: Uuid, kind: EpisodeKind, content: impl Into<String>, importance: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            kind,
            content: content.into(),
            importance,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Note {
    pub fn new(title: impl Into<String>, content: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            content: content.into(),
            tags,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub episodes: Vec<Episode>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            episodes: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_episode(&mut self, kind: EpisodeKind, content: impl Into<String>, importance: f64) -> Uuid {
        let ep = Episode::new(self.id, kind, content, importance);
        let id = ep.id;
        self.episodes.push(ep);
        id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard {
    pub name: String,
    pub version: String,
    pub philosophy_anchors: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl IdentityCard {
    pub fn default_companion() -> Self {
        Self {
            name: "Apeireth Companion 2.0".into(),
            version: "2.0.0".into(),
            philosophy_anchors: vec![
                "0 Pretending".into(),
                "Apeiron Emergence".into(),
                "Tenant Sovereignty".into(),
            ],
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_and_session() {
        let mut sess = Session::new("Test Session");
        let ep_id = sess.add_episode(EpisodeKind::Thought, "Thinking about architecture", 0.85);
        
        assert_eq!(sess.episodes.len(), 1);
        assert_eq!(sess.episodes[0].id, ep_id);
        assert_eq!(sess.episodes[0].kind, EpisodeKind::Thought);
        assert_eq!(sess.episodes[0].importance, 0.85);
    }

    #[test]
    fn test_identity_card() {
        let card = IdentityCard::default_companion();
        assert_eq!(card.name, "Apeireth Companion 2.0");
        assert!(card.philosophy_anchors.contains(&"0 Pretending".to_string()));
    }
}

