//! The structured self-report tool (`tool.self_status`).
//!
//! 产品哲学「不假装」要求模型能**实测自身状态再发言**。本模块把自省设计化为
//! 一个专用只读面: 一次调用返回一份**结构化自述** JSON —— 身份 / 能力名册 /
//! 记忆账本 / 调参状态 / 预算 / 工作区。数据一律取自运行时**真实生效值**
//! (装配时的实际开关、存储可查元数据、进程内生效调参), 不复述配置文本。
//!
//! # 三条硬边界
//!
//! 1. **只读档**: 工具无写无执行, 无参数, 默认可用, 零审批 (治理侧只做 grant)。
//! 2. **计数不回内容**: 记忆账本只回计数元数据, 永不回放记忆原文。
//! 3. **凭据只回存在性**: 工作区一节只回「凭据是否存在」布尔, 不回显凭据本体。
//!
//! # 错误即帧, 缺失显式 null
//!
//! data 系来源 (记忆账本 / 工作区探测) 读取失败时, **该字段显式 `null` 并给出
//! 原因**, 整帧仍然成功返回 —— 自述宁可如实说"这块我没测到", 不整帧失败,
//! 更不编造数字。输出经既有 [`ToolOutcome`] 归一合同冻结
//! (见 [`SelfStatusTool::output_schema`]), 超时归 `timeout.*` 既有 code 族。

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_core::RELEASE_VERSION;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::exec_pipeline::{OutputSchema, SchemaKind};

/// 产品标识 (中性工程事实: 自述身份, 不引任何第三方标识)。
pub const PRODUCT_NAME: &str = "apeireth";

/// 运行时角色标签: 桌面形态下后端进程以侧车身份被宿主监督。
pub const RUNTIME_ROLE_GATEWAY_SIDECAR: &str = "gateway-sidecar";

/// 自述身份一节: 产品名 / 版本 (workspace 单轴版本) / 运行时角色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfStatusIdentity {
    /// 产品名。
    pub product_name: String,
    /// 版本 (workspace 单轴版本值)。
    pub version: String,
    /// 运行时角色。
    pub runtime_role: String,
}

impl Default for SelfStatusIdentity {
    fn default() -> Self {
        Self {
            product_name: PRODUCT_NAME.to_string(),
            version: RELEASE_VERSION.to_string(),
            runtime_role: RUNTIME_ROLE_GATEWAY_SIDECAR.to_string(),
        }
    }
}

/// 能力名册一行: 开关名 + 当前生效值 (装配真实值, 非配置文本)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySwitch {
    /// 开关名 (与装配旋钮同名, snake_case)。
    pub name: String,
    /// 当前生效值。
    pub enabled: bool,
}

impl CapabilitySwitch {
    /// 一行能力名册。
    pub fn new(name: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            enabled,
        }
    }
}

/// 记忆账本计数 (纯计数元数据; 缺失的计数显式 `None` → JSON `null`)。
///
/// **永不回内容原文**: 只有数字, 没有记忆文本。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLedgerStats {
    /// 会话数 (episode 流里的不同会话计数)。
    pub sessions: Option<u64>,
    /// 记忆件数 (episode 计数)。
    pub memories: Option<u64>,
    /// 保护件数 (治理层标记为保护的记忆计数)。
    pub protected: Option<u64>,
    /// 教训数 (反思沉淀的教训计数)。
    pub lessons: Option<u64>,
    /// 局部缺失原因 (某计数探测失败时给出; 与字段级 null 配对)。
    pub reason: Option<String>,
}

/// 四滑杆调参状态 (体验参数生效值 + 预设名 + 自学习开关)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningStatus {
    /// 遗忘衰减强度倍率当前生效值。
    pub memory_fade: f64,
    /// 好奇强度倍率当前生效值。
    pub curiosity_strength: f64,
    /// 语气情绪饱和倍率当前生效值。
    pub tone_saturation: f64,
    /// 整合间隔 (回合数) 当前生效值。
    pub consolidation_cadence: f64,
    /// 当前取值命中的预设名 (`effortless` / `balanced` / `deep_memory` / `custom`)。
    pub preset: String,
    /// 自学习 (自动调参) 开关的当前生效值。
    pub self_learning: bool,
}

