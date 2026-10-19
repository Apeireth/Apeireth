//! `apeireth-orchestration::recovery_protocol` — 再生四期 (SG-17): 故障处置状态机 + 瘢痕账本.
//!
//! # 生物原型
//! - **伤口愈合四期**: 止血 (hemostasis) → 炎症 (inflammation) → 增生 (proliferation)
//!   → 重塑 (remodeling)；四期顺序不可跳步，因为后一期依赖前一期的产物。
//! - **回退与复发是生理事实**: 愈合并不会一次成功——重塑期验证不过要退回增生；
//!   已愈合的伤口也可以重新裂开 (relapse)。状态机必须允许回退，而不是假装线性。
//! - **留瘢痕**: 愈合后功能可以恢复，但组织记得这里受过伤（瘢痕强度低、易再伤）。
//!   工程上 = **不删失败记忆**，而是把「已知坑」记进账本，降低重复踩坑概率。
//!
//! # 工程边界
//! - **纯函数状态机**: [`transition`] 不修改入参、不读系统时钟；时间一律 `u64` 毫秒注入。
//! - **证据串有界**: 每个 case 的 `evidence` 只保留最近 [`EVIDENCE_CAP`] 条，防无界增长。
//! - **非法迁移一律拒绝**: 跳过相位（如未诊断先修复）不被容忍；重复事件按**拒绝**处理
//!   （[`TransitionError::Idempotent`]），不静默 no-op，重放可观测。
//! - **瘢痕账本固定容量**: 按 `signature` 去重，重复学习只累加 `hits` 并刷新 `last_ms`，
//!   **不覆盖**首次的 `cause`/`lesson`（第一次学到的教训才是瘢痕本体）。
//! - **只做状态与记录，不做动作**: 本模块不执行回滚、不调 harness、不发通知；
//!   每期该做什么由调用方按 [`HealingPhase::allowed_events`] 与 [`RecoveryEvent`] 落地。
//! - **0 unsafe / 0 新外部依赖 / 0 系统时间依赖**。
//!
//! # IMPLEMENTED（未接线）
//! 本模块是机制内核 v0：类型、四期迁移表、瘢痕账本与单元测试齐备，落 **IMPLEMENTED** 级；
//! **未接 `harness_patch`、未进失败处理链、无 `self_status` 自述节**，
//! 生产路径 0 调用点（缺口见
//! `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md` SG-17）。

use std::fmt;

use serde::{Deserialize, Serialize};

/// 证据串保留上限（每个 case 只留最近 N 条，超出裁剪最旧的）。
pub const EVIDENCE_CAP: usize = 16;
/// 瘢痕账本默认容量（已知坑条数上限）。
pub const DEFAULT_SCAR_CAPACITY: usize = 128;

/// 愈合相位: 四期 + 已解决.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HealingPhase {
    /// 止血: 先止损/隔离，阻止影响扩散。
    Hemostasis,
    /// 炎症: 诊断与清理（定位根因、收集证据）。
    Inflammation,
    /// 增生: 修复与重建（落修复动作）。
    Proliferation,
    /// 重塑: 验证与加固（确认功能恢复）。
    Remodeling,
    /// 已解决: 验证通过；瘢痕已记录，流程闭合。
    Resolved,
}

impl HealingPhase {
    /// 稳定短名（证据串/日志用，不随 `Debug` 派生漂移）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hemostasis => "hemostasis",
            Self::Inflammation => "inflammation",
            Self::Proliferation => "proliferation",
            Self::Remodeling => "remodeling",
            Self::Resolved => "resolved",
        }
    }

    /// 是否为终态（已解决）。
    pub fn is_terminal(self) -> bool {
        self == Self::Resolved
    }

    /// 该相位允许的事件（与 [`next_phase`] 一致，测试逐项核对）。
    pub fn allowed_events(self) -> &'static [RecoveryEvent] {
        match self {
            Self::Hemostasis => &[RecoveryEvent::Contained],
            Self::Inflammation => &[RecoveryEvent::Diagnosed],
            Self::Proliferation => &[RecoveryEvent::Repaired, RecoveryEvent::Rollback],
            Self::Remodeling => &[
                RecoveryEvent::Verified,
                RecoveryEvent::Rollback,
                RecoveryEvent::Relapse,
            ],
            Self::Resolved => &[RecoveryEvent::Relapse],
        }
    }
}

