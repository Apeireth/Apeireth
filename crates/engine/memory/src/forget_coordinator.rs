//! B2 · RA-15 P0-A 生产接线（opt-in，默认关闭）：三段式协调遗忘。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**（吸收自 arXiv:2609.04875 "execution-state unlearning"）:
//!   精确去学习 = 明文记录 + 派生面 + 执行态**一次性清**，且遗忘后行为等价于
//!   从未观察。对手用确定性迁移系统证明 pre-target 前缀免费共享 / post-target
//!   后缀不可约污染（T−τ+1 重放下界）。
//! - **工程版本**: 人工批准后（`approval_id` 由上游 approval 生命周期解析后
//!   传入）执行三段式——P1 明文（episode sidecar + 持久化遗忘集）→ P2 派生面
//!   （V10 标记 + 可选修复计划）→ P3 执行态（注册表行删除 + 报告清单给调用方
//!   清 runtime 活体工件）。行为契约自验内建。
//! - **默认关闭（铁律 1）**: 旧 `forget_episode` 语义不变；本模块为显式
//!   opt-in 调用，不挂任何生产检索路径。
//! - **设计依据**: `docs/01-architecture/forget-three-phase-production-spec.md`。
//! - **引用**: arXiv:2609.04875；逐行对照见
//!   `docs/03-reference/absorption-2026-09.md` §P0-A。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::derived_repair::RepairOutcome;
use crate::research_derived_memory::{ClosureMode, ClosureReport, DerivedRef, ExecutionStateEntry};
use crate::{MemoryGovernanceStore, MemoryResult, SqliteMemoryStore};

/// 协调遗忘请求：roots 为遗忘根集；`approval_id` 引用上游已通过的审批工件
/// （本模块不内建审批权威——per "无第二审批权威"架构不变量）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinatedForget {
    pub roots: Vec<DerivedRef>,
    pub reason: String,
    pub approval_id: String,
}

/// 三段式执行报告（全部动作有审计留痕）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatedForgetReport {
    /// P1: 闭包中走 episode sidecar 的明文根/节点。
    pub plaintext_forgotten: Vec<DerivedRef>,
    /// P2: 全部闭包节点已入持久化遗忘集（V10 标记）。
    pub derived_marked: Vec<DerivedRef>,
    /// P3: 已删除的执行态注册行（调用方据此清 runtime 活体工件）。
    pub execution_state_cleared: Vec<ExecutionStateEntry>,
    /// 审计闭包（含执行态清单快照，事件已写 lineage 链）。
    pub closure: ClosureReport,
    /// 行为契约自验：执行态清单空 + 根被召回排除 + 持久遗忘集覆盖闭包。
    pub behavior_contract_verified: bool,
}

impl SqliteMemoryStore {
    /// 持久化遗忘集：UPSERT 标记（P0-A P1/P2 段落的落盘点）。
    pub fn research_persist_forgotten(
        &self,
        artifact: &DerivedRef,
        reason: &str,
        approval_id: &str,
    ) -> MemoryResult<usize> {
        let conn = self.conn()?;
        let ts = crate::research_derived_memory::now_ms_pub();
        let n = conn.execute(
            "INSERT INTO research_forgotten_artifacts (kind, id, reason, approval_id, ts) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(kind, id) DO UPDATE SET \
               reason = excluded.reason, approval_id = excluded.approval_id, ts = excluded.ts",
            rusqlite::params![artifact.kind, artifact.id, reason, approval_id, ts],
        )?;
        Ok(n)
    }