/// 预设名: 省心档 (遗忘更强 / 好奇更收敛 / 语气更淡 / 整合更慢)。
pub const PRESET_EFFORTLESS: &str = "effortless";
/// 预设名: 均衡档 (全基线取值)。
pub const PRESET_BALANCED: &str = "balanced";
/// 预设名: 深度记忆档 (遗忘更弱 / 好奇更强 / 语气更浓 / 整合更快)。
pub const PRESET_DEEP_MEMORY: &str = "deep_memory";
/// 预设名: 取值不落在任何命名预设上。
pub const PRESET_CUSTOM: &str = "custom";

/// 命名预设的四滑杆取值表 (与调参面板的预设按钮同一取值集)。
pub const DISPOSITION_PRESETS: &[(&str, [f64; 4])] = &[
    (PRESET_EFFORTLESS, [1.5, 0.75, 0.8, 2.0]),
    (PRESET_BALANCED, [1.0, 1.0, 1.0, 1.0]),
    (PRESET_DEEP_MEMORY, [0.5, 1.5, 1.2, 1.0]),
];

/// 由四滑杆取值推导预设名: 命中命名取值集回预设名, 否则 `custom`。
pub fn derive_tuning_preset(values: [f64; 4]) -> String {
    for (name, preset) in DISPOSITION_PRESETS {
        if preset
            .iter()
            .zip(values.iter())
            .all(|(expected, actual)| (expected - actual).abs() < 1e-9)
        {
            return (*name).to_string();
        }
    }
    PRESET_CUSTOM.to_string()
}

/// 预算一节: 回合轮数上限 / 单轮工具上限 + 取值口径注记。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// 单回合轮数上限。
    pub max_rounds_per_turn: u64,
    /// 单轮工具调用上限。
    pub max_tool_calls_per_round: u64,
    /// 取值口径: `constant` = 读编译期常量; `configured` = 读可配置值。
    pub source: String,
    /// 口径注记 (可配置口径未接线时如实注明)。
    pub note: String,
}

/// 口径注记: 预算读编译期常量, 逐回合可配置口径尚未接线。
pub const BUDGET_SOURCE_CONSTANT_NOTE: &str =
    "read from compile-time constants; per-turn configurable override is not wired yet";

/// 工作区一节: 根路径 / 凭据存在性 (布尔, 不回显凭据本体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    /// 工作区根路径。
    pub root: Option<String>,
    /// 凭据是否存在 (只回布尔; `None` = 探测缺位/失败 → 显式 null)。
    pub credentials_present: Option<bool>,
}

/// 一份完整结构化自述的来源数据 (工具只负责投影成 JSON)。
#[derive(Debug, Clone, PartialEq)]
pub struct SelfStatusSnapshot {
    /// 身份一节。
    pub identity: SelfStatusIdentity,
    /// 能力名册 (当前生效开关全表)。
    pub capabilities: Vec<CapabilitySwitch>,
    /// 记忆账本计数; `None` = 来源不可用 (帧内显式 null + 原因)。
    pub memory_ledger: Option<MemoryLedgerStats>,
    /// 记忆账本不可用的原因; 可用时为 `None`。
    pub memory_ledger_reason: Option<String>,
    /// 调参状态。
    pub tuning: Option<TuningStatus>,
    /// 调参状态不可用的原因; 可用时为 `None`。
    pub tuning_reason: Option<String>,
    /// 预算。
    pub budget: Option<BudgetStatus>,
    /// 预算不可用的原因; 可用时为 `None`。
    pub budget_reason: Option<String>,
    /// 工作区。
    pub workspace: Option<WorkspaceStatus>,
    /// 工作区不可用的原因; 可用时为 `None`。
    pub workspace_reason: Option<String>,
}

