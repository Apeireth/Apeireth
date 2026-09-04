//! B2 · Phase 1: 派生记忆图与遗忘传播审计（Research 前缀，默认关闭）。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**: 记忆遗忘是单点操作（`forget_episode` 只软删根 episode），
//!   衍生面（note/diary/wiki/chronicle/cache）不经血缘传播继续泄漏已遗忘事实。
//! - **假设**: 显式血缘表 + 闭包计算（taint / support(θ)）可以**只审计不删除**地
//!   量化泄漏面；血缘完整度与召回安全构成可测的权衡。
//! - **状态**: 原型已实现（血缘表、闭包、审计、四类探针全部确定性可测）。
//!   LLM-as-judge 双评者为 trait 口，真 LLM 接线留部署层（0 装）。
//! - **引用**: `_research_mem/ra/ra1-candidate-algorithms.md` A.4.1–A.4.4；
//!   治理语义锚定 `memory_governance.rs` V6 sidecar（forgotten 从默认检索排除）。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`（3061 passed）。
//! - **已知局限**: ① diary/wiki/chronicle 是内存计算引擎（无 store 句柄），
//!   血缘由调用方经 helper API 显式登记；② 缓存代际联动为整代失效（粗粒度），
//!   按 query 级驱逐留后续；③ 闭包审计只写本模块的 append-only 事件表，
//!   不改 `episode_governance` 任何行。
//!
//! # 默认关闭（铁律 1 + B2 闸门）
//! - `GovernedRecall` 不挂任何生产检索路径；只有显式调用才过滤。
//! - 闭包/审计**不自动删除任何数据**：结果只写入 `research_lineage_events`。
//! - 旧 `forget_episode` 语义不变（本模块零触碰 `episode_governance`）。

use std::collections::{HashMap, HashSet, VecDeque};

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::gen_cache::GenerationCache;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 派生/来源引用（血缘图中的节点）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DerivedRef {
    /// 'episode' | 'stream' | 'note' | 'diary' | 'wiki' | 'chronicle' | 'cache'
    pub kind: String,
    pub id: String,
}

impl DerivedRef {
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
}

/// 闭包传播模式（RA-1 A.4.3）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClosureMode {
    /// 任一祖先被遗忘 ⇒ 派生项进入闭包。
    Taint,
    /// 丢失来源占比 ≥ θ 才进入闭包（保留部分证据存活的派生项）。
    Support { theta: f64 },
}

impl ClosureMode {
    fn desc(&self) -> String {
        match self {
            Self::Taint => "taint".into(),
            Self::Support { theta } => format!("support({theta})"),
        }
    }
}

/// 闭包中的单个节点（含触发链信息，供审计报告引用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureNode {
    pub kind: String,
    pub id: String,
    /// 距根集的最短传播深度（根 = 0）。
    pub depth: usize,
    /// 触发本节点入闭包的最近祖先（根节点为 None）。
    pub triggered_by: Option<DerivedRef>,
}

/// 闭包计算结果（纯审计：不修改任何产品表）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureReport {
    pub roots: Vec<DerivedRef>,
    pub mode: String,
    /// 闭包节点（含根），按传播广度序。
    pub nodes: Vec<ClosureNode>,
    /// 写入审计事件表后返回的 seq（可溯源）。
    pub audit_event_seq: Option<i64>,
    /// 语义保证：本模块永不删除数据。
    pub deleted_anything: bool,
}

/// 泄漏审计项（forget_propagation_audit 输出行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakAuditItem {
    pub kind: String,
    pub id: String,
    /// 已遗忘来源数 / 来源总数。
    pub lost_sources: usize,
    pub total_sources: usize,
    /// 按当前模式是否应进入闭包。
    pub in_closure: bool,
    /// 已知血缘边的 span 摘要（证明推导关系，空 = 无显式 span）。
    pub span: Option<String>,
}

/// forget_propagation_audit 报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakAuditReport {
    pub forgotten_roots: Vec<DerivedRef>,
    pub mode: String,
    pub items: Vec<LeakAuditItem>,
    /// 未在任何血缘表中出现的派生类，无法审计（诚实标注）。
    pub unobservable_note: String,
}

