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

/// v1-compatible IdentityCard — extends v2 surface with continuity_id/birth_time/
/// carriers/migration_history (per R14 R173 R177 apeireth-life-force integration).
///
/// ## v2 设计 (per apeireth-core R19 / 立体架构 v2)
/// - 字段名延续 v1 表面 (continuity_id / birth_time / carriers / migration_history)
///   以保 apeireth-life-force + apeireth-sovereignty 现有调用方零修改
/// - v2 新增字段 (name / version / philosophy_anchors / created_at) 与 v2 主路径
///   (governance / perception / cognition) 协同
/// - `default_companion()` 同时填两组字段, 现有 v2 调用方零修改
///
/// ## 不假装
/// - 不在 serde 层做 "v1 � v2 字段名映射" — 字段名直接同 v1, 旧 v1 数据可 round-trip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard {
    // v1 R14 surface (per apeireth-life-force / apeireth-sovereignty)
    /// 主体连续性 ID (跨载体同 ID — `did:apeireth:<slug>`).
    #[serde(default)]
    pub continuity_id: String,
    /// 主体出生时间 (epoch seconds).
    #[serde(default)]
    pub birth_time: i64,
    /// 载体列表 (跨设备/进程迁移历史).
    #[serde(default)]
    pub carriers: Vec<String>,
    /// 迁移历史 (carrier 切换记录).
    #[serde(default)]
    pub migration_history: Vec<Migration>,

    // v2 surface (per apeireth-core R19)
    /// 主体名 (e.g. "Apeireth Companion 2.0").
    #[serde(default)]
    pub name: String,
    /// 语义版本.
    #[serde(default = "default_version")]
    pub version: String,
    /// 哲学锚 (e.g. "0 Pretending", "Apeiron Emergence", "Tenant Sovereignty").
    #[serde(default)]
    pub philosophy_anchors: Vec<String>,
    /// 卡片创建时间 (UTC).
    #[serde(default = "default_now")]
    pub created_at: DateTime<Utc>,
}

fn default_version() -> String {
    "2.0.0".into()
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

impl Default for IdentityCard {
    fn default() -> Self {
        Self {
            continuity_id: format!("did:apeireth:{}", Uuid::new_v4()),
            birth_time: Utc::now().timestamp(),
            carriers: vec!["local".into()],
            migration_history: Vec::new(),
            name: "Apeireth Companion 2.0".into(),
            version: default_version(),
            philosophy_anchors: vec![
                "0 Pretending".into(),
                "Apeiron Emergence".into(),
                "Tenant Sovereignty".into(),
            ],
            created_at: Utc::now(),
        }
    }
}

impl IdentityCard {
    /// v2 默认 companion 构造 — 同时填 v1 + v2 字段 (兼容现有调用方).
    pub fn default_companion() -> Self {
        let now = Utc::now();
        Self {
            continuity_id: "did:apeireth:companion-default".into(),
            birth_time: now.timestamp(),
            carriers: vec!["companion-process".into()],
            migration_history: Vec::new(),
            name: "Apeireth Companion 2.0".into(),
            version: "2.0.0".into(),
            philosophy_anchors: vec![
                "0 Pretending".into(),
                "Apeiron Emergence".into(),
                "Tenant Sovereignty".into(),
            ],
            created_at: now,
        }
    }

    /// v1-style 构造: 仅指定 continuity_id + birth_time, 其他默认.
    pub fn with_continuity(continuity_id: impl Into<String>, birth_time: i64) -> Self {
        Self {
            continuity_id: continuity_id.into(),
            birth_time,
            carriers: vec!["local".into()],
            migration_history: Vec::new(),
            name: String::new(),
            version: default_version(),
            philosophy_anchors: Vec::new(),
            created_at: Utc::now(),
        }
    }
}

/// 主体载体迁移记录 (v1 表面 — 用于 apeireth-sovereignty continuity / apeireth-life-force tests).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Migration {
    /// 源 carrier 标识.
    pub from_carrier: String,
    /// 目标 carrier 标识.
    pub to_carrier: String,
    /// 迁移时间戳 (epoch seconds).
    pub timestamp: i64,
}

impl Migration {
    /// 构造一条迁移记录.
    pub fn new(
        from_carrier: impl Into<String>,
        to_carrier: impl Into<String>,
        timestamp: i64,
    ) -> Self {
        Self {
            from_carrier: from_carrier.into(),
            to_carrier: to_carrier.into(),
            timestamp,
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

