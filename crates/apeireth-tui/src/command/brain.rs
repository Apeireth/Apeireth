//! Brain (脑) command 模块 — LLM 调用频率 / 思考状态 / 推理队列
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::IncrementCall`] — 增加某 provider 的 LLM 调用计数
//! 2. [`Command::GetCallCount`] — 读 total 调用数
//! 3. [`Command::GetActiveProvider`] — 当前 active provider
//! 4. [`Command::SetActiveProvider`] — 切换 provider
//! 5. [`Command::GetModelList`] — 已知 provider 列表 (编译期 hardcode)
//! 6. [`Command::GetLastThinking`] — 读最后一段 thinking chain
//!
//! **不假装**:
//! - 5 provider 编译期 hardcode (per `apeireth-tui` Cargo.toml: claude-code / codex /
//!   copilot / gemini-cli / opencode)
//! - thinking chain 走 in-memory Vec, 真实 R25.3 接 http_llm.rs UsageInfo
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: brain 服务 ASI 思考连续
//! - S-2 实事求是: 5 provider hardcode, thinking 标 partial
//! - O-2 走在前人经验上: 借 ratatui + tokio mpsc 模式
//! - O-3 干到底: 6 命令覆盖 LLM 全场景
//! - O-4 任何人都能接手: State 字段 + Command 变体全文档化
//! - O-5 不假装: thinking chain 是 `Vec<String>` 占位, 标 stub
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 5 provider 编译期 hardcode (per `apeireth-tui` Cargo.toml dependencies)
pub const PROVIDERS: &[&str] = &["claude-code", "codex", "copilot", "gemini-cli", "opencode"];

/// 默认 active provider
pub const DEFAULT_PROVIDER: &str = "claude-code";

/// Brain 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 各 provider 累计调用数
    pub call_counts: std::collections::HashMap<String, u64>,
    /// 当前 active provider
    pub active_provider: String,
    /// 最后一段 thinking chain (R25.2 占位 Vec)
    pub last_thinking: Vec<String>,
    /// 累计 total 调用
    pub total_calls: u64,
}

impl Default for State {
    fn default() -> Self {
        let mut call_counts = std::collections::HashMap::new();
        for p in PROVIDERS {
            call_counts.insert((*p).to_string(), 0);
        }
        Self {
            call_counts,
            active_provider: DEFAULT_PROVIDER.to_string(),
            last_thinking: Vec::new(),
            total_calls: 0,
        }
    }
}

/// Brain 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 增加某 provider 的 LLM 调用计数
    IncrementCall {
        /// provider 名, 必须在 [`PROVIDERS`] 内
        provider: String,
    },
    /// 读 total 调用数
    GetCallCount,
    /// 读当前 active provider
    GetActiveProvider,
    /// 切换 active provider
    SetActiveProvider {
        /// provider 名
        provider: String,
    },
    /// 读 5 provider 列表 (编译期 hardcode)
    GetModelList,
    /// 读最后一段 thinking chain (克隆)
    GetLastThinking,
}

/// Brain 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// total 调用数
    CallCount(u64),
    /// active provider 名
    ActiveProvider(String),
    /// provider 列表
    ModelList(Vec<String>),
    /// thinking chain 副本
    LastThinking(Vec<String>),
}

/// 处理 Brain 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — provider 名不在 5 编译期 hardcode 内
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::IncrementCall { provider } => {
            if !PROVIDERS.contains(&provider.as_str()) {
                return Err(OrganError::InvalidArg {
                    command: "IncrementCall",
                    reason: format!(
                        "provider '{provider}' not in {} 编译期 hardcode 列表",
                        PROVIDERS.len()
                    ),
                });
            }
            let entry = state.call_counts.entry(provider).or_insert(0);
            *entry = entry.saturating_add(1);
            state.total_calls = state.total_calls.saturating_add(1);
            Ok(Response::Unit)
        }
        Command::GetCallCount => Ok(Response::CallCount(state.total_calls)),
        Command::GetActiveProvider => Ok(Response::ActiveProvider(state.active_provider.clone())),
        Command::SetActiveProvider { provider } => {
            if !PROVIDERS.contains(&provider.as_str()) {
                return Err(OrganError::InvalidArg {
                    command: "SetActiveProvider",
                    reason: format!("provider '{provider}' not in 5 hardcode 列表"),
                });
            }
            state.active_provider = provider;
            Ok(Response::Unit)
        }
        Command::GetModelList => Ok(Response::ModelList(
            PROVIDERS.iter().map(|s| (*s).to_string()).collect(),
        )),
        Command::GetLastThinking => Ok(Response::LastThinking(state.last_thinking.clone())),
    }
}

/// 器官 ASCII 字符 (跨平台, 跟 `organ/mod.rs` 对齐)
pub const ASCII_CHAR: &str = "[BRAIN]";

/// 器官中文名
pub const NAME_ZH: &str = "脑";

// =====================================================================
// 单元测试 (6 命令 + 5 provider 守门 + 错误路径 = 8+ 测试)
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
        let _ = Command::IncrementCall {
            provider: "claude-code".into(),
        };
        let _ = Command::GetCallCount;
        let _ = Command::GetActiveProvider;
        let _ = Command::SetActiveProvider {
            provider: "codex".into(),
        };
        let _ = Command::GetModelList;
        let _ = Command::GetLastThinking;
    }

    // ---- 5 provider 编译期守门 ----

    #[test]
    fn five_providers_hardcoded() {
        assert_eq!(PROVIDERS.len(), 5, "5 provider 编译期 hardcode");
        // 必须含主人 R22 拍板的 5 个
        for required in ["claude-code", "codex", "copilot", "gemini-cli", "opencode"] {
            assert!(PROVIDERS.contains(&required), "PROVIDERS 应含 {required}");
        }
    }

    // ---- IncrementCall ----

    #[test]
    fn increment_call_increments_count() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::IncrementCall {
                provider: "claude-code".into(),
            },
        );
        assert!(r.is_ok());
        let r = handle(&mut state, Command::GetCallCount).unwrap();
        assert_eq!(r, Response::CallCount(1));
    }

    #[test]
    fn increment_call_rejects_unknown_provider() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::IncrementCall {
                provider: "fake-llm".into(),
            },
        );
        assert!(matches!(
            r,
            Err(OrganError::InvalidArg {
                command: "IncrementCall",
                ..
            })
        ));
    }

    // ---- SetActiveProvider ----

    #[test]
    fn set_active_provider_valid() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::SetActiveProvider {
                provider: "codex".into(),
            },
        );
        assert!(r.is_ok());
        let r = handle(&mut state, Command::GetActiveProvider).unwrap();
        assert_eq!(r, Response::ActiveProvider("codex".into()));
    }

    #[test]
    fn set_active_provider_rejects_unknown() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::SetActiveProvider {
                provider: "gpt-4".into(),
            },
        );
        assert!(matches!(
            r,
            Err(OrganError::InvalidArg {
                command: "SetActiveProvider",
                ..
            })
        ));
        // 状态不变
        assert_eq!(state.active_provider, DEFAULT_PROVIDER);
    }

    // ---- GetModelList ----

    #[test]
    fn get_model_list_returns_5() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetModelList).unwrap();
        match r {
            Response::ModelList(list) => assert_eq!(list.len(), 5),
            _ => panic!("expected ModelList"),
        }
    }

    // ---- GetLastThinking ----

    #[test]
    fn get_last_thinking_empty_initially() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetLastThinking).unwrap();
        match r {
            Response::LastThinking(v) => assert!(v.is_empty()),
            _ => panic!("expected LastThinking"),
        }
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[BRAIN]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "脑");
    }
}
