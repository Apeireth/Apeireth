//! Hand (手) command 模块 — 工具调用统计 / 工具白名单
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::InvokeTool`] — 模拟一次工具调用 (走白名单 + 记录历史)
//! 2. [`Command::GetRecentCalls`] — 读最近 N 次工具调用
//! 3. [`Command::GetWhitelist`] — 读 6 工具白名单 (编译期 hardcode)
//! 4. [`Command::GetCallCount`] — 累计调用数
//! 5. [`Command::ClearHistory`] — 清空工具调用历史
//! 6. [`Command::GetLastError`] — 读最后错误
//!
//! **不假装**:
//! - 6 工具白名单编译期 hardcode (per `error.rs::TOOL_WHITELIST`: calendar / message /
//!   contact / task / search / drive)
//! - 工具调用走 in-memory 历史, 真实 R25.3 接 apeireth-api /v1/tools/{name}/invoke
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: hand 服务 ASI 执行落地
//! - S-2 实事求是: 6 工具 hardcode, 调用历史 in-memory 标 partial
//! - O-2 走在前人经验上: 借 `error.rs::TOOL_WHITELIST` 已有沉淀
//! - O-3 干到底: 6 命令覆盖工具全场景
//! - O-4 任何人都能接手: State + Command 全文档化
//! - O-5 不假装: 调用是 in-memory, 不假装接 apeireth-api
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 6 工具白名单 (编译期 hardcode, 跟 `src/error.rs::TOOL_WHITELIST` 同步 — 不 import
/// 是因为 organ/command/* 通过 `#[path]` 被 tests/ 加载时, 父 crate root 不一定有
/// `mod error;`. 重复定义 = 业界 OK 模式 (per O-2 走在前人经验上 — 借已有沉淀)
pub const TOOL_WHITELIST: &[&str] = &[
    "calendar",
    "message",
    "contact",
    "task",
    "search",
    "drive",
];

/// 单次工具调用记录
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    /// 工具名
    pub name: String,
    /// 参数 (JSON object)
    pub args: serde_json::Value,
    /// 调用时间戳 (epoch ms, 占位)
    pub timestamp_ms: u64,
    /// 状态 (Ok / Err)
    pub status: ToolCallStatus,
}

/// 工具调用状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// 成功
    Ok,
    /// 失败 — 携带错误描述
    Err(String),
}

/// Hand 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 工具调用历史 (新 → 旧, Vec 头是最新的)
    pub history: Vec<ToolCall>,
    /// 累计调用数
    pub call_count: u64,
    /// 最后一次错误信息 (无错 = None)
    pub last_error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            call_count: 0,
            last_error: None,
        }
    }
}

/// Hand 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 模拟一次工具调用 (走白名单 + 记录历史)
    InvokeTool {
        /// 工具名
        name: String,
        /// 参数 (JSON object)
        args: serde_json::Value,
    },
    /// 读最近 N 次工具调用
    GetRecentCalls {
        /// 最多返回条数
        limit: usize,
    },
    /// 读 6 工具白名单
    GetWhitelist,
    /// 读累计调用数
    GetCallCount,
    /// 清空工具调用历史
    ClearHistory,
    /// 读最后错误
    GetLastError,
}

/// Hand 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// 工具调用历史 (新 → 旧)
    RecentCalls(Vec<ToolCall>),
    /// 6 工具白名单
    Whitelist(Vec<&'static str>),
    /// total 调用数
    CallCount(u64),
    /// 最后错误 (None = 无错)
    LastError(Option<String>),
}

