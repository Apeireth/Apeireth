//! W3 移植批: 自我改进闭环的**实验侧** (v1 `donor/apeireth-companion/
//! experiment_field.rs`, 2026-10-10)。
//!
//! # 定位 (与 upgrade_cycle 互补成环)
//!
//! 完整回路: 提案 → **实验** → 通过 → 主人批准 → 部署 → 监控 → 回滚 → **学习**。
//! - 部署侧 = [`crate::upgrade_cycle::UpgradeCycle`] (L0-L5 六步, git tag, 不自动跑);
//! - **本模块 = 实验侧**: "独立的是实验, 批准的是部署" —— 提案先在场内试,
//!   炸了不伤本体, 才谈批准部署。
//! 此前缺的两环 (实验 + 回滚学习) 即本模块补的两环。
//!
//! # 0 装 PASS (v1 口径原样)
//!
//! - [`VMRunner`] trait 口已备; 默认 [`NoopVMRunner`] 诚实 Err (VM 未接, 不假装
//!   能跑实验); 接 smol-vm/libkrun 时实现 trait 即可, 机制件不动。
//! - **机制层无旋钮** (与 upgrade_cycle 同层: 真触发 = L0 主人手动/governance
//!   审计, CLI 命令面为后续批); runner 注入 = 实验场的"开关"。
//!
//! # 回滚学习适配 (v1 → v2 差异如实记录)
//!
//! v1 的 `learn_from_failure` 写 `Experience { scene, practice, result, outcome }`
//! 记录。v2 **无该记录型** (v2 的"经验"= 抽取体系 → WikiEntry 知识容器) →
//! 适配为 [`FailureLearningSink`] trait (调用方注入): 生产实现
//! [`WikiFailureLearningSink`] 把失败学习信号写成 WikiEntry (topic="实验失败:
//! {proposal}", confidence=0.0, tags=["experiment-failure"])。集成而非分立
//! (复用既有经验库, 与 v1 相同的精神)。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_plugin::experience::{WikiEntry, WikiEntryStore};

/// 实验状态 (确定性状态机, v1 原样)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentStatus {
    /// 提案已受理, 待实验。
    Proposed,
    /// VM 内构建中。
    Building,
    /// 构建完成, 测试中 (v1 保留枚举位; run() 裁定直接落 Passed/Failed)。
    Testing,
    /// 构建+测试通过 — 可进部署链 (仍需主人批准)。
    Passed,
    /// 构建或测试失败 — 失败原因即学习信号。
    Failed,
}

/// 实验判决 (runner 产出, v1 原样)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// 构建+测试全过。
    Pass,
    /// 失败 + 原因 (可行动)。
    Fail(String),
}

/// VM 实验执行器 trait 口 (smol-vm/libkrun 接入点, v1 原样)。
///
/// 隔离保证: 炸了不伤本体 (0 装: 是否真隔离由实现者保证)。
pub trait VMRunner: Send + Sync + std::fmt::Debug {
    /// 在隔离 VM 内: 构建 + 测试候选, 返回判决。
    fn run_build_and_test(&self, artifact: &str) -> Result<Verdict, String>;
}

/// 默认实现: VM 未接 → 诚实 Err (0 装 PASS, v1 原样)。
#[derive(Debug, Default)]
pub struct NoopVMRunner;

impl VMRunner for NoopVMRunner {
    fn run_build_and_test(&self, _artifact: &str) -> Result<Verdict, String> {
        Err("NoopVMRunner: VM 实验场未接入 (smol-vm/libkrun 实现 VMRunner 时启用)".into())
    }
}

/// 一个实验候选 (v1 原样字段)。
#[derive(Debug, Clone, PartialEq)]
pub struct Experiment {
    /// 候选 id (自增)。
    pub id: u64,
    /// 来源提案 (能力提案 id/描述)。
    pub proposal: String,
    /// 候选产物 (构建/测试目标描述)。
    pub artifact: String,
    /// 当前状态。
    pub status: ExperimentStatus,
    /// 失败原因 (Failed 时非空; 即学习信号)。
    pub failure_reason: Option<String>,
    /// 是否已获准进部署链 (Passed + 主人批准后置 true)。
    pub approved_for_deploy: bool,
    /// 受理时间 (epoch ms)。
    pub at_ms: i64,
}

/// 失败学习信号 (v1 Experience 记录的等价载荷)。
#[derive(Debug, Clone, PartialEq)]
pub struct FailureLearningRecord {
    /// 来源提案。
    pub proposal: String,
    /// 候选产物。
    pub artifact: String,
    /// 失败原因。
    pub failure_reason: String,
    /// 时间 (epoch ms)。
    pub at_ms: i64,
}

/// 失败学习信号回流 sink (v1 `ExperienceStore.save` 的 v2 适配口)。
pub trait FailureLearningSink: Send + Sync {
    /// 把失败学习信号写回经验域 (实现方决定载体)。
    fn learn(&self, record: &FailureLearningRecord) -> Result<(), String>;
}