impl SelfStatusSnapshot {
    /// 只有身份与名册的最小自述 (其余字段显式 null + 原因)。
    pub fn minimal(identity: SelfStatusIdentity, capabilities: Vec<CapabilitySwitch>) -> Self {
        Self {
            identity,
            capabilities,
            memory_ledger: None,
            memory_ledger_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            tuning: None,
            tuning_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            budget: None,
            budget_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
            workspace: None,
            workspace_reason: Some(DATA_PROBE_NOT_WIRED.to_string()),
        }
    }
}

/// 统一的「来源未接线」原因 (0 装诚实: 显式说没测到, 不编造)。
pub const DATA_PROBE_NOT_WIRED: &str = "status source not wired in this composition";

/// 自述数据来源: 组装根注入真实生效值与探测口, 工具只做只读投影。
pub trait SelfStatusSource: Send + Sync {
    /// 采集一份自述快照 (调用时的生效值)。
    fn snapshot(&self) -> SelfStatusSnapshot;
}

/// data 系探测口 (记忆账本 / 凭据存在性): 每次自述现探测, 失败回 `Err(原因)`。
///
/// 失败**不**让整帧失败: 对应字段显式 null + 原因 (见 [`render_snapshot`])。
pub type StatusProbe<T> = Arc<dyn Fn() -> Result<T, String> + Send + Sync>;

