//! `apeireth-companion::job_object` — Windows Job Object 执行沙箱加固 (审计 P3#16).
//!
//! microsandbox 落地 (Windows 原生机制, 非 KVM — 审计原判 KVM 风险高, 本模块用
//! OS 自带 Job Object, 零虚拟化): 把隔离 worker 子进程放进 Job Object:
//!   - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: 宿主进程退出/崩溃 → 整个 job 进程树
//!     一并终止 (防孤儿进程 — worker 若派生后代也逃不出 job)
//!   - 进程树约束: job 内所有后代受同一生命周期管辖 (per-call 隔离不漏网)
//!
//! 与现有机制的关系 (集成而非分立): ToolBridge 已有 per-call 子进程隔离
//! (exec_worker) + 30s 超时 kill; 本模块是**加固层** — 超时 kill 管"跑太久",
//! Job Object 管"宿主没了它还活着" + "后代失控"。
//!
//! 0 假装 (诚实): 非 Windows 平台为 no-op (Job Object 是 Windows 专属; Linux
//! 用 cgroup/prctl 属未来工作, 本模块如实标注不假装)。
//!
//! 失败语义: 加固失败**不阻断执行** (加固是增强不是门) — 调用方 eprintln 记录后继续。

// 模块级豁免: Windows Job API 是 unsafe FFI; crate 级 deny(unsafe_code) 在此
// 收敛到本模块单一文件 (隔离层 = 唯一 unsafe 源, 可审查).
#![allow(unsafe_code)]

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, SetInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::OpenProcess;

    /// Windows Job Object 守卫: drop 时关闭句柄 (job 随句柄关闭而销毁).
    #[derive(Debug)]
    pub struct JobGuard {
        job: HANDLE,
    }

    impl JobGuard {
        /// 创建 job 并设置 KILL_ON_JOB_CLOSE (空安全属性).
        pub fn new() -> Result<Self, String> {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(format!("CreateJobObjectW 失败 (错误码 {})", std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)));
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation = JOBOBJECT_BASIC_LIMIT_INFORMATION {
                    LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                    ..std::mem::zeroed()
                };
                let ret = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ret == 0 {
                    CloseHandle(job);
                    return Err(format!("SetInformationJobObject 失败 (错误码 {})", std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)));
                }
                Ok(Self { job })
            }
        }

        /// 把子进程 (按 pid) 分配进 job.
        pub fn assign(&self, pid: u32) -> Result<(), String> {
            unsafe {
                let process = OpenProcess(
                    windows_sys::Win32::System::Threading::PROCESS_SET_QUOTA
                        | windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
                    0,
                    pid,
                );
                if process.is_null() {
                    return Err(format!("OpenProcess({pid}) 失败 (错误码 {})", std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)));
                }
                let ret = AssignProcessToJobObject(self.job, process);
                CloseHandle(process);
                if ret == 0 {
                    return Err(format!("AssignProcessToJobObject({pid}) 失败 (错误码 {})", std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)));
                }
                Ok(())
            }
        }
    }

    impl Drop for JobGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.job);
            }
        }
    }
}

/// 非 Windows: no-op (诚实标注, 不假装).
#[cfg(not(windows))]
mod imp {
    #[derive(Debug)]
    pub struct JobGuard;

    impl JobGuard {
        pub fn new() -> Result<Self, String> {
            // 0 装 PASS: Job Object 是 Windows 专属; 其他平台由 OS 机制覆盖 (未实现).
            Ok(Self)
        }
        pub fn assign(&self, _pid: u32) -> Result<(), String> {
            Ok(())
        }
    }
}

pub use imp::JobGuard;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_guard_constructs() {
        // 非 Windows 也是 Ok (no-op); Windows 上 CreateJobObjectW 应成功
        let guard = JobGuard::new();
        assert!(guard.is_ok(), "job 创建应成功或为非 Windows no-op: {guard:?}");
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
        guard.assign(pid).expect("assign 子进程应成功 (真实 Windows API)");
        let _ = child.kill();
        let _ = child.wait();
    }
}
