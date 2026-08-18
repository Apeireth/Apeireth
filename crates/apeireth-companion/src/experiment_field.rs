//! `apeireth-companion::experiment_field` — 自我改进闭环缺环补全: VM 实验场.
//!
//! ## 哲学 (主人 2026-08-18: "自我改进闭环应该立刻补全, 不然她升级自己一直缺一环")
//!
//! 完整回路: 提案 → **实验** → 通过 → 主人批准 → 部署 → 监控 → 回滚 → **学习**。
//! 此前缺两环:
//! 1. **实验**: 提案没有"先试再部署"的地方 — 直接进部署链, 改坏了伤本体。
//!    smol-vm (Rust+libkrun 微 VM) 方向: "**独立的是实验, 批准的是部署**"。
//! 2. **回滚学习**: 部署有回滚收据, 但收据没有回流成学习信号 (yoyo revert-receipt 模式)。
//!
//! ## 0 装 PASS
//!
//! - [`VMRunner`] trait 口已备; 默认 [`NoopVMRunner`] 诚实 Err (VM 未接, 不假装能跑实验).
//! - 接 smol-vm/libkrun 时实现 trait 即可, 机制件不动.
//! - 回滚学习写 [`crate::experience::ExperienceStore`] (既有经验库, 集成而非分立).

/// 实验状态 (确定性状态机).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentStatus {
    /// 提案已受理, 待实验.
    Proposed,
    /// VM 内构建中.
    Building,
    /// 构建完成, 测试中.
    Testing,
    /// 构建+测试通过 — 可进部署链 (仍需主人批准).
    Passed,
    /// 构建或测试失败 — 失败原因即学习信号.
    Failed,
}

/// 实验判决 (runner 产出).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// 构建+测试全过.
    Pass,
    /// 失败 + 原因 (可行动).
    Fail(String),
}

/// VM 实验执行器 trait 口 (smol-vm/libkrun 接入点).
pub trait VMRunner: Send + Sync + std::fmt::Debug {
    /// 在隔离 VM 内: 构建 + 测试候选, 返回判决.
    /// 隔离保证: 炸了不伤本体 (0 装: 是否真隔离由实现者保证).
    fn run_build_and_test(&self, artifact: &str) -> Result<Verdict, String>;
}

/// 默认实现: VM 未接 → 诚实 Err (0 装 PASS).
#[derive(Debug, Default)]
pub struct NoopVMRunner;

impl VMRunner for NoopVMRunner {
    fn run_build_and_test(&self, _artifact: &str) -> Result<Verdict, String> {
        Err("NoopVMRunner: VM 实验场未接入 (smol-vm/libkrun 实现 VMRunner 时启用)".into())
    }
}

/// 一个实验候选.
#[derive(Debug, Clone)]
pub struct Experiment {
    pub id: u64,
    /// 来源提案 (能力提案 id/描述).
    pub proposal: String,
    /// 候选产物 (构建/测试目标描述).
    pub artifact: String,
    pub status: ExperimentStatus,
    /// 失败原因 (Failed 时非空; 即学习信号).
    pub failure_reason: Option<String>,
    /// 是否已获准进部署链 (Passed + 主人批准后置 true).
    pub approved_for_deploy: bool,
    pub at_ms: i64,
}

/// 实验场 (确定性状态机 + trait 口).
#[derive(Debug)]
pub struct ExperimentField {
    runner: Box<dyn VMRunner>,
    items: std::collections::HashMap<u64, Experiment>,
    next_id: u64,
}

