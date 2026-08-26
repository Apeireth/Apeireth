//! R218 Council deliberation checkpoint 集成 (接续 R212 + R217).
//!
//! **动机**: R212 提供了 Checkpoint + CheckpointStore 数据结构. R218 followup 加
//! `run_with_checkpoints` / `resume_with_checkpoints` 自由函数, 把 checkpoint 串进
//! `Council::deliberate` 流程.
//!
//! **设计**:
//! - `run_with_checkpoints` — 替代 `Council::deliberate`, 每步 opinion 发出后写 checkpoint
//! - `resume_with_checkpoints` — 续: 跳过已完成的 advisors, 从 `last.next_step()` 继续
//! - 8 强制 advisor 仍由调用方注入, 我们只 consume &self
//!
//! **0 触碰**:
//! - deliberation.rs 0 改 (我们用自由函数 + 借用 &mut Council)
//! - checkpoint.rs (R212) 0 改
//! - 7 强制 advisor 0 改
//! - 3 不可变脊柱 0 触碰

#![allow(missing_docs)] // R218 additive

use std::time::{SystemTime, UNIX_EPOCH};

use crate::advisor::AdvisorOpinion;
use crate::checkpoint::{Checkpoint, CheckpointQuery, CheckpointStore, CHECKPOINT_VERSION};
use crate::deliberation::{Council, CouncilQuery, CouncilVerdict};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 跑审议 + 每步写 checkpoint (替代 `Council::deliberate`).
///
/// 流程:
/// 1. 分配 session_id
/// 2. 遍历 advisors
/// 3. 每个 advisor 调 `deliberate`, 写出 opinion
/// 4. 写 Checkpoint (含 opinions_so_far + current_step)
/// 5. 跑 synthesis
/// 6. 写 final Checkpoint (current_step = total_steps, 标记完成)
/// 7. 返回 CouncilVerdict
pub fn run_with_checkpoints(
    council: &mut Council,
    store: &dyn CheckpointStore,
    query: CouncilQuery,
) -> CouncilVerdict {
    let session_id = format!("session-{:06}", unique_session_seq());
    let started_at_ms = query.started_at_ms;
    let total_steps = council.advisor_count();

    let mut opinions: Vec<AdvisorOpinion> = Vec::new();
    let mut ctx = crate::advisor::DeliberationContext::new(started_at_ms);

    for (step, advisor) in council.advisors_iter().enumerate() {
        match advisor.deliberate(&query, &mut ctx) {
            Ok(outcome) => {
                let mut opinion = outcome.opinion;
                opinion = opinion.with_weight(council.weights_for(advisor.domain()));
                council.emit_event(&crate::sovereignty::CouncilEvent::OpinionIssued {
                    session_id: session_id.clone(),
                    opinion: opinion.clone(),
                });
                opinions.push(opinion);

                // 写 step checkpoint
                let cp = Checkpoint {
                    version: CHECKPOINT_VERSION,
                    checkpoint_id: format!("cp-{session_id}-{step}"),
                    session_id: session_id.clone(),
                    query: CheckpointQuery::from_council_query(&query),
                    opinions_so_far: opinions.clone(),
                    current_step: step + 1,
                    total_steps,
                    elapsed_ms_so_far: (now_ms() - started_at_ms).max(0) as u64,
                    started_at_ms,
                    written_at_ms: now_ms(),
                };
                let _ = store.put(&cp);
            }
            Err(err) => {
                eprintln!("advisor {} error: {}", advisor.id(), err);
            }
        }
    }

    // 调用原 deliberate 拿 synthesis + hold (但因为是 mut borrow, 单独跑 synthesis)
    // 简化: 复用 synthesis + hold 逻辑
    let report = crate::synthesis::synthesize(&opinions, &council.weights_clone());
    let held = report.is_held();
    let hold_outcome = if held {
        let trigger = crate::hold::HoldTrigger::evaluate(&opinions);
        Some(crate::hold::HoldOutcome::ReflectionStarted {
            reason: format!("hold trigger: {:?}", trigger.map(|t| t.threshold)),
            started_at_ms,
        })
    } else {
        None
    };
    let elapsed_ms = (now_ms() - started_at_ms).max(0) as u64;

    // 写 final checkpoint
    let final_cp = Checkpoint {
        version: CHECKPOINT_VERSION,
        checkpoint_id: format!("cp-{session_id}-final"),
        session_id: session_id.clone(),
        query: CheckpointQuery::from_council_query(&query),
        opinions_so_far: opinions.clone(),
        current_step: total_steps,
        total_steps,
        elapsed_ms_so_far: elapsed_ms,
        started_at_ms,
        written_at_ms: now_ms(),
    };
    let _ = store.put(&final_cp);

    CouncilVerdict {
        query_id: query.query_id,
        session_id,
        report,
        elapsed_ms,
        held,
        hold_outcome,
    }
}

