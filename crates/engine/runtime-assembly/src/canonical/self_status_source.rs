//! 生产自述来源 (自省通道) —— [`SelfStatusSource`] 的生产实现。
//!
//! 装配时把**真实生效值** (能力开关实际取值 / 工作区 / 自学习接线) 与两个
//! data 系探测口 (记忆账本计数 / 凭据存在性) 收进一个只读来源; `self_status`
//! 工具每次调用现采集快照。data 系失败**不**让整帧失败: 对应字段显式
//! `null` + 原因 (0 装诚实: 明说没测到, 不编造)。
//!
//! 预算一节读回合轮数/单轮工具上限: 组装根把**生效中的**可配置预算旋钮
//! 取值注入时按 `configured` 口径报告 (与运行时装配同一条解析路径); 未注入
//! 的组合回退编译期常量并注明 `constant` 口径 (见 [`BudgetStatus::source`])。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_orchestration::self_tuning::TunableParam;
use apeireth_tools_canonical::{
    derive_tuning_preset, BudgetStatus, CapabilitySwitch, MemoryLedgerStats, SelfStatusIdentity,
    SelfStatusSnapshot, SelfStatusSource, StatusProbe, TuningStatus, WorkspaceStatus,
    BUDGET_SOURCE_CONSTANT_NOTE, DATA_PROBE_NOT_WIRED,
};

use super::production::ProductionModulesConfig;
use super::{execute::MAX_TOOL_CALLS_PER_ROUND, runtime::DEFAULT_MAX_ROUNDS};

/// 预算取值口径: 读编译期常量 (可配置口径未接线的回退)。
pub const BUDGET_SOURCE_CONSTANT: &str = "constant";
/// 预算取值口径: 读**生效中的**可配置预算旋钮 (与运行时装配同一条解析路径)。
pub const BUDGET_SOURCE_CONFIGURED: &str = "configured";

/// 口径注记: 预算经可配置旋钮解析 (未设 env 时取默认值)。
pub const BUDGET_SOURCE_CONFIGURED_NOTE: &str =
    "read through the live budget knobs (env-configurable; defaults apply when unset)";

/// 单回合轮数上限的编译期常量。
pub fn max_rounds_per_turn_constant() -> u64 {
    u64::from(DEFAULT_MAX_ROUNDS)
}

/// 单轮工具调用上限的编译期常量。
pub fn max_tool_calls_per_round_constant() -> u64 {
    MAX_TOOL_CALLS_PER_ROUND as u64
}

/// 从编译期常量构造预算一节 (含口径注记)。
pub fn budget_status_from_constants() -> BudgetStatus {
    BudgetStatus {
        max_rounds_per_turn: max_rounds_per_turn_constant(),
        max_tool_calls_per_round: max_tool_calls_per_round_constant(),
        source: BUDGET_SOURCE_CONSTANT.to_string(),
        note: BUDGET_SOURCE_CONSTANT_NOTE.to_string(),
    }
}

/// 从**生效中的**可配置预算旋钮构造预算一节 (组装根把实际装配进运行时的
/// 取值原样传入, 自述与运行时同源)。
pub fn budget_status_from_configured(
    max_rounds_per_turn: u64,
    max_tool_calls_per_round: u64,
) -> BudgetStatus {
    BudgetStatus {
        max_rounds_per_turn,
        max_tool_calls_per_round,
        source: BUDGET_SOURCE_CONFIGURED.to_string(),
        note: BUDGET_SOURCE_CONFIGURED_NOTE.to_string(),
    }
}

/// 生产自述来源: 装配期收集生效值, 调用期现探测 data 系。
pub struct ProductionSelfStatusSource {
    identity: SelfStatusIdentity,
    capabilities: Vec<CapabilitySwitch>,
    ledger: Option<StatusProbe<MemoryLedgerStats>>,
    credentials: Option<StatusProbe<bool>>,
    workspace_root: Option<PathBuf>,
    self_learning: bool,
    budget: BudgetStatus,
}

impl ProductionSelfStatusSource {
    /// 只有生效值部分的来源 (data 系探测口缺位时对应字段显式 null + 原因)。
    pub fn new(
        capabilities: Vec<CapabilitySwitch>,
        self_learning: bool,
        workspace_root: Option<PathBuf>,
    ) -> Self {
        Self {
            identity: SelfStatusIdentity::default(),
            capabilities,
            ledger: None,
            credentials: None,
            workspace_root,
            self_learning,
            budget: budget_status_from_constants(),
        }
    }

    /// 接入记忆账本计数探测口 (会话/记忆/保护/教训计数)。
    #[must_use]
    pub fn with_ledger_probe(mut self, probe: StatusProbe<MemoryLedgerStats>) -> Self {
        self.ledger = Some(probe);
        self
    }

    /// 接入凭据存在性探测口 (只回布尔, 不回显凭据本体)。
    #[must_use]
    pub fn with_credentials_probe(mut self, probe: StatusProbe<bool>) -> Self {
        self.credentials = Some(probe);
        self
    }

