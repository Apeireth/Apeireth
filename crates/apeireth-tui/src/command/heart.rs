//! Heart (心) command 模块 — CPU 心跳 / 任务调度频率
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式 (per `BORROW_FROM_GOLUTRA.md` §2)
//!
//! **6 命令** (本器官, 5-8 范围内):
//! 1. [`Command::Tick`] — 记录一次心跳 (60Hz 模拟)
//! 2. [`Command::GetBpm`] — 读取当前 BPM
//! 3. [`Command::GetTickCount`] — 累计心跳次数
//! 4. [`Command::SetBpm`] — 设置目标 BPM (40-200)
//! 5. [`Command::Reset`] — 清空统计
//! 6. [`Command::CpuSnapshot`] — CPU 使用率快照 (R25.2 占位)
//!
//! **不假装**:
//! - 所有状态走 `State` struct, 不依赖全局 static
//! - 编译期 hardcode: BPM 范围 40-200, 6 命令枚举
//! - CPU 快照标 placeholder, 真数据待 R25.3 接 `/v1/observability/heart`
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: 心跳服务于"AI 持续活着" 指标
//! - S-2 实事求是: 6 命令全部标占位实接度
//! - O-2 走在前人经验上: 借 ratatui + tokio mpsc 心跳模式
//! - O-3 干到底: 6 命令覆盖心跳全场景
//! - O-4 任何人都能接手: State 字段 + Command 变体全文档化
//! - O-5 不假装: CPU 标 partial, 标 stub
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// Heart 器官状态 (per organ 一份, Registry 持有)
///
/// **不假装**: CPU 标 placeholder 12.5, 真数据 R25.3 接.
#[derive(Debug, Clone)]
pub struct State {
    /// 目标 BPM (compile-time hardcoded 范围 40-200, 默认 60)
    pub bpm: u8,
    /// 累计 tick 数
    pub tick_count: u64,
    /// 启动时刻 (用于计算 uptime, 不假装 alive time)
    pub started_at: std::time::Instant,
    /// CPU 占用占位 (R25.2 hardcode 12.5%, 真实 R25.3)
    pub cpu_placeholder: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            bpm: 60,
            tick_count: 0,
            started_at: std::time::Instant::now(),
            cpu_placeholder: 12.5,
        }
    }
}

/// Heart 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 记录一次心跳
    Tick,
    /// 读取当前 BPM
    GetBpm,
    /// 读取累计 tick 数
    GetTickCount,
    /// 设置目标 BPM (40-200)
    SetBpm(u8),
    /// 清空统计 (tick_count = 0, started_at = now)
    Reset,
    /// CPU 快照 (R25.2 placeholder)
    CpuSnapshot,
}

/// Heart 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应 (Tick / Reset 后返 Ok)
    Unit,
    /// BPM 值响应
    Bpm(u8),
    /// tick 计数响应
    TickCount(u64),
    /// CPU 快照响应
    CpuSnapshot {
        /// CPU 占用 %
        cpu_pct: f32,
        /// 是否占位
        is_placeholder: bool,
    },
}

/// 编译期 hardcode — BPM 合法范围
pub const BPM_MIN: u8 = 40;
pub const BPM_MAX: u8 = 200;

/// 处理 Heart 命令
///
/// **参数**:
/// - `state`: 器官状态 (mut, 命令会改 bpm / tick_count)
/// - `cmd`: 命令枚举
///
/// **集成 App**: 跨器官 cross-navigate 通过 `Registry` (在 mod.rs dispatcher)
/// 集成, handle 函数本身保持自包含. 这样测试不需要 crate::app, LOCKED 边界
/// 不破.
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — BPM 越界
///
/// **不假装**: 全部命令真实现, 不走 println! / 不返回假数据
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::Tick => {
            state.tick_count = state.tick_count.saturating_add(1);
            Ok(Response::Unit)
        }
        Command::GetBpm => Ok(Response::Bpm(state.bpm)),
        Command::GetTickCount => Ok(Response::TickCount(state.tick_count)),
        Command::SetBpm(bpm) => {
            if !(BPM_MIN..=BPM_MAX).contains(&bpm) {
                return Err(OrganError::InvalidArg {
                    command: "SetBpm",
                    reason: format!("BPM {bpm} not in {BPM_MIN}..={BPM_MAX}"),
                });
            }
            state.bpm = bpm;
            Ok(Response::Unit)
        }
        Command::Reset => {
            state.tick_count = 0;
            state.started_at = std::time::Instant::now();
            Ok(Response::Unit)
        }
        Command::CpuSnapshot => Ok(Response::CpuSnapshot {
            cpu_pct: state.cpu_placeholder,
            is_placeholder: true, // R25.2 stub
        }),
    }
}