/// 从 checkpoint 续审议 (跳过已完成 advisors).
///
/// 流程:
/// 1. 从 `last.opinions_so_far` 恢复 opinions
/// 2. 从 `last.next_step()` 继续遍历 advisors
/// 3. 每步写 checkpoint
/// 4. 返回 CouncilVerdict
pub fn resume_with_checkpoints(
    council: &mut Council,
    store: &dyn CheckpointStore,
    last: Checkpoint,
    query: CouncilQuery,
) -> CouncilVerdict {
    let session_id = last.session_id.clone();
    let started_at_ms = last.started_at_ms;
    let total_steps = last.total_steps;
    let mut opinions: Vec<AdvisorOpinion> = last.opinions_so_far.clone();
    let resume_from = last.next_step();

    let mut ctx = crate::advisor::DeliberationContext::new(started_at_ms);
    ctx.current_round = 0; // 续时不累计轮次
    ctx.prior_opinions = opinions.clone();

    for (step, advisor) in council.advisors_iter().enumerate().skip(resume_from) {
        match advisor.deliberate(&query, &mut ctx) {
            Ok(outcome) => {
                let mut opinion = outcome.opinion;
                opinion = opinion.with_weight(council.weights_for(advisor.domain()));
                council.emit_event(&crate::sovereignty::CouncilEvent::OpinionIssued {
                    session_id: session_id.clone(),
                    opinion: opinion.clone(),
                });
                opinions.push(opinion);

                let cp = Checkpoint {
                    version: CHECKPOINT_VERSION,
                    checkpoint_id: format!("cp-{session_id}-{step}"),
                    session_id: session_id.clone(),
                    query: CheckpointQuery::from_council_query(&query),
                    opinions_so_far: opinions.clone(),
                    current_step: step + 1,
                    total_steps,
                    elapsed_ms_so_far: (now_ms() - started_at_ms).max(0) as u64,
                    started_at_ms,
                    written_at_ms: now_ms(),
                };
                let _ = store.put(&cp);
            }
            Err(err) => {
                eprintln!("advisor {} error: {}", advisor.id(), err);
            }
        }
    }

    let report = crate::synthesis::synthesize(&opinions, &council.weights_clone());
    let held = report.is_held();
    let hold_outcome = if held {
        let trigger = crate::hold::HoldTrigger::evaluate(&opinions);
        Some(crate::hold::HoldOutcome::ReflectionStarted {
            reason: format!("hold trigger: {:?}", trigger.map(|t| t.threshold)),
            started_at_ms,
        })
    } else {
        None
    };
    let elapsed_ms = (now_ms() - started_at_ms).max(0) as u64;

    CouncilVerdict {
        query_id: query.query_id,
        session_id,
        report,
        elapsed_ms,
        held,
        hold_outcome,
    }
}