    /// 覆写预算一节 (可配置口径落地时由组装根传入实际值)。
    #[must_use]
    pub fn with_budget(mut self, budget: BudgetStatus) -> Self {
        self.budget = budget;
        self
    }

    /// 自述身份 (产品名 / workspace 版本 / 运行时角色)。
    pub fn identity(&self) -> &SelfStatusIdentity {
        &self.identity
    }
}

impl SelfStatusSource for ProductionSelfStatusSource {
    fn snapshot(&self) -> SelfStatusSnapshot {
        // 调参状态: 四滑杆当前**生效值** (进程内自动调整 > env 旋钮 > 基线),
        // 预设名由取值集推导, 自学习开关取装配时的接线事实。
        let values = [
            TunableParam::MemoryFade.effective(),
            TunableParam::CuriosityStrength.effective(),
            TunableParam::ToneSaturation.effective(),
            TunableParam::ConsolidationCadence.effective(),
        ];
        let tuning = TuningStatus {
            memory_fade: values[0],
            curiosity_strength: values[1],
            tone_saturation: values[2],
            consolidation_cadence: values[3],
            preset: derive_tuning_preset(values),
            self_learning: self.self_learning,
        };

        // 记忆账本: 现探测; 失败/缺位 → 计数显式 null + 原因。
        let mut reasons: Vec<String> = Vec::new();
        let memory_ledger = match &self.ledger {
            Some(probe) => match probe() {
                Ok(stats) => {
                    if let Some(reason) = &stats.reason {
                        reasons.push(reason.clone());
                    }
                    Some(stats)
                }
                Err(error) => {
                    reasons.push(format!("memory ledger probe failed: {error}"));
                    None
                }
            },
            None => {
                reasons.push(DATA_PROBE_NOT_WIRED.to_string());
                None
            }
        };
        let memory_ledger_reason = reasons.first().cloned();

        // 工作区: 根路径 + 凭据存在性布尔 (凭据本体绝不回显)。
        let root = self
            .workspace_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());
        let mut workspace_reason: Option<String> = None;
        let credentials_present = match &self.credentials {
            Some(probe) => match probe() {
                Ok(present) => Some(present),
                Err(error) => {
                    workspace_reason = Some(format!("credentials probe failed: {error}"));
                    None
                }
            },
            None => {
                workspace_reason = Some(DATA_PROBE_NOT_WIRED.to_string());
                None
            }
        };

        SelfStatusSnapshot {
            identity: self.identity.clone(),
            capabilities: self.capabilities.clone(),
            memory_ledger,
            memory_ledger_reason,
            tuning: Some(tuning),
            tuning_reason: None,
            budget: Some(self.budget.clone()),
            budget_reason: None,
            workspace: Some(WorkspaceStatus {
                root,
                credentials_present,
            }),
            workspace_reason,
        }
    }
}

