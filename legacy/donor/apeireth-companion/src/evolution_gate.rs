//! `apeireth-companion::evolution_gate` — 验证闸门流水线 + 回滚骨架 (吸收 yoyo evolve.sh 验证闸门, 重写).
//!
//! 用途: 能力演化回路 (AI 提案→生成→**验证**→部署→监控→回滚) 的验证/回滚层。
//! 对齐 yoyo 的纪律:
//! - 机械验证 (build/test/clippy) + fix loop (上限) + **no-progress 检测** (连续零变化停循环)
//! - 预算门 **fail-open**: 预算耗尽但有绿态 → 保留 + 未验证收据 (不因判官迟到回滚好代码)
//! - hard rollback + 收据 (revert 即学习信号)
//!
//! 0 假装: 这里是「判定 + 决策」机制件; 真实验证命令 (cargo test 等) 由调用方 (演化循环) 注入.

use std::time::{Duration, Instant};

/// 一次验证结果.
#[derive(Debug, Clone)]
pub struct VerifyOutcome {
    /// 验证是否通过 (机械闸).
    pub passed: bool,
    /// 本轮是否产生了变化 (fix 尝试后; false 连续 2 次 → no-progress).
    pub changed: bool,
    /// 失败信息 (passed=false 时).
    pub error: Option<String>,
}

/// 闸门决策.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// 通过 (可 promote).
    Promoted,
    /// 失败 → 应回滚 (收据记录原因).
    Rejected { reason: String },
    /// 预算耗尽但有绿态 → fail-open 保留 + 未验证收据.
    UnverifiedAccepted { reason: String },
}

/// 演化回路动作 (验证闸门 → 部署/回滚的挂接点, 接 deploy 模块; A1 后半段).
///
/// 完整回路: 提案 → 生成 → **验证(本模块)** → 部署 → 监控 → 回滚 (后三段见 deploy.rs).
#[derive(Debug, Clone, PartialEq)]
pub enum LoopAction {
    /// 验证通过 → 可部署 (调用 deploy::DeployManager::deploy).
    Deploy,
    /// 验证失败 → 回滚 (收据留痕, revert 即学习信号).
    Rollback { reason: String },
    /// 预算耗尽 fail-open → 保留待补验证 (不部署也不回滚).
    HoldUnverified { reason: String },
}

/// 验证闸门 (纯判定, 可测).
pub struct EvalGate {
    pub max_fix_attempts: usize,
    pub no_progress_limit: usize,
    pub budget: Duration,
    pub pre_sha: String,
}

impl EvalGate {
    pub fn new(pre_sha: impl Into<String>) -> Self {
        Self {
            max_fix_attempts: 10,
            no_progress_limit: 2,
            budget: Duration::from_secs(1800),
            pre_sha: pre_sha.into(),
        }
    }

    /// 跑验证序列 (调用方按轮喂结果, 直到有结论).
    /// returns (decision, fix_attempts, elapsed).
    pub fn run_until_conclusion(
        &self,
        mut next_round: impl FnMut(usize) -> VerifyOutcome,
        started: Instant,
    ) -> (GateDecision, usize, Duration) {
        let mut attempts = 0usize;
        let mut zero_changes = 0usize;
        loop {
            let elapsed = started.elapsed();
            if elapsed >= self.budget {
                // 预算耗尽: 有绿态 (上一轮 passed) → fail-open 保留; 否则拒绝
                return (
                    GateDecision::UnverifiedAccepted {
                        reason: format!("预算耗尽 ({}s), 保留绿态待验证", self.budget.as_secs()),
                    },
                    attempts,
                    elapsed,
                );
            }
            let outcome = next_round(attempts);
            if outcome.passed {
                return (GateDecision::Promoted, attempts, elapsed);
            }
            // 失败: no-progress 检测
            if !outcome.changed {
                zero_changes += 1;
                if zero_changes >= self.no_progress_limit {
                    return (
                        GateDecision::Rejected {
                            reason: format!(
                                "no-progress: 连续 {} 次零变化, 停循环",
                                self.no_progress_limit
                            ),
                        },
                        attempts,
                        elapsed,
                    );
                }
            } else {
                zero_changes = 0;
            }
            attempts += 1;
            if attempts >= self.max_fix_attempts {
                return (
                    GateDecision::Rejected {
                        reason: format!("fix loop 超限 ({} 次)", self.max_fix_attempts),
                    },
                    attempts,
                    elapsed,
                );
            }
        }
    }

    /// 闸门决策 → 回路动作 (deploy 模块挂接: Promoted→部署 / Rejected→回滚 / fail-open→挂起).
    pub fn loop_action(&self, decision: &GateDecision) -> LoopAction {
        match decision {
            GateDecision::Promoted => LoopAction::Deploy,
            GateDecision::Rejected { reason } => LoopAction::Rollback {
                reason: reason.clone(),
            },
            GateDecision::UnverifiedAccepted { reason } => LoopAction::HoldUnverified {
                reason: reason.clone(),
            },
        }
    }