/// 处置事件（由调用方在真实动作完成后投递）.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecoveryEvent {
    /// 已止住/已隔离（对应止血期完成）。
    Contained,
    /// 已定位根因。
    Diagnosed,
    /// 修复动作已落地。
    Repaired,
    /// 验证通过。
    Verified,
    /// 回滚上一个动作（修复不成立或验证不过）。
    Rollback,
    /// 复发（已愈合的伤口重新裂开 / 愈合中倒退）。
    Relapse,
}

impl RecoveryEvent {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contained => "contained",
            Self::Diagnosed => "diagnosed",
            Self::Repaired => "repaired",
            Self::Verified => "verified",
            Self::Rollback => "rollback",
            Self::Relapse => "relapse",
        }
    }
}

/// 一个进行中的恢复案例（纯数据；状态迁移返回新值，不改旧值）.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCase {
    /// 案例标识。
    pub id: String,
    /// 当前相位。
    pub phase: HealingPhase,
    /// 触发描述（现象/告警/入口）。
    pub trigger: String,
    /// 开启时间（毫秒注入）。
    pub started_ms: u64,
    /// 最近更新时间（毫秒注入；单调不回退）。
    pub updated_ms: u64,
    /// 证据串（有界，只留最近 [`EVIDENCE_CAP`] 条）。
    pub evidence: Vec<String>,
}

impl RecoveryCase {
    /// 新案例: 从 [`HealingPhase::Hemostasis`] 开始，并写入一条开启证据.
    pub fn new(id: impl Into<String>, trigger: impl Into<String>, now_ms: u64) -> Self {
        Self {
            id: id.into(),
            phase: HealingPhase::Hemostasis,
            trigger: trigger.into(),
            started_ms: now_ms,
            updated_ms: now_ms,
            evidence: vec![format!("opened@{now_ms}ms")],
        }
    }

    /// 是否已到终态。
    pub fn is_resolved(&self) -> bool {
        self.phase.is_terminal()
    }
}

/// 迁移查询: `(当前相位, 事件) → 下一相位`；`None` 表示非法.
///
/// 合法迁移表（唯一事实源，测试逐相位核对）:
///
/// | 当前相位 | 事件 | 下一相位 |
/// |---|---|---|
/// | `Hemostasis` | `Contained` | `Inflammation` |
/// | `Inflammation` | `Diagnosed` | `Proliferation` |
/// | `Proliferation` | `Repaired` | `Remodeling` |
/// | `Proliferation` | `Rollback` | `Inflammation` |
/// | `Remodeling` | `Verified` | `Resolved` |
/// | `Remodeling` | `Rollback` | `Proliferation` |
/// | `Remodeling` | `Relapse` | `Inflammation` |
/// | `Resolved` | `Relapse` | `Inflammation` |
///
/// 其余组合（跳相位、终态再推进、同相位重复事件）一律非法。
pub fn next_phase(from: HealingPhase, event: RecoveryEvent) -> Option<HealingPhase> {
    use HealingPhase::{Hemostasis, Inflammation, Proliferation, Remodeling, Resolved};
    use RecoveryEvent::{Contained, Diagnosed, Relapse, Repaired, Rollback, Verified};

    match (from, event) {
        (Hemostasis, Contained) => Some(Inflammation),
        (Inflammation, Diagnosed) => Some(Proliferation),
        (Proliferation, Repaired) => Some(Remodeling),
        (Proliferation, Rollback) => Some(Inflammation),
        (Remodeling, Verified) => Some(Resolved),
        (Remodeling, Rollback) => Some(Proliferation),
        (Remodeling, Relapse) => Some(Inflammation),
        (Resolved, Relapse) => Some(Inflammation),
        _ => None,
    }
}