/// 能力名册投影: 把**实际用于装配的** [`ProductionModulesConfig`] 与注入槽
/// 存在性投影成开关全表 (真实生效值, 非配置文本), 外加组装根补充行
/// (例如授权层的 `local_read_tools` 旋钮)。
///
/// 本地文件工具行按**实际注册条件**取值 (配置开启且工作区根已注入才算生效),
/// 名册不复述"配置说开"而注册没成的事实。
pub fn roster_from_config(
    config: &ProductionModulesConfig,
    workspace_root_present: bool,
    self_tuning: bool,
    typed_recall: bool,
    extras: &[CapabilitySwitch],
) -> Vec<CapabilitySwitch> {
    let mut roster = vec![
        CapabilitySwitch::new("filesystem", config.filesystem && workspace_root_present),
        CapabilitySwitch::new("search", config.search && workspace_root_present),
        CapabilitySwitch::new("repo", config.repo && workspace_root_present),
        CapabilitySwitch::new("shell", config.shell.is_some()),
        CapabilitySwitch::new("fetch", config.fetch.is_some()),
        CapabilitySwitch::new("mcp", config.mcp),
        CapabilitySwitch::new("education", config.education),
        CapabilitySwitch::new("memory_recall", config.memory_recall),
        CapabilitySwitch::new("memory_writeback", config.memory_writeback),
        CapabilitySwitch::new("memory_injection", config.memory_injection),
        CapabilitySwitch::new("proactive_recall", config.proactive_recall.is_some()),
        CapabilitySwitch::new("typed_recall", typed_recall),
        CapabilitySwitch::new("preference_recall", config.preference_recall),
        CapabilitySwitch::new("preference_learning", config.preference_learning),
        CapabilitySwitch::new("consolidation", config.consolidation),
        CapabilitySwitch::new("reflexion", config.reflexion),
        CapabilitySwitch::new("self_assessment", config.self_assessment),
        CapabilitySwitch::new("judge", config.judge.enabled),
        CapabilitySwitch::new("council", config.council),
        CapabilitySwitch::new("organs", config.organs),
        CapabilitySwitch::new("partner_bond", config.partner_bond),
        CapabilitySwitch::new("morphology_recall", config.morphology_recall),
        CapabilitySwitch::new("community_triage", config.community_triage),
        CapabilitySwitch::new("absorption_insight", config.absorption_insight),
        CapabilitySwitch::new("self_tuning", self_tuning),
    ];
    roster.extend(extras.iter().cloned());
    roster.sort_by(|a, b| a.name.cmp(&b.name));
    roster
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_status_reads_the_constants_and_says_so() {
        let budget = budget_status_from_constants();
        assert_eq!(budget.max_rounds_per_turn, 8);
        assert_eq!(budget.max_tool_calls_per_round, 16);
        assert_eq!(budget.source, BUDGET_SOURCE_CONSTANT);
        assert!(budget.note.contains("constants"), "{}", budget.note);
    }

    #[test]
    fn configured_budget_status_carries_the_live_knob_values() {
        let budget = budget_status_from_configured(2, 4);
        assert_eq!(budget.max_rounds_per_turn, 2);
        assert_eq!(budget.max_tool_calls_per_round, 4);
        assert_eq!(budget.source, BUDGET_SOURCE_CONFIGURED);
        assert!(budget.note.contains("knobs"), "{}", budget.note);
    }

    #[test]
    fn roster_projects_the_effective_config_switches() {
        let mut config = ProductionModulesConfig::default();
        config.shell = None;
        config.fetch = None;
        config.organs = true;
        let extras = vec![CapabilitySwitch::new("local_read_tools", true)];
        let roster = roster_from_config(&config, true, true, true, &extras);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["shell"], false);
        assert_eq!(by_name["fetch"], false);
        assert_eq!(by_name["organs"], true);
        assert_eq!(by_name["self_tuning"], true);
        assert_eq!(by_name["typed_recall"], true);
        assert_eq!(by_name["local_read_tools"], true);
        assert_eq!(by_name["memory_recall"], true);
        assert_eq!(by_name["filesystem"], true);

        // 无工作区根 = 本地文件工具没注册成, 名册照实说 false。
        let roster = roster_from_config(&config, false, true, true, &extras);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["filesystem"], false);
        assert_eq!(by_name["search"], false);
        assert_eq!(by_name["repo"], false);
    }

    #[test]
    fn missing_data_probes_become_explicit_nulls_with_reasons() {
        let source = ProductionSelfStatusSource::new(
            vec![CapabilitySwitch::new("shell", false)],
            false,
            Some(PathBuf::from("C:\\work")),
        );
        let snapshot = source.snapshot();
        assert!(snapshot.memory_ledger.is_none());
        assert!(snapshot
            .memory_ledger_reason
            .as_deref()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED));
        let workspace = snapshot.workspace.expect("workspace section");
        assert_eq!(workspace.root.as_deref(), Some("C:\\work"));
        assert!(workspace.credentials_present.is_none());
        assert!(snapshot
            .workspace_reason
            .as_deref()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED));
        // 生效值部分照常在场。
        assert!(snapshot.tuning.is_some());
        assert!(snapshot.budget.is_some());
    }

    #[test]
    fn ledger_probe_failure_is_a_reason_not_a_failed_frame() {
        let probe: StatusProbe<MemoryLedgerStats> = Arc::new(|| Err("db locked".to_string()));
        let source =
            ProductionSelfStatusSource::new(Vec::new(), false, None).with_ledger_probe(probe);
        let snapshot = source.snapshot();
        assert!(snapshot.memory_ledger.is_none());
        assert!(
            snapshot
                .memory_ledger_reason
                .as_deref()
                .unwrap()
                .contains("db locked"),
            "{:?}",
            snapshot.memory_ledger_reason
        );
    }

    #[test]
    fn probes_feed_real_counts_and_credentials_presence() {
        let ledger_probe: StatusProbe<MemoryLedgerStats> = Arc::new(|| {
            Ok(MemoryLedgerStats {
                sessions: Some(2),
                memories: Some(5),
                protected: Some(1),
                lessons: Some(3),
                reason: None,
            })
        });
        let credentials_probe: StatusProbe<bool> = Arc::new(|| Ok(true));
        let source = ProductionSelfStatusSource::new(Vec::new(), true, Some(PathBuf::from("root")))
            .with_ledger_probe(ledger_probe)
            .with_credentials_probe(credentials_probe);
        let snapshot = source.snapshot();
        let stats = snapshot.memory_ledger.expect("ledger");
        assert_eq!(stats.sessions, Some(2));
        assert_eq!(stats.memories, Some(5));
        assert_eq!(stats.protected, Some(1));
        assert_eq!(stats.lessons, Some(3));
        assert!(snapshot.memory_ledger_reason.is_none());
        let workspace = snapshot.workspace.expect("workspace");
        assert_eq!(workspace.credentials_present, Some(true));
        assert!(snapshot.workspace_reason.is_none());
        assert!(snapshot.tuning.expect("tuning").self_learning);
    }
}
