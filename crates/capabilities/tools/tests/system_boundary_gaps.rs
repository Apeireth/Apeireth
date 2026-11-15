//! 系统工具边界缺口补齐 (B1–B6)。
//!
//! 覆盖: 真实子进程的取消与可信路径对抗 / 异步路径的 Scope 与端口预算负例 /
//! 无 reactor 时的同步兼容 / 历史同步入口与新异步入口的契约 / `elapsed_ms` 口径 /
//! `invoke_with_context` 的 native context 传播。
//!
//! 全部用既有 sandbox-test-child 的固定模式驱动真实子进程 (不新增依赖、不联网、
//! 不扫公网: 只碰本机回环)。

use std::ffi::OsString;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_core::kernel::{AgentId, PluginId};
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::tool_execution::{native_execution_context, scope_native_execution};
use apeireth_plugin::{
    ToolAuthorization, ToolCaller, ToolCancellation, ToolCapability, ToolExecutionContext,
};
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::exposure_audit::listener_surface::{
    probe_netstat, resolve_trusted_tool_path, trusted_netstat_path, trusted_system_root,
    validate_trusted_path_shape,
};
use apeireth_tools_canonical::exposure_audit::{
    parse_netstat_snapshot, ExposureAuditConfig, ExposureAuditTool,
};
use apeireth_tools_canonical::process::{
    ProcessError, ProcessExecutor, ProcessLimits, ProcessRequest,
};
use apeireth_tools_canonical::recon::net::{
    scan_controlled, scan_controlled_blocking, NetError, ScanOptions, BANNER_PROBE_BUDGET,
};
use apeireth_tools_canonical::recon::{scan_ports, Scope};
use serde_json::json;

/// 定位测试用固定模式子进程 (与 process_executor.rs 同法: 该 bin 标了
/// `test = false`, `CARGO_BIN_EXE_*` 不一定被注入)。
fn helper() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_sandbox-test-child") {
        return PathBuf::from(path);
    }
    let mut path = std::env::current_exe().expect("current exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(if cfg!(windows) {
        "sandbox-test-child.exe"
    } else {
        "sandbox-test-child"
    });
    path
}

fn context() -> ToolExecutionContext {
    ToolExecutionContext {
        agent: AgentId::default(),
        principal: None,
        session: None,
        turn: None,
        call_id: "boundary-gap-fixture".into(),
        caller: ToolCaller::Host,
        module: PluginId::new("test.system-boundary-gaps").unwrap(),
        module_instance: None,
        internal_review: None,
        message_support: false,
        message_route: None,
        delegated_tools: None,
        delegation: None,
        tool_scope: None,
        authorization: ToolAuthorization::default(),
        cancellation: ToolCancellation::default(),
        deadline: Instant::now() + Duration::from_secs(10),
        trace: None,
        module_tasks: ModuleTaskScope::new(),
        resource_lease: None,
    }
}

/// 只接受连接、不发任何数据 ⇒ 被动 banner 读必然耗满预算。
struct SilentListener {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl SilentListener {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = std::thread::spawn(move || {
            let mut connections: Vec<TcpStream> = Vec::new();
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => connections.push(stream),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
        });
        Self {
            port,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for SilentListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn loopback_scope() -> Scope {
    Scope {
        name: "loopback-fixture".into(),
        authorized: true,
        paths: vec![],
        domains: vec![],
        cidrs: vec!["127.0.0.1/32".into()],
    }
}

fn scan_options() -> ScanOptions {
    ScanOptions {
        timeout: Duration::from_millis(500),
        max_ports: 16,
        banner_probe: true,
    }
}

/// 子进程回报的工作目录是否就是期望的那个 (允许规范化前的原样形态)。
fn cwd_matches(printed: &str, expected: &Path) -> bool {
    let Some(path) = printed.strip_prefix("CWD:") else {
        return false;
    };
    let printed = Path::new(path);
    printed == expected
        || std::fs::canonicalize(printed).ok() == std::fs::canonicalize(expected).ok()
}

// ---------------------------------------------------------------- B1 真实进程边界

/// B1: 取消必须**终止真实子进程**并如实报 `Cancelled` (不是等它自己跑完)。
#[test]
fn real_child_process_is_cancelled_before_it_reports() {
    let helper = helper();
    assert!(
        helper.is_file(),
        "helper binary missing: {}",
        helper.display()
    );

    let cancellation = ToolCancellation::default();
    let signal = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        signal.cancel();
    });
    let request = ProcessRequest::new(&helper)
        .with_args(["sleep", "30"])
        .with_cancellation(cancellation)
        .with_limits(ProcessLimits {
            // 明显大于子进程自身时长: 结束原因只能是取消, 不能是超时。
            max_runtime: Duration::from_secs(60),
            ..ProcessLimits::default()
        });
    let started = Instant::now();
    let outcome = ProcessExecutor::new().execute(&request);
    let elapsed = started.elapsed();
    canceller.join().expect("canceller thread");

