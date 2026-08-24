//! `apeireth-companion::job_object` — Windows Job Object 执行沙箱加固 (审计 P3#16, B3 参数化).
//!
//! microsandbox 落地 (Windows 原生机制, 非 KVM — 审计原判 KVM 风险高, 本模块用
//! OS 自带 Job Object, 零虚拟化): 把隔离 worker 子进程放进 Job Object:
//!   - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: 宿主进程退出/崩溃 → 整个 job 进程树
//!     一并终止 (防孤儿进程 — worker 若派生后代也逃不出 job)
//!   - 进程树约束: job 内所有后代受同一生命周期管辖 (per-call 隔离不漏网)
//!
//! **B3 资源限额** (消费 [`crate::sandbox::SandboxConfig`]):
//!   - 内存上限: `JOB_OBJECT_LIMIT_PROCESS_MEMORY` (committed 内存超限 → 系统终止进程)
//!   - CPU 限速: Job Object CPU rate control (HARD_CAP, Win8+; 设置失败降级不阻断)
//!   - CPU 时间上限: `JOB_OBJECT_LIMIT_PROCESS_TIME` (user-mode 累计, 确定性兜底)
//!   - **超限行为明确, 不静默**: 关联 IO completion port, 超限消息
//!     (JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT / END_OF_PROCESS_TIME 等) 实时 eprintln
//!     留痕并记录到 guard, 供调用方把"worker 提前退出"翻译成具体超限原因.
//!
//! 与现有机制的关系 (集成而非分立): ToolBridge 已有 per-call 子进程隔离
//! (exec_worker) + 超时 kill; 本模块是**加固层** — 超时 kill 管"跑太久",
//! Job Object 管"宿主没了它还活着" + "后代失控" + "吃内存/吃 CPU".
//!
//! 0 假装 (诚实): 非 Windows 平台为 no-op (Job Object 是 Windows 专属; Linux
//! 用 cgroup/prctl 属未来工作, 本模块如实标注不假装)。Sandboxie/landlock
//! 参数口见 [`crate::sandbox`] (trait 已备, 未接)。
//!
//! 失败语义: 加固失败**不阻断执行** (加固是增强不是门) — 调用方 eprintln 记录后继续.