    /// 读持久化遗忘集（GovernedRecall::from_store_persisted 的数据源）。
    pub fn research_forgotten_set(&self) -> MemoryResult<HashSet<DerivedRef>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT kind, id FROM research_forgotten_artifacts")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut set = HashSet::new();
        for row in rows {
            let (kind, id) = row?;
            set.insert(DerivedRef { kind, id });
        }
        Ok(set)
    }

    /// 清除遗忘标记（P0-B 修复再发布时调用；幂等）。
    pub fn research_clear_forgotten_mark(&self, artifact: &DerivedRef) -> MemoryResult<usize> {
        let conn = self.conn()?;
        let n = conn.execute(
            "DELETE FROM research_forgotten_artifacts WHERE kind = ?1 AND id = ?2",
            rusqlite::params![artifact.kind, artifact.id],
        )?;
        Ok(n)
    }

    /// 三段式协调遗忘（P0-A 生产升级路径；opt-in）。
    ///
    /// 顺序: ① 审计闭包（写入 lineage 事件）→ ② P1 明文 → ③ P2 派生标记 →
    /// ④ P3 执行态注册行删除 + 报告 → ⑤ 行为契约自验。
    pub fn research_coordinated_forget(
        &self,
        req: &CoordinatedForget,
    ) -> MemoryResult<CoordinatedForgetReport> {
        // ① 审计闭包 (先审计后动手 — 动作依据同一份闭包, 0 装).
        let closure = self.research_forget_closure(
            &req.roots,
            ClosureMode::Taint,
            Some(&req.approval_id),
            Some(&req.reason),
        )?;

        // ② P1 明文: 闭包中 kind=episode 的节点走既有 forget_episode
        //    (governance sidecar, 默认检索排除 — 产品路径).
        let mut plaintext_forgotten: Vec<DerivedRef> = Vec::new();
        for node in &closure.nodes {
            if node.kind == "episode" {
                let _ = self.forget_episode(&node.id, Some(&req.reason), 0);
                plaintext_forgotten.push(DerivedRef::new(&node.kind, &node.id));
            }
        }

        // ③ P2 派生标记: 闭包全部节点入持久化遗忘集.
        for node in &closure.nodes {
            self.research_persist_forgotten(
                &DerivedRef::new(&node.kind, &node.id),
                &req.reason,
                &req.approval_id,
            )?;
        }
        let derived_marked: Vec<DerivedRef> = closure
            .nodes
            .iter()
            .map(|n| DerivedRef::new(&n.kind, &n.id))
            .collect();

        // ④ P3 执行态: 删除注册行 (污染源 ∈ 闭包), 报告被删条目给调用方
        //    清 runtime 活体工件 (store 不越权动 runtime).
        let conn = self.conn()?;
        let mut cleared: Vec<ExecutionStateEntry> = Vec::new();
        for node in &closure.nodes {
            let mut stmt = conn.prepare(
                "SELECT kind, ref_id, injection_step, note FROM research_execution_state \
                 WHERE taint_kind = ?1 AND taint_id = ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![node.kind, node.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?;
            let mut hits: Vec<(String, String, Option<i64>, String)> = Vec::new();
            for row in rows {
                hits.push(row?);
            }
            drop(stmt);
            for (kind, ref_id, step, note) in hits {
                conn.execute(
                    "DELETE FROM research_execution_state WHERE kind = ?1 AND ref_id = ?2",
                    rusqlite::params![kind, ref_id],
                )?;
                let kind_enum =
                    crate::research_derived_memory::execution_state_kind_from_str(&kind);
                if let Some(k) = kind_enum {
                    cleared.push(ExecutionStateEntry {
                        kind: k,
                        ref_id,
                        injection_step: step.map(|s| s as usize),
                        taint_source: DerivedRef::new(&node.kind, &node.id),
                        note,
                    });
                }
            }
        }
        drop(conn);

        // ⑤ 行为契约自验 ("遗忘后 = 从未观察" 的回归断言, 内建不假装):
        //    a) 执行态清单对根集为空;
        //    b) 根集全部在持久遗忘集;
        //    c) 持久遗忘集覆盖闭包全部节点.
        let inventory_empty = self
            .research_execution_state_inventory(&req.roots)?
            .items
            .is_empty();
        let persisted = self.research_forgotten_set()?;
        let roots_marked = req.roots.iter().all(|r| persisted.contains(r));
        let closure_covered = closure
            .nodes
            .iter()
            .all(|n| persisted.contains(&DerivedRef::new(&n.kind, &n.id)));
        let verified = inventory_empty && roots_marked && closure_covered;

        Ok(CoordinatedForgetReport {
            plaintext_forgotten,
            derived_marked,
            execution_state_cleared: cleared,
            closure,
            behavior_contract_verified: verified,
        })
    }

    /// P0-B 持久化接线：把 `RepairExecutor::execute` 的结局应用到持久遗忘集——
    /// 仍撤回的 → 标记；已修复再发布的 → 清标记；并写一条 repair_apply 审计事件。
    pub fn research_apply_repair_outcome(
        &self,
        outcome: &RepairOutcome,
        approval_id: &str,
    ) -> MemoryResult<usize> {
        let mut applied = 0usize;
        for art in &outcome.still_withdrawn {
            applied +=
                self.research_persist_forgotten(art, "repair: still withdrawn", approval_id)?;
        }
        for art in &outcome.repaired_and_republished {
            applied += self.research_clear_forgotten_mark(art)?;
        }
        let detail = serde_json::json!({
            "withdrawn": outcome.withdrawn.len(),
            "repaired": outcome.repaired_and_republished.len(),
            "still_withdrawn": outcome.still_withdrawn.len(),
            "selected_cost": outcome.selected_cost,
            "selected_weight": outcome.selected_weight,
        });
        let _ =
            self.research_write_event("repair_apply", Some(approval_id), None, "repair", &detail)?;
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derived_repair::{RepairExecutor, RepairNode, RepairTradeoff};
    use crate::research_derived_memory::ExecutionStateKind;
    use crate::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
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

    /// 三段式协调遗忘: 明文排除 + V10 标记 + 执行态清空 + 行为契约 verified。
    #[test]
    fn coordinated_forget_three_phase_verified() {
        let s = store();
        put(&s, "ep-1", "me", "根事实");
        let note = DerivedRef::new("note", "note-1");
        s.research_record_derivation(
            &note,
            &[DerivedRef::new("episode", "ep-1")],
            Some("paraphrase"),
        )
        .unwrap();
        s.research_record_execution_state(&ExecutionStateEntry {
            kind: ExecutionStateKind::SessionTokenSpan,
            ref_id: "sess-1".into(),
            injection_step: Some(3),
            taint_source: DerivedRef::new("episode", "ep-1"),
            note: String::new(),
        })
        .unwrap();

        let report = s
            .research_coordinated_forget(&CoordinatedForget {
                roots: vec![DerivedRef::new("episode", "ep-1")],
                reason: "用户要求".into(),
                approval_id: "apv-1".into(),
            })
            .unwrap();

        assert!(report.behavior_contract_verified, "契约自验必须通过");
        assert!(report
            .plaintext_forgotten
            .contains(&DerivedRef::new("episode", "ep-1")));
        assert!(report
            .derived_marked
            .contains(&DerivedRef::new("note", "note-1")));
        assert_eq!(report.execution_state_cleared.len(), 1);
        assert_eq!(report.execution_state_cleared[0].ref_id, "sess-1");
        // 明文排除 (产品检索路径).
        let recent = s.governed_recent_episodes("me", 10).unwrap();
        assert!(recent.is_empty(), "根必须被默认检索排除");
        // 持久遗忘集.
        let persisted = s.research_forgotten_set().unwrap();
        assert!(persisted.contains(&DerivedRef::new("episode", "ep-1")));
        assert!(persisted.contains(&note));
        // 执行态注册表已清空.
        let inv = s
            .research_execution_state_inventory(&[DerivedRef::new("episode", "ep-1")])
            .unwrap();
        assert!(inv.items.is_empty());
    }

    /// 持久化遗忘集驱动 GovernedRecall (from_store_persisted).
    #[test]
    fn governed_recall_reads_persisted_forgotten_set() {
        let s = std::sync::Arc::new(store());
        put(&s, "ep-1", "me", "根事实");
        let note = DerivedRef::new("wiki", "w-1");
        s.research_record_derivation(&note, &[DerivedRef::new("episode", "ep-1")], None)
            .unwrap();
        s.research_persist_forgotten(&DerivedRef::new("episode", "ep-1"), "r", "a")
            .unwrap();

        let gr = crate::research_derived_memory::GovernedRecall::from_store_persisted(
            std::sync::Arc::clone(&s),
            crate::research_derived_memory::ClosureMode::Taint,
        )
        .unwrap();
        let (kept, filtered) = gr.recall(vec![note.clone()]).unwrap();
        assert!(kept.is_empty());
        assert_eq!(filtered, vec![note]);
    }

    /// P0-B 持久化: 修复结局应用 — 仍撤回标记, 已修复清标记, 审计事件落链.
    #[test]
    fn repair_outcome_applied_to_persisted_marks() {
        let s = store();
        // 预置一个旧标记 (模拟先前遗忘).
        s.research_persist_forgotten(&DerivedRef::new("note", "b"), "old", "a0")
            .unwrap();
        let nodes = vec![
            RepairNode {
                artifact: DerivedRef::new("note", "b"),
                sources: vec![DerivedRef::new("episode", "a")],
                weight: 10.0,
                cost: 2.0,
            },
            RepairNode {
                artifact: DerivedRef::new("wiki", "c"),
                sources: vec![DerivedRef::new("note", "b")],
                weight: 9.0,
                cost: 1.0,
            },
        ];
        let outcome = RepairExecutor::execute(
            &nodes,
            &[DerivedRef::new("episode", "a")],
            RepairTradeoff::default(),
        );
        let applied = s.research_apply_repair_outcome(&outcome, "apv-2").unwrap();
        assert!(applied >= 2, "至少标记/清标记各动作");
        let persisted = s.research_forgotten_set().unwrap();
        // b/c 都修复再发布 → 旧标记清除; 根 episode:a 在 still_withdrawn → 标记.
        assert!(persisted.contains(&DerivedRef::new("episode", "a")));
        assert!(!persisted.contains(&DerivedRef::new("note", "b")));
        assert!(!persisted.contains(&DerivedRef::new("wiki", "c")));
    }

    /// V10 迁移存在 (新库自动应用).
    #[test]
    fn v10_migration_applied_on_fresh_db() {
        let s = store();
        let applied = s.applied_migrations().unwrap();
        assert!(applied.contains(&10), "V10 应已应用");
        let conn = s.conn().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_forgotten_artifacts')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(exists);
    }
}