/// 迁移失败原因.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionError {
    /// 非法迁移: 事件与当前相位不匹配（含跳相位与提前投递）。
    Illegal {
        /// 迁移起点。
        from: HealingPhase,
        /// 被拒绝的事件。
        event: RecoveryEvent,
    },
    /// 幂等拒绝: 已 [`HealingPhase::Resolved`] 再收到 [`RecoveryEvent::Verified`]。
    Idempotent {
        /// 当前相位。
        phase: HealingPhase,
        /// 重复投递的事件。
        event: RecoveryEvent,
    },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Illegal { from, event } => {
                write!(f, "非法迁移: {} 不接受 {}", from.as_str(), event.as_str())
            }
            Self::Idempotent { phase, event } => write!(
                f,
                "幂等拒绝: 已处于 {}，重复事件 {} 不再生效",
                phase.as_str(),
                event.as_str()
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

/// 应用一个事件，返回迁移后的新案例（入参保持不变）.
///
/// **幂等选择: 拒绝，不是 no-op**。`Resolved + Verified` 是「同一事件重复投递」的代表，
/// 返回 [`TransitionError::Idempotent`]，让重放/重复回调在调用方可见；其余重复事件因相位
/// 已推进而走 [`TransitionError::Illegal`]。
///
/// `updated_ms` 取 `max(now_ms, 原 updated_ms)`：时间注入乱序时不会把时间轴写回退；
/// 证据串记录的是注入值。
pub fn transition(
    case: &RecoveryCase,
    event: RecoveryEvent,
    now_ms: u64,
) -> Result<RecoveryCase, TransitionError> {
    if case.phase == HealingPhase::Resolved && event == RecoveryEvent::Verified {
        return Err(TransitionError::Idempotent {
            phase: case.phase,
            event,
        });
    }
    let Some(to) = next_phase(case.phase, event) else {
        return Err(TransitionError::Illegal {
            from: case.phase,
            event,
        });
    };

    let mut next = case.clone();
    next.phase = to;
    next.updated_ms = now_ms.max(case.updated_ms);
    next.evidence.push(format!("{}@{now_ms}ms", event.as_str()));
    if next.evidence.len() > EVIDENCE_CAP {
        let drop_n = next.evidence.len() - EVIDENCE_CAP;
        next.evidence.drain(0..drop_n);
    }
    Ok(next)
}

/// 瘢痕: 一条「已知坑」的教训（不是失败日志，是预防性先验）.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scar {
    /// 坑的签名（归一化后的关键指纹，用于去重）。
    pub signature: String,
    /// 成因描述。
    pub cause: String,
    /// 教训/下次如何避开。
    pub lesson: String,
    /// 首次记录时间（毫秒注入）。
    pub created_ms: u64,
}

/// 瘢痕账本条目: 瘢痕本体 + 复发统计.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScarRecord {
    /// 瘢痕本体（首次的 cause/lesson 不被覆盖）。
    pub scar: Scar,
    /// 命中/复现次数（首次记录即 1）。
    pub hits: u32,
    /// 最近一次复现时间（单调不回退）。
    pub last_ms: u64,
}

impl ScarRecord {
    /// 复发风险代理值 `[0, 1)`: `1 - 1/(1 + hits)`，命中的越多越高。
    ///
    /// 它不是概率预测，只用于「踩坑前先看瘢痕」的排序/预检阈值（W2 接线时用）。
    pub fn risk(&self) -> f64 {
        if self.hits == 0 {
            return 0.0;
        }
        1.0 - 1.0 / (1.0 + f64::from(self.hits))
    }
}

/// 固定容量瘢痕账本: 按 `signature` 去重，记住踩过的坑.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScarLedger {
    records: Vec<ScarRecord>,
    capacity: usize,
}

