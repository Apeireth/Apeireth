//! 启动期/沙箱 spawn 分段剖析 (诊断用基准, 不计入 README 目标行)。
//!
//! 两个子命令 (经 `scripts/run-benchmarks.ps1 -Bench <key>` 或
//! `cargo run --release --locked -p apeireth-bench-harness -- <key>` 复跑):
//! - `profile-cold-start`: 冷启动分段计时 —— 记忆存储打开 / 会话存储打开 /
//!   模块组装 / Runtime build 四段首尾相接 + 合计, 另附两个存储的"已迁移库
//!   重开"参照 (把建库 DDL 与迁移簿记成本拆开看)。口径与 [`crate::cold_start`]
//!   完全同实现 (同一 [`crate::assembly::assemble_production`])。
//! - `profile-sandbox`: 沙箱 spawn 分解 —— 子进程地板 (纯 std spawn + 等待 +
//!   输出读取, 无执行器) / 无隔离 execute 全程 / 沙箱 execute 全程 / AppContainer
//!   身份获取 / 目录授权 / 平台能力查询 (探针已缓存)。
//!
//! 全部结果仅记录 (无目标线), 供剖析报告归因; 目标行判定仍以 README 表口径为准。

use std::time::Instant;

use apeireth_memory::SqliteMemoryStore;
use apeireth_runtime_assembly::SqliteSessionStore;

use crate::assembly::{assemble_production_profiled, tokio_runtime};
use crate::support::{cleanup_dir, temp_dir, BenchConfig, Outcome};

/// 运行冷启动分段剖析, 返回逐段结果项。
pub fn run_cold_start(cfg: &BenchConfig) -> Vec<Outcome> {
    let rt = tokio_runtime();
    let samples = cfg.samples(30);
    let warmup = cfg.samples(2);

    for _ in 0..warmup {
        let dir = temp_dir("prof-cold-warm");
        let (runtime, _) = rt.block_on(assemble_production_profiled(&dir));
        drop(runtime);
        cleanup_dir(&dir);
    }

    let mut memory_open = Vec::with_capacity(samples);
    let mut session_open = Vec::with_capacity(samples);
    let mut modules_build = Vec::with_capacity(samples);
    let mut runtime_build = Vec::with_capacity(samples);
    let mut total = Vec::with_capacity(samples);
    for _ in 0..samples {
        let dir = temp_dir("prof-cold");
        let (runtime, seg) = rt.block_on(assemble_production_profiled(&dir));
        memory_open.push(seg.memory_open_ns);
        session_open.push(seg.session_open_ns);
        modules_build.push(seg.modules_build_ns);
        runtime_build.push(seg.runtime_build_ns);
        total.push(seg.total_ns());
        drop(runtime);
        cleanup_dir(&dir);
    }

    // 内存库参照: 同一套迁移集打在纯内存库上 (无文件创建/WAL 文件 I/O),
    // 与文件库首开之差 ≈ 文件系统侧成本。
    let mut memory_memdb = Vec::with_capacity(samples);
    let mut session_memdb = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let tick = Instant::now();
        drop(SqliteMemoryStore::open_in_memory().expect("内存记忆存储"));
        let memory_ns = tick.elapsed().as_nanos() as f64;
        let tick = Instant::now();
        drop(
            rt.block_on(SqliteSessionStore::in_memory())
                .expect("内存会话存储"),
        );
        let session_ns = tick.elapsed().as_nanos() as f64;
        if i >= warmup {
            memory_memdb.push(memory_ns);
            session_memdb.push(session_ns);
        }
    }

    // 已迁移库重开参照: 首开 (建库 DDL + 迁移) 之后立刻重开 (迁移簿记为空跑),
    // 两者之差 ≈ 首建 schema 的 DDL 执行成本。
    let mut memory_reopen = Vec::with_capacity(samples);
    let mut session_reopen = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let dir = temp_dir("prof-cold-reopen");
        let memory_path = dir.join("memory.sqlite3");
        let session_path = dir.join("session.sqlite3");
        drop(SqliteMemoryStore::open(&memory_path).expect("首开记忆存储"));
        rt.block_on(SqliteSessionStore::open(&session_path))
            .expect("首开会话存储");
        let tick = Instant::now();
        drop(SqliteMemoryStore::open(&memory_path).expect("重开记忆存储"));
        let memory_ns = tick.elapsed().as_nanos() as f64;
        let tick = Instant::now();
        drop(
            rt.block_on(SqliteSessionStore::open(&session_path))
                .expect("重开会话存储"),
        );
        let session_ns = tick.elapsed().as_nanos() as f64;
        if i >= warmup {
            memory_reopen.push(memory_ns);
            session_reopen.push(session_ns);
        }
        cleanup_dir(&dir);
    }

    let segment = |key: &str, title: &str, op: &str, values: Vec<f64>, note: &str| {
        Outcome::from_ns(key, title, op, "ms", 1, warmup, values, None, note)
    };

    vec![
        segment(
            "profile-cold-memory-open",
            "冷启动分段-记忆存储打开",
            "SqliteMemoryStore::open 单次 (建库/开连接 + 记忆 schema 迁移)",
            memory_open,
            "剖析分段 (仅记录, 无目标线); 与 cold-start 同一实现路径",
        ),
        segment(
            "profile-cold-session-open",
            "冷启动分段-会话存储打开",
            "SqliteSessionStore::open 单次 (连接池建立 + 存储迁移)",
            session_open,
            "剖析分段 (仅记录, 无目标线); 与 cold-start 同一实现路径",
        ),
        segment(
            "profile-cold-modules-build",
            "冷启动分段-模块集组装",
            "ProductionModules::build 单次 (纯内存构造)",
            modules_build,
            "剖析分段 (仅记录, 无目标线); 与 cold-start 同一实现路径",
        ),
        segment(
            "profile-cold-runtime-build",
            "冷启动分段-Runtime build 到 ready",
            "register_into + build().await 单次 (含就绪校验)",
            runtime_build,
            "剖析分段 (仅记录, 无目标线); 与 cold-start 同一实现路径",
        ),
        segment(
            "profile-cold-total",
            "冷启动分段-全程合计",
            "四段首尾相接合计 (≈ cold-start 口径, 仅多纳秒级计时器调用)",
            total,
            "剖析分段 (仅记录, 无目标线); 用于校验分段之和 ≈ 全程",
        ),
        segment(
            "profile-cold-memory-memdb",
            "冷启动参照-记忆存储内存库打开",
            "SqliteMemoryStore::open_in_memory 单次 (同迁移集, 无文件 I/O)",
            memory_memdb,
            "参照 (仅记录); 与 profile-cold-memory-open 之差 ≈ 文件系统侧成本",
        ),
        segment(
            "profile-cold-session-memdb",
            "冷启动参照-会话存储内存库打开",
            "SqliteSessionStore::in_memory 单次 (同迁移集 + 连接池, 无文件 I/O)",
            session_memdb,
            "参照 (仅记录); 与 profile-cold-session-open 之差 ≈ 文件系统侧成本",
        ),
        segment(
            "profile-cold-memory-reopen",
            "冷启动参照-记忆存储重开",
            "同库二次 SqliteMemoryStore::open 单次 (迁移已应用, 空跑簿记)",
            memory_reopen,
            "参照 (仅记录); 与 profile-cold-memory-open 之差 ≈ 首建 schema DDL 成本",
        ),
        segment(
            "profile-cold-session-reopen",
            "冷启动参照-会话存储重开",
            "同库二次 SqliteSessionStore::open 单次 (迁移已应用, 空跑簿记)",
            session_reopen,
            "参照 (仅记录); 与 profile-cold-session-open 之差 ≈ 首建 schema DDL 成本",
        ),
    ]
}