impl ExperimentField {
    pub fn new(runner: Box<dyn VMRunner>) -> Self {
        Self {
            runner,
            items: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    /// 受理提案 → 实验候选 (Proposed).
    pub fn propose(
        &mut self,
        proposal: impl Into<String>,
        artifact: impl Into<String>,
    ) -> Experiment {
        let e = Experiment {
            id: self.next_id,
            proposal: proposal.into(),
            artifact: artifact.into(),
            status: ExperimentStatus::Proposed,
            failure_reason: None,
            approved_for_deploy: false,
            at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.next_id += 1;
        self.items.insert(e.id, e.clone());
        e
    }

    /// 跑实验: VM 内构建+测试 → Passed/Failed.
    /// Runner Err (VM 未接) → 状态保持 Proposed, Err 返回 (0 装).
    pub fn run(&mut self, id: u64) -> Result<ExperimentStatus, String> {
        let e = self.items.get_mut(&id).ok_or("实验不存在")?;
        if e.status != ExperimentStatus::Proposed {
            return Err(format!("状态 {:?} 不可重跑 (仅 Proposed 可)", e.status));
        }
        e.status = ExperimentStatus::Building;
        let verdict = match self.runner.run_build_and_test(&e.artifact) {
            Ok(v) => v,
            Err(msg) => {
                // 0 装 PASS: 实验未执行 (VM 未接/运行器故障) → 状态回 Proposed, 不假装已实验.
                e.status = ExperimentStatus::Proposed;
                return Err(msg);
            }
        };
        match verdict {
            Verdict::Pass => {
                e.status = ExperimentStatus::Passed;
            }
            Verdict::Fail(reason) => {
                e.status = ExperimentStatus::Failed;
                e.failure_reason = Some(reason);
            }
        }
        Ok(e.status)
    }

    /// 通过 + 主人批准 → 可部署 (Passed 才可; 独立的是实验, 批准的是部署).
    pub fn approve_for_deploy(&mut self, id: u64) -> Result<(), String> {
        let e = self.items.get_mut(&id).ok_or("实验不存在")?;
        if e.status != ExperimentStatus::Passed {
            return Err(format!("状态 {:?} 不可批准部署 (仅 Passed 可)", e.status));
        }
        e.approved_for_deploy = true;
        Ok(())
    }

    /// 回滚学习信号: 失败实验 → 经验候选 (yoyo revert-receipt 模式).
    /// 集成而非分立: 写 [`crate::experience::ExperienceStore`].
    pub fn learn_from_failure(
        &self,
        id: u64,
        store: &crate::experience::ExperienceStore,
    ) -> Result<(), String> {
        let e = self.items.get(&id).ok_or("实验不存在")?;
        if e.status != ExperimentStatus::Failed {
            return Err(format!("状态 {:?} 无失败可学 (仅 Failed 可)", e.status));
        }
        let reason = e.failure_reason.clone().unwrap_or_else(|| "未知".into());
        let now = chrono::Utc::now().timestamp_millis();
        let exp = crate::experience::Experience {
            id: format!("exp-{}", e.id),
            chain: format!("experiment-fail-{}", e.id),
            rev: 1,
            scene: format!("实验失败: {}", e.proposal),
            practice: e.artifact.clone(),
            result: reason.clone(),
            outcome: "failure".into(),
            verify_count: 0,
            score: 0.0,
            ready: false,
            proposed: false,
            created_at: now,
            updated_at: now,
        };
        store.save(&exp)
    }

    pub fn get(&self, id: u64) -> Option<&Experiment> {
        self.items.get(&id)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性 Mock runner: 预置通过/失败.
    #[derive(Debug)]
    struct MockRunner {
        fail_artifact: String,
    }

    impl VMRunner for MockRunner {
        fn run_build_and_test(&self, artifact: &str) -> Result<Verdict, String> {
            if artifact.contains(&self.fail_artifact) {
                Ok(Verdict::Fail(format!("构建失败: {artifact}")))
            } else {
                Ok(Verdict::Pass)
            }
        }
    }

    #[test]
    fn propose_run_pass_approve_flow() {
        let mut field = ExperimentField::new(Box::new(MockRunner {
            fail_artifact: "bad".into(),
        }));
        let e = field.propose("cap-1 改进", "artifact-good");
        assert_eq!(e.status, ExperimentStatus::Proposed);
        let st = field.run(e.id).unwrap();
        assert_eq!(st, ExperimentStatus::Passed);
        field.approve_for_deploy(e.id).unwrap();
        assert!(field.get(e.id).unwrap().approved_for_deploy);
    }

    #[test]
    fn failed_experiment_learns() {
        let store = crate::experience::ExperienceStore::new(std::sync::Arc::new(
            apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap(),
        ));
        let mut field = ExperimentField::new(Box::new(MockRunner {
            fail_artifact: "bad".into(),
        }));
        let e = field.propose("cap-2 改进", "artifact-bad");
        let st = field.run(e.id).unwrap();
        assert_eq!(st, ExperimentStatus::Failed);
        assert!(field.get(e.id).unwrap().failure_reason.is_some());
        // 失败实验不能批准部署
        assert!(field.approve_for_deploy(e.id).is_err());
        // 回滚学习: 失败原因 → 经验库
        field.learn_from_failure(e.id, &store).unwrap();
        assert_eq!(store.list(None).len(), 1);
        assert!(store.list(None)[0].scene.contains("实验失败"));
        assert!(store.list(None)[0].result.contains("构建失败"));
    }

    #[test]
    fn noop_runner_is_honest() {
        let mut field = ExperimentField::new(Box::new(NoopVMRunner));
        let e = field.propose("cap-3", "artifact");
        let err = field.run(e.id).unwrap_err();
        assert!(err.contains("未接入"), "{err}");
        assert_eq!(
            field.get(e.id).unwrap().status,
            ExperimentStatus::Proposed,
            "VM 未接不假装已实验"
        );
    }

    #[test]
    fn cannot_rerun_or_approve_wrong_state() {
        let mut field = ExperimentField::new(Box::new(MockRunner {
            fail_artifact: "bad".into(),
        }));
        let e = field.propose("cap-4", "artifact-ok");
        assert!(field.approve_for_deploy(e.id).is_err(), "Proposed 不可批准");
        field.run(e.id).unwrap();
        assert!(field.run(e.id).is_err(), "Passed 不可重跑");
    }

    #[test]
    fn failed_experiment_learn_requires_failed_state() {
        let store = crate::experience::ExperienceStore::new(std::sync::Arc::new(
            apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap(),
        ));
        let mut field = ExperimentField::new(Box::new(MockRunner {
            fail_artifact: "bad".into(),
        }));
        let e = field.propose("cap-5", "artifact-ok");
        field.run(e.id).unwrap(); // Passed
        assert!(
            field.learn_from_failure(e.id, &store).is_err(),
            "Passed 无失败可学"
        );
    }
}
