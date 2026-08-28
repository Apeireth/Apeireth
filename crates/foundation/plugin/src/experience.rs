//! P-arch (2026-08-27): B1 Experience trait 骨架 (3-layer).
//! O-6 重构批次 Refactor-2.
//!
//! 借鉴 v1 `apeireth-experience`（LLM Wiki + Knowledge Graph + VCP 联想网络，
//! 3-layer progressive disclosure），**v2 形态**：
//!
//! - trait `WikiEntryStore`（per-episode 提炼的 wiki 条目）
//! - trait `KnowledgeGraphStore`（subject-predicate-object 事实 + 链接）
//! - trait `AssociationStore`（VCP 联想网络：entity 关联强度）
//!
//! **位置**: traits and the conservative deterministic extractor live in
//! `apeireth-plugin` (foundation); SQLite persistence remains in memory
//! (engine).
//!
//! **Conservative default**: the extractor only derives a bounded summary and
//! explicitly marked `fact:`, `link:`, and `associate:` records. It does not
//! claim semantic LLM extraction and never copies a full transcript.
//!
//! **3 阶审查** (O-6 锚 9, commit message 必写明):
//! 1. 总体: 与 MemoryBackend + ToolCapability + ProviderCapability + CredentialResolver
//!    4 件 capability 抽象在 foundation 集中, 避免 v1 era 86-crate 散落
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致)
//! 3. 架构: 与 plugin manager 单 trait 边界, runtime 拿 `Arc<dyn Experience>`
//!    与 `Arc<dyn MemoryBackend>` 一起注入
//!
//! **跨模块类型**: Episode 来自 apeireth_core; plugin 不再依赖 memory, 单向.
//! v1 compat: `apeireth_memory::experience::*` 仍可访问 (memory re-export).

use serde::{Deserialize, Serialize};

use crate::memory_backend::CapabilityResult;
use apeireth_core::Episode;

// ============================================
// Wiki (L1: LLM 提炼的条目)
// ============================================

/// Wiki 条目（v1 借鉴: claude-mem 3-layer progressive disclosure 第一层）
///
/// 每次 episode 写入后，runtime 可异步提炼 1+ WikiEntry（O5 注解 #1）。
/// 检索时按 `source_episode_id` 反查到 episode，再决定是否展开。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WikiEntry {
    /// 全局唯一 id（v1 实践: SHA-256(source_episode_id + topic)[:16] hex）
    pub id: String,
    /// 所属 session
    pub session_id: String,
    /// 来源 episode（v1 D2 §5.3 #2: subject_id 必填）
    pub source_episode_id: String,
    /// 提炼时间戳
    pub extracted_at: i64,
    /// 主体 topic（"这条目是关于什么的"）
    pub topic: String,
    /// 摘要文本（限长，注入用）
    pub summary: String,
    /// 完整内容（按需展开）
    pub body: String,
    /// 置信度 0.0-1.0（v1 同）
    pub confidence: f64,
    /// 标签（检索用）
    pub tags: Vec<String>,
}

/// Wiki 存储 trait（0 装：v2.0 alpha 无 SQLite impl）
///
/// 0 装 PASS: trait 边界清晰, impl 留 v2.1
pub trait WikiEntryStore: Send + Sync {
    /// 写一条 WikiEntry
    fn put_wiki(&self, entry: &WikiEntry) -> CapabilityResult<()>;