/// 运行沙箱 spawn 分解剖析 (仅 Windows; 其余平台返回空)。
#[cfg(windows)]
pub fn run_sandbox(cfg: &BenchConfig) -> Vec<Outcome> {
    use std::process::Stdio;

    use apeireth_tools_canonical::process::appcontainer::AppContainerSandbox;
    use apeireth_tools_canonical::process::{
        current_platform_capabilities, IsolationCapability, IsolationRequirement, ProcessExecutor,
        ProcessRequest,
    };

    let ws = temp_dir("prof-sandbox-ws");
    let sandboxed = IsolationRequirement::new()
        .require_enforced(IsolationCapability::FilesystemIsolation)
        .require_enforced(IsolationCapability::NetworkIsolation);
    let samples = cfg.samples(50);
    let warmup = cfg.samples(3);

    // ---- 子进程地板: 纯 std spawn + 等待 + 输出读取 (无执行器/无 JobObject/无沙箱) ----
    let mut child_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let mut command = std::process::Command::new("cmd.exe");
        command
            .args(["/D", "/S", "/C", "exit 0"])
            .current_dir(ws.as_path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let start = Instant::now();
        let mut child = command.spawn().expect("子进程创建");
        let status = child.wait().expect("等待子进程退出");
        let mut sink = Vec::new();
        use std::io::Read;
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_end(&mut sink);
        }
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_end(&mut sink);
        }
        let ns = start.elapsed().as_nanos() as f64;
        assert!(status.success(), "子进程必须正常退出");
        if i >= warmup {
            child_samples.push(ns);
        }
    }

    // ---- 无隔离 execute 全程 (对照; 与 os-sandbox-plain-spawn 同请求口径) ----
    let plain_executor = ProcessExecutor::new();
    let mut plain_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let request = ProcessRequest::new("cmd.exe")
            .with_raw_arg("/D /S /C \"exit 0\"")
            .with_working_directory(ws.as_path());
        let start = Instant::now();
        let result = plain_executor.execute(&request).expect("无隔离 execute");
        let ns = start.elapsed().as_nanos() as f64;
        if i >= warmup {
            plain_samples.push(ns);
        }
        std::hint::black_box(result);
    }

    // ---- 沙箱 execute 全程 (与 os-sandbox-spawn 同请求口径) ----
    let sandbox_executor = ProcessExecutor::new();
    let mut sandbox_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let request = ProcessRequest::new("cmd.exe")
            .with_raw_arg("/D /S /C \"exit 0\"")
            .with_working_directory(ws.as_path())
            .with_isolation(sandboxed.clone());
        let start = Instant::now();
        let result = sandbox_executor
            .execute(&request)
            .expect("沙箱 spawn 必须成功");
        let ns = start.elapsed().as_nanos() as f64;
        assert!(result.exit_code().is_some(), "子进程必须正常退出");
        if i >= warmup {
            sandbox_samples.push(ns);
        }
        std::hint::black_box(result);
    }

    // ---- AppContainer 身份获取 (profile/SID, 每次 execute 都走的路径) ----
    let mut acquire_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let start = Instant::now();
        let sandbox = AppContainerSandbox::acquire().expect("获取沙箱身份");
        let ns = start.elapsed().as_nanos() as f64;
        drop(sandbox);
        if i >= warmup {
            acquire_samples.push(ns);
        }
    }

    // ---- 工作区目录授权 (ACL 合并写, 每次沙箱 execute 都走的路径) ----
    let grant_sandbox = AppContainerSandbox::acquire().expect("获取沙箱身份");
    let mut grant_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let start = Instant::now();
        grant_sandbox
            .grant_directory(ws.as_path())
            .expect("目录授权");
        let ns = start.elapsed().as_nanos() as f64;
        if i >= warmup {
            grant_samples.push(ns);
        }
    }
    drop(grant_sandbox);

    // ---- 平台能力查询 (探针首跑后缓存, 每次 execute 都走的路径) ----
    let mut capability_samples = Vec::with_capacity(samples);
    for i in 0..(warmup + samples) {
        let start = Instant::now();
        let caps = current_platform_capabilities();
        let ns = start.elapsed().as_nanos() as f64;
        std::hint::black_box(caps);
        if i >= warmup {
            capability_samples.push(ns);
        }
    }

    cleanup_dir(&ws);

    let segment = |key: &str, title: &str, op: &str, unit: &str, values: Vec<f64>, note: &str| {
        Outcome::from_ns(key, title, op, unit, 1, warmup, values, None, note)
    };

    vec![
        segment(
            "profile-sandbox-child-only",
            "沙箱分解-子进程地板",
            "cmd.exe exit 0 纯 std spawn + 等待退出 + 输出读取 (无执行器/无 JobObject/无沙箱)",
            "ms",
            child_samples,
            "剖析分段 (仅记录, 无目标线); 子进程自身启动成本下界",
        ),
        segment(
            "profile-sandbox-plain-execute",
            "沙箱分解-无隔离 execute 全程",
            "ProcessExecutor 执行 cmd.exe exit 0 (无隔离要求) 全程",
            "ms",
            plain_samples,
            "剖析分段 (仅记录); 与子进程地板之差 = 执行器 + 受控创建/回收开销",
        ),
        segment(
            "profile-sandbox-sandboxed-execute",
            "沙箱分解-沙箱 execute 全程",
            "ProcessExecutor 沙箱执行 cmd.exe exit 0 全程 (文件/网络隔离 Enforced)",
            "ms",
            sandbox_samples,
            "剖析分段 (仅记录); 与无隔离之差 = 受控边界增量",
        ),
        segment(
            "profile-sandbox-acquire",
            "沙箱分解-AppContainer 身份获取",
            "AppContainerSandbox::acquire 单次 (profile 校验 + SID 派生)",
            "us",
            acquire_samples,
            "剖析分段 (仅记录); 每次沙箱 execute 都支付",
        ),
        segment(
            "profile-sandbox-grant",
            "沙箱分解-工作区目录授权",
            "grant_directory 单次 (ACL 读-合并-写)",
            "us",
            grant_samples,
            "剖析分段 (仅记录); 每次沙箱 execute 都支付",
        ),
        segment(
            "profile-sandbox-capabilities",
            "沙箱分解-平台能力查询",
            "current_platform_capabilities 单次 (实测探针, 首跑后缓存)",
            "us",
            capability_samples,
            "剖析分段 (仅记录); 每次 execute 都支付",
        ),
    ]
}

/// 非 Windows 平台: 沙箱分解无对应边界, 返回空 (如实呈现)。
#[cfg(not(windows))]
pub fn run_sandbox(_cfg: &BenchConfig) -> Vec<Outcome> {
    Vec::new()
}