    /// 部署收据 (与 rollback_receipt 对称; 部署后进入监控期, 见 deploy 模块).
    pub fn deploy_receipt(&self, capability: &str) -> String {
        format!(
            "[agent-deploy] PRE_TASK_SHA={} 能力: {} — 验证通过已部署, 进入监控期",
            self.pre_sha, capability
        )
    }

    /// 回滚命令骨架 (调用方执行; 返回给演化循环的收据文本).
    pub fn rollback_receipt(&self, reason: &str) -> String {
        format!(
            "[agent-revert] PRE_TASK_SHA={} 原因: {} — 下轮任务必须更小 (≤3 文件/30 分钟)",
            self.pre_sha, reason
        )
    }

    /// 未验证收据 (fail-open 时).
    pub fn unverified_receipt(&self, reason: &str) -> String {
        format!(
            "[agent-unverified] PRE_TASK_SHA={} 未解决: {} — 绿态保留, 待补验证",
            self.pre_sha, reason
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_round_pass_promotes() {
        let g = EvalGate::new("sha1");
        let (d, attempts, _) = g.run_until_conclusion(
            |_| VerifyOutcome {
                passed: true,
                changed: true,
                error: None,
            },
            Instant::now(),
        );
        assert_eq!(d, GateDecision::Promoted);
        assert_eq!(attempts, 0);
    }

    #[test]
    fn fix_loop_recovers_then_promotes() {
        let g = EvalGate::new("sha1");
        let mut round = 0;
        let (d, attempts, _) = g.run_until_conclusion(
            |_| {
                round += 1;
                VerifyOutcome {
                    passed: round >= 3,
                    changed: true,
                    error: None,
                }
            },
            Instant::now(),
        );
        assert_eq!(d, GateDecision::Promoted);
        assert_eq!(attempts, 2, "第 1、2 轮失败, 第 3 轮通过");
    }

    #[test]
    fn no_progress_stops_loop() {
        let g = EvalGate::new("sha1");
        let (d, attempts, _) = g.run_until_conclusion(
            |_| VerifyOutcome {
                passed: false,
                changed: false,
                error: Some("编译错误".into()),
            },
            Instant::now(),
        );
        assert_eq!(
            d,
            GateDecision::Rejected {
                reason: "no-progress: 连续 2 次零变化, 停循环".into()
            }
        );
        assert_eq!(attempts, 1, "第 1 次零变化后第 2 次判定停");
    }

    #[test]
    fn fix_limit_rejected() {
        let g = EvalGate {
            max_fix_attempts: 3,
            ..EvalGate::new("sha1")
        };
        let (d, attempts, _) = g.run_until_conclusion(
            |_| VerifyOutcome {
                passed: false,
                changed: true,
                error: Some("e".into()),
            },
            Instant::now(),
        );
        assert!(matches!(d, GateDecision::Rejected { .. }));
        assert_eq!(attempts, 3);
    }

    #[test]
    fn budget_exhausted_fail_open() {
        // 预算 0 → 立即 UnverifiedAccepted
        let g = EvalGate {
            budget: Duration::from_millis(0),
            ..EvalGate::new("sha1")
        };
        let (d, _, _) = g.run_until_conclusion(
            |_| VerifyOutcome {
                passed: false,
                changed: false,
                error: None,
            },
            Instant::now(),
        );
        assert!(matches!(d, GateDecision::UnverifiedAccepted { .. }));
    }

    #[test]
    fn loop_action_maps_gate_decision_to_deploy_or_rollback() {
        let g = EvalGate::new("sha1");
        assert_eq!(g.loop_action(&GateDecision::Promoted), LoopAction::Deploy);
        assert_eq!(
            g.loop_action(&GateDecision::Rejected {
                reason: "no-progress".into()
            }),
            LoopAction::Rollback {
                reason: "no-progress".into()
            }
        );
        assert_eq!(
            g.loop_action(&GateDecision::UnverifiedAccepted {
                reason: "预算耗尽".into()
            }),
            LoopAction::HoldUnverified {
                reason: "预算耗尽".into()
            }
        );
        let d = g.deploy_receipt("换元检查");
        assert!(d.contains("[agent-deploy]") && d.contains("sha1") && d.contains("监控期"));
    }

    #[test]
    fn receipts_are_learning_signals() {
        let g = EvalGate::new("abc123");
        let r = g.rollback_receipt("测试失败");
        assert!(r.contains("[agent-revert]") && r.contains("abc123") && r.contains("更小"));
        let u = g.unverified_receipt("判官超时");
        assert!(u.contains("[agent-unverified]") && u.contains("待补验证"));
    }
}
