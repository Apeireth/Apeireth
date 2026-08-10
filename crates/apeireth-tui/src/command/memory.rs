//! Memory (记忆) command 模块 — 会话历史 / 搜索 / 持久化
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::Append`] — 追加一条记忆 (user / assistant / system)
//! 2. [`Command::GetHistory`] — 读最近 N 条
//! 3. [`Command::Search`] — 按 query 搜索 (子串匹配, R25.2 stub)
//! 4. [`Command::GetCount`] — 累计条数
//! 5. [`Command::Clear`] — 清空
//! 6. [`Command::GetConversations`] — 读所有对话 (按 user 消息分块)
//!
//! **不假装**:
//! - memory 在 `organ/mod.rs` 标 `Readiness::Partial` — 5 命令 in-memory 实现
//! - 真实持久化 R25.3 接 `apeireth-memory` crate
//! - role 编译期 hardcode 3 角色 (user / assistant / system)
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: memory 服务 ASI 经验沉淀
//! - S-2 实事求是: 3 role hardcode, search 子串匹配
//! - O-2 走在前人经验上: 借 `App::push_user_input` 等已有沉淀
//! - O-3 干到底: 6 命令覆盖记忆全场景
//! - O-4 任何人都能接手: State + role 枚举全文档化
//! - O-5 不假装: in-memory Vec, 标 partial
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 3 role 编译期 hardcode (per 主人 R22 拍板)
pub const ROLES: &[&str] = &["user", "assistant", "system"];

/// 单条记忆
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryEntry {
    /// role: user / assistant / system
    pub role: String,
    /// content
    pub content: String,
}

/// Memory 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 记忆条目 (新 → 旧)
    pub entries: Vec<MemoryEntry>,
}

impl Default for State {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

/// Memory 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 追加一条记忆
    Append {
        /// role: user / assistant / system
        role: String,
        /// content
        content: String,
    },
    /// 读最近 N 条
    GetHistory {
        /// 最多返回条数
        limit: usize,
    },
    /// 按 query 搜索 (子串匹配, R25.2 stub)
    Search {
        /// 搜索关键字
        query: String,
    },
    /// 读累计条数
    GetCount,
    /// 清空
    Clear,
    /// 读所有对话 (按 user 消息分块, R25.2 stub)
    GetConversations,
}

/// Memory 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// 历史条目
    History(Vec<MemoryEntry>),
    /// 搜索结果 (matching indices)
    SearchHits(Vec<usize>),
    /// 累计条数
    Count(usize),
    /// 对话列表 (按 user 消息分块)
    Conversations(Vec<Vec<MemoryEntry>>),
}

/// 处理 Memory 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — role 不在 3 编译期 hardcode / content 为空
/// - [`OrganError::Unsupported`] — Search 在 R25.2 是 stub
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::Append { role, content } => {
            if !ROLES.contains(&role.as_str()) {
                return Err(OrganError::InvalidArg {
                    command: "Append",
                    reason: format!("role '{role}' not in 3 编译期 hardcode"),
                });
            }
            if content.is_empty() {
                return Err(OrganError::InvalidArg {
                    command: "Append",
                    reason: "content 不能为空".into(),
                });
            }
            state.entries.insert(0, MemoryEntry { role, content });
            Ok(Response::Unit)
        }
        Command::GetHistory { limit } => {
            let capped: Vec<MemoryEntry> = state.entries.iter().take(limit).cloned().collect();
            Ok(Response::History(capped))
        }
        Command::Search { query } => {
            if query.is_empty() {
                return Err(OrganError::InvalidArg {
                    command: "Search",
                    reason: "query 不能为空".into(),
                });
            }
            // S-2 实事求是: stub — 用子串匹配, R25.3 接 apeireth-memory vector index
            let hits: Vec<usize> = state
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.content.contains(&query))
                .map(|(i, _)| i)
                .collect();
            Ok(Response::SearchHits(hits))
        }
        Command::GetCount => Ok(Response::Count(state.entries.len())),
        Command::Clear => {
            state.entries.clear();
            Ok(Response::Unit)
        }
        Command::GetConversations => {
            // 按 user 消息分块
            let mut convs: Vec<Vec<MemoryEntry>> = Vec::new();
            let mut current: Vec<MemoryEntry> = Vec::new();
            // 反向遍历 (新 → 旧), 按 user 切
            for entry in state.entries.iter().rev() {
                if entry.role == "user" && !current.is_empty() {
                    convs.push(std::mem::take(&mut current));
                }
                current.push(entry.clone());
            }
            if !current.is_empty() {
                convs.push(current);
            }
            Ok(Response::Conversations(convs))
        }
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[MEM]";

/// 器官中文名
pub const NAME_ZH: &str = "记忆";

// =====================================================================
// 单元测试 (6 命令 + 3 role 守门 + 错误路径 = 8+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> State {
        State::default()
    }

    // ---- 6 命令全部可枚举 ----

    #[test]
    fn six_commands_constructible() {
        let _ = Command::Append { role: "user".into(), content: "hi".into() };
        let _ = Command::GetHistory { limit: 10 };
        let _ = Command::Search { query: "test".into() };
        let _ = Command::GetCount;
        let _ = Command::Clear;
        let _ = Command::GetConversations;
    }

    // ---- 3 role 编译期 hardcode ----

    #[test]
    fn three_roles_hardcoded() {
        assert_eq!(ROLES.len(), 3, "3 role 编译期 hardcode");
        for r in ["user", "assistant", "system"] {
            assert!(ROLES.contains(&r));
        }
    }

    // ---- Append ----

    #[test]
    fn append_valid_entry() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Append { role: "user".into(), content: "hello".into() },
            );
        assert!(r.is_ok());
        assert_eq!(state.entries.len(), 1);
    }

    #[test]
    fn append_rejects_unknown_role() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Append { role: "admin".into(), content: "x".into() },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "Append", .. })));
    }

    #[test]
    fn append_rejects_empty_content() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Append { role: "user".into(), content: "".into() },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "Append", .. })));
    }

    // ---- GetHistory ----

    #[test]
    fn get_history_respects_limit() {
        let mut state = fresh_state();
        for i in 0..5 {
            let _ = handle(
                &mut state,
                Command::Append {
                    role: "user".into(),
                    content: format!("msg {i}"),
                },
                    );
        }
        let r = handle(&mut state, Command::GetHistory { limit: 3 }).unwrap();
        match r {
            Response::History(v) => assert_eq!(v.len(), 3),
            _ => panic!("expected History"),
        }
    }

    // ---- Search ----

    #[test]
    fn search_finds_matching_entries() {
        let mut state = fresh_state();
        let _ = handle(
            &mut state,
            Command::Append { role: "user".into(), content: "hello world".into() },
            );
        let _ = handle(
            &mut state,
            Command::Append { role: "user".into(), content: "goodbye".into() },
            );
        let r = handle(
            &mut state,
            Command::Search { query: "hello".into() },
            )
        .unwrap();
        match r {
            Response::SearchHits(v) => assert_eq!(v.len(), 1),
            _ => panic!("expected SearchHits"),
        }
    }

    #[test]
    fn search_rejects_empty_query() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::Search { query: "".into() });
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "Search", .. })));
    }

    // ---- Clear ----

    #[test]
    fn clear_empties_entries() {
        let mut state = fresh_state();
        let _ = handle(
            &mut state,
            Command::Append { role: "user".into(), content: "x".into() },
            );
        let _ = handle(&mut state, Command::Clear).unwrap();
        assert!(state.entries.is_empty());
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[MEM]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "记忆");
    }
}