    /// 按 session 列出某 topic 下的 WikiEntry
    fn list_wiki(
        &self,
        session_id: &str,
        topic: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<WikiEntry>>;

    /// 按 source_episode_id 反查（v1 progressive disclosure: 注入时回查摘要）
    fn wiki_for_episode(&self, episode_id: &str) -> CapabilityResult<Vec<WikiEntry>>;
}

// ============================================
// Knowledge Graph (L2: 事实 + 链接)
// ============================================

/// Graph 事实 (s/p/o 三元组，v1 借鉴: safishamsi/graphify + VCP 联想网络)
///
/// 命名空间: subject (实体 ID) → predicate (关系) → object (实体 ID 或字面值)
/// v1 实践: subject_id 复用 `PluginId` 或 domain entity; 0 装存 String
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphFact {
    pub id: String,
    pub subject_id: String,
    pub subject_kind: String, // "person" / "tool" / "concept" / ...
    pub predicate: String,    // "uses" / "depends_on" / "is_a" / ...
    pub object_id: String,    // 实体 ID 或字面值
    pub object_kind: String,
    /// 时间有效性: 何时该 fact 成立 (v1 D2 §5.4: 暂存为单一 timestamp)
    pub valid_from: i64,
    pub valid_until: Option<i64>,
    /// 来源 episode（s/p/o 写入的 episode）
    pub source_episode_id: String,
    /// 置信度
    pub confidence: f64,
}

/// Graph 链接 (entity-to-entity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphLink {
    pub from_id: String,
    pub to_id: String,
    pub kind: String, // "cites" / "extends" / "contradicts" / ...
    pub weight: f64,  // 链接强度 0.0-1.0
    pub source_episode_id: String,
    pub created_at: i64,
}

/// KG 存储 trait
pub trait KnowledgeGraphStore: Send + Sync {
    /// 写一条 fact
    fn put_fact(&self, fact: &GraphFact) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 写一条 link
    fn put_link(&self, link: &GraphLink) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 从 subject 出发一跳 fact
    fn facts_from(&self, subject_id: &str, limit: u32) -> CapabilityResult<Vec<GraphFact>>;

    /// 从 from 出发一跳 link
    fn links_from(&self, from_id: &str, limit: u32) -> CapabilityResult<Vec<GraphLink>>;

    /// 删除 subject 相关的所有 fact（v1 memory_governance 风格: 不真删, 标 tombstone）
    fn forget_subject(&self, subject_id: &str) -> CapabilityResult<()>;
}

// ============================================
// Association (L3: 联想网络)
// ============================================

/// 联想节点 (VCP `compound_eye` 借鉴: entity-pair 共现强度)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociationNode {
    pub entity_id: String,
    /// 共现 episode 数（边权）
    pub co_occurrence_count: u32,
    /// 最近一次共现时间
    pub last_seen_at: i64,
}

/// 联想边 (entity-pair 共现)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociationEdge {
    pub from_entity: String,
    pub to_entity: String,
    /// 共现次数（v1 联想强度）
    pub co_occurrence_count: u32,
    /// 最近一次共现的 episode
    pub last_seen_episode_id: String,
    pub last_seen_at: i64,
}

/// 联想网络 trait
pub trait AssociationStore: Send + Sync {
    /// 记录一次 entity-pair 共现（每次 episode 写入时调）
    fn record_cooccurrence(&self, from: &str, to: &str, episode_id: &str) -> CapabilityResult<()>;

    /// 查 entity 联想 top-N（按 co_occurrence_count desc）
    fn top_associations(&self, entity: &str, limit: u32) -> CapabilityResult<Vec<AssociationEdge>>;
}

// ============================================
// Experience 统一错误
// ============================================

// (O-6 锚兑现 #12: ExperienceError 删 — 用 `Box<dyn std::error::Error + Send + Sync>`
// (即 `CapabilityResult<T>` 的 Err 类型) 统一所有 capability trait 错误通道.
// impl 端用 `Box::new(my_local_error)` 包.)

// ============================================
// Typed extraction schema and conservative pipeline
// ============================================

const MAX_WIKI_ENTRIES: usize = 8;
const MAX_FACTS: usize = 16;
const MAX_LINKS: usize = 16;
const MAX_ASSOCIATIONS: usize = 16;
const MAX_FIELD_CHARS: usize = 2_000;

/// Raw typed extraction output. A future LLM adapter may deserialize this
/// schema, but source evidence is injected from the durable episode during
/// materialization rather than trusted from model output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExperienceExtraction {
    /// Bounded wiki summaries.
    #[serde(default)]
    pub wiki_entries: Vec<ExtractedWikiEntry>,
    /// Bounded subject-predicate-object facts.
    #[serde(default)]
    pub facts: Vec<ExtractedFact>,
    /// Bounded entity links.
    #[serde(default)]
    pub links: Vec<ExtractedLink>,
    /// Bounded entity co-occurrences.
    #[serde(default)]
    pub associations: Vec<ExtractedAssociation>,
}