/// 处理 Hand 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — 工具名不在 6 工具白名单 / args 非 JSON object
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::InvokeTool { name, args } => {
            // K-1.3 守门 (跟 error.rs::validate_tool_name 复用)
            if !TOOL_WHITELIST.contains(&name.as_str()) {
                state.last_error = Some(format!("tool '{name}' not in 6-tool whitelist"));
                return Err(OrganError::InvalidArg {
                    command: "InvokeTool",
                    reason: format!("tool '{name}' not in 6-tool whitelist (K-1.3)"),
                });
            }
            // K-1.4 守门 — args 必须是 JSON object
            if !args.is_object() {
                state.last_error = Some(format!("args not JSON object: {args}"));
                return Err(OrganError::InvalidArg {
                    command: "InvokeTool",
                    reason: format!("args must be JSON object (K-1.4), got: {args}"),
                });
            }
            // 模拟成功调用 (R25.2 stub — 真实 R25.3 走 HTTP)
            state.history.insert(
                0,
                ToolCall {
                    name,
                    args,
                    timestamp_ms: 0, // 占位
                    status: ToolCallStatus::Ok,
                },
            );
            state.call_count = state.call_count.saturating_add(1);
            state.last_error = None;
            Ok(Response::Unit)
        }
        Command::GetRecentCalls { limit } => {
            let capped = state.history.iter().take(limit).cloned().collect();
            Ok(Response::RecentCalls(capped))
        }
        Command::GetWhitelist => Ok(Response::Whitelist(TOOL_WHITELIST.to_vec())),
        Command::GetCallCount => Ok(Response::CallCount(state.call_count)),
        Command::ClearHistory => {
            state.history.clear();
            state.last_error = None;
            Ok(Response::Unit)
        }
        Command::GetLastError => Ok(Response::LastError(state.last_error.clone())),
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[HAND]";

/// 器官中文名
pub const NAME_ZH: &str = "手";

// =====================================================================
// 单元测试 (6 命令 + 6 工具白名单 + K-1.3/K-1.4 守门 = 8+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fresh_state() -> State {
        State::default()
    }

    // ---- 6 命令全部可枚举 ----

    #[test]
    fn six_commands_constructible() {
        let _ = Command::InvokeTool { name: "calendar".into(), args: json!({}) };
        let _ = Command::GetRecentCalls { limit: 10 };
        let _ = Command::GetWhitelist;
        let _ = Command::GetCallCount;
        let _ = Command::ClearHistory;
        let _ = Command::GetLastError;
    }

    // ---- 6 工具白名单编译期守门 ----

    #[test]
    fn six_tools_hardcoded() {
        assert_eq!(TOOL_WHITELIST.len(), 6, "6 工具白名单编译期 hardcode");
        for required in ["calendar", "message", "contact", "task", "search", "drive"] {
            assert!(TOOL_WHITELIST.contains(&required), "白名单应含 {required}");
        }
    }

    // ---- InvokeTool ----

    #[test]
    fn invoke_tool_valid_accepted() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::InvokeTool {
                name: "calendar".into(),
                args: json!({"date": "2026-08-06"}),
            },
            );
        assert!(r.is_ok());
        assert_eq!(state.call_count, 1);
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn invoke_tool_rejects_unknown_tool() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::InvokeTool { name: "unknown-tool".into(), args: json!({}) },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "InvokeTool", .. })));
        // last_error 被记录
        assert!(state.last_error.is_some());
    }

    #[test]
    fn invoke_tool_rejects_non_object_args() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::InvokeTool {
                name: "calendar".into(),
                args: json!(["array", "not", "object"]),
            },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "InvokeTool", .. })));
    }

    // ---- GetRecentCalls ----

    #[test]
    fn get_recent_calls_respects_limit() {
        let mut state = fresh_state();
        for name in TOOL_WHITELIST.iter().take(4) {
            let _ = handle(
                &mut state,
                Command::InvokeTool { name: (*name).to_string(), args: json!({}) },
                    );
        }
        let r = handle(&mut state, Command::GetRecentCalls { limit: 2 }).unwrap();
        match r {
            Response::RecentCalls(v) => assert_eq!(v.len(), 2),
            _ => panic!("expected RecentCalls"),
        }
    }

    // ---- GetWhitelist ----

    #[test]
    fn get_whitelist_returns_6() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetWhitelist).unwrap();
        match r {
            Response::Whitelist(v) => assert_eq!(v.len(), 6),
            _ => panic!("expected Whitelist"),
        }
    }

    // ---- GetCallCount ----

    #[test]
    fn get_call_count_zero_initially() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetCallCount).unwrap();
        assert_eq!(r, Response::CallCount(0));
    }

    // ---- ClearHistory ----

    #[test]
    fn clear_history_empties_vec() {
        let mut state = fresh_state();
        let _ = handle(
            &mut state,
            Command::InvokeTool { name: "search".into(), args: json!({}) },
            );
        let _ = handle(&mut state, Command::ClearHistory).unwrap();
        assert!(state.history.is_empty());
        assert!(state.last_error.is_none());
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[HAND]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "手");
    }
}
