//! 工具卡面固定数据（**自动生成** —— 勿手改）。
//!
//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/toolcard-cases.json`
//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`

use crate::toolcard_fixture::ToolCardCase;
use apeireth_ui_kit::json::JsonValue;
use apeireth_ui_kit::tool_card::{
    ToolCallDetails, ToolCallLifecycleCall, ToolCallLifecycleStatus, ToolCallStatus,
};

/// 对照用的固定卡面用例（两侧喂同一份数据）。
pub fn tool_card_cases() -> Vec<ToolCardCase> {
    vec![
        ToolCardCase {
            id: String::from("pending-basic"),
            tool_call: ToolCallDetails {
                status: ToolCallStatus::Pending,
                name: String::from("fs.read"),
                duration_ms: Some(120.0),
                args: Some(JsonValue::Object(vec![(String::from("path"), JsonValue::String(String::from("/卷宗/工作区/notes.md"))), (String::from("depth"), JsonValue::Number(2.0)), (String::from("recursive"), JsonValue::Bool(false))])),
                raw_args: None,
                result_summary: None,
                result_full: None,
                error: None,
            },
            lifecycle_call: ToolCallLifecycleCall {
                id: String::from("call-1"),
                name: String::from("fs.read"),
                status: ToolCallLifecycleStatus::Pending,
                latency_ms: Some(120.0),
                error: None,
                arguments: None,
                result: None,
            },
        },
        ToolCardCase {
            id: String::from("running-summary"),
            tool_call: ToolCallDetails {
                status: ToolCallStatus::Running,
                name: String::from("shell.exec"),
                duration_ms: Some(1500.0),
                args: Some(JsonValue::Object(vec![(String::from("command"), JsonValue::String(String::from("apeireth fs list --scope workspace")))])),
                raw_args: None,
                result_summary: Some(String::from("正在列举工作区…")),
                result_full: None,
                error: None,
            },
            lifecycle_call: ToolCallLifecycleCall {
                id: String::from("call-2"),
                name: String::from("shell.exec"),
                status: ToolCallLifecycleStatus::Running,
                latency_ms: Some(1500.0),
                error: None,
                arguments: Some(JsonValue::Object(vec![(String::from("command"), JsonValue::String(String::from("apeireth fs list --scope workspace")))])),
                result: None,
            },
        },
        ToolCardCase {
            id: String::from("succeeded-full"),
            tool_call: ToolCallDetails {
                status: ToolCallStatus::Succeeded,
                name: String::from("fs.write"),
                duration_ms: Some(42.0),
                args: Some(JsonValue::Object(vec![(String::from("path"), JsonValue::String(String::from("/卷宗/工作区/out.md"))), (String::from("content"), JsonValue::String(String::from("第一行\n第二行")))])),
                raw_args: None,
                result_summary: Some(String::from("已写入 2 行")),
                result_full: Some(JsonValue::Object(vec![(String::from("written"), JsonValue::Bool(true)), (String::from("bytes"), JsonValue::Number(24.0)), (String::from("lines"), JsonValue::Number(2.0))])),
                error: None,
            },
            lifecycle_call: ToolCallLifecycleCall {
                id: String::from("call-3"),
                name: String::from("fs.write"),
                status: ToolCallLifecycleStatus::Success,
                latency_ms: Some(42.0),
                error: None,
                arguments: Some(JsonValue::Object(vec![(String::from("path"), JsonValue::String(String::from("/卷宗/工作区/out.md")))])),
                result: Some(JsonValue::Object(vec![(String::from("written"), JsonValue::Bool(true)), (String::from("bytes"), JsonValue::Number(24.0))])),
            },
        },
        ToolCardCase {
            id: String::from("failed-zero-duration"),
            tool_call: ToolCallDetails {
                status: ToolCallStatus::Failed,
                name: String::from("shell.exec"),
                duration_ms: Some(0.0),
                args: Some(JsonValue::Object(vec![(String::from("command"), JsonValue::String(String::from("apeireth fs rm /卷宗/工作区/tmp")))])),
                raw_args: None,
                result_summary: None,
                result_full: None,
                error: Some(String::from("退出码 1：目标路径不可写")),
            },
            lifecycle_call: ToolCallLifecycleCall {
                id: String::from("call-4"),
                name: String::from("shell.exec"),
                status: ToolCallLifecycleStatus::Error,
                latency_ms: Some(87.0),
                error: Some(String::from("退出码 1：目标路径不可写")),
                arguments: Some(JsonValue::Object(vec![(String::from("command"), JsonValue::String(String::from("apeireth fs rm /卷宗/工作区/tmp")))])),
                result: None,
            },
        },
        ToolCardCase {
            id: String::from("cancelled-raw-args"),
            tool_call: ToolCallDetails {
                status: ToolCallStatus::Cancelled,
                name: String::from("fs.watch"),
                duration_ms: None,
                args: None,
                raw_args: Some(String::from("{\n  \"glob\": \"**/*.md\",\n  \"follow\": true\n}")),
                result_summary: None,
                result_full: None,
                error: None,
            },
            lifecycle_call: ToolCallLifecycleCall {
                id: String::from("call-5"),
                name: String::from("fs.watch"),
                status: ToolCallLifecycleStatus::Success,
                latency_ms: Some(2000.0),
                error: None,
                arguments: None,
                result: Some(JsonValue::String(String::from("卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段很长的结果文本：卷宗工作区的一段"))),
            },
        },
    ]
}
