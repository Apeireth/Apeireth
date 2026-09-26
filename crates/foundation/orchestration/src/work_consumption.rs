//! 工作消耗记账: 从事件日志单趟纯折叠出「本回合实际消耗了什么」。
//!
//! # 可折叠事实
//!
//! 回合的消耗不是过程中的临时估算, 而是**事件日志里的可折叠事实**:
//! [`WorkEvent`] 追加进 [`WorkLog`] (append-only, 不可变), [`fold_consumed_work`]
//! 一趟线性折叠出 [`ConsumedWork`]。同一份日志折多少遍都是同一个答案,
//! 与谁在什么时候折无关。
//!
//! # 取消/中断同答案
//!
//! 取消 ([`WorkEvent::Cancelled`]) 与中断 ([`WorkEvent::Interrupted`]) 不回滚
//! 已经发生的消耗 —— 事件在日志里就在。因此「干没干活」在任何取消/中断路径上
//! 都读到同一答案: 折叠只看日志, 不看调用方走到了哪条收场路径。
//!
//! # 单趟确定性
//!
//! [`fold_consumed_work`] 一趟顺序扫描、无排序、无回看、无副作用: 给定同一
//! 事件序列, 结果逐字段相等 ([`ConsumedWork`] 全字段可比)。
//!
//! # 与回合生命周期叠加 (不改语义)
//!
//! [`WorkEvent`] 只携带轮次号与时刻, 不解释、不改写任何既有回合生命周期状态;
//! 输入侧的耐久队列见 [`crate::durable_inbox`]。本模块无 IO、无时钟、无全局态。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 消耗的工件种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkKind {
    /// 一步工作 (step)。
    Step,
    /// 一次工具调用 (tool call)。
    ToolCall,
}

/// 回合工作事件 (可折叠事实, 只追加)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkEvent {
    /// 回合开始。
    TurnStarted {
        /// 轮次号。
        turn: u64,
        /// 时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 记一笔消耗 (完成一个工件并消耗 `units` 单位工作量)。
    Consumed {
        /// 轮次号。
        turn: u64,
        /// 时刻 (epoch 毫秒)。
        at_ms: i64,
        /// 工件种类。
        kind: WorkKind,
        /// 消耗的工作量单位 (必须 > 0)。
        units: u64,
    },
    /// 回合被显式取消 (已发生的消耗不回滚)。
    Cancelled {
        /// 轮次号。
        turn: u64,
        /// 时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 回合被外部中断 (已发生的消耗不回滚)。
    Interrupted {
        /// 轮次号。
        turn: u64,
        /// 时刻 (epoch 毫秒)。
        at_ms: i64,
    },
    /// 回合正常收尾。
    Finished {
        /// 轮次号。
        turn: u64,
        /// 时刻 (epoch 毫秒)。
        at_ms: i64,
    },
}

impl WorkEvent {
    /// 事件归属的轮次号。
    pub const fn turn(&self) -> u64 {
        match self {
            Self::TurnStarted { turn, .. }
            | Self::Consumed { turn, .. }
            | Self::Cancelled { turn, .. }
            | Self::Interrupted { turn, .. }
            | Self::Finished { turn, .. } => *turn,
        }
    }
}

/// 回合收场口径 (日志里最后一个生命周期事件说了算)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnOutcome {
    /// 尚未收场 (无生命周期收场事件)。
    Open,
    /// 正常收尾。
    Completed,
    /// 被显式取消。
    Cancelled,
    /// 被外部中断。
    Interrupted,
}

/// 单趟折叠结果: 本回合实际消耗了什么。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumedWork {
    /// 轮次号。
    pub turn: u64,
    /// 消耗的 step 数。
    pub steps: u64,
    /// 消耗的工具调用数。
    pub tool_calls: u64,
    /// 消耗的工作量单位合计。
    pub units: u64,
    /// 折过的事件条数 (只数归属本回合的)。
    pub events_folded: u64,
    /// 收场口径 (日志里最后一个生命周期事件; 无则 [`TurnOutcome::Open`])。
    pub outcome: TurnOutcome,
}

impl ConsumedWork {
    /// 空消耗 (回合号已知, 一无所有)。
    pub const fn empty(turn: u64) -> Self {
        Self {
            turn,
            steps: 0,
            tool_calls: 0,
            units: 0,
            events_folded: 0,
            outcome: TurnOutcome::Open,
        }
    }
}

