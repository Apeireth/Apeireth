//! OS 物理沙箱 spawn 基准: JobObject/AppContainer 边界初始化 + 进程隔离。
//!
//! 口径 (与 README「OS Sandbox Spawn」行同义):
//! - `os-sandbox-spawn`: `ProcessExecutor::execute` 沙箱执行一个最小子进程
//!   (`cmd.exe /D /S /C "exit 0"`) 全程 = JobObject 创建 + 限额设置 + 沙箱
//!   进程创建 (文件/网络隔离 Enforced 要求, Windows 走 AppContainer 路径) +
//!   等待回收 + 输出捕获。子进程自身启动成本 (CreateProcess + cmd.exe 初始化)
//!   **包含在内** —— 这是"起一个受控进程"的真实成本口径。
//! - `os-sandbox-jobobject`: 仅 JobObject 创建 + 限额设置 (不含子进程), 单独
//!   呈现边界初始化的开销下界。
//!
//! 平台边界 (如实标注): 文件/网络隔离 Enforced 在 Windows 经 AppContainer;
//! 平台不满足时被测执行器按契约拒绝执行 (fail-closed), 本基准随之报错退出。

use std::hint::black_box;
use std::time::Instant;

use apeireth_tools_canonical::process::{
    IsolationCapability, IsolationRequirement, ProcessExecutor, ProcessRequest,
};

use crate::support::{cleanup_dir, temp_dir, BenchConfig, Outcome, Target};

/// 沙箱执行的子进程命令 (最小、确定性)。
#[cfg(windows)]
const CHILD_ARG: &str = "/D /S /C \"exit 0\"";

/// 运行 OS 沙箱 spawn 基准, 返回 [沙箱 spawn 全程, JobObject 初始化] 两个结果项。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let ws = temp_dir("sandbox-ws");
    let sandboxed = IsolationRequirement::new()
        .require_enforced(IsolationCapability::FilesystemIsolation)
        .require_enforced(IsolationCapability::NetworkIsolation);

    let samples = cfg.samples(50);
    let warmup = cfg.samples(3);

    // ---- 沙箱 spawn 全程 ----
    #[cfg(windows)]
    let spawn_samples = {
        let executor = ProcessExecutor::new();
        let mut samples_ns = Vec::with_capacity(samples);
        for i in 0..(warmup + samples) {
            let request = ProcessRequest::new("cmd.exe")
                .with_raw_arg(CHILD_ARG)
                .with_working_directory(ws.as_path())
                .with_isolation(sandboxed.clone());
            let start = Instant::now();
            let result = executor
                .execute(&request)
                .expect("沙箱 spawn 必须成功 (隔离墙存在 != 起不来)");
            let ns = start.elapsed().as_nanos() as f64;
            assert!(result.exit_code().is_some(), "子进程必须正常退出");
            if i >= warmup {
                samples_ns.push(ns);
            }
            black_box(result);
        }
        samples_ns
    };
    #[cfg(not(windows))]
    let spawn_samples: Vec<f64> = Vec::new();

    // ---- 对照基线: 同款 execute 不带隔离要求 (子进程启动+等待成本) ----
    #[cfg(windows)]
    let plain_samples = {
        let executor = ProcessExecutor::new();
        let mut samples_ns = Vec::with_capacity(samples);
        for i in 0..(warmup + samples) {
            let request = ProcessRequest::new("cmd.exe")
                .with_raw_arg(CHILD_ARG)
                .with_working_directory(ws.as_path());
            let start = Instant::now();
            let result = executor.execute(&request).expect("对照 spawn");
            let ns = start.elapsed().as_nanos() as f64;
            if i >= warmup {
                samples_ns.push(ns);
            }
            black_box(result);
        }
        samples_ns
    };
    #[cfg(not(windows))]
    let plain_samples: Vec<f64> = Vec::new();

    // ---- JobObject/限额初始化 (不含子进程) ----
    let job_samples = job_object_samples(cfg, warmup, samples);

    cleanup_dir(&ws);

    let mut outcomes = Vec::new();
    #[cfg(windows)]
    {
        let mean = |v: &[f64]| {
            if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<f64>() / v.len() as f64 / 1_000_000.0
            }
        };
        let delta_note = format!(
            "对照: 同款无隔离 execute 均值 {:.4} ms, 沙箱增量 ≈ {:.4} ms (均值口径); execute 全程含等待退出+输出捕获 (被测边界无公开 spawn-不等待 API, 如实以最小子进程全程计)",
            mean(&plain_samples),
            mean(&spawn_samples) - mean(&plain_samples)
        );
        outcomes.push(Outcome::from_ns(
            "os-sandbox-spawn",
            "OS 物理沙箱 spawn (JobObject + AppContainer 初始化 + 进程隔离)",
            "ProcessExecutor 沙箱执行 cmd.exe exit 0 全程: 边界初始化 + 受控进程创建 + 回收 + 输出捕获",
            "ms",
            1,
            warmup,
            spawn_samples,
            Some(Target {
                label: "< 15.0 ms".to_string(),
                value: 15.0,
            }),
            &format!(
                "含子进程自身启动成本 (CreateProcess + cmd.exe 初始化); 文件/网络隔离 Enforced 要求 (AppContainer 路径, 平台不满足即 fail-closed 拒绝); {delta_note}"
            ),
        ));
        outcomes.push(Outcome::from_ns(
            "os-sandbox-plain-spawn",
            "对照基线-无隔离进程 spawn (同款 execute)",
            "ProcessExecutor 执行 cmd.exe exit 0 (无隔离要求) 全程",
            "ms",
            1,
            warmup,
            plain_samples,
            None,
            "仅作对照基线 (子进程启动 + 等待退出 + 输出捕获), 不是目标行; 沙箱行与本行之差 ≈ 受控边界增量开销",
        ));
    }
    #[cfg(not(windows))]
    {
        let _ = (&spawn_samples, &plain_samples, &sandboxed);
    }
    outcomes.push(Outcome::from_ns(
        "os-sandbox-jobobject",
        "OS 沙箱边界初始化 (JobObject 创建 + 限额设置)",
        "JobObject::create(ProcessLimits::default) 单次 (不含子进程)",
        "us",
        1,
        warmup,
        job_samples,
        None,
        "边界初始化开销下界; 无 README 目标行, 仅记录",
    ));
    outcomes
}

/// JobObject 创建 + 限额设置计时 (平台分支)。
#[cfg(windows)]
fn job_object_samples(cfg: &BenchConfig, warmup: usize, samples: usize) -> Vec<f64> {
    use apeireth_tools_canonical::process::windows::JobObject;
    use apeireth_tools_canonical::process::ProcessLimits;

    let _ = cfg;
    for _ in 0..warmup {
        black_box(JobObject::create(&ProcessLimits::default()).expect("创建 JobObject"));
    }
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let job = JobObject::create(&ProcessLimits::default()).expect("创建 JobObject");
        out.push(start.elapsed().as_nanos() as f64);
        black_box(job);
    }
    out
}

#[cfg(not(windows))]
fn job_object_samples(_cfg: &BenchConfig, _warmup: usize, _samples: usize) -> Vec<f64> {
    Vec::new()
}
