//! AgentBench stub (v2-strategy §05 Step 6)
//!
//! 目标:留出 AgentBench 子集跑分接口的占位实现,不实际执行任何任务。
//!
//! ## 为什么是 stub
//! - AgentBench 完整版有 8 个 domain(Operating System / Database / Web Shopping / Web Browsing /
//!   Knowledge Graph / Digital Card Game / Lateral Thinking / House Holding),每个 domain 都需要
//!   独立的执行环境。
//! - 当前阶段只关心框架能加载;真实跑分留到 P1 阶段(见 `APEIRETH-ROADMAP.md`)。
//!
//! ## 升级路径 (ceiling)
//! - 真实接入时:把 `StubAgentBenchTask` 换成读 parquet 的 `ParquetAgentBenchTask`;
//! - 把 `StubExecutor` 换成调 docker / vbox 的真实执行器;
//! - 保留 `AgentBenchRunner::run_and_summarize()` 签名,下游聚合层无需改。
//!
//! ## ponytail 原则
//! - 不写任何未使用的 trait method;
//! - 不构造任何假数据去"看起来在跑";
//! - stub 行为 = "返回 0/0 占位 summary",让上游能感知到"还没接"。

use serde::{Deserialize, Serialize};

/// AgentBench 任务类别 (与官方 8 domain 对齐,先列 5 个代表性 domain)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentBenchCategory {
    /// Operating System(shell / file system 操作).
    OperatingSystem,
    /// Database(SQL CRUD + schema).
    Database,
    /// Web Shopping(电商场景多轮交互).
    WebShopping,
    /// Web Browsing(网页信息抽取).
    WebBrowsing,
    /// Knowledge Graph(基于 KG 的多跳推理).
    KnowledgeGraph,
    /// 其它 / 未分类.
    Other,
}

/// AgentBench 任务 trait。
///
/// 真实实现会持有 `id / category / environment_spec / scorer_fn`;
/// stub 实现只填 id + category,其余字段留空。
pub trait AgentBenchTask {
    fn id(&self) -> &str;
    fn category(&self) -> AgentBenchCategory;
}

/// Stub 任务:仅返回 id + category。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubAgentBenchTask {
    pub id: String,
    pub category: AgentBenchCategory,
}

impl AgentBenchTask for StubAgentBenchTask {
    fn id(&self) -> &str {
        &self.id
    }
    fn category(&self) -> AgentBenchCategory {
        self.category
    }
}

/// Stub executor:不跑任何任务,直接产出零结果。
///
/// 这样调用方能感知"stub 模式",而不是误以为跑成功了。
pub struct StubExecutor;

impl StubExecutor {
    pub fn run(&self, _task: &dyn AgentBenchTask) -> StubReport {
        StubReport {
            task_id: _task.id().to_string(),
            category: _task.category(),
            ran: false,
            note: "stub: 真实执行器未接入 (P1+ 实现)".to_string(),
        }
    }
}

/// Stub 单任务报告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StubReport {
    pub task_id: String,
    pub category: AgentBenchCategory,
    pub ran: bool,
    pub note: String,
}

/// AgentBench runner(stub 状态):持有 task list + stub executor。
#[derive(Default)]
pub struct AgentBenchRunner {
    tasks: Vec<Box<dyn AgentBenchTask + Send + Sync>>,
}

impl AgentBenchRunner {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, task: impl AgentBenchTask + Send + Sync + 'static) -> &mut Self {
        self.tasks.push(Box::new(task));
        self
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// 跑全部 task(stub 模式)。
    pub fn run(&self) -> Vec<StubReport> {
        self.tasks
            .iter()
            .map(|t| StubExecutor.run(t.as_ref()))
            .collect()
    }
}

/// 聚合 summary(stub 状态):目前只能告诉你 "N 个 task, 0 个真跑了"。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBenchSummary {
    pub total: usize,
    pub ran: usize,
}

impl AgentBenchSummary {
    pub fn from_reports(reports: &[StubReport]) -> Self {
        Self {
            total: reports.len(),
            ran: reports.iter().filter(|r| r.ran).count(),
        }
    }

    /// 人读格式。
    pub fn format(&self) -> String {
        format!(
            "[agent-bench stub] total={} ran={} (0 真跑;P1+ 接 executor)",
            self.total, self.ran
        )
    }
}

/// 构造一个空 runner(stub 状态),供 example / 后续扩展使用。
pub fn stub_runner() -> AgentBenchRunner {
    AgentBenchRunner::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_task_reports_id_and_category() {
        let t = StubAgentBenchTask {
            id: "os-shell-001".to_string(),
            category: AgentBenchCategory::OperatingSystem,
        };
        assert_eq!(t.id(), "os-shell-001");
        assert_eq!(t.category(), AgentBenchCategory::OperatingSystem);
    }

    #[test]
    fn stub_executor_marks_unran() {
        let t = StubAgentBenchTask {
            id: "db-crud-002".to_string(),
            category: AgentBenchCategory::Database,
        };
        let r = StubExecutor.run(&t);
        assert!(!r.ran);
        assert!(r.note.contains("stub"));
    }

    #[test]
    fn stub_runner_summary_is_zero_ran() {
        let mut r = stub_runner();
        r.add_task(StubAgentBenchTask {
            id: "a".into(),
            category: AgentBenchCategory::WebShopping,
        })
        .add_task(StubAgentBenchTask {
            id: "b".into(),
            category: AgentBenchCategory::KnowledgeGraph,
        });
        assert_eq!(r.task_count(), 2);
        let reports = r.run();
        let summary = AgentBenchSummary::from_reports(&reports);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.ran, 0);
        assert!(summary.format().contains("0 真跑"));
    }
}