    match outcome {
        Err(ProcessError::Cancelled) => {}
        Err(other) => panic!("取消必须报 Cancelled, 实际: {other:?}"),
        Ok(result) => panic!(
            "取消必须终止子进程, 实际成功返回 (exit={:?}, timed_out={})",
            result.exit_code(),
            result.timed_out()
        ),
    }
    assert!(
        elapsed < Duration::from_secs(10),
        "取消没有及时终止真实子进程: {elapsed:?}"
    );
}

/// B1: 可信路径的**形状闸门**必须拒掉相对路径 / `.` / `..` / 空格 / 引号 /
/// shell 元字符 / ADS 分隔符 / 控制字符, 且拒绝理由要能判真。
#[test]
fn trusted_path_gate_rejects_adversarial_shapes() {
    let cases: Vec<(String, &str)> = if cfg!(windows) {
        vec![
            (r"netstat.exe".into(), "绝对路径"),
            (
                r"C:\Windows\System32\..\SysWOW64\netstat.exe".into(),
                "路径组件",
            ),
            (r"C:\Windows\System32\.\netstat.exe".into(), "路径组件"),
            (r"C:\Windows\System32\net stat.exe".into(), "文件名"),
            (r#"C:\Windows\System32\net"stat.exe"#.into(), "文件名"),
            (r"C:\Windows\System32\net;stat.exe".into(), "文件名"),
            (r"C:\Windows\System32\netstat.exe:evil".into(), "文件名"),
            ("C:\\Windows\\System32\\netstat.exe\n".into(), "控制字符"),
        ]
    } else {
        vec![
            ("netstat".into(), "绝对路径"),
            ("/usr/bin/../bin/netstat".into(), "路径组件"),
            ("/usr/bin/./netstat".into(), "路径组件"),
            ("/usr/bin/net stat".into(), "文件名"),
            ("/usr/bin/net\"stat".into(), "文件名"),
            ("/usr/bin/net;stat".into(), "文件名"),
            ("/usr/bin/netstat:evil".into(), "文件名"),
            ("/usr/bin/netstat\n".into(), "控制字符"),
        ]
    };
    for (candidate, expected_reason) in cases {
        let error =
            validate_trusted_path_shape(Path::new(&candidate)).expect_err("对抗形状必须被拒");
        assert!(
            error.contains(expected_reason),
            "拒绝理由不对: {candidate:?} => {error} (期望含 {expected_reason:?})"
        );
    }

    let accepted = if cfg!(windows) {
        Path::new(r"C:\Windows\System32\netstat.exe")
    } else {
        Path::new("/usr/bin/netstat")
    };
    assert!(
        validate_trusted_path_shape(accepted).is_ok(),
        "平台正常的系统工具路径必须被接受: {accepted:?}"
    );
}

/// B1: 可信工具必须**直接位于信任根内** (根外、根内的子目录都要拒), 且拒绝
/// 理由是归属而不是形状。
#[test]
fn trusted_tool_path_must_live_directly_in_the_system_root() {
    let (root, outside, nested) = if cfg!(windows) {
        (
            PathBuf::from(r"C:\Windows\System32"),
            PathBuf::from(r"C:\Windows\SysWOW64\netstat.exe"),
            PathBuf::from(r"C:\Windows\System32\sub\netstat.exe"),
        )
    } else {
        (
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin/netstat"),
            PathBuf::from("/usr/bin/sub/netstat"),
        )
    };
    for candidate in [&outside, &nested] {
        let error =
            resolve_trusted_tool_path(&root, candidate).expect_err("系统根外的候选必须被拒");
        assert!(
            error.contains("信任根"),
            "拒绝理由必须是归属而不是形状: {} => {error}",
            candidate.display()
        );
    }
}

/// B1: 生产取用的可信路径要么过闸门且直接位于信任根内, 要么采集显式失败
/// (不许旁路、不许猜路径)。
#[test]
fn production_trusted_tool_path_passes_the_gate_or_fails_closed() {
    match trusted_netstat_path() {
        Some(path) => {
            let root = trusted_system_root().expect("可信工具存在 ⇒ 信任根必须存在");
            assert!(
                resolve_trusted_tool_path(&root, &path).is_ok(),
                "生产可信路径必须过闸门: {}",
                path.display()
            );
            assert_eq!(
                path.parent(),
                Some(root.as_path()),
                "生产可信路径必须直接位于信任根内"
            );
            assert!(path.is_file());
        }
        None => {
            let reason = probe_netstat().expect_err("缺少可信系统工具时采集必须失败");
            assert!(reason.contains("受控采集不可用"), "{reason}");
        }
    }
}

// ------------------------------------------------------------- B2 异步路径的负例

/// B2: 异步路径的 Scope 与端口预算负例 —— 超限必须被**拒**, 且如实报数
/// (不得静默放宽、不得截断后照常返回)。
#[tokio::test(flavor = "current_thread")]
async fn async_scan_refuses_out_of_scope_target_and_over_budget_ports() {
    let scope = Scope {
        name: "budget-fixture".into(),
        authorized: true,
        paths: vec![],
        domains: vec![],
        cidrs: vec!["10.0.0.0/8".into()],
    };
    let options = ScanOptions::default();
    match scan_controlled(IpAddr::from([127, 0, 0, 1]), &[80], &scope, &options).await {
        Err(NetError::Scope(_)) => {}
        other => panic!("越界目标必须被拒, 实际: {other:?}"),
    }

    let scope = loopback_scope();
    let options = ScanOptions {
        max_ports: 8,
        ..ScanOptions::default()
    };
    let ports: Vec<u16> = (1..=9).collect();
    match scan_controlled(IpAddr::from([127, 0, 0, 1]), &ports, &scope, &options).await {
        Err(NetError::BudgetExceeded { requested, max }) => {
            assert_eq!(requested, 9, "预算拒绝必须如实报请求数");
            assert_eq!(max, 8, "预算拒绝必须如实报上限");
        }
        other => panic!("超预算必须被拒, 实际: {other:?}"),
    }
}

// ------------------------------------------------------------- B3 无 reactor 兼容

/// B3: 不依赖 reactor 的同步受控入口必须能在普通线程上跑完 (不 panic)。
#[test]
fn scan_controlled_blocking_works_without_a_reactor() {
    let fixture = SilentListener::new();
    let scope = loopback_scope();
    let options = scan_options();
    let report = scan_controlled_blocking(
        IpAddr::from([127, 0, 0, 1]),
        &[fixture.port],
        &scope,
        &options,
    )
    .expect("同步受控入口必须能在没有 reactor 的线程上跑完");
    assert_eq!(report.ports.len(), 1);
    assert_eq!(report.ports[0].state, "open");
    assert_eq!(report.open, 1);
}

/// B3: 异步入口在没有 Tokio 运行时时**不得 panic**, 必须回落到同步受控入口。
#[test]
fn async_scan_entry_falls_back_without_a_reactor_instead_of_panicking() {
    let fixture = SilentListener::new();
    let scope = loopback_scope();
    let options = scan_options();
    // futures 的 block_on: 没有 Tokio runtime ⇒ 没有 reactor、没有 timer/IO 驱动。
    let report = futures::executor::block_on(async {
        scan_controlled(
            IpAddr::from([127, 0, 0, 1]),
            &[fixture.port],
            &scope,
            &options,
        )
        .await
    })
    .expect("无 reactor 的调用不得 panic, 必须回落到同步受控入口");
    assert_eq!(report.ports[0].state, "open");
    assert_eq!(report.open, 1);
}

// ------------------------------------------------------- B4 历史同步入口的契约

/// B4: 历史同步入口 (`recon::scan_ports`) 的**原行为**与新受控路径的契约:
/// 观测口径一致, 但历史入口**不**承接调用级期限与取消 (受控入口承接)。
#[tokio::test(flavor = "current_thread")]
async fn legacy_sync_scan_entry_keeps_its_original_behaviour() {
    let fixture = SilentListener::new();
    let scope = loopback_scope();
    let options = scan_options();

    let legacy = scan_ports(
        IpAddr::from([127, 0, 0, 1]),
        &[fixture.port],
        &scope,
        &options,
    )
    .expect("历史同步入口必须可用");
    assert_eq!(legacy.ports.len(), 1);
    assert_eq!(legacy.ports[0].state, "open");
    assert_eq!(legacy.open, 1);

    let mut context = context();
    context.deadline = Instant::now() - Duration::from_millis(1);
    let legacy_under_expired = scope_native_execution(&context, async {
        scan_ports(
            IpAddr::from([127, 0, 0, 1]),
            &[fixture.port],
            &scope,
            &options,
        )
    })
    .await;
    assert!(
        legacy_under_expired.is_ok(),
        "历史同步入口的原行为 = 不承接调用级期限"
    );

    let controlled_under_expired = scope_native_execution(&context, async {
        scan_controlled_blocking(
            IpAddr::from([127, 0, 0, 1]),
            &[fixture.port],
            &scope,
            &options,
        )
    })
    .await;
    match controlled_under_expired {
        Err(NetError::DeadlineExceeded) => {}
        other => panic!("受控入口必须承接调用级期限, 实际: {other:?}"),
    }
}

// ------------------------------------------------------------- B5 elapsed_ms 口径

/// B5: `elapsed_ms` 的口径 = connect 发起 → connect 返回, **不含** banner 读取等待。
#[test]
fn elapsed_ms_measures_connect_only_and_excludes_the_banner_wait() {
    let fixture = SilentListener::new();
    let scope = loopback_scope();
    let options = scan_options();
    let started = Instant::now();
    let report = scan_controlled_blocking(
        IpAddr::from([127, 0, 0, 1]),
        &[fixture.port],
        &scope,
        &options,
    )
    .expect("scan");
    let total_ms = started.elapsed().as_millis() as u64;
    let finding = &report.ports[0];
    assert_eq!(finding.state, "open");
    assert!(finding.banner.is_none(), "静默监听不应有 banner");
    assert!(
        finding.elapsed_ms < BANNER_PROBE_BUDGET.as_millis() as u64,
        "elapsed_ms 混入了 banner 等待: {}ms",
        finding.elapsed_ms
    );
    // 反证: banner 等待确实发生 (总耗时 ≥ banner 预算) ⇒ 两个口径确实不同。
    assert!(
        total_ms >= BANNER_PROBE_BUDGET.as_millis() as u64,
        "banner 等待未发生, 断言失去意义: {total_ms}ms"
    );
}

/// B5: 异步路径的 `elapsed_ms` 与同步路径**同口径** (都只量 connect)。
#[tokio::test(flavor = "current_thread")]
async fn async_elapsed_ms_uses_the_same_connect_only_accounting() {
    let fixture = SilentListener::new();
    let scope = loopback_scope();
    let options = scan_options();
    let started = Instant::now();
    let report = scan_controlled(
        IpAddr::from([127, 0, 0, 1]),
        &[fixture.port],
        &scope,
        &options,
    )
    .await
    .expect("scan");
    let total_ms = started.elapsed().as_millis() as u64;
    let finding = &report.ports[0];
    assert_eq!(finding.state, "open");
    assert!(
        finding.elapsed_ms < BANNER_PROBE_BUDGET.as_millis() as u64,
        "异步路径的 elapsed_ms 混入了 banner 等待: {}ms",
        finding.elapsed_ms
    );
    assert!(
        total_ms >= BANNER_PROBE_BUDGET.as_millis() as u64,
        "banner 等待未发生, 断言失去意义: {total_ms}ms"
    );
}

// ------------------------------------------------- B6 native context 的传播验证

/// B6: `invoke_with_context` 路径上, native context 与子进程的 env / 工作目录 /
/// 可信可执行路径必须真的到位 (不是只在测试壳里自说自话)。
#[tokio::test(flavor = "multi_thread")]
async fn invoke_with_context_propagates_env_cwd_and_trusted_path_to_the_child() {
    let workspace = tempfile::tempdir().expect("temp dir");
    // 可信路径 fixture: 把固定模式子进程复制到**唯一**临时目录 ⇒ 该目录里只有
    // 这一份副本, 子进程回报的工作目录就证明跑的是这份副本 (而不是 PATH 上的
    // 别的同名程序)。
    let trusted = workspace.path().join(if cfg!(windows) {
        "trusted-child.exe"
    } else {
        "trusted-child"
    });
    std::fs::copy(helper(), &trusted).expect("copy helper into the unique directory");
    assert_eq!(trusted.parent(), Some(workspace.path()));

    let observed = Arc::new(Mutex::new(Vec::<String>::new()));
    let probe = {
        let observed = Arc::clone(&observed);
        let trusted = trusted.clone();
        let work_dir = workspace.path().to_path_buf();
        Arc::new(move || {
            // ① 调用级 native context 真的到达了工具内部。
            let native = native_execution_context().expect("native context 未到达工具内部");
            assert!(
                native.tasks.active_tasks() > 0,
                "工具内部缺少 native 任务租约"
            );
            // ② 环境变量真的传到了子进程。
            let env_run = ProcessExecutor::new()
                .execute(
                    &ProcessRequest::new(&trusted)
                        .with_explicit_env(vec![(
                            OsString::from("BOUNDARY_ENV"),
                            OsString::from("boundary-value"),
                        )])
                        .with_working_directory(&work_dir)
                        .with_args(["print-env", "BOUNDARY_ENV"]),
                )
                .expect("env 子进程");
            // ③ 工作目录 (⇒ 也就是那份可信副本) 真的传到了子进程。
            let cwd_run = ProcessExecutor::new()
                .execute(
                    &ProcessRequest::new(&trusted)
                        .with_clear_env()
                        .with_working_directory(&work_dir)
                        .with_args(["print-cwd"]),
                )
                .expect("cwd 子进程");
            let mut lines = observed.lock().expect("observed lock");
            lines.push(String::from_utf8_lossy(&env_run.stdout).trim().to_string());
            lines.push(String::from_utf8_lossy(&cwd_run.stdout).trim().to_string());
            Ok(parse_netstat_snapshot(""))
        })
    };

    let config = ExposureAuditConfig::new(workspace.path()).with_listeners(probe);
    let tool = ExposureAuditTool::new(config);
    let call = ToolCall {
        id: "boundary-context".into(),
        name: "exposure_audit".into(),
        arguments: json!({}),
    };
    let context = context();
    let result =
        scope_native_execution(&context, tool.invoke_with_context(&call, None, &context)).await;
    assert!(result.is_ok(), "{}", result.render());

    let lines = observed.lock().expect("observed lock").clone();
    assert_eq!(lines.len(), 2, "探测口必须跑过一次: {lines:?}");
    assert_eq!(
        lines[0], "ENV:boundary-value",
        "环境变量没传到子进程: {:?}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("CWD:"),
        "子进程没回报工作目录: {:?}",
        lines[1]
    );
    assert!(
        cwd_matches(&lines[1], workspace.path()),
        "工作目录没传到子进程: {:?} (期望 {})",
        lines[1],
        workspace.path().display()
    );
}
