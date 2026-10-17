//! 性格周报工具面 (`tool.trait_report`, 裁定 ⑤) —— **只读档、零审批**。
//!
//! # 是什么
//!
//! 一次调用返回一份**性格周报**: 近 [`apeireth_orchestration::trait_report::TRAIT_REPORT_WINDOW_DAYS`]
//! 天四根特质滑杆的**移动量** / **移动原因 top** / **锁定态** (+ 每杆最近一次
//! 移动)。数据 = **成长计既有账** (自校准沉积记录 `tuning-log.jsonl`),
//! **不新造账**; 本工具不写任何文件、不执行、不外联。
//!
//! # 三条硬边界
//!
//! 1. **只读档**: 无写无执行无外联, 无参数 (未知字段拒绝), 默认可用,
//!    **零审批** (治理侧只做 grant, 与 `tool.self_status` 同档);
//! 2. **只回数字与原因, 不回对话内容**: 移动原因取自沉积记录的 reason 原文
//!    (成长账自记的调整理由), 不含记忆/对话正文;
//! 3. **错误即帧**: 账面/锁定档读不出来 = 失败帧报真因 (不编数字)。
//!
//! # 输出帧 (字段合同冻结, 见 [`TRAIT_REPORT_FRAME_FIELDS`])
//!
//! | 字段 | 含义 |
//! |---|---|
//! | `write_face` | 恒 `"read_only"` |
//! | `window_days` / `window_from_ms` / `window_to_ms` / `generated_at_ms` | 7 天窗口径 |
//! | `sliders` | 四杆一行: `param` / `label` / `value` / `locked` / `net_delta` / `move_count` / `locked_skipped` / `last_move` |
//! | `records_in_window` | 窗内成长账记录总数 (含零移动留痕) |
//! | `locked_skipped` | 窗内「锁定跳过沉积」审计留痕数 |
//! | `skipped_records` | 账面坏行跳过计数 (0 装诚实, 不静默吞) |
//! | `top_reasons` | 移动原因 top (仅实际移动的沉积记录) |

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_orchestration::trait_report::TraitReport;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_tools_canonical::{OutputSchema, SchemaKind};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::trait_growth::{now_epoch_ms, TraitGrowthFacts, TraitGrowthProbe};

/// 稳定能力标识 (注册表 / 治理授权共用)。
pub const TRAIT_REPORT_CAPABILITY_ID: &str = "tool.trait_report";
/// 模型侧声明名。
pub const TRAIT_REPORT_TOOL_NAME: &str = "trait_report";

/// 输出帧冻结字段 (形状合同; 演进须注明理由)。
pub const TRAIT_REPORT_FRAME_FIELDS: &[&str] = &[
    "write_face",
    "window_days",
    "window_from_ms",
    "window_to_ms",
    "generated_at_ms",
    "sliders",
    "records_in_window",
    "locked_skipped",
    "skipped_records",
    "top_reasons",
];

/// 空参数 (周报无参数; 未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraitReportParams {}

/// 性格周报工具 (`tool.trait_report`): 成长计既有账的只读投影。
pub struct TraitReportTool {
    id: CapabilityId,
    growth: TraitGrowthProbe,
}

impl TraitReportTool {
    /// 绑定成长账面探测口 (与自述性格节同一真值源)。
    pub fn new(growth: TraitGrowthProbe) -> Self {
        Self {
            id: CapabilityId::new(TRAIT_REPORT_CAPABILITY_ID).unwrap(),
            growth,
        }
    }

    /// 输出归一合同: 周报帧必须是对象且 [`TRAIT_REPORT_FRAME_FIELDS`] 恒在场。
    pub fn output_schema() -> OutputSchema {
        let mut schema = OutputSchema::new();
        schema = schema.require("write_face", SchemaKind::String);
        schema = schema.require("window_days", SchemaKind::Number);
        schema = schema.require("window_from_ms", SchemaKind::Number);
        schema = schema.require("window_to_ms", SchemaKind::Number);
        schema = schema.require("generated_at_ms", SchemaKind::Number);
        schema = schema.require("sliders", SchemaKind::Array);
        schema = schema.require("records_in_window", SchemaKind::Number);
        schema = schema.require("locked_skipped", SchemaKind::Number);
        schema = schema.require("skipped_records", SchemaKind::Number);
        schema.require("top_reasons", SchemaKind::Array)
    }