// 模块级豁免: Windows Job API 是 unsafe FFI; crate 级 deny(unsafe_code) 在此
// 收敛到本模块单一文件 (隔离层 = 唯一 unsafe 源, 可审查).
#![allow(unsafe_code)]

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectAssociateCompletionPortInformation,
        JobObjectCpuRateControlInformation, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_ASSOCIATE_COMPLETION_PORT,
        JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        JOB_OBJECT_LIMIT_PROCESS_TIME,
    };
    use windows_sys::Win32::System::SystemServices::{
        JOB_OBJECT_MSG_ACTIVE_PROCESS_LIMIT, JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO,
        JOB_OBJECT_MSG_END_OF_JOB_TIME, JOB_OBJECT_MSG_END_OF_PROCESS_TIME,
        JOB_OBJECT_MSG_EXIT_PROCESS, JOB_OBJECT_MSG_JOB_MEMORY_LIMIT, JOB_OBJECT_MSG_NEW_PROCESS,
        JOB_OBJECT_MSG_NOTIFICATION_LIMIT, JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT,
    };
    use windows_sys::Win32::System::IO::{
        CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED,
    };

    use crate::sandbox::SandboxConfig;

    /// Windows Job Object 守卫: drop 时停监控线程 → 关句柄 (job 随句柄关闭而销毁).
    ///
    /// ⚠️ 生命周期: KILL_ON_JOB_CLOSE 语义下 guard **必须活到 worker 结束**
    /// (句柄关闭即终止 job 内全部进程) — tool_bridge 已按此持有.
    #[derive(Debug)]
    pub struct JobGuard {
        job: HANDLE,
        /// 超限留痕: watcher 线程写入的超限原因 (无 = 未观测到超限).
        violation: Arc<Mutex<Option<String>>>,
        /// IO completion port (超限消息监听; 无限额时为 null).
        port: HANDLE,
        /// watcher 线程停止信号.
        stop: Arc<AtomicBool>,
        watcher: Option<std::thread::JoinHandle<()>>,
    }

    // SAFETY: HANDLE 是 Windows 内核对象句柄, CloseHandle 跨线程调用安全;
    // 本 guard 的句柄引用同一内核对象, drop 可在任意线程执行 (watcher 线程
    // 内部已用 usize 移交自身句柄副本, 与此处 Send 无关). 无 Send 会让持有
    // guard 的 async block 无法跨 await (tokio Send 边界), 故显式标注.
    unsafe impl Send for JobGuard {}

    fn os_err() -> i32 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
    }

    /// job 消息 → 人类可读描述 (留痕用; 超限消息带"超限"字样, 供断言区分).
    fn msg_desc(msg: u32) -> String {
        match msg {
            JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT | JOB_OBJECT_MSG_JOB_MEMORY_LIMIT => {
                "内存上限超限 (进程树 committed 内存超出限额)".to_string()
            }
            JOB_OBJECT_MSG_END_OF_PROCESS_TIME | JOB_OBJECT_MSG_END_OF_JOB_TIME => {
                "CPU 时间上限超限 (user-mode 累计时间超出限额)".to_string()
            }
            JOB_OBJECT_MSG_ACTIVE_PROCESS_LIMIT => "活跃进程数上限超限".to_string(),
            JOB_OBJECT_MSG_NOTIFICATION_LIMIT => "通知限额触发 (内存/CPU 速率超限)".to_string(),
            JOB_OBJECT_MSG_NEW_PROCESS => "job 内新进程加入".to_string(),
            JOB_OBJECT_MSG_EXIT_PROCESS => "job 内进程退出".to_string(),
            JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO => "job 内进程已清空".to_string(),
            other => format!("job 消息 {other}"),
        }
    }

    /// 该消息是否代表资源超限 (区别于纯生命周期通知 NEW/EXIT/ACTIVE_PROCESS_ZERO).
    fn is_violation_msg(msg: u32) -> bool {
        matches!(
            msg,
            JOB_OBJECT_MSG_END_OF_PROCESS_TIME
                | JOB_OBJECT_MSG_END_OF_JOB_TIME
                | JOB_OBJECT_MSG_PROCESS_MEMORY_LIMIT
                | JOB_OBJECT_MSG_JOB_MEMORY_LIMIT
                | JOB_OBJECT_MSG_NOTIFICATION_LIMIT
                | JOB_OBJECT_MSG_ACTIVE_PROCESS_LIMIT
        )
    }

    impl JobGuard {
        /// 创建 job, 只设 KILL_ON_JOB_CLOSE (无限额; 历史行为).
        pub fn new() -> Result<Self, String> {
            Self::with_config(&SandboxConfig::default())
        }

        /// 创建 job 并按 [`SandboxConfig`] 设置资源限额.
        ///
        /// 超限行为: 系统直接终止 job 内进程 (Windows 原生语义); 本 guard 通过
        /// IO completion port 留痕 (eprintln + `violation()` 可读), **不静默**.
        /// CPU rate control (Win8+) 设置失败 → 降级 eprintln, 不阻断 (其余限额仍生效).
        pub fn with_config(cfg: &SandboxConfig) -> Result<Self, String> {
            let violation = Arc::new(Mutex::new(None));
            let stop = Arc::new(AtomicBool::new(false));
            let mut port: HANDLE = std::ptr::null_mut();
            let mut watcher = None;
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(format!("CreateJobObjectW 失败 (错误码 {})", os_err()));
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                let mut flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if let Some(mb) = cfg.memory_limit_mb {
                    flags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
                    info.ProcessMemoryLimit = (mb as usize) * 1024 * 1024;
                }
                if let Some(secs) = cfg.cpu_time_secs {
                    flags |= JOB_OBJECT_LIMIT_PROCESS_TIME;
                    // LARGE_INTEGER, 100ns 单位
                    info.BasicLimitInformation.PerProcessUserTimeLimit = (secs as i64) * 10_000_000;
                }
                info.BasicLimitInformation.LimitFlags = flags;
                let ret = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ret == 0 {
                    CloseHandle(job);
                    return Err(format!(
                        "SetInformationJobObject 失败 (错误码 {})",
                        os_err()
                    ));
                }
                // CPU 限速 (Win8+): best-effort — 失败降级不阻断 (诚实 eprintln).
                if let Some(pct) = cfg.cpu_percent {
                    let mut cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = std::mem::zeroed();
                    cpu.ControlFlags =
                        JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
                    // CpuRate: 1/100 百分点 (50% → 5000)
                    cpu.Anonymous.CpuRate = pct * 100;
                    if SetInformationJobObject(
                        job,
                        JobObjectCpuRateControlInformation,
                        (&cpu as *const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION).cast(),
                        std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
                    ) == 0
                    {
                        eprintln!(
                            "[sandbox] CPU 限速 ({pct}%) 设置失败 (错误码 {}), 降级为不限 CPU 速率 (其余限额仍生效)",
                            os_err()
                        );
                    }
                }
                // 超限留痕: 配了限额才关联 IO completion port (无限额无消息可收, 省线程).
                if cfg.has_limits() {
                    port = CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 1);
                    if port.is_null() {
                        eprintln!("[sandbox] 超限留痕端口创建失败 (错误码 {}), 降级为不留痕 (限额本身仍生效)", os_err());
                    } else {
                        let assoc = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
                            CompletionKey: job,
                            CompletionPort: port,
                        };
                        if SetInformationJobObject(
                            job,
                            JobObjectAssociateCompletionPortInformation,
                            (&assoc as *const JOBOBJECT_ASSOCIATE_COMPLETION_PORT).cast(),
                            std::mem::size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
                        ) == 0
                        {
                            eprintln!("[sandbox] 超限留痕关联失败 (错误码 {}), 降级为不留痕 (限额本身仍生效)", os_err());
                            CloseHandle(port);
                            port = std::ptr::null_mut();
                        } else {
                            let v = Arc::clone(&violation);
                            let s = Arc::clone(&stop);
                            // HANDLE 是裸指针 (!Send); 转 usize 跨线程移交, 线程内转回
                            // (该句柄仅监控线程内 GetQueuedCompletionStatus 使用, 移交安全)
                            let p = port as usize;
                            watcher = Some(std::thread::spawn(move || {
                                let mut code: u32 = 0;
                                let mut key: usize = 0;
                                let mut ovl: *mut OVERLAPPED = std::ptr::null_mut();
                                loop {
                                    if s.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    let ok = GetQueuedCompletionStatus(
                                        p as HANDLE,
                                        &mut code,
                                        &mut key,
                                        &mut ovl,
                                        100,
                                    );
                                    if ok == 0 {
                                        let e = std::io::Error::last_os_error()
                                            .raw_os_error()
                                            .unwrap_or(0)
                                            as u32;
                                        if e != WAIT_TIMEOUT && !s.load(Ordering::Relaxed) {
                                            eprintln!(
                                                "[sandbox] 超限监听异常 (错误码 {e}), 继续监听"
                                            );
                                        }
                                        continue;
                                    }
                                    let desc = msg_desc(code);
                                    eprintln!("[sandbox] Job Object 消息: {desc}");
                                    // 只留痕超限原因, 且保留首个 (退出/新建类
                                    // 生命周期通知不覆盖超限留痕; 首个超限即终止因).
                                    if is_violation_msg(code) {
                                        if let Ok(mut g) = v.lock() {
                                            if g.is_none() {
                                                *g = Some(desc);
                                            }
                                        }
                                    }
                                }
                            }));
                        }
                    }
                }
                Ok(Self {
                    job,
                    violation,
                    port,
                    stop,
                    watcher,
                })
            }
        }

        /// 把子进程 (按 pid) 分配进 job.
        pub fn assign(&self, pid: u32) -> Result<(), String> {
            unsafe {
                let process = windows_sys::Win32::System::Threading::OpenProcess(
                    windows_sys::Win32::System::Threading::PROCESS_SET_QUOTA
                        | windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
                    0,
                    pid,
                );
                if process.is_null() {
                    return Err(format!("OpenProcess({pid}) 失败 (错误码 {})", os_err()));
                }
                let ret = AssignProcessToJobObject(self.job, process);
                CloseHandle(process);
                if ret == 0 {
                    return Err(format!(
                        "AssignProcessToJobObject({pid}) 失败 (错误码 {})",
                        os_err()
                    ));
                }
                Ok(())
            }
        }

        /// 观测到的超限原因 (留痕查询; None = 未超限或无限额).
        pub fn violation(&self) -> Option<String> {
            self.violation.lock().ok().and_then(|g| g.clone())
        }
    }

    impl Drop for JobGuard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.watcher.take() {
                let _ = h.join(); // watcher 最多 100ms 一轮, 快速退出
            }
            unsafe {
                if !self.port.is_null() {
                    CloseHandle(self.port);
                }
                CloseHandle(self.job); // KILL_ON_JOB_CLOSE: job 内进程随之终止
            }
        }
    }
}

