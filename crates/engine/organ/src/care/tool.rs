//! 关怀规则管理工具面 (`tool.care_schedule`, 声明名 `care_schedule`)。
//!
//! 三动作 (闭合词表, 增删列):
//!
//! | 动作 | 写面 | 语义 |
//! |---|---|---|
//! | `care_add` | 改规则 = 写文件 (write_intent 同族) | 增规则 (四类触发器词表) |
//! | `care_remove` | 改规则 = 写文件 (write_intent 同族) | 删规则 |
//! | `care_list` | 纯读 | 列规则 + 打扰纪律 + 最近关怀台账 |
//!
//! # 治理 (钉死: RequireApproval 级, 无自动放行档)
//!
//! 改规则 = 写文件 (write_intent 同族), 每次写都停人工审批 (与本地审批面板 /
//! IM 审批卡同一条审批链); `care_list` 纯读但**不设免审动作** (双面能力宁严勿
//! 松, 只读自述走 `tool.self_status` 的关怀节, 零审批)。档位由
//! [`CareScheduleApprovalHook`] 唯一承载。触发器/素材参数是**闭合校验**:
//! 非法参数拒绝即帧 (不进审批), 与规则面装载/写出同口径。
//!
//! # 输出面 (零新消息通道)
//!
//! 本工具只管规则面; 关怀发起不走本工具 —— 发起 = 双引擎判定 → 标准回合
//! (生成的关怀消息进会话流 + IM 推送缝), 见 [`super::engine`]。

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::rules::{
    validate_care_material, validate_care_rule_id, CareRecord, CareRule, CareStore, CareStoreError,
    CareTrigger, CareWriteFace, CARE_MATERIAL_MAX_BYTES, CARE_RULE_ID_MAX_BYTES,
};
use crate::care::engine::{CareClock, SystemCareClock};

/// 稳定能力标识 (注册表 / 治理授权 / 审批档共用)。
pub const CARE_SCHEDULE_CAPABILITY_ID: &str = "tool.care_schedule";
/// 模型侧声明名。
pub const CARE_SCHEDULE_TOOL_NAME: &str = "care_schedule";

/// 动作: 增规则。
pub const ACTION_CARE_ADD: &str = "care_add";
/// 动作: 删规则。
pub const ACTION_CARE_REMOVE: &str = "care_remove";
/// 动作: 列规则 (纯读)。
pub const ACTION_CARE_LIST: &str = "care_list";

/// 闭合动作词表 (增删列)。
pub const CARE_SCHEDULE_ACTIONS: &[&str] = &[ACTION_CARE_ADD, ACTION_CARE_REMOVE, ACTION_CARE_LIST];

/// 动作的写面归类 (write_intent 同族口径; 未知动作无写面)。
pub fn care_write_face_for_action(action: &str) -> Option<CareWriteFace> {
    match action {
        ACTION_CARE_ADD | ACTION_CARE_REMOVE => Some(CareWriteFace::RuleFileWrite),
        ACTION_CARE_LIST => Some(CareWriteFace::ReadOnly),
        _ => None,
    }
}

/// 审批档位理由 (审批卡载荷同文)。按动作如实标注写面: 改规则 = 写文件
/// (write_intent 同族) / 只读列随面同档。
pub fn care_schedule_approval_reason(action: Option<&str>) -> String {
    let face = match action.map(str::as_ref) {
        Some(ACTION_CARE_ADD) | Some(ACTION_CARE_REMOVE) => {
            "care_add/care_remove = 改规则 = 写文件 (write_intent 同族)"
        }
        Some(ACTION_CARE_LIST) => "care_list = 只读列规则, 随双面能力同档 (本能力无免审动作)",
        _ => "动作未识别, fail-closed 停人工审批",
    };
    format!(
        "care_schedule ({face}) —— RequireApproval 级: 每次调用都停人工审批, 无自动放行档; \
         关怀规则面改写必须有人拍板"
    )
}