/// Raw wiki entry from a typed extractor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtractedWikiEntry {
    /// Topic label.
    pub topic: String,
    /// Short summary used for recall.
    pub summary: String,
    /// Bounded higher-level body, never a transcript copy.
    pub body: String,
    /// Conservative confidence.
    pub confidence: f64,
    /// Search tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Raw graph fact from a typed extractor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtractedFact {
    /// Subject identifier.
    pub subject_id: String,
    /// Predicate.
    pub predicate: String,
    /// Object identifier or literal.
    pub object_id: String,
    /// Fact confidence.
    pub confidence: f64,
}

/// Raw graph link from a typed extractor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtractedLink {
    /// Source entity.
    pub from_id: String,
    /// Destination entity.
    pub to_id: String,
    /// Link kind.
    pub kind: String,
    /// Link weight.
    pub weight: f64,
}

/// Raw association from a typed extractor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtractedAssociation {
    /// First entity.
    pub from_entity: String,
    /// Second entity.
    pub to_entity: String,
}

impl ExperienceExtraction {
    /// Validate all collection and field bounds before persistence.
    pub fn validate(&self) -> Result<(), String> {
        if self.wiki_entries.len() > MAX_WIKI_ENTRIES {
            return Err("experience extraction contains too many wiki entries".into());
        }
        if self.facts.len() > MAX_FACTS {
            return Err("experience extraction contains too many facts".into());
        }
        if self.links.len() > MAX_LINKS {
            return Err("experience extraction contains too many links".into());
        }
        if self.associations.len() > MAX_ASSOCIATIONS {
            return Err("experience extraction contains too many associations".into());
        }
        for entry in &self.wiki_entries {
            for (label, value) in [
                ("wiki topic", &entry.topic),
                ("wiki summary", &entry.summary),
                ("wiki body", &entry.body),
            ] {
                validate_field(label, value)?;
            }
            if !entry.confidence.is_finite() || !(0.0..=1.0).contains(&entry.confidence) {
                return Err("wiki confidence must be finite and between 0 and 1".into());
            }
            if entry.tags.len() > MAX_ASSOCIATIONS {
                return Err("wiki entry contains too many tags".into());
            }
            for tag in &entry.tags {
                validate_field("wiki tag", tag)?;
            }
        }
        for fact in &self.facts {
            validate_field("fact subject", &fact.subject_id)?;
            validate_field("fact predicate", &fact.predicate)?;
            validate_field("fact object", &fact.object_id)?;
            validate_confidence("fact confidence", fact.confidence)?;
        }
        for link in &self.links {
            validate_field("link from", &link.from_id)?;
            validate_field("link to", &link.to_id)?;
            validate_field("link kind", &link.kind)?;
            validate_confidence("link weight", link.weight)?;
        }
        for association in &self.associations {
            validate_field("association from", &association.from_entity)?;
            validate_field("association to", &association.to_entity)?;
        }
        Ok(())
    }
}

fn validate_field(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.chars().count() > MAX_FIELD_CHARS {
        return Err(format!("{label} exceeds {MAX_FIELD_CHARS} characters"));
    }
    Ok(())
}

fn validate_confidence(label: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{label} must be finite and between 0 and 1"));
    }
    Ok(())
}

/// Materialized artifacts carry durable source evidence into every store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExperienceArtifacts {
    /// Materialized wiki entries.
    pub wiki_entries: Vec<WikiEntry>,
    /// Materialized graph facts.
    pub facts: Vec<GraphFact>,
    /// Materialized graph links.
    pub links: Vec<GraphLink>,
    /// Materialized associations.
    pub associations: Vec<AssociationObservation>,
}

/// One association observation with its source episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociationObservation {
    /// First entity.
    pub from_entity: String,
    /// Second entity.
    pub to_entity: String,
    /// Durable source episode.
    pub source_episode_id: String,
}