/// 双评者合并结果（LLM-as-judge 协议）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DualRaterResult {
    /// 双评者一致。
    Agree { leaked: bool },
    /// 不一致：保守取泄漏=真（安全侧）。
    Disagree { a: bool, b: bool, conservative: bool },
}

/// LLM-as-judge 评审判定（真 LLM 留部署层，测试用确定性 stub）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVerdict {
    pub leaked: bool,
    pub confidence: f32,
    pub rationale: String,
}

/// 评审 trait 口（0 装：本 crate 只提供协议与确定性 stub）。
pub trait ResearchJudge {
    /// 给定探针问题与候选文本（如召回片段），判定是否泄漏已遗忘事实。
    fn judge(&self, question: &str, candidate_text: &str) -> JudgeVerdict;
}

/// 双评者协议：两个 judge 独立评分；一致取共同结论，不一致保守取"泄漏"。
pub fn dual_rater_protocol(a: &JudgeVerdict, b: &JudgeVerdict) -> DualRaterResult {
    if a.leaked == b.leaked {
        DualRaterResult::Agree { leaked: a.leaked }
    } else {
        DualRaterResult::Disagree {
            a: a.leaked,
            b: b.leaked,
            conservative: true,
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl SqliteMemoryStore {
    /// 登记派生关系：d 由来源集 S 生成（RA-1 A.4.1 + A5 派生必记血缘）。
    /// 幂等（主键去重）；span 可选（引用片段/offset 摘要）。
    pub fn research_record_derivation(
        &self,
        derived: &DerivedRef,
        sources: &[DerivedRef],
        span: Option<&str>,
    ) -> MemoryResult<usize> {
        if sources.is_empty() {
            return Err(MemoryError::Invalid(
                "research_record_derivation: sources must not be empty".into(),
            ));
        }
        let conn = self.conn()?;
        let ts = now_ms();
        let mut inserted = 0usize;
        for s in sources {
            let n = conn.execute(
                "INSERT OR IGNORE INTO research_derived_from \
                 (derived_kind, derived_id, source_kind, source_id, span, ts) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![derived.kind, derived.id, s.kind, s.id, span, ts],
            )?;
            inserted += n;
        }
        Ok(inserted)
    }

    /// 写一条 append-only 审计事件（A8），返回 seq。
    fn research_write_event(
        &self,
        op: &str,
        actor: Option<&str>,
        reason: Option<&str>,
        subject: &str,
        detail: &serde_json::Value,
    ) -> MemoryResult<i64> {
        let conn = self.conn()?;
        let ts = now_ms();
        conn.execute(
            "INSERT INTO research_lineage_events \
             (op, actor, ts, reason, subject, revision_before, revision_after, detail_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6)",
            params![op, actor, ts, reason, subject, detail.to_string()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 遗忘闭包（RA-1 A.4.3）：以 roots 为根集，沿血缘表做 BFS 传播。
    /// **只审计不删除**：结果仅写 `research_lineage_events`，不触碰任何产品表。
    pub fn research_forget_closure(
        &self,
        roots: &[DerivedRef],
        mode: ClosureMode,
        actor: Option<&str>,
        reason: Option<&str>,
    ) -> MemoryResult<ClosureReport> {
        let conn = self.conn()?;
        let mut closure: HashSet<DerivedRef> = HashSet::new();
        let mut nodes: Vec<ClosureNode> = Vec::new();
        let mut queue: VecDeque<(DerivedRef, usize, Option<DerivedRef>)> = VecDeque::new();
        for r in roots {
            queue.push_back((r.clone(), 0, None));
        }
        while let Some((node, depth, triggered_by)) = queue.pop_front() {
            if closure.contains(&node) {
                continue;
            }
            closure.insert(node.clone());
            nodes.push(ClosureNode {
                kind: node.kind.clone(),
                id: node.id.clone(),
                depth,
                triggered_by,
            });
            // 派生后代（谁把本节点列为来源）。
            let mut stmt = conn.prepare(
                "SELECT derived_kind, derived_id FROM research_derived_from \
                 WHERE source_kind = ?1 AND source_id = ?2",
            )?;
            let rows = stmt.query_map(params![node.kind, node.id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut derived: Vec<(String, String)> = Vec::new();
            for row in rows {
                let (k, i) = row?;
                if closure.contains(&DerivedRef { kind: k.clone(), id: i.clone() }) {
                    continue;
                }
                derived.push((k, i));
            }
            drop(stmt);
            for (dk, di) in derived {
                let dref = DerivedRef { kind: dk, id: di };
                match &mode {
                    ClosureMode::Taint => {
                        queue.push_back((dref, depth + 1, Some(node.clone())));
                    }
                    ClosureMode::Support { theta } => {
                        // 已丢失来源 = 与当前闭包的交集（按已入闭包节点计）。
                        let total = self.research_source_count(&conn, &dref)?;
                        if total == 0 {
                            continue;
                        }
                        let lost = self.research_lost_source_count(&conn, &dref, &closure)?;
                        if (lost as f64) >= theta * (total as f64) {
                            queue.push_back((dref, depth + 1, Some(node.clone())));
                        }
                    }
                }
            }
        }
        drop(conn);
        let detail = serde_json::json!({
            "roots": roots,
            "mode": mode.desc(),
            "closure_size": nodes.len(),
            "deleted_anything": false,
        });
        let seq = self.research_write_event("forget_closure", actor, reason, "closure", &detail)?;
        Ok(ClosureReport {
            roots: roots.to_vec(),
            mode: mode.desc(),
            nodes,
            audit_event_seq: Some(seq),
            deleted_anything: false,
        })
    }

    /// forget_propagation_audit：给定已遗忘根集，列出所有血缘可见派生项的泄漏状态。
    /// 只读审计（不写事件、不删数据、不改状态）。
    pub fn research_audit_forgotten_leaks(
        &self,
        forgotten: &[DerivedRef],
        mode: ClosureMode,
    ) -> MemoryResult<LeakAuditReport> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT derived_kind, derived_id, span FROM research_derived_from \
             WHERE source_kind = ?1 AND source_id = ?2",
        )?;
        let mut items: Vec<LeakAuditItem> = Vec::new();
        let mut seen: HashSet<DerivedRef> = HashSet::new();
        for f in forgotten {
            let rows = stmt.query_map(params![f.kind, f.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows {
                let (dk, di, span) = row?;
                let dref = DerivedRef { kind: dk, id: di };
                if !seen.insert(dref.clone()) {
                    continue;
                }
                let total = self.research_source_count(&conn, &dref)?;
                let lost = self.research_lost_source_count(&conn, &dref, &forgotten.iter().cloned().collect::<HashSet<_>>())?;
                let in_closure = match &mode {
                    ClosureMode::Taint => lost > 0,
                    ClosureMode::Support { theta } => {
                        total > 0 && (lost as f64) >= theta * (total as f64)
                    }
                };
                items.push(LeakAuditItem {
                    kind: dref.kind,
                    id: dref.id,
                    lost_sources: lost,
                    total_sources: total,
                    in_closure,
                    span,
                });
            }
        }
        Ok(LeakAuditReport {
            forgotten_roots: forgotten.to_vec(),
            mode: mode.desc(),
            items,
            unobservable_note: "血缘表外的派生面（未登记 derive 的 diary/wiki/chronicle/cache）无法审计，需先经 research_record_derivation 登记".into(),
        })
    }

    fn research_source_count(&self, conn: &rusqlite::Connection, d: &DerivedRef) -> MemoryResult<usize> {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM research_derived_from \
                 WHERE derived_kind = ?1 AND derived_id = ?2",
                params![d.kind, d.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(MemoryError::from)?
            .unwrap_or(0);
        Ok(n as usize)
    }

    fn research_lost_source_count(
        &self,
        conn: &rusqlite::Connection,
        d: &DerivedRef,
        lost_set: &HashSet<DerivedRef>,
    ) -> MemoryResult<usize> {
        let mut stmt = conn.prepare(
            "SELECT source_kind, source_id FROM research_derived_from \
             WHERE derived_kind = ?1 AND derived_id = ?2",
        )?;
        let rows = stmt.query_map(params![d.kind, d.id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut lost = 0usize;
        for row in rows {
            let (sk, si) = row?;
            if lost_set.contains(&DerivedRef { kind: sk, id: si }) {
                lost += 1;
            }
        }
        Ok(lost)
    }

    /// 从 notes.source_episode_ids_json 回填血缘（note 侧"已有"的桥接，只读导入）。
    /// 返回导入的边数。notes 无来源的行跳过。
    pub fn research_import_note_lineage(&self) -> MemoryResult<usize> {
        let notes: Vec<(String, Vec<String>)> = {
            let conn = self.conn()?;
            let mut stmt = conn.prepare("SELECT id, source_episode_ids_json FROM notes")?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (id, json) = row?;
                let ids: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
                out.push((id, ids));
            }
            out
        }; // conn guard 在此释放 — 后续 record 各自取锁, 避免同锁嵌套死锁.
        let mut imported = 0usize;
        for (id, ids) in notes {
            if ids.is_empty() {
                continue;
            }
            let sources: Vec<DerivedRef> = ids
                .into_iter()
                .map(|eid| DerivedRef::new("episode", eid))
                .collect();
            imported += self.research_record_derivation(
                &DerivedRef::new("note", id),
                &sources,
                Some("backfilled from notes.source_episode_ids_json"),
            )?;
        }
        Ok(imported)
    }
}

/// 遗忘-缓存代际联动（RA-1 泄漏向量 1 的粗粒度缓解）：
/// 闭包中出现**派生节点（depth > 0）**⇒ 整代失效（advance），强制下游重算，
/// 杜绝旧代派生快照被继续命中。仅根集遗忘（无派生受影响）不推进。
/// 细粒度（按 query_hash 驱逐）留后续；返回当前代际。
pub fn research_invalidate_cache_on_forget<V>(
    cache: &GenerationCache<V>,
    report: &ClosureReport,
) -> u64 {
    let derived_affected = report.nodes.iter().any(|n| n.depth > 0);
    if !derived_affected {
        return cache.generation();
    }
    cache.advance()
}

/// GovernedRecall：血缘感知的召回过滤门面。**默认关闭**——不挂任何生产检索路径，
/// 只有显式构造并调用 `recall` 才生效（B2 闸门：不替换生产检索）。
pub struct GovernedRecall {
    store: std::sync::Arc<SqliteMemoryStore>,
    /// 过滤模式；None = 不过滤（纯透传，等价于无治理召回）。
    mode: Option<ClosureMode>,
    /// 遗忘根集（显式提供；空 = 不过滤任何东西）。
    forgotten: HashSet<DerivedRef>,
}

impl GovernedRecall {
    pub fn new(store: std::sync::Arc<SqliteMemoryStore>) -> Self {
        Self {
            store,
            mode: None,
            forgotten: HashSet::new(),
        }
    }

    /// 启用过滤（显式调用才生效；不调用 = 默认关闭）。
    pub fn with_filter(mut self, mode: ClosureMode, forgotten: Vec<DerivedRef>) -> Self {
        self.mode = Some(mode);
        self.forgotten = forgotten.into_iter().collect();
        self
    }

    /// 对候选派生项做血缘过滤：闭包内的项被排除；血缘不可见的项放行并标注
    /// （不假装能过滤未登记的血缘）。返回 (放行项, 被滤项)。
    pub fn recall(
        &self,
        candidates: Vec<DerivedRef>,
    ) -> MemoryResult<(Vec<DerivedRef>, Vec<DerivedRef>)> {
        let Some(mode) = &self.mode else {
            return Ok((candidates, Vec::new()));
        };
        if self.forgotten.is_empty() {
            return Ok((candidates, Vec::new()));
        }
        let mut kept = Vec::new();
        let mut filtered = Vec::new();
        for c in candidates {
            let guard = self.store.conn()?;
            let conn_ref: &rusqlite::Connection = &guard;
            let total = self.store.research_source_count(conn_ref, &c)?;
            if total == 0 {
                // 血缘不可见：不假装可过滤，放行（0 装）。
                kept.push(c);
                continue;
            }
            let lost = self
                .store
                .research_lost_source_count(conn_ref, &c, &self.forgotten)?;
            let drop = match mode {
                ClosureMode::Taint => lost > 0,
                ClosureMode::Support { theta } => (lost as f64) >= theta * (total as f64),
            };
            if drop {
                filtered.push(c);
            } else {
                kept.push(c);
            }
        }
        Ok((kept, filtered))
    }
}

/// 确定性 stub judge（测试与基准用；真 LLM judge 经 `ResearchJudge` trait 在部署层接入）。
#[derive(Debug, Clone)]
pub struct DeterministicLeakJudge {
    /// 命中即判泄漏的敏感词（遗忘事实的关键 token）。
    pub sensitive_tokens: Vec<String>,
}

impl ResearchJudge for DeterministicLeakJudge {
    fn judge(&self, _question: &str, candidate_text: &str) -> JudgeVerdict {
        let lower = candidate_text.to_lowercase();
        let hit = self
            .sensitive_tokens
            .iter()
            .any(|t| lower.contains(&t.to_lowercase()));
        JudgeVerdict {
            leaked: hit,
            confidence: if hit { 1.0 } else { 0.9 },
            rationale: if hit {
                "deterministic token hit".into()
            } else {
                "no sensitive token present".into()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EpisodeStore, MemoryGovernanceStore};
    use apeireth_core::kernel::memory::Episode;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().unwrap()
    }

    fn put(store: &SqliteMemoryStore, id: &str, session: &str, content: &str) {
        store
            .put_episode(&Episode {
                id: id.into(),
                timestamp: 1000,
                role: "user".into(),
                content: content.into(),
                session_id: session.into(),
            })
            .unwrap();
    }

    /// 探针 1: 直接召回 — 根遗忘后, 产品检索路径不再返回根内容 (既有语义, 回归锚).
    #[test]
    fn probe1_direct_recall_root_excluded_after_forget() {
        let s = store();
        put(&s, "ep-1", "me", "根事实");
        s.forget_episode("ep-1", Some("probe1"), 0).unwrap();
        let recent = s.governed_recent_episodes("me", 10).unwrap();
        assert!(recent.is_empty(), "forgotten 根必须从默认检索排除");
        assert_eq!(s.governed_recent_episodes("me", 10).unwrap().len(), 0);
    }

    /// 探针 2: 转述召回 — note 派生自根, taint 闭包必须覆盖 note (审计可见).
    #[test]
    fn probe2_paraphrase_recall_taint_closure_covers_note() {
        let s = store();
        put(&s, "ep-1", "me", "根事实");
        let note = DerivedRef::new("note", "note-1");
        s.research_record_derivation(
            &note,
            &[DerivedRef::new("episode", "ep-1")],
            Some("paraphrase of ep-1"),
        )
        .unwrap();
        // 根遗忘后: 转述召回面审计必须命中 note.
        let audit = s
            .research_audit_forgotten_leaks(&[DerivedRef::new("episode", "ep-1")], ClosureMode::Taint)
            .unwrap();
        assert_eq!(audit.items.len(), 1);
        assert_eq!(audit.items[0].id, "note-1");
        assert!(audit.items[0].in_closure);
        // taint 闭包同样覆盖 note.
        let report = s
            .research_forget_closure(
                &[DerivedRef::new("episode", "ep-1")],
                ClosureMode::Taint,
                Some("probe2"),
                None,
            )
            .unwrap();
        assert!(report.nodes.iter().any(|n| n.kind == "note" && n.id == "note-1"));
        assert!(!report.deleted_anything, "闭包只审计不删除");
        assert!(report.audit_event_seq.is_some());
    }

    /// 探针 3: 跨会话推理 — 血缘跨 session 传播, taint 沿多跳传播 (note → wiki).
    #[test]
    fn probe3_cross_session_inference_multihop_taint() {
        let s = store();
        put(&s, "ep-a", "s1", "A 会话事实");
        put(&s, "ep-b", "s2", "B 会话引用");
        let note = DerivedRef::new("note", "n-1");
        let wiki = DerivedRef::new("wiki", "w-1");
        s.research_record_derivation(
            &note,
            &[DerivedRef::new("episode", "ep-a"), DerivedRef::new("episode", "ep-b")],
            None,
        )
        .unwrap();
        // wiki 由 note 派生 (跨会话推理链: ep-a → note → wiki).
        s.research_record_derivation(&wiki, &[note.clone()], Some("compiled from n-1"))
            .unwrap();
        let report = s
            .research_forget_closure(&[DerivedRef::new("episode", "ep-a")], ClosureMode::Taint, None, None)
            .unwrap();
        let ids: Vec<&str> = report.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"ep-a"));
        assert!(ids.contains(&"n-1"));
        assert!(ids.contains(&"w-1"), "taint 必须多跳传播");
    }

    /// 探针 4: 衍生知识重建 — support(θ) 保留部分证据存活的派生项, taint 删除之.
    #[test]
    fn probe4_derived_knowledge_reconstruction_support_theta() {
        let s = store();
        put(&s, "ep-1", "me", "事实 1");
        put(&s, "ep-2", "me", "事实 2");
        put(&s, "ep-3", "me", "事实 3");
        let summary = DerivedRef::new("chronicle", "ch-1");
        s.research_record_derivation(
            &summary,
            &[
                DerivedRef::new("episode", "ep-1"),
                DerivedRef::new("episode", "ep-2"),
                DerivedRef::new("episode", "ep-3"),
            ],
            None,
        )
        .unwrap();
        // 遗忘 1/3 来源: θ=0.5 → 闭包 (0.33 < 0.5 不触发? 需 lost/total >= θ).
        let report = s
            .research_forget_closure(
                &[DerivedRef::new("episode", "ep-1")],
                ClosureMode::Support { theta: 0.5 },
                None,
                None,
            )
            .unwrap();
        assert!(
            !report.nodes.iter().any(|n| n.id == "ch-1"),
            "lost 1/3 < 0.5 → 派生项保留"
        );
        // taint 模式: 任一祖先遗忘 → ch-1 进闭包.
        let taint = s
            .research_forget_closure(
                &[DerivedRef::new("episode", "ep-1")],
                ClosureMode::Taint,
                None,
                None,
            )
            .unwrap();
        assert!(taint.nodes.iter().any(|n| n.id == "ch-1"));
        // θ=0.3 → 1/3 ≥ 0.3 → 进闭包.
        let s03 = s
            .research_forget_closure(
                &[DerivedRef::new("episode", "ep-1")],
                ClosureMode::Support { theta: 0.3 },
                None,
                None,
            )
            .unwrap();
        assert!(s03.nodes.iter().any(|n| n.id == "ch-1"));
    }

    /// 血缘登记幂等 + 空来源拒绝 (A5 语义).
    #[test]
    fn derivation_record_idempotent_and_rejects_empty_sources() {
        let s = store();
        let d = DerivedRef::new("diary", "d-1");
        let src = DerivedRef::new("episode", "ep-1");
        let first = s.research_record_derivation(&d, &[src.clone()], None).unwrap();
        assert_eq!(first, 1);
        let again = s.research_record_derivation(&d, &[src.clone()], None).unwrap();
        assert_eq!(again, 0, "主键去重幂等");
        let err = s.research_record_derivation(&d, &[], None).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    /// notes.source_episode_ids_json 回填桥接.
    #[test]
    fn note_lineage_import_backfills_from_notes() {
        let s = store();
        let conn = s.conn().unwrap();
        conn.execute(
            "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence) \
             VALUES ('n-9', 1, '派生笔记', '[\"ep-1\",\"ep-2\"]', 0.8)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO notes (id, timestamp, content, source_episode_ids_json, confidence) \
             VALUES ('n-10', 1, '无来源', '[]', 0.8)",
            [],
        )
        .unwrap();
        drop(conn);
        let imported = s.research_import_note_lineage().unwrap();
        assert_eq!(imported, 2);
        let audit = s
            .research_audit_forgotten_leaks(&[DerivedRef::new("episode", "ep-1")], ClosureMode::Taint)
            .unwrap();
        assert!(audit.items.iter().any(|i| i.id == "n-9"));
        assert!(!audit.items.iter().any(|i| i.id == "n-10"));
    }

    /// GovernedRecall 默认关闭 (透传) / 显式启用后过滤 / 血缘不可见放行 (0 装).
    #[test]
    fn governed_recall_default_off_then_explicit_filter() {
        let s = std::sync::Arc::new(store());
        s.research_record_derivation(
            &DerivedRef::new("wiki", "w-1"),
            &[DerivedRef::new("episode", "ep-1")],
            None,
        )
        .unwrap();
        let wiki = DerivedRef::new("wiki", "w-1");
        let unseen = DerivedRef::new("wiki", "w-ghost");
        // 默认 (无 filter): 全透传.
        let gr = GovernedRecall::new(std::sync::Arc::clone(&s));
        let (kept, filtered) = gr.recall(vec![wiki.clone(), unseen.clone()]).unwrap();
        assert_eq!(kept.len(), 2);
        assert!(filtered.is_empty());
        // 显式启用 taint 过滤: w-1 (来源已遗忘) 被滤; w-ghost 血缘不可见 → 放行 (0 装).
        let gr2 = GovernedRecall::new(std::sync::Arc::clone(&s))
            .with_filter(ClosureMode::Taint, vec![DerivedRef::new("episode", "ep-1")]);
        let (kept2, filtered2) = gr2.recall(vec![wiki.clone(), unseen.clone()]).unwrap();
        assert!(filtered2.iter().any(|d| d.id == "w-1"));
        assert!(kept2.iter().any(|d| d.id == "w-ghost"));
    }

    /// 缓存代际联动: 闭包非空 → 整代失效 (泄漏向量 1 粗粒度缓解).
    #[test]
    fn cache_link_advances_generation_on_closure() {
        let s = store();
        let cache: GenerationCache<String> = GenerationCache::new();
        cache.put("q1", std::sync::Arc::new("旧代派生快照".to_string()));
        assert!(cache.get("q1").is_some());
        // 空闭包: 不推进.
        let empty = s
            .research_forget_closure(&[DerivedRef::new("episode", "ghost")], ClosureMode::Taint, None, None)
            .unwrap();
        research_invalidate_cache_on_forget(&cache, &empty);
        assert_eq!(cache.generation(), 0);
        assert!(cache.get("q1").is_some());
        // 非空闭包: 推进 + 旧快照失效.
        let d = DerivedRef::new("cache", "c-1");
        s.research_record_derivation(&d, &[DerivedRef::new("episode", "ep-1")], None)
            .unwrap();
        let report = s
            .research_forget_closure(&[DerivedRef::new("episode", "ep-1")], ClosureMode::Taint, None, None)
            .unwrap();
        research_invalidate_cache_on_forget(&cache, &report);
        assert!(cache.generation() >= 1);
        assert!(cache.get("q1").is_none(), "旧代快照必须失效 (防脏读)");
    }

    /// 双评者协议: 一致 / 不一致保守取泄漏.
    #[test]
    fn dual_rater_protocol_conservative_on_disagreement() {
        let leak = JudgeVerdict {
            leaked: true,
            confidence: 1.0,
            rationale: "hit".into(),
        };
        let clean = JudgeVerdict {
            leaked: false,
            confidence: 0.9,
            rationale: "clean".into(),
        };
        assert_eq!(
            dual_rater_protocol(&leak, &leak),
            DualRaterResult::Agree { leaked: true }
        );
        match dual_rater_protocol(&leak, &clean) {
            DualRaterResult::Disagree { conservative, .. } => assert!(conservative),
            other => panic!("expected Disagree, got {other:?}"),
        }
    }

    /// 确定性 stub judge: 敏感 token 命中判泄漏.
    #[test]
    fn deterministic_judge_token_hit() {
        let j = DeterministicLeakJudge {
            sensitive_tokens: vec!["根事实".into()],
        };
        assert!(j.judge("q", "这段转述了根事实的内容").leaked);
        assert!(!j.judge("q", "无关内容").leaked);
    }

    /// V8 迁移存在 (新库自动应用).
    #[test]
    fn v8_migration_applied_on_fresh_db() {
        let s = store();
        let applied = s.applied_migrations().unwrap();
        assert!(applied.contains(&8), "V8 应已应用");
        let conn = s.conn().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_derived_from')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(exists);
        let ev_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_lineage_events')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ev_exists);
    }
}