/// 治理档位钩子: `tool.care_schedule` 每次调用停 [`Decision::RequireApproval`]
/// (无自动放行档, 任何预设/设置都不得越人)。与内容流/技能沉淀同一承载方式:
/// 档位在治理层的钩子上, 与本地审批面板 / IM 审批卡同一条审批链。
#[derive(Debug, Clone, Copy, Default)]
pub struct CareScheduleApprovalHook;

impl CareScheduleApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for CareScheduleApprovalHook {
    fn name(&self) -> &str {
        "care_schedule_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != CARE_SCHEDULE_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(Value::as_str);
        Decision::require_approval(care_schedule_approval_reason(action))
    }
}

/// `tool.care_schedule`: 关怀规则管理 (增删列, 写文件面走 write_intent 族)。
pub struct CareScheduleTool {
    id: CapabilityId,
    store: CareStore,
}

impl CareScheduleTool {
    /// 绑定规则表存储 (数据目录 `care_rules.json`)。
    pub fn new(data_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            id: CapabilityId::new(CARE_SCHEDULE_CAPABILITY_ID).unwrap(),
            store: CareStore::new(data_dir),
        }
    }

    /// 规则表存储 (自述探测口同源读它)。
    pub fn store(&self) -> &CareStore {
        &self.store
    }

    fn parse_params(&self, call: &ToolCall) -> Result<CareScheduleParams, ToolResult> {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if !arguments.is_object() {
            return Err(permanent(
                call,
                "care_schedule arguments must be a JSON object",
            ));
        }
        serde_json::from_value::<CareScheduleParams>(arguments)
            .map_err(|error| permanent(call, format!("invalid care_schedule parameters: {error}")))
    }

    /// 解析并闭合校验一次调用 (非法参数拒绝即帧, 不进审批)。
    fn prepare(&self, call: &ToolCall) -> Result<PreparedCareCall, ToolResult> {
        let params = self.parse_params(call)?;
        if !CARE_SCHEDULE_ACTIONS.contains(&params.action.as_str()) {
            return Err(permanent(
                call,
                format!(
                    "unknown care_schedule action {:?}; the closed vocabulary is care_add / \
                     care_remove / care_list",
                    params.action
                ),
            ));
        }
        match params.action.as_str() {
            ACTION_CARE_ADD => {
                let id = params
                    .id
                    .ok_or_else(|| permanent(call, "care_add requires \"id\""))?;
                validate_care_rule_id(&id)
                    .map_err(|reason| permanent(call, format!("care rule id refused: {reason}")))?;
                let trigger = params
                    .trigger
                    .ok_or_else(|| permanent(call, "care_add requires \"trigger\""))?;
                trigger
                    .validate()
                    .map_err(|reason| permanent(call, format!("care trigger refused: {reason}")))?;
                let material = params
                    .material
                    .ok_or_else(|| permanent(call, "care_add requires \"material\""))?;
                validate_care_material(&material).map_err(|reason| {
                    permanent(call, format!("care material refused: {reason}"))
                })?;
                Ok(PreparedCareCall::Add {
                    id,
                    trigger,
                    material,
                    enabled: params.enabled.unwrap_or(true),
                })
            }
            ACTION_CARE_REMOVE => {
                let id = params
                    .id
                    .ok_or_else(|| permanent(call, "care_remove requires \"id\""))?;
                validate_care_rule_id(&id)
                    .map_err(|reason| permanent(call, format!("care rule id refused: {reason}")))?;
                Ok(PreparedCareCall::Remove { id })
            }
            _ => Ok(PreparedCareCall::List),
        }
    }
}

fn permanent(call: &ToolCall, message: impl Into<String>) -> ToolResult {
    ToolResult::permanent_error(&call.id, message).with_name(CARE_SCHEDULE_TOOL_NAME)
}

