//! B6 · Phase 5: 审批状态机形式化 (Research 前缀, 默认关闭)。
//!
//! # 学术账本 (铁律 3)
//! - **问题定义**: 生产 `ApprovalStatus` 的 `Claimed` 同时承担"已批准未派发"与
//!   "已派发未落账"两种含义, 崩溃恢复无法精确表达效果不确定性; `Claimed→Interrupted`
//!   转移缺持久化 (G2); 工具无副作用类别声明 (G4)。
//! - **假设**: 拆出 `Dispatched` 状态 + durable 前缀崩溃模型 + 副作用描述符,
//!   三条不变量 (InvA 无双副作用 / InvB 批准意图不丢 / InvC 效果不确定强制
//!   Interrupted) 可被确定性状态机机械验证。
//! - **状态**: 原型已实现 — 纯状态机 (Next 关系 + 非法转移拒绝) + 崩溃前缀模型 +
//!   三不变量判定 + 副作用描述符 schema + 恢复动作映射 + 模型级故障注入 harness
//!   (6 持久化点 × 100 轮)。TLA+/TLC 机器验证已完成 (2026-09-05, `research/verification/tla/`:
//!   单记录 36 状态 / 三记录 3164 状态全通过); Kani 3/3 harness 机器证明通过
//!   (2026-09-05, `.github/workflows/kani.yml` run 33945573291, unwind 32 有界口径)。
//! - **引用**: `_research_mem/ra/ra5-approval-state-machine-spec.md` §2–§7 与
//!   `ra5-formal-proof-plan.md` §4 (P1–P6 注入点); Newcombe et al. CACM 2015
//!   (TLA+ 工业先例); Pillai et al. OSDI'14 (fsync 语义, 只能测不能证)。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`。
//! - **已知局限**: ① 本模块是**模型级**验证 (纯状态机), 生产 `approval.rs`/
//!   `execute.rs` 的 G1–G7 差距未改代码 (默认关闭, 铁律 1); ② 崩溃模型用
//!   durable 前缀布尔抽象, 未建模真实 fsync 语义; ③ Kani harness 以 `#[cfg(kani)]`
//!   门控写好, 由 `.github/workflows/kani.yml` 在 ubuntu runner 上运行。
//!
//! # 默认关闭 (铁律 1 + Phase 5 闸门)
//! - 本模块不挂生产审批路径; `approval.rs` / `execute.rs` 零改动。

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// 审批状态 (RA-5 §2: 生产六态 + 提议新增 Dispatched)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchApprovalStatus {
    Pending,
    Claimed,
    /// 副作用已开始派发, 效果未知 (在途或已发生但 result 未落账)。
    Dispatched,
    Consumed,
    Rejected,
    Expired,
    /// fail-closed 终态: 效果未知或已被否决, 禁止自动重试。
    Interrupted,
}

impl ResearchApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Dispatched => "dispatched",
            Self::Consumed => "consumed",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Interrupted => "interrupted",
        }
    }

    /// 终态锁 (RA-5 §3 非法转移): 终态无出边。
    pub fn is_final(self) -> bool {
        matches!(
            self,
            Self::Consumed | Self::Rejected | Self::Expired | Self::Interrupted
        )
    }
}

/// 状态机事件 (RA-5 §3 Next 关系)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchApprovalEvent {
    Approve,
    Reject,
    Cancel,
    Expire {
        now: u64,
    },
    BeginDispatch,
    Complete,
    Interrupt,
    /// 崩溃恢复路径: 重开一个 Claimed (安全侧转 Interrupted)。
    RecoverClaimed,
}

/// 状态机错误: 非法转移 (Safety, RA-5 §3 末尾)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchApprovalError {
    NotFound(String),
    IllegalTransition {
        id: String,
        from: ResearchApprovalStatus,
        event: &'static str,
    },
}

impl std::fmt::Display for ResearchApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "approval `{id}` not found"),
            Self::IllegalTransition { id, from, event } => {
                write!(
                    f,
                    "approval `{id}`: illegal transition {event} from {}",
                    from.as_str()
                )
            }
        }
    }
}
impl std::error::Error for ResearchApprovalError {}