/// WikiEntry 载体实现: 失败学习信号 → WikiEntry (v2 经验域容器)。
pub struct WikiFailureLearningSink {
    store: Arc<dyn WikiEntryStore>,
    session_id: String,
}

impl WikiFailureLearningSink {
    /// 构造 (session_id = 归属会话, 用于条目归属)。
    pub fn new(store: Arc<dyn WikiEntryStore>, session_id: impl Into<String>) -> Self {
        Self {
            store,
            session_id: session_id.into(),
        }
    }
}

impl FailureLearningSink for WikiFailureLearningSink {
    fn learn(&self, record: &FailureLearningRecord) -> Result<(), String> {
        let entry = WikiEntry {
            id: format!("experiment-fail-{}-{}", record.at_ms, record.proposal),
            session_id: self.session_id.clone(),
            source_episode_id: format!("experiment-fail-{}", record.at_ms),
            extracted_at: record.at_ms,
            topic: format!("实验失败: {}", record.proposal),
            summary: record.failure_reason.clone(),
            body: record.artifact.clone(),
            confidence: 0.0,
            tags: vec![
                "experiment-failure".to_string(),
                "rollback-learning".to_string(),
            ],
        };
        self.store
            .put_wiki(&entry)
            .map_err(|error| error.to_string())
    }
}

/// 实验场 (确定性状态机 + trait 口, v1 原样机制)。
#[derive(Debug)]
pub struct ExperimentField {
    runner: Box<dyn VMRunner>,
    items: HashMap<u64, Experiment>,
    next_id: u64,
}

impl ExperimentField {
    /// 构造 (runner 注入 = 实验能力开关; 默认 NoopVMRunner = 诚实未接)。
    pub fn new(runner: Box<dyn VMRunner>) -> Self {
        Self {
            runner,
            items: HashMap::new(),
            next_id: 1,
        }
    }

    /// 受理提案 → 实验候选 (Proposed)。
    pub fn propose(
        &mut self,
        proposal: impl Into<String>,
        artifact: impl Into<String>,
    ) -> Experiment {
        let experiment = Experiment {
            id: self.next_id,
            proposal: proposal.into(),
            artifact: artifact.into(),
            status: ExperimentStatus::Proposed,
            failure_reason: None,
            approved_for_deploy: false,
            at_ms: now_epoch_ms(),
        };
        self.next_id += 1;
        self.items.insert(experiment.id, experiment.clone());
        experiment
    }

    /// 跑实验: VM 内构建+测试 → Passed/Failed。
    /// Runner Err (VM 未接) → 状态保持 Proposed, Err 返回 (0 装: 不假装已实验)。
    pub fn run(&mut self, id: u64) -> Result<ExperimentStatus, String> {
        let experiment = self.items.get_mut(&id).ok_or("实验不存在")?;
        if experiment.status != ExperimentStatus::Proposed {
            return Err(format!(
                "状态 {:?} 不可重跑 (仅 Proposed 可)",
                experiment.status
            ));
        }
        experiment.status = ExperimentStatus::Building;
        let verdict = match self.runner.run_build_and_test(&experiment.artifact) {
            Ok(verdict) => verdict,
            Err(message) => {
                // 0 装 PASS: 实验未执行 (VM 未接/运行器故障) → 状态回 Proposed。
                experiment.status = ExperimentStatus::Proposed;
                return Err(message);
            }
        };
        match verdict {
            Verdict::Pass => {
                experiment.status = ExperimentStatus::Passed;
            }
            Verdict::Fail(reason) => {
                experiment.status = ExperimentStatus::Failed;
                experiment.failure_reason = Some(reason);
            }
        }
        Ok(experiment.status)
    }

    /// 通过 + 主人批准 → 可部署 (Passed 才可; 独立的是实验, 批准的是部署)。
    pub fn approve_for_deploy(&mut self, id: u64) -> Result<(), String> {
        let experiment = self.items.get_mut(&id).ok_or("实验不存在")?;
        if experiment.status != ExperimentStatus::Passed {
            return Err(format!(
                "状态 {:?} 不可批准部署 (仅 Passed 可)",
                experiment.status
            ));
        }
        experiment.approved_for_deploy = true;
        Ok(())
    }

    /// 回滚学习信号: 失败实验 → sink (yoyo revert-receipt 模式; v1 的
    /// `ExperienceStore.save` 适配为注入 sink)。
    pub fn learn_from_failure(
        &self,
        id: u64,
        sink: &dyn FailureLearningSink,
    ) -> Result<(), String> {
        let experiment = self.items.get(&id).ok_or("实验不存在")?;
        if experiment.status != ExperimentStatus::Failed {
            return Err(format!(
                "状态 {:?} 无失败可学 (仅 Failed 可)",
                experiment.status
            ));
        }
        let record = FailureLearningRecord {
            proposal: experiment.proposal.clone(),
            artifact: experiment.artifact.clone(),
            failure_reason: experiment
                .failure_reason
                .clone()
                .unwrap_or_else(|| "未知".into()),
            at_ms: experiment.at_ms,
        };
        sink.learn(&record)
    }