impl Default for ScarLedger {
    fn default() -> Self {
        Self::new(DEFAULT_SCAR_CAPACITY)
    }
}

impl ScarLedger {
    /// 以容量构造账本（容量 0 抬到 1）。
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// 条数上限。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 当前条数。
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 账本是否为空。
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// 全部条目（插入序）。
    pub fn records(&self) -> &[ScarRecord] {
        &self.records
    }

    /// 按签名查瘢痕。
    pub fn get(&self, signature: &str) -> Option<&ScarRecord> {
        self.records.iter().find(|r| r.scar.signature == signature)
    }

    /// 按签名查复发风险代理值。
    pub fn risk(&self, signature: &str) -> Option<f64> {
        self.get(signature).map(ScarRecord::risk)
    }

    /// 学习一条瘢痕: 已存在则 `hits += 1` 并刷新 `last_ms`，否则新增.
    ///
    /// 已存在时**不覆盖** `cause`/`lesson`/`created_ms`——首次学到的教训是瘢痕本体，
    /// 后续复现只增强「这里真的会再踩」的先验。容量满时淘汰「命中数最少 →
    /// 最久未复现 → 签名最小」的一条。
    pub fn learn(
        &mut self,
        signature: impl Into<String>,
        cause: impl Into<String>,
        lesson: impl Into<String>,
        now_ms: u64,
    ) -> ScarRecord {
        let signature = signature.into();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|r| r.scar.signature == signature)
        {
            record.hits = record.hits.saturating_add(1);
            record.last_ms = record.last_ms.max(now_ms);
            return record.clone();
        }

        while self.records.len() >= self.capacity {
            let mut victim = 0usize;
            for i in 1..self.records.len() {
                if is_weaker(&self.records[i], &self.records[victim]) {
                    victim = i;
                }
            }
            self.records.remove(victim);
        }

        let record = ScarRecord {
            scar: Scar {
                signature,
                cause: cause.into(),
                lesson: lesson.into(),
                created_ms: now_ms,
            },
            hits: 1,
            last_ms: now_ms,
        };
        self.records.push(record.clone());
        record
    }
}