/// 单条审批记录 (变量抽象: status/executed/result_appended/durable)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchApprovalRecord {
    pub id: String,
    pub status: ResearchApprovalStatus,
    /// 是否已真实派发过副作用 (单调 FALSE→TRUE)。
    pub executed: bool,
    /// 工具结果是否已 append 进 transcript。
    pub result_appended: bool,
    /// 当前状态是否已持久化 (durable 前缀语义)。
    pub durable: bool,
    /// 人类是否批准过 (InvB 前提 decision=Approve; 崩溃丢账后回 false)。
    #[serde(default)]
    pub approved: bool,
    pub expires_at: Option<u64>,
}

impl ResearchApprovalRecord {
    fn new(id: impl Into<String>, expires_at: Option<u64>) -> Self {
        Self {
            id: id.into(),
            status: ResearchApprovalStatus::Pending,
            executed: false,
            result_appended: false,
            durable: false,
            approved: false,
            expires_at,
        }
    }
}

/// 审批状态机 (RA-5 §2–§5 的 Rust 编码, 纯内存模型)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResearchApprovalMachine {
    pub records: HashMap<String, ResearchApprovalRecord>,
    pub active: Option<String>,
}

impl ResearchApprovalMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// P1: 创建 Pending 审批。
    pub fn create(&mut self, id: impl Into<String>, expires_at: Option<u64>) {
        let id = id.into();
        self.records
            .insert(id.clone(), ResearchApprovalRecord::new(id, expires_at));
    }

    /// Next 关系: 事件驱动转移; 非法转移拒绝 (Safety)。
    pub fn next(
        &mut self,
        id: &str,
        event: ResearchApprovalEvent,
    ) -> Result<ResearchApprovalStatus, ResearchApprovalError> {
        let rec = self
            .records
            .get(id)
            .ok_or_else(|| ResearchApprovalError::NotFound(id.to_string()))?;
        let from = rec.status;
        let ev_name = match event {
            ResearchApprovalEvent::Approve => "approve",
            ResearchApprovalEvent::Reject => "reject",
            ResearchApprovalEvent::Cancel => "cancel",
            ResearchApprovalEvent::Expire { .. } => "expire",
            ResearchApprovalEvent::BeginDispatch => "begin_dispatch",
            ResearchApprovalEvent::Complete => "complete",
            ResearchApprovalEvent::Interrupt => "interrupt",
            ResearchApprovalEvent::RecoverClaimed => "recover_claimed",
        };
        // 终态锁: 终态无出边。
        if from.is_final() {
            return Err(ResearchApprovalError::IllegalTransition {
                id: id.to_string(),
                from,
                event: ev_name,
            });
        }
        let rec = self.records.get_mut(id).expect("checked above");
        match (from, event) {
            (ResearchApprovalStatus::Pending, ResearchApprovalEvent::Approve) => {
                rec.status = ResearchApprovalStatus::Claimed;
                rec.durable = true; // P2 claim-before-effect 落账
                rec.approved = true;
                self.active = Some(id.to_string());
            }
            (ResearchApprovalStatus::Pending, ResearchApprovalEvent::Reject)
            | (ResearchApprovalStatus::Pending, ResearchApprovalEvent::Cancel) => {
                rec.status = ResearchApprovalStatus::Rejected;
            }
            (ResearchApprovalStatus::Pending, ResearchApprovalEvent::Expire { now })
                if rec.expires_at.is_some_and(|t| now > t) =>
            {
                rec.status = ResearchApprovalStatus::Expired;
                rec.durable = true; // P6 落账
                self.active = None;
            }
            (ResearchApprovalStatus::Pending, ResearchApprovalEvent::Expire { .. }) => {
                // 未到过期点: no-op (保持 Pending)。
            }
            (ResearchApprovalStatus::Claimed, ResearchApprovalEvent::BeginDispatch) => {
                rec.executed = true; // 单调翻转
                rec.status = ResearchApprovalStatus::Dispatched;
                rec.durable = true; // P3 Dispatched 先持久化再发请求
            }
            (ResearchApprovalStatus::Dispatched, ResearchApprovalEvent::Complete) => {
                rec.result_appended = true;
                rec.status = ResearchApprovalStatus::Consumed;
                rec.durable = true; // P4 落账
                self.active = None;
            }
            (ResearchApprovalStatus::Claimed, ResearchApprovalEvent::Interrupt)
            | (ResearchApprovalStatus::Dispatched, ResearchApprovalEvent::Interrupt) => {
                rec.status = ResearchApprovalStatus::Interrupted;
                rec.durable = true; // P5 落账 (G2 修复语义)
                self.active = None;
            }
            (ResearchApprovalStatus::Claimed, ResearchApprovalEvent::RecoverClaimed) => {
                rec.status = ResearchApprovalStatus::Interrupted;
                rec.durable = true; // G3 语义: 重开落账 Interrupted
                self.active = None;
            }
            _ => {
                return Err(ResearchApprovalError::IllegalTransition {
                    id: id.to_string(),
                    from,
                    event: ev_name,
                });
            }
        }
        Ok(self.records[id].status)
    }

    /// 崩溃模型 (RA-5 §5): durable 前缀回退。
    ///
    /// - 非 durable 记录丢失 (回退到 Pending 之前的语义 = 不存在)。
    /// - durable 且 `Claimed` → active 恢复为该 id (可续跑或 Interrupt, 见
    ///   [`Self::recover_after_crash`])。
    /// - durable 且 `Dispatched ∧ ¬result_appended` → 强制 Interrupted (InvC)。
    pub fn simulate_crash(&mut self) {
        let mut recovered_active: Option<String> = None;
        for rec in self.records.values_mut() {
            if !rec.durable {
                // 未落账状态在崩溃后消失: 视为从未发生 (回退 Pending 前)。
                rec.status = ResearchApprovalStatus::Pending;
                rec.executed = false;
                rec.result_appended = false;
                rec.approved = false;
                continue;
            }
            if rec.status == ResearchApprovalStatus::Dispatched && !rec.result_appended {
                // InvC: 效果不确定 → 强制 Interrupted, 禁止自动重试。
                rec.status = ResearchApprovalStatus::Interrupted;
            }
            if rec.status == ResearchApprovalStatus::Claimed {
                recovered_active = Some(rec.id.clone());
            }
        }
        self.active = recovered_active;
    }

    /// 崩溃恢复后的处置建议 (结合副作用描述符, RA-5 §7)。
    ///
    /// - durable `Claimed` 且未派发 → 效果未发生, 可安全续跑 (`ResumeDeterministic`)
    ///   或由人类决定 Interrupt; 本方法只报告状态, 不自动行动。
    /// - `Dispatched` 恢复后必为 `Interrupted` (见 [`Self::simulate_crash`])。
    pub fn recovery_advice(&self, id: &str) -> Option<ResearchRecoveryAdvice> {
        let rec = self.records.get(id)?;
        Some(match rec.status {
            ResearchApprovalStatus::Claimed => {
                if rec.executed {
                    ResearchRecoveryAdvice::InFlight
                } else {
                    ResearchRecoveryAdvice::SafeToResume
                }
            }
            ResearchApprovalStatus::Interrupted if rec.executed && !rec.result_appended => {
                ResearchRecoveryAdvice::EffectUncertain
            }
            ResearchApprovalStatus::Pending => ResearchRecoveryAdvice::Wait,
            ResearchApprovalStatus::Consumed => ResearchRecoveryAdvice::Done,
            ResearchApprovalStatus::Rejected | ResearchApprovalStatus::Expired => {
                ResearchRecoveryAdvice::TerminalNoEffect
            }
            ResearchApprovalStatus::Dispatched => ResearchRecoveryAdvice::InFlight,
            ResearchApprovalStatus::Interrupted => ResearchRecoveryAdvice::FailClosed,
        })
    }

    /// InvA — no double side effect (RA-5 §4.1, 修正版)。
    ///
    /// 修正说明 (0 装): 规格原文写 `Interrupted ⇒ executed=TRUE`, 但规格自身的
    /// `Claimed→Interrupt` 动作 executed=FALSE (claim 后未派发即中断), 两者矛盾。
    /// 本实现按"副作用派发"语义修正: `Dispatched/Consumed ⇒ executed=TRUE`;
    /// `executed=TRUE ⇒ status ∉ {Pending, Claimed, Rejected, Expired}`;
    /// `Interrupted` 允许 executed=FALSE (显式 fail-closed, 无副作用发生过)。
    /// "至多一次 BeginDispatch" 由 Next 关系拒绝保证。
    pub fn inv_a_no_double_side_effect(&self) -> bool {
        self.records.values().all(|r| {
            let post = matches!(
                r.status,
                ResearchApprovalStatus::Dispatched | ResearchApprovalStatus::Consumed
            );
            // executed=TRUE ⇒ 不在 Pending/Claimed/Rejected/Expired; 且 Dispatched/Consumed ⇒ executed
            (!r.executed
                || !matches!(
                    r.status,
                    ResearchApprovalStatus::Pending
                        | ResearchApprovalStatus::Claimed
                        | ResearchApprovalStatus::Rejected
                        | ResearchApprovalStatus::Expired
                ))
                && (!post || r.executed)
        })
    }

    /// InvB — no lost approval (RA-5 §4.2, 修正版)。
    ///
    /// 修正说明 (0 装): 规格原文只允许 `Claimed/Dispatched`, 但其自身
    /// `Interrupt`/`RecoverClaimed` 动作产生 durable 的 `Interrupted` 且 executed=FALSE,
    /// 自相矛盾。本实现按 liveness 语义 (`Approve ⟹ ◇(Consumed ∨ Interrupted)`)
    /// 把 `Interrupted` 视为**显式 fail-closed 终态**而非静默丢失:
    /// 已持久化的未执行意图只能处于 Claimed/Dispatched/Interrupted,
    /// 绝不回 Pending/Rejected/Expired。
    pub fn inv_b_no_lost_approval(&self) -> bool {
        self.records.values().all(|r| {
            if r.durable && r.approved && !r.executed && !r.result_appended {
                matches!(
                    r.status,
                    ResearchApprovalStatus::Claimed
                        | ResearchApprovalStatus::Dispatched
                        | ResearchApprovalStatus::Interrupted
                )
            } else {
                true
            }
        })
    }

    /// InvC — fail-closed under uncertain effect: Dispatched 且结果未落账
    /// 的恢复后状态必须是 Interrupted, 且 Dispatched 无再次 BeginDispatch 出口。
    pub fn inv_c_fail_closed_under_uncertain_effect(&self) -> bool {
        self.records.values().all(|r| {
            if r.executed && !r.result_appended {
                matches!(
                    r.status,
                    ResearchApprovalStatus::Dispatched | ResearchApprovalStatus::Interrupted
                )
            } else {
                true
            }
        })
    }

    /// 三不变量联合判定。
    pub fn all_invariants(&self) -> bool {
        self.inv_a_no_double_side_effect()
            && self.inv_b_no_lost_approval()
            && self.inv_c_fail_closed_under_uncertain_effect()
    }
}