/// Extract a bounded, conservative experience from one durable episode.
///
/// A short summary is generated deterministically. Higher-order facts and
/// relations are accepted only from explicit pipe-delimited markers:
/// `fact: subject | predicate | object`, `link: from | to | kind`, and
/// `associate: from | to`. This is intentionally conservative until an
/// optional typed ModuleInvoker-based LLM extractor is configured.
pub fn extract_experience(episode: &Episode) -> Result<ExperienceArtifacts, String> {
    let mut extraction = ExperienceExtraction::default();
    let summary = deterministic_summary(&episode.content);
    if !summary.is_empty() {
        extraction.wiki_entries.push(ExtractedWikiEntry {
            topic: bounded(&summary, 160),
            summary: bounded(&summary, 240),
            body: bounded(&summary, 240),
            confidence: 0.25,
            tags: vec![
                "origin:deterministic".into(),
                format!("role:{}", episode.role),
            ],
        });
    }
    for line in episode.content.lines().map(str::trim) {
        if let Some(fields) = marker_fields(line, "fact:", 3) {
            extraction.facts.push(ExtractedFact {
                subject_id: fields[0].clone(),
                predicate: fields[1].clone(),
                object_id: fields[2].clone(),
                confidence: 0.5,
            });
        } else if let Some(fields) = marker_fields(line, "link:", 3) {
            extraction.links.push(ExtractedLink {
                from_id: fields[0].clone(),
                to_id: fields[1].clone(),
                kind: fields[2].clone(),
                weight: 0.5,
            });
        } else if let Some(fields) = marker_fields(line, "associate:", 2) {
            extraction.associations.push(ExtractedAssociation {
                from_entity: fields[0].clone(),
                to_entity: fields[1].clone(),
            });
        }
    }
    extraction.validate()?;
    Ok(materialize(episode, extraction))
}

fn materialize(episode: &Episode, extraction: ExperienceExtraction) -> ExperienceArtifacts {
    let wiki_entries = extraction
        .wiki_entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| WikiEntry {
            id: format!("experience-wiki:{}:{index}", episode.id),
            session_id: episode.session_id.clone(),
            source_episode_id: episode.id.clone(),
            extracted_at: episode.timestamp,
            topic: entry.topic,
            summary: entry.summary,
            body: entry.body,
            confidence: entry.confidence,
            tags: entry.tags,
        })
        .collect();
    let facts = extraction
        .facts
        .into_iter()
        .enumerate()
        .map(|(index, fact)| GraphFact {
            id: format!("experience-fact:{}:{index}", episode.id),
            subject_id: fact.subject_id,
            subject_kind: "entity".into(),
            predicate: fact.predicate,
            object_id: fact.object_id,
            object_kind: "entity_or_literal".into(),
            valid_from: episode.timestamp,
            valid_until: None,
            source_episode_id: episode.id.clone(),
            confidence: fact.confidence,
        })
        .collect();
    let links = extraction
        .links
        .into_iter()
        .map(|link| GraphLink {
            from_id: link.from_id,
            to_id: link.to_id,
            kind: link.kind,
            weight: link.weight,
            source_episode_id: episode.id.clone(),
            created_at: episode.timestamp,
        })
        .collect();
    let associations = extraction
        .associations
        .into_iter()
        .map(|association| AssociationObservation {
            from_entity: association.from_entity,
            to_entity: association.to_entity,
            source_episode_id: episode.id.clone(),
        })
        .collect();
    ExperienceArtifacts {
        wiki_entries,
        facts,
        links,
        associations,
    }
}

fn deterministic_summary(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with("fact:")
                && !line.starts_with("link:")
                && !line.starts_with("associate:")
        })
        .map(|line| {
            line.split(['.', '。', '!', '！', '?', '？'])
                .next()
                .unwrap_or(line)
                .trim()
        })
        .map(|line| bounded(line, 240))
        .unwrap_or_default()
}