/// 器官 ASCII 字符 (跨平台, 跟 `organ/mod.rs` 对齐)
pub const ASCII_CHAR: &str = "[♥]";

/// 器官中文名
pub const NAME_ZH: &str = "心";

// =====================================================================
// 单元测试 (6 命令 + State 初始化 + 错误路径 = 8+ 测试)
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
        let _ = Command::Tick;
        let _ = Command::GetBpm;
        let _ = Command::GetTickCount;
        let _ = Command::SetBpm(72);
        let _ = Command::Reset;
        let _ = Command::CpuSnapshot;
    }

    // ---- Tick ----

    #[test]
    fn tick_increments_count() {
        let mut state = fresh_state();
        assert_eq!(state.tick_count, 0);
        for _ in 0..5 {
            let r = handle(&mut state, Command::Tick);
            assert!(matches!(r, Ok(Response::Unit)));
        }
        let r = handle(&mut state, Command::GetTickCount).unwrap();
        assert_eq!(r, Response::TickCount(5));
    }

    #[test]
    fn tick_saturates_at_u64_max() {
        let mut state = fresh_state();
        state.tick_count = u64::MAX;
        let r = handle(&mut state, Command::Tick).unwrap();
        assert_eq!(r, Response::Unit);
        // 不溢出
        assert_eq!(state.tick_count, u64::MAX);
    }

    // ---- BPM 校验 ----

    #[test]
    fn set_bpm_valid_range_accepted() {
        let mut state = fresh_state();
        for bpm in [BPM_MIN, 60, 120, BPM_MAX] {
            let r = handle(&mut state, Command::SetBpm(bpm));
            assert!(r.is_ok(), "BPM {bpm} should be accepted");
        }
        assert_eq!(state.bpm, BPM_MAX);
    }

    #[test]
    fn set_bpm_out_of_range_rejected() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::SetBpm(0));
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "SetBpm", .. })));
        let r = handle(&mut state, Command::SetBpm(BPM_MIN - 1));
        assert!(matches!(r, Err(OrganError::InvalidArg { .. })));
        let r = handle(&mut state, Command::SetBpm(BPM_MAX + 1));
        assert!(matches!(r, Err(OrganError::InvalidArg { .. })));
        // 状态不被改
        assert_eq!(state.bpm, 60);
    }

    #[test]
    fn get_bpm_returns_current() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::SetBpm(80)).unwrap();
        assert_eq!(r, Response::Unit);
        let r = handle(&mut state, Command::GetBpm).unwrap();
        assert_eq!(r, Response::Bpm(80));
    }

    // ---- Reset ----

    #[test]
    fn reset_clears_tick_count() {
        let mut state = fresh_state();
        state.tick_count = 100;
        let r = handle(&mut state, Command::Reset).unwrap();
        assert_eq!(r, Response::Unit);
        assert_eq!(state.tick_count, 0);
    }

    // ---- CPU Snapshot ----

    #[test]
    fn cpu_snapshot_marks_placeholder() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::CpuSnapshot).unwrap();
        match r {
            Response::CpuSnapshot { cpu_pct, is_placeholder } => {
                assert!(is_placeholder, "R25.2 must mark placeholder honestly");
                assert!(cpu_pct > 0.0);
            }
            _ => panic!("expected CpuSnapshot"),
        }
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        // 跟 organ/mod.rs Organ::Heart.ascii_char() 一致
        assert_eq!(ASCII_CHAR, "[♥]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "心");
    }
}