/// 崩溃恢复处置建议 (只报告, 不自动行动)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRecoveryAdvice {
    /// Pending 等待。
    Wait,
    /// durable Claimed 且未派发: 效果未发生, 可安全续跑。
    SafeToResume,
    /// 在途。
    InFlight,
    /// 效果不确定 (已派发未落账): 禁止自动重试, 交人类。
    EffectUncertain,
    /// fail-closed 终态。
    FailClosed,
    /// 已完成。
    Done,
    /// 终态且无效果发生。
    TerminalNoEffect,
}

/// 副作用类别 (RA-5 §7)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSideEffectCategory {
    Idempotent,
    Compensable,
    Irreversible,
    Stateful,
    Nondeterministic,
}

/// 幂等键约定 (§7 idempotency)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchIdempotency {
    /// 幂等键来源 (默认 operation_fingerprint)。
    pub key: String,
    /// 去重窗口 ms。
    pub dedup_ttl_ms: u64,
}

/// 补偿动作约定 (§7 compensation)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchCompensation {
    pub action: String,
    /// 补偿本身必须可重复。
    pub idempotent: bool,
}

/// 工具副作用描述符 (§7; 缺省 = irreversible + requires_reauthorization, fail-closed)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchSideEffectDescriptor {
    pub category: ResearchSideEffectCategory,
    pub read_only: bool,
    pub idempotency: Option<ResearchIdempotency>,
    pub compensation: Option<ResearchCompensation>,
    pub requires_reauthorization: bool,
}