    /// 由账面读数投影一份周报帧 (纯投影; `now_ms` 由调用方给)。
    pub fn render_frame(facts: &TraitGrowthFacts, now_ms: i64) -> Value {
        let report = TraitReport::build(&facts.records, &facts.locks, facts.values, now_ms);
        let sliders: Vec<Value> = report
            .sliders
            .iter()
            .map(|row| {
                json!({
                    "param": row.param.id(),
                    "label": row.label,
                    "value": row.value,
                    "locked": row.locked,
                    "net_delta": row.net_delta,
                    "move_count": row.move_count,
                    "locked_skipped": row.locked_skipped,
                    "last_move": row.last_move.as_ref().map(|movement| json!({
                        "param": movement.param.id(),
                        "label": movement.label,
                        "at_epoch_ms": movement.at_epoch_ms,
                        "delta": movement.delta,
                        "reason": movement.reason,
                    })),
                })
            })
            .collect();
        json!({
            "write_face": "read_only",
            "window_days": report.window_days,
            "window_from_ms": report.window_from_ms,
            "window_to_ms": report.window_to_ms,
            "generated_at_ms": report.generated_at_ms,
            "sliders": sliders,
            "records_in_window": report.records_in_window,
            "locked_skipped": report.locked_skipped,
            "skipped_records": facts.skipped,
            "top_reasons": report.top_reasons.iter().map(|row| json!({
                "reason": row.reason,
                "count": row.count,
            })).collect::<Vec<Value>>(),
        })
    }

    async fn invoke_scoped(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if let Err(error) = serde_json::from_value::<TraitReportParams>(arguments) {
            return ToolResult::permanent_error(
                &call.id,
                format!("invalid trait_report parameters: {error}"),
            )
            .with_name(TRAIT_REPORT_TOOL_NAME);
        }
        // 账面现读跑在阻塞池上: 文件读卡住时执行链的到期计时器仍能触发,
        // 超时归既有 `timeout.*` code 族 (与 `tool.self_status` 同口径)。
        let growth = Arc::clone(&self.growth);
        let collected = tokio::task::spawn_blocking(move || growth()).await;
        match collected {
            Ok(Ok(facts)) => ToolResult::ok(&call.id, Self::render_frame(&facts, now_epoch_ms()))
                .with_name(TRAIT_REPORT_TOOL_NAME),
            Ok(Err(error)) => ToolResult::permanent_error(
                &call.id,
                format!("trait growth ledger refused: {error}"),
            )
            .with_name(TRAIT_REPORT_TOOL_NAME),
            Err(error) => ToolResult::permanent_error(
                &call.id,
                format!("trait_report collection task failed: {error}"),
            )
            .with_name(TRAIT_REPORT_TOOL_NAME),
        }
    }
}