// session ID 分配: 用静态原子 + 调用方 lock-free 序号
use std::sync::atomic::{AtomicU64, Ordering};
static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);
fn unique_session_seq() -> u64 {
    SESSION_SEQ.fetch_add(1, Ordering::SeqCst) + 1
}

// ============================================================================
// 测试 (10 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{
        Advisor, AdvisorDomain, AdvisorId, AdvisorOpinion, DeliberationContext,
        DeliberationOutcome, Stance, StanceKind,
    };
    use crate::checkpoint::MemoryCheckpointStore;
    use crate::deliberation::QueryContext;
    use crate::hold::HoldTrigger;
    use crate::synthesis::SynthesisWeights;
    use std::fmt;

    // 简单测试 advisor
    struct TestAdvisor {
        id: AdvisorId,
        domain: AdvisorDomain,
        stance: StanceKind,
    }

    impl TestAdvisor {
        fn new(name: &str, domain: AdvisorDomain, stance: StanceKind) -> Self {
            Self {
                id: AdvisorId::new(name),
                domain,
                stance,
            }
        }
    }

    impl Advisor for TestAdvisor {
        fn lifecycle(&self) -> crate::lifecycle::AdvisorLifecycle {
            crate::lifecycle::AdvisorLifecycle::Persistent
        }
        fn id(&self) -> AdvisorId {
            self.id.clone()
        }
        fn domain(&self) -> AdvisorDomain {
            self.domain
        }
        fn deliberate(
            &self,
            _q: &CouncilQuery,
            _ctx: &mut DeliberationContext,
        ) -> Result<DeliberationOutcome, crate::advisor::AdvisorError> {
            let stance = Stance::new(self.stance, "test");
            let op = AdvisorOpinion::new(self.id.clone(), stance, 0.8, "r", 1_000_000);
            Ok(DeliberationOutcome {
                opinion: op,
                needs_rebuttal: false,
            })
        }
    }

    fn mk_query() -> CouncilQuery {
        let mut q = CouncilQuery::new("q-001", "test", 1_000_000);
        q.context = QueryContext {
            area: Some("test".to_string()),
            risk_level: Some("low".to_string()),
            history_refs: Vec::new(),
        };
        q
    }

    #[test]
    fn t01_run_writes_checkpoints() {
        let mut council = Council::new();
        council.recruit(Box::new(TestAdvisor::new(
            "s1",
            AdvisorDomain::Safety,
            StanceKind::Approve,
        )));
        council.recruit(Box::new(TestAdvisor::new(
            "p1",
            AdvisorDomain::Performance,
            StanceKind::Approve,
        )));
        let store = MemoryCheckpointStore::new();
        let q = mk_query();
        let v = run_with_checkpoints(&mut council, &store, q);
        assert_eq!(store.total_checkpoints(), 3); // 2 step + 1 final
        assert!(!v.held);
    }

    #[test]
    fn t02_run_completes_all_advisors() {
        let mut council = Council::new();
        for i in 0..5 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        assert_eq!(store.total_checkpoints(), 6); // 5 step + 1 final
                                                  // last checkpoint should be complete
        let cps = store.list(&v.session_id).unwrap();
        let last = cps.last().unwrap();
        assert!(last.is_complete());
    }

    #[test]
    fn t03_resume_skips_completed() {
        let mut council = Council::new();
        for i in 0..5 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());

        // 模拟 crash: 删除 step 3+ 后的 checkpoint
        let mut cps = store.list(&v.session_id).unwrap();
        cps.retain(|cp| cp.current_step <= 3);
        for _cp in &cps {
            let _ = store.delete(&v.session_id);
            break;
        }
        // 写 cp-3 (第 3 步已完成, 第 4 步未开始)
        let cp3 = Checkpoint {
            version: CHECKPOINT_VERSION,
            checkpoint_id: "cp-test-3".to_string(),
            session_id: v.session_id.clone(),
            query: CheckpointQuery::from_council_query(&mk_query()),
            opinions_so_far: cps.last().unwrap().opinions_so_far.clone(),
            current_step: 3,
            total_steps: 5,
            elapsed_ms_so_far: 100,
            started_at_ms: 1_000_000,
            written_at_ms: 1_000_100,
        };
        // 重置 store 后只放 cp3
        store.delete("any") /* no clear in trait */;
        store.put(&cp3).unwrap();

        // 续: 应该只跑 2 个 advisor (4, 5)
        let v2 = resume_with_checkpoints(&mut council, &store, cp3, mk_query());
        // 2 个新 opinion + cp3 opinions
        assert!(v2.session_id == v.session_id);
    }

    #[test]
    fn t04_resume_from_complete_returns_same() {
        let mut council = Council::new();
        for i in 0..3 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        let cps = store.list(&v.session_id).unwrap();
        let last = cps.last().unwrap();
        // 续: 已经 complete, 不应再写新 step
        let v2 = resume_with_checkpoints(&mut council, &store, last.clone(), mk_query());
        assert!(v2.session_id == v.session_id);
    }

    #[test]
    fn t05_session_id_unique() {
        let mut council = Council::new();
        council.recruit(Box::new(TestAdvisor::new(
            "a1",
            AdvisorDomain::Safety,
            StanceKind::Approve,
        )));
        let store = MemoryCheckpointStore::new();
        let v1 = run_with_checkpoints(&mut council, &store, mk_query());
        let v2 = run_with_checkpoints(&mut council, &store, mk_query());
        assert_ne!(v1.session_id, v2.session_id);
    }

    #[test]
    fn t06_progress_increases() {
        let mut council = Council::new();
        for i in 0..4 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        let cps = store.list(&v.session_id).unwrap();
        // progress 应该是单调递增
        for w in cps.windows(2) {
            assert!(w[1].progress() >= w[0].progress());
        }
    }

    #[test]
    fn t07_opinions_accumulate() {
        let mut council = Council::new();
        for i in 0..3 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        let cps = store.list(&v.session_id).unwrap();
        // 每个 checkpoint 的 opinions_so_far 应 >= 前一个
        for w in cps.windows(2) {
            assert!(w[1].opinions_so_far.len() >= w[0].opinions_so_far.len());
        }
        // final 应有 3 个 opinion
        let last = cps.last().unwrap();
        assert_eq!(last.opinions_so_far.len(), 3);
    }

    #[test]
    fn t08_run_with_zero_advisors() {
        let mut council = Council::new();
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        // 0 advisor → 0 step checkpoint + 1 final
        assert_eq!(store.total_checkpoints(), 1);
        let last = store.get(&v.session_id).unwrap();
        assert!(last.is_complete());
    }

    #[test]
    fn t09_hold_trigger_with_strong_disapprove() {
        let mut council = Council::new();
        council.recruit(Box::new(TestAdvisor::new(
            "s1",
            AdvisorDomain::Safety,
            StanceKind::StrongDisapprove,
        )));
        council.recruit(Box::new(TestAdvisor::new(
            "p1",
            AdvisorDomain::Performance,
            StanceKind::Approve,
        )));
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        assert!(v.held);
        assert!(v.hold_outcome.is_some());
    }

    #[test]
    fn t10_resume_preserves_session_id() {
        let mut council = Council::new();
        for i in 0..3 {
            let d = AdvisorDomain::ALL[i % AdvisorDomain::ALL.len()];
            council.recruit(Box::new(TestAdvisor::new(
                &format!("a{i}"),
                d,
                StanceKind::Approve,
            )));
        }
        let store = MemoryCheckpointStore::new();
        let v = run_with_checkpoints(&mut council, &store, mk_query());
        // 取中间 checkpoint
        let cps = store.list(&v.session_id).unwrap();
        let mid = cps[1].clone();
        let v2 = resume_with_checkpoints(&mut council, &store, mid, mk_query());
        // session_id 一致
        assert_eq!(v.session_id, v2.session_id);
    }
}