/// 把一份自述投影成结构化 JSON (字段合同的唯一实现)。
///
/// 合同要点: **所有字段恒在场**; 来源缺失/失败的字段显式 `null` 并在同节
/// `reason` 给出原因 —— 不是缺字段, 不是整帧失败。
pub fn render_snapshot(snapshot: &SelfStatusSnapshot) -> Value {
    let capabilities: serde_json::Map<String, Value> = snapshot
        .capabilities
        .iter()
        .map(|entry| (entry.name.clone(), Value::Bool(entry.enabled)))
        .collect();

    let memory_ledger = match &snapshot.memory_ledger {
        Some(stats) => json!({
            "sessions": stats.sessions,
            "memories": stats.memories,
            "protected": stats.protected,
            "lessons": stats.lessons,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .or_else(|| stats.reason.clone()),
        }),
        None => json!({
            "sessions": null,
            "memories": null,
            "protected": null,
            "lessons": null,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let tuning = match &snapshot.tuning {
        Some(status) => json!({
            "values": {
                "memory_fade": status.memory_fade,
                "curiosity_strength": status.curiosity_strength,
                "tone_saturation": status.tone_saturation,
                "consolidation_cadence": status.consolidation_cadence,
            },
            "preset": status.preset,
            "self_learning": status.self_learning,
            "reason": snapshot.tuning_reason,
        }),
        None => json!({
            "values": {
                "memory_fade": null,
                "curiosity_strength": null,
                "tone_saturation": null,
                "consolidation_cadence": null,
            },
            "preset": null,
            "self_learning": null,
            "reason": snapshot
                .tuning_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let budget = match &snapshot.budget {
        Some(status) => json!({
            "max_rounds_per_turn": status.max_rounds_per_turn,
            "max_tool_calls_per_round": status.max_tool_calls_per_round,
            "source": status.source,
            "note": status.note,
            "reason": snapshot.budget_reason,
        }),
        None => json!({
            "max_rounds_per_turn": null,
            "max_tool_calls_per_round": null,
            "source": null,
            "note": null,
            "reason": snapshot
                .budget_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    let workspace = match &snapshot.workspace {
        Some(status) => json!({
            "root": status.root,
            "credentials_present": status.credentials_present,
            "reason": snapshot.workspace_reason,
        }),
        None => json!({
            "root": null,
            "credentials_present": null,
            "reason": snapshot
                .workspace_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    };

    json!({
        "identity": {
            "product_name": snapshot.identity.product_name,
            "version": snapshot.identity.version,
            "runtime_role": snapshot.identity.runtime_role,
        },
        "capabilities": Value::Object(capabilities),
        "memory_ledger": memory_ledger,
        "tuning": tuning,
        "budget": budget,
        "workspace": workspace,
    })
}

/// 空参数 (工具无参数; 未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelfStatusParams {}

/// 结构化自述工具 (`tool.self_status`, 声明名 `self_status`)。
pub struct SelfStatusTool {
    id: CapabilityId,
    source: Arc<dyn SelfStatusSource>,
}

impl SelfStatusTool {
    /// 绑定一个自述来源。
    pub fn new(source: Arc<dyn SelfStatusSource>) -> Self {
        Self {
            id: CapabilityId::new("tool.self_status").unwrap(),
            source,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且六节恒在场 (每节为对象)。
    ///
    /// 具体字段允许显式 null (来源缺失), 但**节**不可缺席 —— 自述的形状冻结。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("identity", SchemaKind::Object)
            .require("capabilities", SchemaKind::Object)
            .require("memory_ledger", SchemaKind::Object)
            .require("tuning", SchemaKind::Object)
            .require("budget", SchemaKind::Object)
            .require("workspace", SchemaKind::Object)
    }

    /// 采集并投影当前自述 (结构化值)。
    pub fn collect(&self) -> Value {
        render_snapshot(&self.source.snapshot())
    }
}

#[async_trait]
impl ToolCapability for SelfStatusTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new("self_status")
            .with_description(
                "Read-only structured self-report: identity, capability switches in force, memory \
                 ledger counts (numbers only), tuning state, turn budgets, and workspace facts. \
                 Reports measured state only; credential values are never echoed.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if let Err(error) = serde_json::from_value::<SelfStatusParams>(arguments) {
            return ToolResult::permanent_error(
                &call.id,
                format!("invalid self_status parameters: {error}"),
            );
        }
        // 采集跑在阻塞池上: data 系探测 (库查询/文件读) 卡住时, 执行链的
        // 到期计时器仍能触发, 超时归既有 `timeout.*` code 族 (可重试帧),
        // 而不是把回合一起挂死。
        let source = Arc::clone(&self.source);
        match tokio::task::spawn_blocking(move || render_snapshot(&source.snapshot())).await {
            Ok(value) => ToolResult::ok(&call.id, value),
            Err(error) => ToolResult::permanent_error(
                &call.id,
                format!("self_status collection task failed: {error}"),
            ),
        }
    }
}

/// 能力名册投影辅助: 把 (名, 值) 迭代器收成稳定排序的名册。
pub fn capability_roster<I, S>(entries: I) -> Vec<CapabilitySwitch>
where
    I: IntoIterator<Item = (S, bool)>,
    S: Into<String>,
{
    let mut map: BTreeMap<String, bool> = BTreeMap::new();
    for (name, enabled) in entries {
        map.insert(name.into(), enabled);
    }
    map.into_iter()
        .map(|(name, enabled)| CapabilitySwitch::new(name, enabled))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> SelfStatusSnapshot {
        SelfStatusSnapshot {
            identity: SelfStatusIdentity::default(),
            capabilities: capability_roster([("shell", false), ("search", true)]),
            memory_ledger: Some(MemoryLedgerStats {
                sessions: Some(2),
                memories: Some(5),
                protected: Some(1),
                lessons: Some(0),
                reason: None,
            }),
            memory_ledger_reason: None,
            tuning: Some(TuningStatus {
                memory_fade: 1.5,
                curiosity_strength: 0.75,
                tone_saturation: 0.8,
                consolidation_cadence: 2.0,
                preset: derive_tuning_preset([1.5, 0.75, 0.8, 2.0]),
                self_learning: false,
            }),
            tuning_reason: None,
            budget: Some(BudgetStatus {
                max_rounds_per_turn: 8,
                max_tool_calls_per_round: 16,
                source: "constant".to_string(),
                note: BUDGET_SOURCE_CONSTANT_NOTE.to_string(),
            }),
            budget_reason: None,
            workspace: Some(WorkspaceStatus {
                root: Some("C:\\work".to_string()),
                credentials_present: Some(true),
            }),
            workspace_reason: None,
        }
    }

    struct FixedSource {
        snapshot: SelfStatusSnapshot,
    }

    impl SelfStatusSource for FixedSource {
        fn snapshot(&self) -> SelfStatusSnapshot {
            self.snapshot.clone()
        }
    }

    async fn invoke_tool(snapshot: SelfStatusSnapshot) -> ToolResult {
        let tool = SelfStatusTool::new(Arc::new(FixedSource { snapshot }));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        tool.invoke(&call).await
    }

    #[tokio::test]
    async fn every_section_arrives_with_real_values() {
        let result = invoke_tool(fixture_snapshot()).await;
        assert!(result.is_ok(), "{}", result.render());
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");
        for section in [
            "identity",
            "capabilities",
            "memory_ledger",
            "tuning",
            "budget",
            "workspace",
        ] {
            assert!(value.get(section).is_some(), "missing section {section}");
        }
        assert_eq!(value["identity"]["product_name"], PRODUCT_NAME);
        assert_eq!(value["identity"]["version"], RELEASE_VERSION);
        assert_eq!(
            value["identity"]["runtime_role"],
            RUNTIME_ROLE_GATEWAY_SIDECAR
        );
        assert_eq!(value["memory_ledger"]["sessions"], 2);
        assert_eq!(value["memory_ledger"]["memories"], 5);
        assert_eq!(value["memory_ledger"]["protected"], 1);
        assert_eq!(value["memory_ledger"]["lessons"], 0);
        assert_eq!(value["tuning"]["preset"], PRESET_EFFORTLESS);
        assert_eq!(value["budget"]["max_rounds_per_turn"], 8);
        assert_eq!(value["budget"]["max_tool_calls_per_round"], 16);
        assert_eq!(value["workspace"]["root"], "C:\\work");
        assert_eq!(value["workspace"]["credentials_present"], true);
    }

    #[tokio::test]
    async fn missing_data_sections_are_explicit_null_with_a_reason() {
        let mut snapshot = fixture_snapshot();
        snapshot.memory_ledger = None;
        snapshot.memory_ledger_reason = Some("memory store unavailable: disk error".to_string());
        snapshot.workspace = None;
        snapshot.workspace_reason = Some("workspace probe failed".to_string());

        let result = invoke_tool(snapshot).await;
        assert!(result.is_ok(), "a missing source must not fail the frame");
        let value: Value = serde_json::from_str(&result.render()).expect("structured json");

        assert_eq!(value["memory_ledger"]["sessions"], Value::Null);
        assert_eq!(value["memory_ledger"]["memories"], Value::Null);
        assert_eq!(value["memory_ledger"]["protected"], Value::Null);
        assert_eq!(value["memory_ledger"]["lessons"], Value::Null);
        assert!(
            value["memory_ledger"]["reason"]
                .as_str()
                .unwrap()
                .contains("memory store unavailable"),
            "{}",
            value["memory_ledger"]["reason"]
        );
        assert_eq!(value["workspace"]["root"], Value::Null);
        assert_eq!(value["workspace"]["credentials_present"], Value::Null);
        assert!(
            value["workspace"]["reason"]
                .as_str()
                .unwrap()
                .contains("workspace probe failed"),
            "{}",
            value["workspace"]["reason"]
        );
        // 其余节不受牵连: 不整帧失败。
        assert_eq!(value["identity"]["product_name"], PRODUCT_NAME);
        assert!(!value["memory_ledger"]["reason"].is_null());
    }

    #[test]
    fn missing_leaf_counts_stay_null_without_losing_the_section() {
        let mut snapshot = fixture_snapshot();
        snapshot.memory_ledger = Some(MemoryLedgerStats {
            sessions: Some(1),
            memories: None,
            protected: None,
            lessons: None,
            reason: Some("lesson store not wired".to_string()),
        });
        snapshot.memory_ledger_reason = None;
        let value = render_snapshot(&snapshot);
        assert_eq!(value["memory_ledger"]["sessions"], 1);
        assert_eq!(value["memory_ledger"]["memories"], Value::Null);
        assert_eq!(value["memory_ledger"]["lessons"], Value::Null);
        assert!(value["memory_ledger"]["reason"]
            .as_str()
            .unwrap()
            .contains("lesson store not wired"));
    }

    #[test]
    fn output_schema_accepts_the_rendered_shape() {
        let value = render_snapshot(&fixture_snapshot());
        assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
    }

    #[test]
    fn output_schema_rejects_a_drifted_shape() {
        let mut value = render_snapshot(&fixture_snapshot());
        value.as_object_mut().unwrap().remove("memory_ledger");
        let error = SelfStatusTool::output_schema()
            .validate(&value)
            .expect_err("missing section must be a contract violation");
        assert_eq!(error.code(), "output_contract.schema_mismatch");
    }

    #[test]
    fn capability_roster_is_sorted_and_lossless() {
        let roster = capability_roster([("shell", true), ("fetch", false), ("search", true)]);
        let names: Vec<&str> = roster.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["fetch", "search", "shell"]);
        assert!(!roster[0].enabled);
        assert!(roster[1].enabled);
    }

    #[test]
    fn preset_derivation_matches_the_named_value_sets() {
        assert_eq!(
            derive_tuning_preset([1.5, 0.75, 0.8, 2.0]),
            PRESET_EFFORTLESS
        );
        assert_eq!(derive_tuning_preset([1.0, 1.0, 1.0, 1.0]), PRESET_BALANCED);
        assert_eq!(
            derive_tuning_preset([0.5, 1.5, 1.2, 1.0]),
            PRESET_DEEP_MEMORY
        );
        assert_eq!(derive_tuning_preset([2.0, 2.0, 2.0, 3.0]), PRESET_CUSTOM);
    }

    #[tokio::test]
    async fn unknown_arguments_are_refused() {
        let tool = SelfStatusTool::new(Arc::new(FixedSource {
            snapshot: fixture_snapshot(),
        }));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({ "surprise": 1 }),
        };
        let result = tool.invoke(&call).await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("invalid self_status parameters"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn pipeline_freezes_the_frame_into_the_output_contract() {
        use crate::exec_pipeline::{ExecutionStage, ToolExecutionPipeline};

        let tool: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(FixedSource {
            snapshot: fixture_snapshot(),
        })));
        let pipeline =
            ToolExecutionPipeline::new().with_output_schema(SelfStatusTool::output_schema());
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        let executed = pipeline.run(tool.as_ref(), &call).await;
        assert!(executed.failure.is_none(), "{:?}", executed.failure);
        assert!(executed.outcome.ok);
        let structured = executed.outcome.structured().expect("structured sidecar");
        for section in [
            "identity",
            "capabilities",
            "memory_ledger",
            "tuning",
            "budget",
            "workspace",
        ] {
            assert!(
                structured.get(section).is_some(),
                "missing section {section}"
            );
        }
        assert!(executed
            .record
            .stages()
            .contains(&ExecutionStage::Normalize));
    }

    #[tokio::test]
    async fn a_hanging_collection_closes_as_a_timeout_frame() {
        use crate::exec_pipeline::{AroundPolicy, PipelineFailure, ToolExecutionPipeline};

        struct HangingSource;

        impl SelfStatusSource for HangingSource {
            fn snapshot(&self) -> SelfStatusSnapshot {
                std::thread::sleep(std::time::Duration::from_millis(600));
                fixture_snapshot()
            }
        }

        let tool: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(Arc::new(HangingSource)));
        let pipeline =
            ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(50, 120_000));
        let call = ToolCall {
            id: "call_1".into(),
            name: "self_status".into(),
            arguments: json!({}),
        };
        let executed = pipeline.run(tool.as_ref(), &call).await;
        assert!(
            matches!(executed.failure, Some(PipelineFailure::Timeout { .. })),
            "{:?}",
            executed.failure
        );
        let rendered = executed.result.render();
        assert!(rendered.contains("timeout."), "{rendered}");
        assert!(
            rendered.contains("timeout.deadline_expired"),
            "超时必须归 timeout.* code 族: {rendered}"
        );
    }
}