#[async_trait]
impl ToolCapability for TraitReportTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(TRAIT_REPORT_TOOL_NAME)
            .with_description(
                "Read-only personality growth report over the last 7 days of the existing growth \
                 ledger (self-tuning deposit records; nothing new is recorded). For each of the \
                 four sliders (memory fade / curiosity strength / tone saturation / consolidation \
                 cadence) it reports the current value, whether the slider is locked, its net \
                 movement and moved-deposit count inside the window, and its most recent movement \
                 with the recorded reason; plus the top movement reasons, how many deposits were \
                 skipped because their slider is locked (audit trace), and how many ledger lines \
                 were unreadable. Read-only: no writes, no execution, no network, no approval.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_scoped(call).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::trait_growth::TraitGrowthPaths;
    use apeireth_orchestration::self_tuning::{
        TunableParam, TuningRecord, TuningValues, DAY_MS, TRAIT_LOCKED_SKIP_PREFIX,
    };

    fn facts_with(
        records: Vec<TuningRecord>,
        locks: apeireth_orchestration::TraitLocks,
    ) -> TraitGrowthFacts {
        TraitGrowthFacts {
            records,
            skipped: 1,
            locks,
            values: TuningValues::baseline(),
        }
    }

    fn record(
        seq: u64,
        param: TunableParam,
        previous: f64,
        next: f64,
        reason: &str,
        at: i64,
    ) -> TuningRecord {
        TuningRecord {
            seq,
            param,
            previous,
            next,
            reason: reason.to_string(),
            at_epoch_ms: at,
        }
    }

    fn tool_of(facts: TraitGrowthFacts) -> TraitReportTool {
        TraitReportTool::new(Arc::new(move || Ok(facts.clone())))
    }

    fn call() -> ToolCall {
        ToolCall {
            id: "call_trait_report".into(),
            name: TRAIT_REPORT_TOOL_NAME.into(),
            arguments: json!({}),
        }
    }

    // 周报数值从账面真值投影: 移动量 / 移动原因 / 锁定态 / 跳过计数逐一可核。
    #[tokio::test]
    async fn frame_reports_movement_reasons_and_lock_state_from_the_ledger() {
        // 记录时刻取"现在"相对位 (走真 invoke 路径, 窗口口径与纯汇总测试同源)。
        let now = crate::canonical::trait_growth::now_epoch_ms();
        let locks = apeireth_orchestration::TraitLocks {
            memory_fade: true,
            ..apeireth_orchestration::TraitLocks::default()
        };
        let facts = facts_with(
            vec![
                record(
                    1,
                    TunableParam::MemoryFade,
                    1.0,
                    1.25,
                    "上调一档",
                    now - DAY_MS,
                ),
                record(
                    2,
                    TunableParam::MemoryFade,
                    1.25,
                    1.25,
                    "已达参数上限: x",
                    now - 2 * DAY_MS,
                ),
                record(
                    3,
                    TunableParam::MemoryFade,
                    1.25,
                    1.25,
                    &format!("{TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 被锁"),
                    now - DAY_MS,
                ),
                record(
                    4,
                    TunableParam::ToneSaturation,
                    1.0,
                    0.8,
                    "过饱和",
                    now - DAY_MS,
                ),
            ],
            locks,
        );
        let tool = tool_of(facts);
        let result = tool.invoke(&call()).await;
        assert!(result.is_ok(), "{}", result.render());
        let value: Value = serde_json::from_str(&result.render()).expect("json");
        for field in TRAIT_REPORT_FRAME_FIELDS {
            assert!(value.get(field).is_some(), "冻结帧字段缺 {field}");
        }
        assert_eq!(value["write_face"], "read_only");
        assert_eq!(value["window_days"], 7);
        assert_eq!(value["records_in_window"], 4);
        assert_eq!(value["locked_skipped"], 1);
        assert_eq!(value["skipped_records"], 1);

        let sliders = value["sliders"].as_array().expect("sliders");
        assert_eq!(sliders.len(), 4);
        let fade = &sliders[0];
        assert_eq!(fade["param"], "memory_fade");
        assert_eq!(fade["label"], "遗忘衰减强度");
        assert_eq!(fade["locked"], true, "锁定态如实");
        assert_eq!(fade["value"], 1.0);
        assert_eq!(fade["move_count"], 1);
        assert_eq!(fade["locked_skipped"], 1);
        assert_eq!(fade["last_move"]["reason"], "上调一档");
        assert_eq!(fade["last_move"]["delta"], 0.25);
        let tone = &sliders[2];
        assert_eq!(tone["param"], "tone_saturation");
        assert_eq!(tone["locked"], false);
        let tone_net = tone["net_delta"].as_f64().expect("net_delta");
        assert!((tone_net - (-0.2)).abs() < 1e-9, "{tone_net}");

        let reasons = value["top_reasons"].as_array().expect("top reasons");
        let texts: Vec<&str> = reasons
            .iter()
            .map(|row| row["reason"].as_str().expect("reason"))
            .collect();
        assert!(texts.contains(&"上调一档"), "{texts:?}");
        assert!(texts.contains(&"过饱和"), "{texts:?}");
        assert!(
            !texts
                .iter()
                .any(|t| t.starts_with(TRAIT_LOCKED_SKIP_PREFIX)),
            "锁定跳过留痕不算移动原因"
        );
    }

    // 只读档: 无参数 (未知字段拒绝即帧), 无写无执行; 帧恒标 read_only。
    #[tokio::test]
    async fn read_only_face_refuses_parameters_and_never_writes() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-trait-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let paths = TraitGrowthPaths::new(dir.join("tuning-log.jsonl"));
        let before = std::fs::read_dir(&dir).expect("dir").count();
        let growth = crate::canonical::trait_growth::trait_growth_probe(paths.clone());
        let tool = TraitReportTool::new(growth);

        let mut bogus = call();
        bogus.arguments = json!({ "days": 30 });
        let refused = tool.invoke(&bogus).await;
        assert!(!refused.is_ok(), "未知参数必须拒绝即帧");
        assert!(
            refused.render().contains("invalid trait_report parameters"),
            "{}",
            refused.render()
        );

        let result = tool.invoke(&call()).await;
        assert!(result.is_ok(), "{}", result.render());
        let value: Value = serde_json::from_str(&result.render()).expect("json");
        assert_eq!(value["write_face"], "read_only");
        assert_eq!(value["window_days"], 7);
        assert_eq!(value["records_in_window"], 0, "空账 = 如实空周报");
        let after = std::fs::read_dir(&dir).expect("dir").count();
        assert_eq!(before, after, "只读档不得落任何文件");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // 声明面: 只读零审批 (无写无执行), 无参数对象 + 未知字段拒绝。
    #[test]
    fn declaration_is_read_only_zero_approval() {
        let growth: TraitGrowthProbe = Arc::new(|| Ok(facts_with(vec![], Default::default())));
        let tool = TraitReportTool::new(growth);
        let declared = tool.declaration();
        assert_eq!(declared.name, TRAIT_REPORT_TOOL_NAME);
        assert_eq!(tool.id().as_str(), TRAIT_REPORT_CAPABILITY_ID);
        let description = declared.description.clone().expect("description 在场");
        assert!(description.contains("Read-only"), "{description}");
        assert!(description.contains("no approval"), "{description}");
        let properties = declared
            .parameters
            .get("properties")
            .and_then(Value::as_object)
            .expect("properties");
        assert!(properties.is_empty(), "周报无参数面: {properties:?}");
        assert_eq!(
            declared.parameters.get("additionalProperties"),
            Some(&Value::Bool(false)),
            "未知字段拒绝"
        );
    }
}