/// 淘汰序: `hits` 越少越弱；同命中按 `last_ms` 越旧越弱；再同按签名升序。
fn is_weaker(a: &ScarRecord, b: &ScarRecord) -> bool {
    (a.hits, a.last_ms, &a.scar.signature) < (b.hits, b.last_ms, &b.scar.signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHASES: [HealingPhase; 5] = [
        HealingPhase::Hemostasis,
        HealingPhase::Inflammation,
        HealingPhase::Proliferation,
        HealingPhase::Remodeling,
        HealingPhase::Resolved,
    ];
    const EVENTS: [RecoveryEvent; 6] = [
        RecoveryEvent::Contained,
        RecoveryEvent::Diagnosed,
        RecoveryEvent::Repaired,
        RecoveryEvent::Verified,
        RecoveryEvent::Rollback,
        RecoveryEvent::Relapse,
    ];

    fn resolved_case() -> RecoveryCase {
        let mut case = RecoveryCase::new("case-1", "provider 5xx", 1_000);
        for (event, at) in [
            (RecoveryEvent::Contained, 1_100u64),
            (RecoveryEvent::Diagnosed, 1_200),
            (RecoveryEvent::Repaired, 1_300),
            (RecoveryEvent::Verified, 1_400),
        ] {
            case = transition(&case, event, at).expect("线性路径应合法");
        }
        case
    }

    #[test]
    fn new_case_starts_at_hemostasis() {
        let case = RecoveryCase::new("c-1", "disk_full", 1_000);
        assert_eq!(case.phase, HealingPhase::Hemostasis);
        assert_eq!(case.started_ms, 1_000);
        assert_eq!(case.updated_ms, 1_000);
        assert_eq!(case.evidence, vec!["opened@1000ms".to_string()]);
        assert!(!case.is_resolved());
        assert!(!HealingPhase::Hemostasis.is_terminal());
    }

    #[test]
    fn happy_path_walks_four_phases_to_resolved() {
        let c0 = RecoveryCase::new("c-1", "disk_full", 1_000);
        let c1 = transition(&c0, RecoveryEvent::Contained, 1_100).expect("止血→炎症");
        assert_eq!(c1.phase, HealingPhase::Inflammation);
        let c2 = transition(&c1, RecoveryEvent::Diagnosed, 1_200).expect("炎症→增生");
        assert_eq!(c2.phase, HealingPhase::Proliferation);
        let c3 = transition(&c2, RecoveryEvent::Repaired, 1_300).expect("增生→重塑");
        assert_eq!(c3.phase, HealingPhase::Remodeling);
        let c4 = transition(&c3, RecoveryEvent::Verified, 1_400).expect("重塑→已解决");
        assert_eq!(c4.phase, HealingPhase::Resolved);
        assert!(c4.is_resolved());

        assert_eq!(c4.id, "c-1");
        assert_eq!(c4.trigger, "disk_full");
        assert_eq!(c4.started_ms, 1_000, "started_ms 不随迁移变化");
        assert_eq!(c4.updated_ms, 1_400);
        assert_eq!(c4.evidence.len(), 5);
        assert_eq!(c4.evidence[1], "contained@1100ms");
        assert_eq!(c4.evidence[4], "verified@1400ms");
    }

    #[test]
    fn every_phase_walk_uses_only_legal_transitions() {
        // 与 `allowed_events` 逐相位核对，防止迁移表与文档漂移。
        let base = RecoveryCase::new("c-1", "trigger", 1_000);
        for phase in PHASES {
            let case = RecoveryCase {
                phase,
                ..base.clone()
            };
            for event in EVENTS {
                let outcome = transition(&case, event, 2_000);
                if phase == HealingPhase::Resolved && event == RecoveryEvent::Verified {
                    assert_eq!(
                        outcome,
                        Err(TransitionError::Idempotent { phase, event }),
                        "已解决再验证应幂等拒绝"
                    );
                } else if phase.allowed_events().contains(&event) {
                    assert!(outcome.is_ok(), "{phase:?} + {event:?} 应合法");
                } else {
                    assert_eq!(
                        outcome,
                        Err(TransitionError::Illegal { from: phase, event }),
                        "{phase:?} + {event:?} 应非法"
                    );
                }
            }
            assert_eq!(case.updated_ms, 1_000, "迁移是纯函数，入参不变");
            assert_eq!(case.phase, phase);
        }
    }

    #[test]
    fn rollback_paths_only_from_proliferation_or_remodeling() {
        let c0 = RecoveryCase::new("c-1", "trigger", 1_000);
        let inflamed = transition(&c0, RecoveryEvent::Contained, 1_100).expect("合法");
        let proliferating = transition(&inflamed, RecoveryEvent::Diagnosed, 1_200).expect("合法");
        let remodeling = transition(&proliferating, RecoveryEvent::Repaired, 1_300).expect("合法");

        let back_to_proliferation =
            transition(&remodeling, RecoveryEvent::Rollback, 1_400).expect("重塑可退回增生");
        assert_eq!(back_to_proliferation.phase, HealingPhase::Proliferation);
        let back_to_inflammation =
            transition(&back_to_proliferation, RecoveryEvent::Rollback, 1_500)
                .expect("增生可退回炎症");
        assert_eq!(back_to_inflammation.phase, HealingPhase::Inflammation);

        for case in [&c0, &inflamed, &resolved_case()] {
            assert_eq!(
                transition(case, RecoveryEvent::Rollback, 9_000),
                Err(TransitionError::Illegal {
                    from: case.phase,
                    event: RecoveryEvent::Rollback
                }),
                "止血/炎症/已解决阶段没有可回滚的修复动作"
            );
        }
    }

    #[test]
    fn relapse_reopens_from_resolved_and_remodeling() {
        let resolved = resolved_case();
        let reopened = transition(&resolved, RecoveryEvent::Relapse, 2_000).expect("已解决可复发");
        assert_eq!(reopened.phase, HealingPhase::Inflammation);
        assert!(!reopened.is_resolved());

        let remodeling = RecoveryCase {
            phase: HealingPhase::Remodeling,
            ..resolved.clone()
        };
        let regressed =
            transition(&remodeling, RecoveryEvent::Relapse, 2_100).expect("重塑期可倒退");
        assert_eq!(regressed.phase, HealingPhase::Inflammation);

        let early = RecoveryCase::new("c-2", "trigger", 0);
        assert_eq!(
            transition(&early, RecoveryEvent::Relapse, 1),
            Err(TransitionError::Illegal {
                from: HealingPhase::Hemostasis,
                event: RecoveryEvent::Relapse
            }),
            "止血期尚无伤口可复发"
        );
    }

    #[test]
    fn repeated_verified_is_rejected_as_idempotent() {
        let resolved = resolved_case();
        let first = transition(&resolved, RecoveryEvent::Verified, 2_000);
        assert_eq!(
            first,
            Err(TransitionError::Idempotent {
                phase: HealingPhase::Resolved,
                event: RecoveryEvent::Verified
            }),
            "重复验证必须拒绝而不是静默 no-op"
        );
        assert!(!matches!(first, Err(TransitionError::Illegal { .. })));
        assert_eq!(resolved.phase, HealingPhase::Resolved, "拒绝不改变原案例");
    }

    #[test]
    fn repeated_non_terminal_event_is_illegal() {
        let inflamed = transition(
            &RecoveryCase::new("c-1", "trigger", 1_000),
            RecoveryEvent::Contained,
            1_100,
        )
        .expect("合法");
        let again = transition(&inflamed, RecoveryEvent::Contained, 1_200);
        assert_eq!(
            again,
            Err(TransitionError::Illegal {
                from: HealingPhase::Inflammation,
                event: RecoveryEvent::Contained
            }),
            "同事件重复投递因相位已推进而非法"
        );
        // 错误可展示、可转换为 std::error::Error。
        let err = again.expect_err("应失败");
        assert!(err.to_string().contains("inflammation"));
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn evidence_is_bounded_and_keeps_newest() {
        let mut case = resolved_case();
        for i in 0..10u64 {
            let at = 2_000 + i * 10;
            case = transition(&case, RecoveryEvent::Relapse, at).expect("合法");
            case = transition(&case, RecoveryEvent::Diagnosed, at + 1).expect("合法");
            case = transition(&case, RecoveryEvent::Repaired, at + 2).expect("合法");
            case = transition(&case, RecoveryEvent::Verified, at + 3).expect("合法");
        }
        assert_eq!(case.phase, HealingPhase::Resolved);
        assert_eq!(case.evidence.len(), EVIDENCE_CAP, "证据串应有界");
        assert!(case
            .evidence
            .last()
            .is_some_and(|e| e.starts_with("verified@")));
        assert!(
            !case.evidence.iter().any(|e| e.starts_with("opened@")),
            "最旧证据被裁剪"
        );
    }

    #[test]
    fn updated_ms_never_goes_backwards() {
        let c1 = transition(
            &RecoveryCase::new("c-1", "trigger", 1_000),
            RecoveryEvent::Contained,
            5_000,
        )
        .expect("合法");
        assert_eq!(c1.updated_ms, 5_000);
        let c2 = transition(&c1, RecoveryEvent::Diagnosed, 1_000).expect("合法");
        assert_eq!(c2.updated_ms, 5_000, "注入时间回退不写回时间轴");
        assert_eq!(
            c2.evidence.last().map(String::as_str),
            Some("diagnosed@1000ms")
        );
    }

    #[test]
    fn transition_is_deterministic() {
        let case = resolved_case();
        let a = transition(&case, RecoveryEvent::Relapse, 3_000);
        let b = transition(&case, RecoveryEvent::Relapse, 3_000);
        assert_eq!(a, b, "同输入同输出");

        let next = a.expect("已解决可复发");
        assert_eq!(next.phase, HealingPhase::Inflammation);
        assert_eq!(next.updated_ms, 3_000);
        assert_eq!(next.started_ms, case.started_ms);
        assert_eq!(next.evidence.len(), case.evidence.len() + 1);
        assert_eq!(
            next.evidence.last().map(String::as_str),
            Some("relapse@3000ms")
        );
    }

    #[test]
    fn scar_dedup_accumulates_hits_and_keeps_first_lesson() {
        let mut ledger = ScarLedger::new(8);
        let first = ledger.learn("http_403", "权限配置缺 scope", "调用前先校验 scope", 100);
        assert_eq!(first.hits, 1);
        assert_eq!(first.risk(), 0.5);

        let second = ledger.learn("http_403", "另一条原因", "另一条教训", 200);
        assert_eq!(second.hits, 2);
        assert_eq!(second.scar.cause, "权限配置缺 scope", "首次成因是瘢痕本体");
        assert_eq!(second.scar.lesson, "调用前先校验 scope", "首次教训不被覆盖");
        assert_eq!(second.scar.created_ms, 100);
        assert_eq!(second.last_ms, 200);
        assert_eq!(ledger.len(), 1, "按 signature 去重");
        assert!(second.risk() > first.risk());
        assert!(second.risk() < 1.0, "风险代理有界");
        assert_eq!(ledger.risk("http_403"), Some(second.risk()));
        assert_eq!(ledger.risk("missing"), None);
    }

    #[test]
    fn scar_capacity_evicts_least_reproduced() {
        let mut ledger = ScarLedger::new(2);
        ledger.learn("a", "ca", "la", 100);
        ledger.learn("b", "cb", "lb", 200);
        ledger.learn("a", "ca", "la", 300); // a: hits 2, last 300
        ledger.learn("c", "cc", "lc", 400); // 淘汰 hits 最少的 b
        assert_eq!(ledger.len(), 2);
        assert_eq!(ledger.capacity(), 2);
        assert!(ledger.get("b").is_none());
        assert_eq!(ledger.get("a").map(|r| r.hits), Some(2));
        assert_eq!(ledger.get("c").map(|r| r.hits), Some(1));
    }

    #[test]
    fn scar_capacity_tie_breaks_by_age_then_signature() {
        let mut ledger = ScarLedger::new(2);
        ledger.learn("sig_z", "cz", "lz", 100);
        ledger.learn("sig_a", "ca", "la", 100);
        // 同 hits、同 last_ms → 签名字典序最小者（sig_a）淘汰。
        ledger.learn("sig_m", "cm", "lm", 200);
        assert!(ledger.get("sig_a").is_none());
        assert!(ledger.get("sig_z").is_some());
        assert!(ledger.get("sig_m").is_some());
    }

    #[test]
    fn scar_zero_capacity_and_determinism() {
        let mut ledger = ScarLedger::new(0);
        assert_eq!(ledger.capacity(), 1);
        ledger.learn("a", "ca", "la", 0);
        ledger.learn("b", "cb", "lb", 1);
        assert_eq!(ledger.len(), 1, "容量 0 抬到 1 且不 panic");
        assert!(ledger.get("a").is_none());
        assert!(ledger.get("b").is_some());

        let mut x = ScarLedger::new(4);
        let mut y = ScarLedger::new(4);
        for (sig, at) in [("a", 10u64), ("b", 20), ("a", 30), ("c", 40)] {
            assert_eq!(
                x.learn(sig, "cause", "lesson", at),
                y.learn(sig, "cause", "lesson", at)
            );
        }
        assert_eq!(x, y, "同序列同账本");
    }
}