/// 非 Windows: no-op (诚实标注, 不假装).
#[cfg(not(windows))]
mod imp {
    use crate::sandbox::SandboxConfig;

    #[derive(Debug)]
    pub struct JobGuard;

    impl JobGuard {
        pub fn new() -> Result<Self, String> {
            Self::with_config(&SandboxConfig::default())
        }
        pub fn with_config(_cfg: &SandboxConfig) -> Result<Self, String> {
            // 0 装 PASS: Job Object 是 Windows 专属; 其他平台由 OS 机制覆盖 (未实现).
            Ok(Self)
        }
        pub fn assign(&self, _pid: u32) -> Result<(), String> {
            Ok(())
        }
        pub fn violation(&self) -> Option<String> {
            None
        }
    }
}

pub use imp::JobGuard;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::SandboxConfig;

    #[test]
    fn job_guard_constructs() {
        // 非 Windows 也是 Ok (no-op); Windows 上 CreateJobObjectW 应成功
        let guard = JobGuard::new();
        assert!(
            guard.is_ok(),
            "job 创建应成功或为非 Windows no-op: {guard:?}"
        );
    }

    #[test]
    fn job_guard_with_config_constructs() {
        let cfg = SandboxConfig {
            memory_limit_mb: Some(512),
            cpu_percent: Some(50),
            cpu_time_secs: Some(10),
            timeout_secs: 30,
            ..Default::default()
        };
        let guard = JobGuard::with_config(&cfg);
        assert!(guard.is_ok(), "带限额 job 创建应成功: {guard:?}");
        assert!(
            guard.unwrap().violation().is_none(),
            "未跑进程不应有超限记录"
        );
    }

    #[cfg(windows)]
    #[test]
    fn job_guard_assigns_child_process() {
        // 注意: 不能 assign 当前测试进程 — KILL_ON_JOB_CLOSE 会在句柄关闭时
        // 终止 job 内全部进程 (测试进程自杀)。用真实子进程验证 assign 路径。
        let guard = JobGuard::new().expect("job 创建");
        let mut child = std::process::Command::new("cmd")
            .args(["/c", "ping -n 3 127.0.0.1 >nul"])
            .spawn()
            .expect("spawn 子进程");
        let pid = child.id();
        guard
            .assign(pid)
            .expect("assign 子进程应成功 (真实 Windows API)");
        let _ = child.kill();
        let _ = child.wait();
    }

    #[cfg(windows)]
    #[test]
    fn cpu_time_limit_kills_child_and_leaves_trace() {
        // 超限终止 + 留痕: CPU 时间限 2s, 子进程死循环吃满 CPU → 约 2s 后被系统终止
        let cfg = SandboxConfig {
            cpu_time_secs: Some(2),
            ..SandboxConfig::default()
        };
        let guard = JobGuard::with_config(&cfg).expect("job 创建");
        let start = std::time::Instant::now();
        // 直接 spawn powershell (不经 cmd /c — cmd 嵌套引号会把含 () {} ; 的
        // 脚本解析坏, 子进程"正常退出"导致测试误报; Rust Command 的 args 会正确
        // 转义为带引号的单参数).
        let mut child = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$s=[Diagnostics.Stopwatch]::StartNew(); while($s.Elapsed.TotalSeconds -lt 60){}",
            ])
            .spawn()
            .expect("spawn 子进程");
        guard.assign(child.id()).expect("assign");
        let status = child.wait().expect("wait");
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(50),
            "子进程应被 CPU 时间限额提前终止 (实际 {elapsed:?})"
        );
        assert!(!status.success(), "超限终止不应是正常退出");
        // 留痕: watcher 线程应已记录超限原因 (100ms 轮询, 等一小会儿)
        std::thread::sleep(std::time::Duration::from_millis(500));
        let v = guard.violation();
        assert!(
            v.as_deref().unwrap_or("").contains("CPU 时间上限"),
            "应留痕 CPU 超限原因, 实际: {v:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn memory_limit_denies_allocation_and_leaves_trace() {
        // 超限 + 留痕: 内存限 300MB, 子进程尝试申请 800MB → Windows 硬内存限制
        // 的语义是**拒绝超限 commit (分配失败/OOM)**, 不是终止进程 (与 CPU 时间
        // 限制"系统强制终止"不同 — 0 装 PASS, 不假装进程被杀). 脚本 try/catch:
        // 分配成功 → exit 7 (不应发生); 分配被拒 → exit 42 (限制生效的诚实信号).
        let cfg = SandboxConfig {
            memory_limit_mb: Some(300),
            ..SandboxConfig::default()
        };
        let guard = JobGuard::with_config(&cfg).expect("job 创建");
        let start = std::time::Instant::now();
        // 同上: 不经 cmd, 直接 spawn powershell (Rust 转义保证单参数传递).
        let mut child = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "try { $b = [byte[]]::new(800MB); exit 7 } catch { exit 42 }",
            ])
            .spawn()
            .expect("spawn 子进程");
        guard.assign(child.id()).expect("assign");
        let status = child.wait().expect("wait");
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(25),
            "分配被拒应立即退出 (实际 {elapsed:?})"
        );
        assert_eq!(
            status.code(),
            Some(42),
            "800MB 申请应被 300MB 限额拒绝 (exit 42); 若 exit 7 说明限额未生效"
        );
        std::thread::sleep(std::time::Duration::from_millis(500));
        let v = guard.violation();
        assert!(
            v.as_deref().unwrap_or("").contains("内存上限"),
            "应留痕内存超限原因, 实际: {v:?}"
        );
    }
}