impl Default for ResearchSideEffectDescriptor {
    fn default() -> Self {
        Self {
            category: ResearchSideEffectCategory::Irreversible,
            read_only: false,
            idempotency: None,
            compensation: None,
            requires_reauthorization: true,
        }
    }
}

/// 恢复动作 (RA-5 §7 类别→约束映射)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRecoveryAction {
    /// 效果不确定 → 只能 Interrupt (InvC)。
    Interrupt,
    /// 幂等且去重窗口内 → 允许自动 resume。
    ResumeWithinDedup,
    /// 未派发 (Claimed) → 可安全续跑。
    ResumeDeterministic,
    /// 不适用 (无恢复需求)。
    Ineligible,
}

/// §7 映射规则 (可机器检查):
/// - 效果不确定 (executed ∧ ¬result_appended):
///   * irreversible 或 requires_reauthorization → Interrupt
///   * idempotent 且 dedup_ttl_ms > 0 → ResumeWithinDedup
///   * nondeterministic → Interrupt (零重执行)
///   * 其余 → Interrupt (保守)
/// - durable Claimed 未派发 → ResumeDeterministic
pub fn research_allowed_recovery(
    desc: &ResearchSideEffectDescriptor,
    executed: bool,
    result_appended: bool,
    status: ResearchApprovalStatus,
) -> ResearchRecoveryAction {
    if status == ResearchApprovalStatus::Claimed && !executed {
        return ResearchRecoveryAction::ResumeDeterministic;
    }
    if executed && !result_appended {
        if desc.category == ResearchSideEffectCategory::Irreversible
            || desc.requires_reauthorization
        {
            return ResearchRecoveryAction::Interrupt;
        }
        if desc.category == ResearchSideEffectCategory::Idempotent {
            if let Some(idem) = &desc.idempotency {
                if idem.dedup_ttl_ms > 0 {
                    return ResearchRecoveryAction::ResumeWithinDedup;
                }
            }
        }
        if desc.category == ResearchSideEffectCategory::Nondeterministic {
            return ResearchRecoveryAction::Interrupt;
        }
        return ResearchRecoveryAction::Interrupt;
    }
    ResearchRecoveryAction::Ineligible
}

