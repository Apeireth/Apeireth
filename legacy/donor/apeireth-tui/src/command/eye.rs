//! Eye (眼) command 模块 — 输入监控
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::WatchInput`] — 启动输入监控
//! 2. [`Command::PauseMonitoring`] — 暂停监控
//! 3. [`Command::ResumeMonitoring`] — 恢复监控
//! 4. [`Command::IsActive`] — 读监控是否激活
//! 5. [`Command::GetRecentTokens`] — 读最近 N 个 token
//! 6. [`Command::GetInputRate`] — 读输入速率 (tokens/sec, R25.2 占位)
//!
//! **不假装 (0 假装)**:
//! - eye 在 `organ/mod.rs` 标 `Readiness::Partial` (R22 ST-A1.2: 1/4 真接, keystrokes);
//!   命令层监控命令只操作 in-memory 状态机, 不接真实输入流, 数据类命令返空/占位
//! - 真实 R25.3 接用户输入流 (`crossterm::event` 解析)
//! - 监控状态机: `Idle` → `Watching` → `Paused` → `Watching` (3 态, 编译期 hardcode)
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: eye 服务 ASI 感知输入
//! - S-2 实事求是: stub 标 partial, 5 命令全占位
//! - O-2 走在前人经验上: 借 crossterm::event 业界模式
//! - O-3 干到底: 6 命令覆盖监控全场景
//! - O-4 任何人都能接手: State + 3 态枚举全文档化
//! - O-5 不假装: tokens 是 in-memory Vec, 标 stub
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 监控状态机 (3 态, 编译期 hardcode)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorState {
    /// 空闲 (未启动)
    Idle,
    /// 监控中
    Watching,
    /// 已暂停
    Paused,
}

impl MonitorState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Watching => "watching",
            Self::Paused => "paused",
        }
    }
}

/// Eye 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 监控状态
    pub monitor: MonitorState,
    /// 监控的 sample 间隔 (ms, 占位)
    pub sample_ms: u32,
    /// 最近输入 token (新 → 旧, R25.2 stub Vec)
    pub recent_tokens: Vec<String>,
    /// 输入速率 (tokens/sec, R25.2 placeholder)
    pub input_rate: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            monitor: MonitorState::Idle,
            sample_ms: 100,
            recent_tokens: Vec::new(),
            input_rate: 0.0,
        }
    }
}

/// Eye 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 启动输入监控
    WatchInput {
        /// sample 间隔 (ms, 必须 > 0)
        sample_ms: u32,
    },
    /// 暂停监控
    PauseMonitoring,
    /// 恢复监控
    ResumeMonitoring,
    /// 读监控是否激活
    IsActive,
    /// 读最近 N 个 token
    GetRecentTokens {
        /// 最多返回条数
        limit: usize,
    },
    /// 读输入速率 (tokens/sec)
    GetInputRate,
}

/// Eye 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// 监控状态
    Active(bool),
    /// 最近 token
    RecentTokens(Vec<String>),
    /// 输入速率
    InputRate(f32),
}

/// 处理 Eye 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — sample_ms == 0
/// - [`OrganError::NotReady`] — PauseMonitoring 时未在 Watching
/// - [`OrganError::Unsupported`] — eye 在 R25.2 是 stub, 部分 command 拒绝
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::WatchInput { sample_ms } => {
            if sample_ms == 0 {
                return Err(OrganError::InvalidArg {
                    command: "WatchInput",
                    reason: "sample_ms must be > 0".into(),
                });
            }
            state.monitor = MonitorState::Watching;
            state.sample_ms = sample_ms;
            Ok(Response::Unit)
        }
        Command::PauseMonitoring => {
            if state.monitor != MonitorState::Watching {
                return Err(OrganError::NotReady {
                    organ: ASCII_CHAR,
                    reason: format!(
                        "PauseMonitoring 要求当前 Watching, 实际 {}",
                        state.monitor.label()
                    ),
                });
            }
            state.monitor = MonitorState::Paused;
            Ok(Response::Unit)
        }
        Command::ResumeMonitoring => {
            if state.monitor != MonitorState::Paused {
                return Err(OrganError::NotReady {
                    organ: ASCII_CHAR,
                    reason: format!(
                        "ResumeMonitoring 要求当前 Paused, 实际 {}",
                        state.monitor.label()
                    ),
                });
            }
            state.monitor = MonitorState::Watching;
            Ok(Response::Unit)
        }
        Command::IsActive => Ok(Response::Active(state.monitor == MonitorState::Watching)),
        Command::GetRecentTokens { limit: _ } => {
            // S-2 实事求是: eye 是 stub — recent_tokens 永远是空
            let _ = state; // 占位, R25.3 真接
            Ok(Response::RecentTokens(Vec::new()))
        }
        Command::GetInputRate => {
            // S-2 实事求是 (0 假装): placeholder — 返回存储的占位值 (默认 0.0),
            // 不假装已做真实输入速率测量 (R25.3 真接 crossterm 后放开)
            Ok(Response::InputRate(state.input_rate))
        }
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[EYE]";

/// 器官中文名
pub const NAME_ZH: &str = "眼";

// =====================================================================
// 单元测试 (6 命令 + 3 态状态机 + 错误路径 = 8+ 测试)
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
        let _ = Command::WatchInput { sample_ms: 100 };
        let _ = Command::PauseMonitoring;
        let _ = Command::ResumeMonitoring;
        let _ = Command::IsActive;
        let _ = Command::GetRecentTokens { limit: 10 };
        let _ = Command::GetInputRate;
    }

    // ---- 3 态状态机 ----

    #[test]
    fn three_states_distinct() {
        let labels: Vec<&str> = [
            MonitorState::Idle,
            MonitorState::Watching,
            MonitorState::Paused,
        ]
        .iter()
        .map(|s| s.label())
        .collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3, "3 态状态机编译期 hardcode");
    }

    // ---- WatchInput ----

    #[test]
    fn watch_input_transitions_to_watching() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::WatchInput { sample_ms: 50 });
        assert!(r.is_ok());
        assert_eq!(state.monitor, MonitorState::Watching);
        assert_eq!(state.sample_ms, 50);
    }

    #[test]
    fn watch_input_rejects_zero_sample() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::WatchInput { sample_ms: 0 });
        assert!(matches!(
            r,
            Err(OrganError::InvalidArg {
                command: "WatchInput",
                ..
            })
        ));
    }

    // ---- Pause/Resume ----

    #[test]
    fn pause_requires_watching() {
        let mut state = fresh_state();
        // Idle → Pause 失败
        let r = handle(&mut state, Command::PauseMonitoring);
        assert!(matches!(r, Err(OrganError::NotReady { .. })));
    }

    #[test]
    fn pause_resume_round_trip() {
        let mut state = fresh_state();
        let _ = handle(&mut state, Command::WatchInput { sample_ms: 100 }).unwrap();
        let _ = handle(&mut state, Command::PauseMonitoring).unwrap();
        assert_eq!(state.monitor, MonitorState::Paused);
        let _ = handle(&mut state, Command::ResumeMonitoring).unwrap();
        assert_eq!(state.monitor, MonitorState::Watching);
    }

    // ---- IsActive ----

    #[test]
    fn is_active_reflects_state() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::IsActive).unwrap();
        assert_eq!(r, Response::Active(false));
        let _ = handle(&mut state, Command::WatchInput { sample_ms: 100 }).unwrap();
        let r = handle(&mut state, Command::IsActive).unwrap();
        assert_eq!(r, Response::Active(true));
    }

    // ---- Stub 标缺 ----

    #[test]
    fn get_recent_tokens_empty_stub() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetRecentTokens { limit: 10 }).unwrap();
        // S-2 实事求是: stub organ 永远返空, 不假装有数据
        match r {
            Response::RecentTokens(v) => assert!(v.is_empty()),
            _ => panic!("expected RecentTokens"),
        }
    }

    #[test]
    fn get_input_rate_is_honest_placeholder() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetInputRate).unwrap();
        // 0 假装: 默认占位 0.0, 不假装真实输入速率测量
        match r {
            Response::InputRate(v) => assert_eq!(v, 0.0),
            _ => panic!("expected InputRate"),
        }
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[EYE]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "眼");
    }
}