    /// 查询实验候选。
    pub fn get(&self, id: u64) -> Option<&Experiment> {
        self.items.get(&id)
    }

    /// 候选数。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 是否空场。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod experiment_field_tests {
    use super::*;
    use std::sync::Mutex;

    /// 确定性 Mock runner (v1 原样): 预置失败产物。
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

    fn mock_field() -> ExperimentField {
        ExperimentField::new(Box::new(MockRunner {
            fail_artifact: "bad".into(),
        }))
    }

    /// 记录型 sink (测试替身, 断言学习信号回流)。
    #[derive(Default)]
    struct RecordingSink {
        records: Mutex<Vec<FailureLearningRecord>>,
    }

    impl FailureLearningSink for RecordingSink {
        fn learn(&self, record: &FailureLearningRecord) -> Result<(), String> {
            self.records
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(record.clone());
            Ok(())
        }
    }

    /// Wiki 假 store (捕获 put_wiki 条目)。
    #[derive(Default)]
    struct FakeWikiStore {
        entries: Mutex<Vec<WikiEntry>>,
    }

    impl WikiEntryStore for FakeWikiStore {
        fn put_wiki(
            &self,
            entry: &WikiEntry,
        ) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
            self.entries
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(entry.clone());
            Ok(())
        }
        fn list_wiki(
            &self,
            _session_id: &str,
            _topic: &str,
            _limit: u32,
        ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<WikiEntry>> {
            Ok(Vec::new())
        }
        fn wiki_for_episode(
            &self,
            _episode_id: &str,
        ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<WikiEntry>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn propose_run_pass_approve_flow() {
        // v1 happy path 原样: propose → run(Pass) → approve_for_deploy。
        let mut field = mock_field();
        let experiment = field.propose("cap-1 改进", "artifact-good");
        assert_eq!(experiment.status, ExperimentStatus::Proposed);
        let status = field.run(experiment.id).unwrap();
        assert_eq!(status, ExperimentStatus::Passed);
        field.approve_for_deploy(experiment.id).unwrap();
        assert!(field.get(experiment.id).unwrap().approved_for_deploy);
    }

    #[test]
    fn failed_experiment_learns_through_sink() {
        // 回滚学习: 失败信号 → sink (记录型替身验证载荷)。
        let mut field = mock_field();
        let experiment = field.propose("cap-2 改进", "artifact-bad");
        assert_eq!(field.run(experiment.id).unwrap(), ExperimentStatus::Failed);
        assert!(field.get(experiment.id).unwrap().failure_reason.is_some());
        assert!(
            field.approve_for_deploy(experiment.id).is_err(),
            "失败实验不可批准部署"
        );

        let sink = RecordingSink::default();
        field.learn_from_failure(experiment.id, &sink).unwrap();
        let records = sink
            .records
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        assert_eq!(records.len(), 1);
        assert!(records[0].proposal.contains("cap-2"));
        assert!(records[0].failure_reason.contains("构建失败"));
    }

    #[test]
    fn failed_learning_maps_to_wiki_entry() {
        // 生产 sink: 失败信号 → WikiEntry (topic/summary/confidence/tags 映射)。
        let store = Arc::new(FakeWikiStore::default());
        let sink = WikiFailureLearningSink::new(store.clone(), "session-x");
        let record = FailureLearningRecord {
            proposal: "cap-9".into(),
            artifact: "artifact-bad".into(),
            failure_reason: "构建失败: artifact-bad".into(),
            at_ms: 1_700_000_000_000,
        };
        sink.learn(&record).unwrap();
        let entries = store
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].topic, "实验失败: cap-9");
        assert_eq!(entries[0].summary, "构建失败: artifact-bad");
        assert_eq!(entries[0].body, "artifact-bad");
        assert_eq!(entries[0].confidence, 0.0);
        assert!(entries[0].tags.contains(&"experiment-failure".to_string()));
    }

    #[test]
    fn noop_runner_is_honest() {
        // 0 装: VM 未接 → run Err 且状态回 Proposed (不假装已实验)。
        let mut field = ExperimentField::new(Box::new(NoopVMRunner));
        let experiment = field.propose("cap-3", "artifact");
        let error = field.run(experiment.id).unwrap_err();
        assert!(error.contains("未接入"), "{error}");
        assert_eq!(
            field.get(experiment.id).unwrap().status,
            ExperimentStatus::Proposed
        );
    }

    #[test]
    fn cannot_rerun_or_approve_wrong_state() {
        let mut field = mock_field();
        let experiment = field.propose("cap-4", "artifact-ok");
        assert!(
            field.approve_for_deploy(experiment.id).is_err(),
            "Proposed 不可批准"
        );
        field.run(experiment.id).unwrap();
        assert!(field.run(experiment.id).is_err(), "Passed 不可重跑");
        assert!(
            field
                .learn_from_failure(experiment.id, &RecordingSink::default())
                .is_err(),
            "Passed 无失败可学"
        );
    }
}