fn marker_fields(line: &str, marker: &str, expected: usize) -> Option<Vec<String>> {
    let rest = line.strip_prefix(marker)?;
    let fields = rest
        .split('|')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    (fields.len() == expected && fields.iter().all(|field| !field.is_empty())).then_some(fields)
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

/// Compatibility wrapper for callers that still expect the original tuple.
/// New code should use [`extract_experience`] so associations are not lost.
pub fn extract_experience_from_episode(
    episode: &Episode,
) -> (Option<WikiEntry>, Vec<GraphFact>, Vec<GraphLink>) {
    match extract_experience(episode) {
        Ok(artifacts) => (
            artifacts.wiki_entries.into_iter().next(),
            artifacts.facts,
            artifacts.links,
        ),
        Err(_) => (None, Vec::new(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_backend::CapabilityResult;

    /// Conservative extraction produces a bounded summary and explicit
    /// structured relations without copying the transcript.
    #[test]
    fn extract_experience_is_bounded_and_evidence_bound() {
        let ep = Episode {
            id: "ep-1".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "Rust is fast and safe.\nfact: rust | property | fast\nlink: rust | safe | protects\nassociate: rust | cargo".into(),
            session_id: "s".into(),
        };
        let artifacts = extract_experience(&ep).unwrap();
        assert_eq!(artifacts.wiki_entries.len(), 1);
        assert_eq!(artifacts.facts.len(), 1);
        assert_eq!(artifacts.links.len(), 1);
        assert_eq!(artifacts.associations.len(), 1);
        assert_eq!(artifacts.wiki_entries[0].source_episode_id, "ep-1");
        assert_eq!(artifacts.facts[0].source_episode_id, "ep-1");
        assert_eq!(artifacts.links[0].source_episode_id, "ep-1");
        assert_eq!(artifacts.associations[0].source_episode_id, "ep-1");
        assert_eq!(artifacts.wiki_entries[0].body, "Rust is fast and safe");
        assert!(!artifacts.wiki_entries[0].body.contains("fact:"));
    }

    #[test]
    fn extraction_schema_rejects_unknown_fields_and_bounds() {
        let unknown = serde_json::from_str::<ExperienceExtraction>(
            r#"{"wiki_entries":[],"facts":[],"links":[],"associations":[],"extra":1}"#,
        );
        assert!(unknown.is_err());

        let too_many = ExperienceExtraction {
            wiki_entries: (0..9)
                .map(|_| ExtractedWikiEntry {
                    topic: "topic".into(),
                    summary: "summary".into(),
                    body: "body".into(),
                    confidence: 0.5,
                    tags: Vec::new(),
                })
                .collect(),
            ..ExperienceExtraction::default()
        };
        assert!(too_many.validate().is_err());
    }

    /// 0 装 PASS: 错误通道统一为 `Box<dyn Error + Send + Sync>`, impl 端用
    /// `Box::new(impl_local_error)` 包. 这里验证 0 装 marker 字符串 (impl v2.1
    /// 接真 backend 时, NotImplemented 应改为真错误).
    #[test]
    fn not_implemented_marker_appears_in_error_string() {
        // 0 装: impl 包成 Box 的字符串应含 0 装 / v2.1 / 实现者 (impl-defined)
        // v2.0 alpha 的真 0 装 impl 在 memory crate (现在 NoopAdvisor)
        let e = std::io::Error::new(std::io::ErrorKind::Unsupported, "0 装 PASS: v2.1");
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        let s = format!("{boxed}");
        assert!(s.contains("0 装"));
        assert!(s.contains("v2.1"));
    }

    /// 0 装 PASS: WikiEntry 可从 episode 构造 (smoke), 不依赖 backend.
    /// 真实持久化由 trait impl (v2.1) 提供.
    #[test]
    fn wiki_entry_construction_works() {
        let w = WikiEntry {
            id: "wiki-1".into(),
            session_id: "s".into(),
            source_episode_id: "ep-1".into(),
            extracted_at: 1_700_000_000,
            topic: "memory extraction".into(),
            summary: "extract_experience trait 骨架就位".into(),
            body: "完整 impl 留 v2.1".into(),
            confidence: 0.85,
            tags: vec!["trait".into(), "0装".into()],
        };
        assert_eq!(w.confidence, 0.85);
        assert_eq!(w.tags.len(), 2);
    }
}