/// 单趟纯折叠: 从事件日志折出 `turn` 回合实际消耗了什么。
///
/// 一趟顺序扫描、无排序、无回看、无副作用; 给定同一事件序列, 结果逐字段相等。
/// 取消/中断不回滚已记账的消耗 —— 任何收场路径读到同一答案。
pub fn fold_consumed_work(turn: u64, events: &[WorkEvent]) -> ConsumedWork {
    let mut acc = ConsumedWork::empty(turn);
    for event in events {
        if event.turn() != turn {
            continue;
        }
        acc.events_folded = acc.events_folded.saturating_add(1);
        match event {
            WorkEvent::TurnStarted { .. } => {}
            WorkEvent::Consumed { kind, units, .. } => {
                match kind {
                    WorkKind::Step => acc.steps = acc.steps.saturating_add(1),
                    WorkKind::ToolCall => acc.tool_calls = acc.tool_calls.saturating_add(1),
                }
                acc.units = acc.units.saturating_add(*units);
            }
            WorkEvent::Cancelled { .. } => acc.outcome = TurnOutcome::Cancelled,
            WorkEvent::Interrupted { .. } => acc.outcome = TurnOutcome::Interrupted,
            WorkEvent::Finished { .. } => acc.outcome = TurnOutcome::Completed,
        }
    }
    acc
}

/// 追加式事件日志 (可折叠事实的载体; append-only, 不回改)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLog {
    events: Vec<WorkEvent>,
}

impl WorkLog {
    /// 空日志。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条事件 (按追加序折叠)。
    pub fn append(&mut self, event: WorkEvent) {
        self.events.push(event);
    }

    /// 全部事件 (追加序)。
    pub fn events(&self) -> &[WorkEvent] {
        &self.events
    }

    /// 折出指定回合的实际消耗 ([`fold_consumed_work`] 的便捷口)。
    pub fn fold(&self, turn: u64) -> ConsumedWork {
        fold_consumed_work(turn, &self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;

    fn sample_log() -> WorkLog {
        let mut log = WorkLog::new();
        log.append(WorkEvent::TurnStarted { turn: 3, at_ms: T0 });
        log.append(WorkEvent::Consumed {
            turn: 3,
            at_ms: T0 + 1,
            kind: WorkKind::Step,
            units: 2,
        });
        log.append(WorkEvent::Consumed {
            turn: 3,
            at_ms: T0 + 2,
            kind: WorkKind::ToolCall,
            units: 5,
        });
        log
    }

    #[test]
    fn fold_consumed_work_is_a_single_pass_deterministic_fold() {
        let log = sample_log();
        let first = fold_consumed_work(3, log.events());
        let second = fold_consumed_work(3, log.events());
        // 单趟确定性: 同一日志折多少遍都是同一个答案。
        assert_eq!(first, second);
        assert_eq!(
            first,
            ConsumedWork {
                turn: 3,
                steps: 1,
                tool_calls: 1,
                units: 7,
                events_folded: 3,
                outcome: TurnOutcome::Open,
            }
        );

        // 他回合事件不掺入本答案 (轮次隔离)。
        let mut extended = log.clone();
        extended.append(WorkEvent::Consumed {
            turn: 4,
            at_ms: T0 + 3,
            kind: WorkKind::Step,
            units: 99,
        });
        assert_eq!(fold_consumed_work(3, extended.events()), first);
    }

    #[test]
    fn consumed_work_stays_queryable_after_cancel() {
        let mut log = sample_log();
        log.append(WorkEvent::Cancelled {
            turn: 3,
            at_ms: T0 + 9,
        });

        // 取消后「干没干活」照常可查: 已发生的消耗不回滚。
        let folded = log.fold(3);
        assert_eq!(folded.units, 7);
        assert_eq!(folded.steps, 1);
        assert_eq!(folded.tool_calls, 1);
        assert_eq!(folded.outcome, TurnOutcome::Cancelled);
        assert_eq!(folded.events_folded, 4);

        // 取消路径与不取消路径读同一份日志 = 同一答案的消耗部分。
        let mut no_cancel = sample_log();
        no_cancel.append(WorkEvent::Finished {
            turn: 3,
            at_ms: T0 + 9,
        });
        let other = no_cancel.fold(3);
        assert_eq!(other.units, folded.units);
        assert_eq!(other.steps, folded.steps);
        assert_eq!(other.tool_calls, folded.tool_calls);
        assert_ne!(other.outcome, folded.outcome, "收场口径不同, 消耗口径相同");
    }

    #[test]
    fn interrupted_and_finished_outcomes_come_from_the_last_lifecycle_event() {
        let mut log = sample_log();
        log.append(WorkEvent::Interrupted {
            turn: 3,
            at_ms: T0 + 4,
        });
        assert_eq!(log.fold(3).outcome, TurnOutcome::Interrupted);
        log.append(WorkEvent::Finished {
            turn: 3,
            at_ms: T0 + 5,
        });
        assert_eq!(log.fold(3).outcome, TurnOutcome::Completed);

        // 中断同样不回滚消耗。
        assert_eq!(log.fold(3).units, 7);
    }

    #[test]
    fn zero_work_turn_reads_as_empty_consumption() {
        let log = WorkLog::new();
        let folded = fold_consumed_work(9, log.events());
        assert_eq!(folded, ConsumedWork::empty(9));
        assert_eq!(folded.units, 0);
        assert_eq!(folded.outcome, TurnOutcome::Open);
    }
}