/// 模型级故障注入报告。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchFaultInjectionReport {
    pub rounds: u64,
    pub steps: u64,
    pub invariant_violations: u64,
    pub illegal_transitions_rejected: u64,
}

/// 模型级故障注入 harness (RA-5 proof-plan §4): 每轮在 6 个持久化点前后
/// 随机注入事件/崩溃交错, 每步断言三不变量与非法转移拒绝。
pub fn research_run_fault_injection(seed: u64, rounds: u64) -> ResearchFaultInjectionReport {
    let mut state = seed.max(1);
    let mut next_u = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    let mut report = ResearchFaultInjectionReport {
        rounds,
        steps: 0,
        invariant_violations: 0,
        illegal_transitions_rejected: 0,
    };
    let events = [
        ResearchApprovalEvent::Approve,
        ResearchApprovalEvent::BeginDispatch,
        ResearchApprovalEvent::Complete,
        ResearchApprovalEvent::Interrupt,
        ResearchApprovalEvent::RecoverClaimed,
        ResearchApprovalEvent::Reject,
        ResearchApprovalEvent::Expire { now: 10_000 },
    ];
    for _ in 0..rounds {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", Some(5000));
        for step in 0..40u64 {
            let ev = events[(next_u() % events.len() as u64) as usize];
            let outcome = m.next("a1", ev);
            match outcome {
                Ok(_) => {}
                Err(ResearchApprovalError::IllegalTransition { .. }) => {
                    report.illegal_transitions_rejected += 1;
                }
                Err(_) => {}
            }
            // 每 5 步注入一次崩溃 (durable 前缀语义)。
            if step % 5 == 4 {
                m.simulate_crash();
            }
            if !m.all_invariants() {
                report.invariant_violations += 1;
                break;
            }
            report.steps += 1;
        }
    }
    report
}