enum PreparedCareCall {
    Add {
        id: String,
        trigger: CareTrigger,
        material: String,
        enabled: bool,
    },
    Remove {
        id: String,
    },
    List,
}

/// 工具参数面 (动作 + 可选 id / trigger / material / enabled)。
#[derive(Debug, serde::Deserialize)]
struct CareScheduleParams {
    action: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    trigger: Option<CareTrigger>,
    #[serde(default)]
    material: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

fn rule_json(rule: &CareRule) -> Value {
    json!({
        "id": rule.id,
        "trigger": rule.trigger,
        "trigger_kind": rule.trigger.kind_label(),
        "material": rule.material,
        "enabled": rule.enabled,
        "created_at_ms": rule.created_at_ms,
    })
}

fn record_json(record: &CareRecord) -> Value {
    json!({
        "rule_id": record.rule_id,
        "at_ms": record.at_ms,
        "judge": record.judge,
        "spoke": record.spoke,
        "note": record.note,
    })
}

fn store_error_result(call: &ToolCall, error: CareStoreError) -> ToolResult {
    permanent(call, format!("care_rules store refused: {error}"))
}

#[async_trait]
impl ToolCapability for CareScheduleTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": CARE_SCHEDULE_ACTIONS,
                        "description": "care_add / care_remove / care_list (closed vocabulary)."
                    },
                    "id": {
                        "type": "string",
                        "maxLength": CARE_RULE_ID_MAX_BYTES,
                        "description": "Rule id for care_add / care_remove ([A-Za-z0-9._-])."
                    },
                    "trigger": {
                        "type": "object",
                        "description": "Trigger for care_add (closed vocabulary). inactive/interval: {\"kind\": ..., \"hours\": N}; daily_moment: {\"kind\": \"daily_moment\", \"minutes_of_day\": 0..1439}; anniversary: {\"kind\": \"anniversary\", \"month\": 1..12, \"day\": 1..31}.",
                        "properties": {
                            "kind": {
                                "type": "string",
                                "enum": ["inactive", "daily_moment", "anniversary", "interval"]
                            },
                            "hours": { "type": "integer", "minimum": 1 },
                            "minutes_of_day": { "type": "integer", "minimum": 0, "maximum": 1439 },
                            "month": { "type": "integer", "minimum": 1, "maximum": 12 },
                            "day": { "type": "integer", "minimum": 1, "maximum": 31 }
                        },
                        "required": ["kind"],
                        "additionalProperties": false
                    },
                    "material": {
                        "type": "string",
                        "maxLength": CARE_MATERIAL_MAX_BYTES,
                        "description": "Care material (topic/direction) carried into the care candidate."
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Whether the rule is active (default true)."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(CARE_SCHEDULE_TOOL_NAME)
            .with_description(
                "Proactive-care rule scheduling (V3-a dual-engine care): add / remove / list \
                 deterministic care rules over the closed trigger vocabulary (inactive = no \
                 interaction for N hours; daily_moment = every day at HH:mm; anniversary = every \
                 year on month-day; interval = every N hours). Each due rule becomes a care \
                 candidate (material + timing) that the dual engine turns into a standard turn; \
                 the generated care message enters the normal conversation flow (no separate \
                 message channel). Disturbance discipline is configured on the same rules \
                 document: quiet window, focus suppression, frequency cap; the care budget shares \
                 the session spend ledger (care turns are billed like every other turn). \
                 Governance: changing rules is a file write (same governance family as other file \
                 writes) and every call stops at human approval — there is no auto-pass tier; \
                 invalid trigger/material parameters are refused outright before approval.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let prepared = match self.prepare(call) {
            Ok(prepared) => prepared,
            Err(result) => return result,
        };
        let now_ms = SystemCareClock.now_ms();
        let value = match prepared {
            PreparedCareCall::Add {
                id,
                trigger,
                material,
                enabled,
            } => {
                let rule = CareRule {
                    id,
                    trigger,
                    material,
                    enabled,
                    created_at_ms: now_ms,
                };
                let id = rule.id.clone();
                let trigger_label = rule.trigger.kind_label().to_string();
                match self.store.add_rule(rule) {
                    Ok(body) => json!({
                        "action": ACTION_CARE_ADD,
                        "id": id,
                        "trigger_kind": trigger_label,
                        "rules": body.rules.len(),
                        "write_face": CareWriteFace::RuleFileWrite.label(),
                    }),
                    Err(error) => return store_error_result(call, error),
                }
            }
            PreparedCareCall::Remove { id } => match self.store.remove_rule(&id) {
                Ok(body) => json!({
                    "action": ACTION_CARE_REMOVE,
                    "id": id,
                    "rules": body.rules.len(),
                    "write_face": CareWriteFace::RuleFileWrite.label(),
                }),
                Err(error) => return store_error_result(call, error),
            },
            PreparedCareCall::List => match self.store.list() {
                Ok(body) => json!({
                    "action": ACTION_CARE_LIST,
                    "write_face": CareWriteFace::ReadOnly.label(),
                    "rules": body.rules.iter().map(rule_json).collect::<Vec<_>>(),
                    "discipline": {
                        "quiet_start_minutes": body.discipline.quiet_start_minutes,
                        "quiet_end_minutes": body.discipline.quiet_end_minutes,
                        "frequency_max": body.discipline.frequency_max,
                        "frequency_window_hours": body.discipline.frequency_window_hours,
                        "focus_gap_minutes": body.discipline.focus_gap_minutes,
                        "catchup_minutes": body.discipline.catchup_minutes,
                    },
                    "last_care": body.last_care().map(record_json),
                }),
                Err(error) => return store_error_result(call, error),
            },
        };
        ToolResult::ok(&call.id, value).with_name(CARE_SCHEDULE_TOOL_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{SessionId, TraceId};

    fn tool(dir: &std::path::Path) -> CareScheduleTool {
        CareScheduleTool::new(dir)
    }

    fn call(id: &str, arguments: Value) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            name: CARE_SCHEDULE_TOOL_NAME.to_string(),
            arguments,
        }
    }

    fn dispatch(action: Option<&str>) -> GovernanceRequest<'static> {
        let capability: &'static CapabilityId = Box::leak(Box::new(
            CapabilityId::new(CARE_SCHEDULE_CAPABILITY_ID).expect("capability id"),
        ));
        let arguments: &'static Value = Box::leak(Box::new(match action {
            Some(action) => json!({ "action": action }),
            None => json!({}),
        }));
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        )
    }

    /// 治理档语义: 每个动作都停人工审批 (无免审动作), 理由按写面如实标注。
    #[tokio::test]
    async fn every_care_schedule_action_stops_at_require_approval() {
        let hook = CareScheduleApprovalHook::new();
        for action in CARE_SCHEDULE_ACTIONS {
            let decision = hook.evaluate(&dispatch(Some(action))).await;
            let Decision::RequireApproval { reason } = decision else {
                panic!("{action} 必须停人工审批 (无自动放行档): {decision:?}");
            };
            assert!(reason.contains("RequireApproval"), "{reason}");
            assert!(reason.contains("无自动放行档"), "{reason}");
            assert!(reason.contains("有人拍板"), "{reason}");
        }
        // 动作缺失/未知 fail-closed 同样停审批 (不静默放行)。
        assert!(matches!(
            hook.evaluate(&dispatch(None)).await,
            Decision::RequireApproval { .. }
        ));
        assert!(matches!(
            hook.evaluate(&dispatch(Some("care_free"))).await,
            Decision::RequireApproval { .. }
        ));

        // 写面口径进审批卡载荷: 改规则标写文件同族。
        let Decision::RequireApproval { reason } =
            hook.evaluate(&dispatch(Some(ACTION_CARE_ADD))).await
        else {
            panic!("care_add 必须停审批");
        };
        assert!(reason.contains("改规则 = 写文件"), "{reason}");
        assert!(reason.contains("write_intent 同族"), "{reason}");

        // 本钩子只管自己的面: 其它能力一律放行。
        let capability = CapabilityId::new("tool.fetch").expect("capability id");
        let arguments = json!({ "url": "https://example.com" });
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        assert_eq!(hook.evaluate(&request).await, Decision::Allow, "不越权误伤");
    }

    /// 写面归类逐动作钉死 (write_intent 同族口径)。
    #[test]
    fn write_face_classification_is_pinned_per_action() {
        assert_eq!(
            care_write_face_for_action(ACTION_CARE_ADD),
            Some(CareWriteFace::RuleFileWrite)
        );
        assert_eq!(
            care_write_face_for_action(ACTION_CARE_REMOVE),
            Some(CareWriteFace::RuleFileWrite)
        );
        assert_eq!(
            care_write_face_for_action(ACTION_CARE_LIST),
            Some(CareWriteFace::ReadOnly)
        );
        assert_eq!(care_write_face_for_action("rm -rf /"), None);
    }

    /// 工具面增删列真往返 + 非法参数拒绝即帧。
    #[tokio::test]
    async fn care_schedule_add_remove_list_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = tool(dir.path());

        let added = tool
            .invoke(&call(
                "call_add",
                json!({
                    "action": "care_add",
                    "id": "check-in",
                    "trigger": { "kind": "inactive", "hours": 48 },
                    "material": "问问他最近怎么样"
                }),
            ))
            .await;
        let value: Value = serde_json::from_str(&added.render()).expect("json");
        assert_eq!(value["action"], "care_add");
        assert_eq!(value["rules"], 1);
        assert_eq!(value["write_face"], "rule_file_write");

        let listed = tool
            .invoke(&call("call_list", json!({ "action": "care_list" })))
            .await;
        let value: Value = serde_json::from_str(&listed.render()).expect("json");
        assert_eq!(value["action"], "care_list");
        assert_eq!(value["rules"][0]["id"], "check-in");
        assert_eq!(value["rules"][0]["trigger_kind"], "inactive");
        assert_eq!(value["last_care"], Value::Null);

        let removed = tool
            .invoke(&call(
                "call_remove",
                json!({ "action": "care_remove", "id": "check-in" }),
            ))
            .await;
        let value: Value = serde_json::from_str(&removed.render()).expect("json");
        assert_eq!(value["rules"], 0);
    }

    /// 非法参数拒绝即帧 (不进审批, 与规则面校验同口径)。
    #[tokio::test]
    async fn invalid_parameters_are_refused_as_permanent_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = tool(dir.path());
        let cases = [
            json!({ "action": "care_free" }),
            json!({ "action": "care_add" }),
            json!({ "action": "care_add", "id": "bad id", "trigger": { "kind": "interval", "hours": 1 }, "material": "m" }),
            json!({ "action": "care_add", "id": "x", "trigger": { "kind": "cron" }, "material": "m" }),
            json!({ "action": "care_add", "id": "x", "trigger": { "kind": "interval", "hours": 0 }, "material": "m" }),
            json!({ "action": "care_add", "id": "x", "trigger": { "kind": "daily_moment", "minutes_of_day": 2000 }, "material": "m" }),
            json!({ "action": "care_add", "id": "x", "trigger": { "kind": "anniversary", "month": 4, "day": 31 }, "material": "m" }),
            json!({ "action": "care_add", "id": "x", "trigger": { "kind": "interval", "hours": 1 }, "material": "" }),
            json!({ "action": "care_remove" }),
        ];
        for arguments in cases {
            let result = tool.invoke(&call("call_bad", arguments.clone())).await;
            let rendered = result.render();
            assert!(
                rendered.contains("error")
                    || rendered.contains("refused")
                    || rendered.contains("requires"),
                "{arguments} 必须拒绝即帧: {rendered}"
            );
        }
    }
}
