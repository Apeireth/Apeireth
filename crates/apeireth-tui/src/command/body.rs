//! Body (体) command 模块 — 进程 / 内存 / 磁盘 / 系统资源
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::GetProcessInfo`] — 读 PID + 进程名 (编译期 hardcode 占位)
//! 2. [`Command::GetMemoryUsage`] — 读进程内存 MB
//! 3. [`Command::GetDiskUsage`] — 读磁盘占用 %
//! 4. [`Command::GetCpuSnapshot`] — 读 CPU 占用 %
//! 5. [`Command::GetThreadCount`] — 读线程数
//! 6. [`Command::GetUptime`] — 读进程 uptime (从 State.started_at)
//!
//! **不假装**:
//! - body 在 `organ/mod.rs` 标 `Readiness::Partial` — 6 命令全部标 placeholder
//! - 真实数据 R25.3 接 `sysinfo` (需先动 Cargo.toml, 留 R25.3 拍板)
//! - 所有资源值编译期 hardcode 占位
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: body 服务 ASI 载体稳定
//! - S-2 实事求是: sysinfo 未引, 6 命令全占位
//! - O-2 走在前人经验上: 借 unix top / Windows tasklist 设计
//! - O-3 干到底: 6 命令覆盖资源全场景
//! - O-4 任何人都能接手: State + 6 字段全文档化
//! - O-5 不假装: 全部用占位, 标 partial
//!
//! **8 项承诺**: 全部遵守
//!
//! **保守原则**: 不加 `sysinfo` 依赖 (会动 Cargo.toml = LOCKED), 用占位数据

use super::error::OrganError;

/// 进程 PID (编译期 hardcode, 标占位)
pub const PLACEHOLDER_PID: u32 = 12345;

/// 进程名 (编译期 hardcode)
pub const PLACEHOLDER_PROCESS: &str = "apeireth-tui";

/// CPU 占用占位 (%)
pub const PLACEHOLDER_CPU_PCT: f32 = 12.5;

/// 内存占位 (MB)
pub const PLACEHOLDER_MEM_MB: u32 = 256;

/// 磁盘占用占位 (%)
pub const PLACEHOLDER_DISK_PCT: f32 = 45.0;

/// 线程数占位
pub const PLACEHOLDER_THREADS: u32 = 8;

/// Body 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 启动时刻
    pub started_at: std::time::Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            started_at: std::time::Instant::now(),
        }
    }
}

/// Body 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 读 PID + 进程名
    GetProcessInfo,
    /// 读进程内存 MB
    GetMemoryUsage,
    /// 读磁盘占用 %
    GetDiskUsage,
    /// 读 CPU 占用 %
    GetCpuSnapshot,
    /// 读线程数
    GetThreadCount,
    /// 读进程 uptime
    GetUptime,
}

/// Body 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// 进程信息
    ProcessInfo {
        /// PID
        pid: u32,
        /// 进程名
        process: &'static str,
    },
    /// 内存 MB
    MemoryUsage(u32),
    /// 磁盘占用 %
    DiskUsage(f32),
    /// CPU 占用 %
    CpuSnapshot(f32),
    /// 线程数
    ThreadCount(u32),
    /// uptime 秒
    Uptime(u64),
}

/// 处理 Body 命令
///
/// **不假装**: 6 命令全部用 PLACEHOLDER_* 编译期 hardcode 占位
pub fn handle(state: &State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::GetProcessInfo => Ok(Response::ProcessInfo {
            pid: PLACEHOLDER_PID,
            process: PLACEHOLDER_PROCESS,
        }),
        Command::GetMemoryUsage => Ok(Response::MemoryUsage(PLACEHOLDER_MEM_MB)),
        Command::GetDiskUsage => Ok(Response::DiskUsage(PLACEHOLDER_DISK_PCT)),
        Command::GetCpuSnapshot => Ok(Response::CpuSnapshot(PLACEHOLDER_CPU_PCT)),
        Command::GetThreadCount => Ok(Response::ThreadCount(PLACEHOLDER_THREADS)),
        Command::GetUptime => {
            // 真数据 — 从 State.started_at 算
            let uptime = state.started_at.elapsed().as_secs();
            Ok(Response::Uptime(uptime))
        }
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[BODY]";

/// 器官中文名
pub const NAME_ZH: &str = "体";

// =====================================================================
// 单元测试 (6 命令 + 6 占位常量 + 真 uptime = 8+ 测试)
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
        let _ = Command::GetProcessInfo;
        let _ = Command::GetMemoryUsage;
        let _ = Command::GetDiskUsage;
        let _ = Command::GetCpuSnapshot;
        let _ = Command::GetThreadCount;
        let _ = Command::GetUptime;
    }

    // ---- 6 占位常量编译期 hardcode ----

    #[test]
    fn six_placeholders_hardcoded() {
        // 改任何一个必须改这个测试 (S-2 实事求是)
        assert_eq!(PLACEHOLDER_PID, 12345);
        assert_eq!(PLACEHOLDER_PROCESS, "apeireth-tui");
        assert_eq!(PLACEHOLDER_CPU_PCT, 12.5);
        assert_eq!(PLACEHOLDER_MEM_MB, 256);
        assert_eq!(PLACEHOLDER_DISK_PCT, 45.0);
        assert_eq!(PLACEHOLDER_THREADS, 8);
    }

    // ---- GetProcessInfo ----

    #[test]
    fn get_process_info() {
        let mut state = fresh_state();
        let r = handle(&state, Command::GetProcessInfo).unwrap();
        match r {
            Response::ProcessInfo { pid, process } => {
                assert_eq!(pid, PLACEHOLDER_PID);
                assert_eq!(process, "apeireth-tui");
            }
            _ => panic!("expected ProcessInfo"),
        }
    }

    // ---- GetMemoryUsage ----

    #[test]
    fn get_memory_usage_returns_placeholder() {
        let mut state = fresh_state();
        let r = handle(&state, Command::GetMemoryUsage).unwrap();
        assert_eq!(r, Response::MemoryUsage(PLACEHOLDER_MEM_MB));
    }

    // ---- GetDiskUsage ----

    #[test]
    fn get_disk_usage_returns_placeholder() {
        let mut state = fresh_state();
        let r = handle(&state, Command::GetDiskUsage).unwrap();
        assert_eq!(r, Response::DiskUsage(PLACEHOLDER_DISK_PCT));
    }

    // ---- GetCpuSnapshot ----

    #[test]
    fn get_cpu_snapshot_returns_placeholder() {
        let mut state = fresh_state();
        let r = handle(&state, Command::GetCpuSnapshot).unwrap();
        assert_eq!(r, Response::CpuSnapshot(PLACEHOLDER_CPU_PCT));
    }

    // ---- GetThreadCount ----

    #[test]
    fn get_thread_count_returns_placeholder() {
        let mut state = fresh_state();
        let r = handle(&state, Command::GetThreadCount).unwrap();
        assert_eq!(r, Response::ThreadCount(PLACEHOLDER_THREADS));
    }

    // ---- GetUptime (真数据) ----

    #[test]
    fn get_uptime_real_data() {
        let mut state = fresh_state();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let r = handle(&state, Command::GetUptime).unwrap();
        // 唯一真数据 — uptime 应该 >= 0 且很小 (因为才 sleep 10ms)
        match r {
            Response::Uptime(s) => assert!(s < 60, "uptime 异常大: {s}"),
            _ => panic!("expected Uptime"),
        }
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[BODY]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "体");
    }
}