// ===== Kani harness (cfg 门控: 工具链安装后 `cargo kani -p apeireth-runtime`) =====

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // 注 (2026-09-05, CI 实测): Kani 把 String 字节缓冲当符号长度处理,
    // HashMap 的 SipHash Hasher::write 循环会无限展开 (实测 1900+ 迭代 × 2s)。
    // 本 harness 全部使用具体短键 ("a1"), 真实展开深度 ≤ 2 字节 + 桶遍历;
    // 上界 32 覆盖全部真实执行 (超出上界的符号路径不检查, 属有界模型检查口径,
    // 与 TLC 穷举互相印证)。该属性仅 cfg(kani) 生效, 生产零影响。

    #[kani::proof]
    #[kani::unwind(32)]
    fn kani_terminal_lock_no_outgoing_transitions() {
        for status in [
            ResearchApprovalStatus::Consumed,
            ResearchApprovalStatus::Rejected,
            ResearchApprovalStatus::Expired,
            ResearchApprovalStatus::Interrupted,
        ] {
            assert!(status.is_final(), "终态必须 is_final");
        }
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        m.next("a1", ResearchApprovalEvent::Reject).unwrap();
        assert!(
            m.next("a1", ResearchApprovalEvent::Approve).is_err(),
            "Rejected 无出边"
        );
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn kani_executed_monotonic_once() {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        m.next("a1", ResearchApprovalEvent::Approve).unwrap();
        m.next("a1", ResearchApprovalEvent::BeginDispatch).unwrap();
        // Dispatched 下再次 BeginDispatch 非法 (InvA/InvC: 至多一次派发)。
        assert!(m.next("a1", ResearchApprovalEvent::BeginDispatch).is_err());
        assert!(m.inv_a_no_double_side_effect());
        assert!(m.inv_c_fail_closed_under_uncertain_effect());
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn kani_crash_recovery_invariant_c() {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        m.next("a1", ResearchApprovalEvent::Approve).unwrap();
        m.next("a1", ResearchApprovalEvent::BeginDispatch).unwrap();
        m.simulate_crash();
        let rec = m.records.get("a1").unwrap();
        assert_eq!(rec.status, ResearchApprovalStatus::Interrupted);
        assert!(m.inv_c_fail_closed_under_uncertain_effect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approved() -> ResearchApprovalMachine {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", Some(5000));
        m.next("a1", ResearchApprovalEvent::Approve).unwrap();
        m
    }

    #[test]
    fn happy_path_pending_claimed_consumed() {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        assert_eq!(
            m.next("a1", ResearchApprovalEvent::Approve).unwrap(),
            ResearchApprovalStatus::Claimed
        );
        assert_eq!(
            m.next("a1", ResearchApprovalEvent::BeginDispatch).unwrap(),
            ResearchApprovalStatus::Dispatched
        );
        assert_eq!(
            m.next("a1", ResearchApprovalEvent::Complete).unwrap(),
            ResearchApprovalStatus::Consumed
        );
        assert!(m.inv_a_no_double_side_effect());
        assert!(m.inv_b_no_lost_approval());
        assert!(m.inv_c_fail_closed_under_uncertain_effect());
    }

    #[test]
    fn terminal_lock_rejects_outgoing() {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        m.next("a1", ResearchApprovalEvent::Reject).unwrap();
        for ev in [
            ResearchApprovalEvent::Approve,
            ResearchApprovalEvent::BeginDispatch,
            ResearchApprovalEvent::Complete,
            ResearchApprovalEvent::Interrupt,
        ] {
            assert!(matches!(
                m.next("a1", ev),
                Err(ResearchApprovalError::IllegalTransition { .. })
            ));
        }
    }

    #[test]
    fn illegal_transitions_rejected_everywhere() {
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        // Pending 不能 BeginDispatch / Complete / Interrupt
        for ev in [
            ResearchApprovalEvent::BeginDispatch,
            ResearchApprovalEvent::Complete,
            ResearchApprovalEvent::Interrupt,
        ] {
            assert!(matches!(
                m.next("a1", ev),
                Err(ResearchApprovalError::IllegalTransition { .. })
            ));
        }
        // Claimed 不能 Complete (必须先 BeginDispatch)
        let mut m2 = approved();
        assert!(matches!(
            m2.next("a1", ResearchApprovalEvent::Complete),
            Err(ResearchApprovalError::IllegalTransition { .. })
        ));
        // 未到过期点: Pending Expire no-op
        let mut m3 = ResearchApprovalMachine::new();
        m3.create("a1", Some(5000));
        assert_eq!(
            m3.next("a1", ResearchApprovalEvent::Expire { now: 100 })
                .unwrap(),
            ResearchApprovalStatus::Pending
        );
        // 到点过期
        assert_eq!(
            m3.next("a1", ResearchApprovalEvent::Expire { now: 6000 })
                .unwrap(),
            ResearchApprovalStatus::Expired
        );
    }

    #[test]
    fn crash_before_claim_persist_loses_pending() {
        // P2 前崩溃: Approve 前 (Pending 未 durable) → 恢复后回 Pending (意图丢失但未批准, 合法)。
        let mut m = ResearchApprovalMachine::new();
        m.create("a1", None);
        m.simulate_crash();
        assert_eq!(m.records["a1"].status, ResearchApprovalStatus::Pending);
        assert!(m.all_invariants());
    }

    #[test]
    fn crash_after_claim_keeps_intent() {
        // P2 后崩溃: durable Claimed → active 恢复, InvB 保持。
        let mut m = approved();
        m.simulate_crash();
        assert_eq!(m.records["a1"].status, ResearchApprovalStatus::Claimed);
        assert_eq!(m.active.as_deref(), Some("a1"));
        assert_eq!(
            m.recovery_advice("a1"),
            Some(ResearchRecoveryAdvice::SafeToResume)
        );
        assert!(m.inv_b_no_lost_approval());
    }

    #[test]
    fn crash_after_dispatch_forces_interrupted() {
        // P3 后/P4 前崩溃: 效果不确定 → InvC 强制 Interrupted。
        let mut m = approved();
        m.next("a1", ResearchApprovalEvent::BeginDispatch).unwrap();
        m.simulate_crash();
        assert_eq!(m.records["a1"].status, ResearchApprovalStatus::Interrupted);
        assert_eq!(
            m.recovery_advice("a1"),
            Some(ResearchRecoveryAdvice::EffectUncertain)
        );
        assert!(m.inv_c_fail_closed_under_uncertain_effect());
    }

    #[test]
    fn recover_claimed_turns_interrupted_and_persists() {
        // G3 语义: RecoverClaimed → Interrupted 且 durable (不再回 Claimed)。
        let mut m = approved();
        m.next("a1", ResearchApprovalEvent::RecoverClaimed).unwrap();
        assert_eq!(m.records["a1"].status, ResearchApprovalStatus::Interrupted);
        assert!(m.records["a1"].durable);
        m.simulate_crash();
        assert_eq!(m.records["a1"].status, ResearchApprovalStatus::Interrupted);
    }

    #[test]
    fn side_effect_descriptor_default_is_fail_closed() {
        let d = ResearchSideEffectDescriptor::default();
        assert_eq!(d.category, ResearchSideEffectCategory::Irreversible);
        assert!(d.requires_reauthorization);
        assert_eq!(
            research_allowed_recovery(&d, true, false, ResearchApprovalStatus::Dispatched),
            ResearchRecoveryAction::Interrupt
        );
    }

    #[test]
    fn recovery_mapping_follows_spec() {
        // idempotent + dedup 窗口 → 允许自动 resume
        let idem = ResearchSideEffectDescriptor {
            category: ResearchSideEffectCategory::Idempotent,
            read_only: false,
            idempotency: Some(ResearchIdempotency {
                key: "operation_fingerprint".into(),
                dedup_ttl_ms: 86_400_000,
            }),
            compensation: None,
            requires_reauthorization: false,
        };
        assert_eq!(
            research_allowed_recovery(&idem, true, false, ResearchApprovalStatus::Dispatched),
            ResearchRecoveryAction::ResumeWithinDedup
        );
        // compensable 无 reauthorization → 保守 Interrupt (映射规则默认分支)
        let comp = ResearchSideEffectDescriptor {
            category: ResearchSideEffectCategory::Compensable,
            read_only: false,
            idempotency: None,
            compensation: Some(ResearchCompensation {
                action: "delete_bucket".into(),
                idempotent: true,
            }),
            requires_reauthorization: false,
        };
        assert_eq!(
            research_allowed_recovery(&comp, true, false, ResearchApprovalStatus::Dispatched),
            ResearchRecoveryAction::Interrupt
        );
        // Claimed 未派发 → 可安全续跑
        assert_eq!(
            research_allowed_recovery(
                &ResearchSideEffectDescriptor::default(),
                false,
                false,
                ResearchApprovalStatus::Claimed
            ),
            ResearchRecoveryAction::ResumeDeterministic
        );
        // nondeterministic → 零重执行 (Interrupt)
        let nondet = ResearchSideEffectDescriptor {
            category: ResearchSideEffectCategory::Nondeterministic,
            ..Default::default()
        };
        assert_eq!(
            research_allowed_recovery(&nondet, true, false, ResearchApprovalStatus::Dispatched),
            ResearchRecoveryAction::Interrupt
        );
    }

    #[test]
    fn fault_injection_harness_zero_violations() {
        for seed in [1u64, 7, 42, 99] {
            let report = research_run_fault_injection(seed, 100);
            assert_eq!(
                report.invariant_violations, 0,
                "seed {seed}: 注入后出现不变量违例 {report:?}"
            );
            assert!(report.steps > 0);
            assert!(
                report.illegal_transitions_rejected > 0,
                "应观察到非法转移被拒"
            );
        }
    }
}
