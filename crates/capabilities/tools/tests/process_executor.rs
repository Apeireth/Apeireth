//! Real-process tests for the canonical process execution boundary.
//!
//! These tests prove enforcement against a real child process. No mock process
//! executor is used anywhere in this file.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use apeireth_tools_canonical::process::{
    current_platform_capabilities, EnforcementLevel, EnvironmentSpec, IsolationCapability,
    IsolationRequirement, ProcessError, ProcessExecutor, ProcessLimits, ProcessRequest,
    ProcessResult,
};

/// 子进程测试用的 helper 可执行文件路径。
///
/// **为什么不用 `env!("CARGO_BIN_EXE_sandbox-test-child")`**：那个编译期变量只在
/// cargo 为集成测试**构建了同名 bin** 时才注入。本仓的 `[[bin]] sandbox-test-child`
/// 标了 `test = false / bench = false / doc = false` ⇒ 在**部分 cargo 版本**
/// （本地钉的 1.97.1 实测：变量不存在 ⇒ `env!` 直接编译失败，整个
/// `cargo build --workspace --tests` 步骤红）里不注入 ⇒ 用 `env!` 会让
/// **整个 workspace 的测试编译**挂掉（CI 跑的是 `--workspace --tests`）。
///
/// 改用「`option_env!` + 运行时同目录回退」：两种 cargo 版本都能编过，
/// 且路径指向同一个真身（cargo 把 bin 与测试可执行文件放在同一个 target 目录）。
fn helper() -> std::path::PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_sandbox-test-child") {
        return std::path::PathBuf::from(path);
    }
    // cargo 的布局：测试可执行文件在 `<target>/<profile>/deps/`，bin 在
    // `<target>/<profile>/` ⇒ 从 current_exe 往上退**一级**再放 bin 名。
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps/
    dir.pop(); // <profile>/
    dir.push(if cfg!(windows) {
        "sandbox-test-child.exe"
    } else {
        "sandbox-test-child"
    });
    dir
}

fn execute(request: &ProcessRequest) -> Result<ProcessResult, ProcessError> {
    ProcessExecutor::new().execute(request)
}

fn request_for(mode: &str, arg: &str) -> ProcessRequest {
    ProcessRequest::new(helper())
        .with_args([mode, arg])
        .with_limits(ProcessLimits::default())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn successful_child_execution_captures_stdout() {
    let request = request_for("echo-args", "hello");
    let result = execute(&request).unwrap();
    assert!(result.success());
    assert_eq!(text(&result.stdout), "ARG:hello\n");
    assert!(!result.stdout_truncated);
    assert!(result.stderr.is_empty());
}

#[test]
fn structured_args_are_preserved_with_unicode_and_spaces() {
    let request = ProcessRequest::new(helper())
        .with_args(["echo-args", "héllo 世界", "sp ace", "🙂"])
        .with_limits(ProcessLimits::default());
    let result = execute(&request).unwrap();
    assert!(result.success());
    assert_eq!(text(&result.stdout), "ARG:héllo 世界\nARG:sp ace\nARG:🙂\n");
}

#[test]
fn working_directory_is_explicit_not_ambient() {
    let dir = tempfile::Builder::new()
        .prefix("apeireth m2b cwd ")
        .tempdir()
        .unwrap();
    let request = ProcessRequest::new(helper())
        .with_args(["print-cwd"])
        .with_working_directory(dir.path())
        .with_limits(ProcessLimits::default());

    let result = execute(&request).unwrap();
    assert!(result.success());
    let stdout = text(&result.stdout);
    let expected = format!("CWD:{}", dir.path().display());
    let canonical = dir
        .path()
        .canonicalize()
        .unwrap_or_else(|_| dir.path().to_path_buf());
    let expected_canonical = format!("CWD:{}", canonical.display());
    assert!(
        stdout.contains(&expected) || stdout.contains(&expected_canonical),
        "{stdout}"
    );
}

#[test]
fn stderr_is_captured_separately() {
    let request = ProcessRequest::new(helper())
        .with_args(["print", "stderr", "64"])
        .with_limits(ProcessLimits::default());
    let result = execute(&request).unwrap();
    assert!(result.success());
    assert!(result.stdout.is_empty());
    assert_eq!(result.stderr.len(), 64);
}

#[test]
fn nonzero_exit_is_a_result_not_an_executor_error() {
    let request = request_for("exit-code", "7");
    let result = execute(&request).unwrap();
    assert!(!result.success());
    assert_eq!(result.exit_code(), Some(7));
}

#[test]
fn spawn_failure_is_an_executor_error() {
    let request = ProcessRequest::new("__apeireth_no_such_executable__")
        .with_limits(ProcessLimits::default());
    let error = execute(&request).unwrap_err();
    assert!(
        matches!(error, ProcessError::SpawnFailed { .. }),
        "expected SpawnFailed, got {error:?}"
    );
}

#[test]
fn timeout_terminates_the_child() {
    let mut limits = ProcessLimits::default();
    limits.max_runtime = Duration::from_secs(1);
    let request = ProcessRequest::new(helper())
        .with_args(["sleep", "10"])
        .with_limits(limits);

    let start = Instant::now();
    let result = execute(&request).unwrap();
    assert!(
        result.timed_out(),
        "expected TimedOut, got {:?}",
        result.termination
    );
    assert!(
        start.elapsed() < Duration::from_secs(8),
        "timeout test hung"
    );
}

#[test]
fn stdout_limit_truncates_and_reports() {
    let mut limits = ProcessLimits::default();
    limits.max_stdout_bytes = 1024;
    let request = ProcessRequest::new(helper())
        .with_args(["print", "stdout", "100000"])
        .with_limits(limits);

    let result = execute(&request).unwrap();
    assert!(result.success());
    assert!(result.stdout_truncated);
    assert_eq!(result.stdout.len(), 1024);
}

#[test]
fn stderr_limit_truncates_and_reports() {
    let mut limits = ProcessLimits::default();
    limits.max_stderr_bytes = 1024;
    let request = ProcessRequest::new(helper())
        .with_args(["print", "stderr", "100000"])
        .with_limits(limits);

    let result = execute(&request).unwrap();
    assert!(result.success());
    assert!(result.stderr_truncated);
    assert_eq!(result.stderr.len(), 1024);
}

#[test]
fn unbounded_usize_max_output_limits_are_refused() {
    // L 组 (2026-09-24 审计): usize::MAX 的"近无界"输出上限必须 fail-closed
    // (读取侧 take(max+1) 会回绕/panic), 调用方必须给真实的大数字。
    for target in ["stdout", "stderr"] {
        let mut limits = ProcessLimits::default();
        if target == "stdout" {
            limits.max_stdout_bytes = usize::MAX;
        } else {
            limits.max_stderr_bytes = usize::MAX;
        }
        let request = ProcessRequest::new(helper())
            .with_args(["print", target, "10"])
            .with_limits(limits);

        let error = execute(&request).unwrap_err();
        assert!(
            matches!(error, ProcessError::InvalidConfiguration(ref m) if m.contains("usize::MAX")),
            "{target}: expected InvalidConfiguration, got {error:?}"
        );
    }
}

#[test]
fn large_stdout_and_stderr_do_not_deadlock() {
    let mut limits = ProcessLimits::default();
    limits.max_stdout_bytes = 300_000;
    limits.max_stderr_bytes = 300_000;
    let request = ProcessRequest::new(helper())
        .with_args(["print-both", "200000", "200000"])
        .with_limits(limits);

    let result = execute(&request).unwrap();
    assert!(result.success(), "{}", text(&result.stderr));
    assert_eq!(result.stdout.len(), 200000);
    assert_eq!(result.stderr.len(), 200000);
}

#[test]
fn environment_clearing_denies_ambient_secrets() {
    let request = ProcessRequest::new(helper())
        .with_args(["print-env", "APEIRETH_M2B_TEST_ENV"])
        .with_environment(EnvironmentSpec::Clear)
        .with_limits(ProcessLimits::default());

    let result = execute(&request).unwrap();
    assert!(result.success());
    assert_eq!(text(&result.stdout), "ENV_MISSING\n");
}

#[test]
fn environment_explicit_mode_sets_only_requested_vars() {
    let request = ProcessRequest::new(helper())
        .with_args(["print-env", "APEIRETH_M2B_TEST_ENV"])
        .with_explicit_env(vec![(
            OsString::from("APEIRETH_M2B_TEST_ENV"),
            OsString::from("explicit-value"),
        )])
        .with_limits(ProcessLimits::default());

    let result = execute(&request).unwrap();
    assert!(result.success());
    assert_eq!(text(&result.stdout), "ENV:explicit-value\n");
}

#[test]
fn unsupported_network_requirement_fails_closed_before_child_starts() {
    let capabilities = current_platform_capabilities();
    if capabilities.network_isolation == EnforcementLevel::Enforced {
        eprintln!("network isolation is enforced on this platform; skipping fail-closed test");
        return;
    }

    let marker = tempfile::Builder::new()
        .prefix("apeireth m2b failclosed ")
        .tempdir()
        .unwrap()
        .path()
        .join("child-started.marker");
    let request = ProcessRequest::new(helper())
        .with_args(["write-file", marker.to_str().unwrap()])
        .with_isolation(IsolationRequirement::new().require(
            IsolationCapability::NetworkIsolation,
            EnforcementLevel::Enforced,
        ))
        .with_limits(ProcessLimits::default());

    let error = execute(&request).unwrap_err();
    assert!(
        matches!(error, ProcessError::IsolationRequirementUnsatisfied { .. }),
        "expected IsolationRequirementUnsatisfied, got {error:?}"
    );
    assert!(
        !marker.exists(),
        "child must never start when isolation requirements are unsatisfied"
    );
}

#[test]
fn unsupported_optional_limit_fails_before_child_starts() {
    let capabilities = current_platform_capabilities();
    if capabilities.cpu_limit != EnforcementLevel::Unsupported {
        eprintln!("cpu limit is supported on this platform; skipping unsupported-limit test");
        return;
    }

    let marker = tempfile::Builder::new()
        .prefix("apeireth m2b limitfail ")
        .tempdir()
        .unwrap()
        .path()
        .join("child-started.marker");
    let mut limits = ProcessLimits::default();
    limits.max_cpu_seconds = Some(1);
    let request = ProcessRequest::new(helper())
        .with_args(["write-file", marker.to_str().unwrap()])
        .with_limits(limits);

    let error = execute(&request).unwrap_err();
    assert!(
        matches!(error, ProcessError::UnsupportedLimit(_)),
        "expected UnsupportedLimit, got {error:?}"
    );
    assert!(
        !marker.exists(),
        "child must never start for unsupported limits"
    );
}

#[test]
fn result_enforcement_reports_platform_capabilities() {
    let result = execute(&request_for("print-pid", "")).unwrap();
    assert!(result.success(), "{}", text(&result.stderr));
    assert!(
        result.enforcement.capabilities.structured_spawn == EnforcementLevel::Enforced,
        "structured spawn must be enforced on every backend"
    );
    assert!(
        result.enforcement.capabilities.stdout_limit == EnforcementLevel::Enforced,
        "stdout limit must be enforced on every backend"
    );
}

/// P3-9 审计 (2026-10-31): 默认请求是 `EnvironmentSpec::Inherit` ——
/// `apply_request_to_command` 在 Inherit 分支**不做** `env_clear` ⇒ 子进程继承父
/// 环境 (含 provider 的 API key)。三平台后端却都无条件宣称
/// `EnvironmentIsolation=Enforced` ⇒ 执行记录谎报「本次实际生效」的隔离属性。
#[test]
fn default_request_does_not_claim_environment_isolation() {
    let result = execute(&request_for("print-pid", "")).unwrap();
    assert!(result.success(), "{}", text(&result.stderr));
    assert!(
        result.enforcement.capabilities.environment_isolation != EnforcementLevel::Enforced,
        "an inherited environment must not be reported as isolated; got {:?}",
        result.enforcement.capabilities.environment_isolation
    );

    // 对照组: 显式环境 (从空环境起步 = env_clear) 时, 平台能隔离环境 ⇒ 记录才允许
    // 报 Enforced。
    if current_platform_capabilities().environment_isolation == EnforcementLevel::Enforced {
        let explicit = execute(
            &ProcessRequest::new(helper())
                .with_args(["print-pid"])
                .with_explicit_env(Vec::new())
                .with_limits(ProcessLimits::default()),
        )
        .unwrap();
        assert_eq!(
            explicit.enforcement.capabilities.environment_isolation,
            EnforcementLevel::Enforced,
            "an explicit (env-cleared) environment must report EnvironmentIsolation=Enforced"
        );
    }
}

#[cfg(windows)]
mod windows_tests {
    use super::*;
    use apeireth_tools_canonical::process::windows::JobObject;
    use std::os::windows::io::AsRawHandle;
    use std::process::{Command, Stdio};
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    #[test]
    fn process_executor_attaches_child_to_a_real_job_object() {
        let request = request_for("check-job", "");
        let result = execute(&request).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        assert_eq!(text(&result.stdout), "IN_JOB\n");
    }

    #[test]
    fn kill_on_job_close_terminates_a_running_child() {
        let job = JobObject::create(&ProcessLimits::default()).unwrap();
        let mut child = Command::new(helper())
            .arg("sleep")
            .arg("60")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .unwrap();

        job.assign(child.as_raw_handle().cast()).unwrap();

        drop(job);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("child survived kill-on-job-close for 5 seconds");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn timeout_terminates_the_whole_job_tree() {
        let mut limits = ProcessLimits::default();
        limits.max_runtime = Duration::from_secs(2);
        let request = ProcessRequest::new(helper())
            .with_args(["spawn-child", "sleep", "60"])
            .with_limits(limits);

        let result = execute(&request).unwrap();
        assert!(
            result.timed_out(),
            "expected TimedOut, got {:?}",
            result.termination
        );

        let stdout = text(&result.stdout);
        let grandchild_pid = stdout
            .lines()
            .find_map(|line| line.strip_prefix("SPAWN_OK "))
            .map(|pid| pid.parse::<u32>().unwrap())
            .unwrap_or_else(|| panic!("expected SPAWN_OK <pid> in child stdout, got {stdout:?}"));

        assert_process_terminated(grandchild_pid);
    }

    #[test]
    fn active_process_limit_blocks_extra_child_creation() {
        let mut limits = ProcessLimits::default();
        limits.max_active_processes = Some(1);
        let request = ProcessRequest::new(helper())
            .with_args(["spawn-child", "sleep", "5"])
            .with_limits(limits);

        let result = execute(&request).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        let stdout = text(&result.stdout);
        assert!(
            stdout.contains("SPAWN_FAIL"),
            "expected SPAWN_FAIL with ActiveProcessLimit=1, got {stdout:?}"
        );
    }

    #[test]
    fn cpu_time_limit_is_advertised_enforced() {
        let capabilities = current_platform_capabilities();
        assert_eq!(
            capabilities.cpu_limit,
            EnforcementLevel::Enforced,
            "Windows backend must advertise JOB_OBJECT_LIMIT_PROCESS_TIME as Enforced"
        );
    }

    #[test]
    fn cpu_time_limit_terminates_a_cpu_bound_child() {
        let mut limits = ProcessLimits::default();
        limits.max_cpu_seconds = Some(1);
        limits.max_runtime = Duration::from_secs(15);
        let request = ProcessRequest::new(helper())
            .with_args(["burn-cpu", "30"])
            .with_limits(limits);

        let started = Instant::now();
        let result = execute(&request).unwrap();
        let elapsed = started.elapsed();
        assert!(
            !result.timed_out(),
            "CPU-time Job Object limit must kill the child itself, not the wall-clock supervisor; got {:?}",
            result.termination
        );
        assert!(
            !result.success(),
            "CPU-time Job Object limit must terminate a 30s burn; got {:?}",
            result.termination
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "1s CPU-time limit should fire well under 10s wall clock, elapsed={elapsed:?}"
        );
        assert!(
            !text(&result.stdout).contains("BURN_DONE"),
            "child must not finish the 30s burn under a 1s CPU-time limit, stdout={:?}",
            text(&result.stdout)
        );
    }

    #[test]
    fn process_memory_limit_rejects_oversized_allocation() {
        let mut limits = ProcessLimits::default();
        limits.max_process_memory_bytes = Some(128 * 1024 * 1024);
        let request = ProcessRequest::new(helper())
            .with_args(["allocate", "256"])
            .with_limits(limits);

        let result = execute(&request).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        let stdout = text(&result.stdout);
        assert!(
            stdout.contains("ALLOC_FAIL"),
            "expected ALLOC_FAIL with ProcessMemoryLimit=128MiB, got {stdout:?}"
        );
    }

    #[test]
    fn privilege_reduction_requirement_is_enforced_or_fails_closed() {
        let capabilities = current_platform_capabilities();
        let requirement = IsolationRequirement::new().require(
            IsolationCapability::PrivilegeReduction,
            EnforcementLevel::Enforced,
        );
        let request = ProcessRequest::new(helper())
            .with_args(["platform-security-info"])
            .with_isolation(requirement)
            .with_limits(ProcessLimits::default());

        match capabilities.privilege_reduction {
            EnforcementLevel::Enforced => {
                let result = execute(&request).unwrap();
                assert!(result.success(), "{}", text(&result.stderr));
                assert_eq!(text(&result.stdout), "TOKEN_RESTRICTED\n");
            }
            _ => {
                let error = execute(&request).unwrap_err();
                assert!(
                    matches!(error, ProcessError::IsolationRequirementUnsatisfied { .. }),
                    "expected IsolationRequirementUnsatisfied, got {error:?}"
                );
            }
        }
    }

    /// L 组 (2026-09-24 审计): 受限 token 路径不支持 raw_arg 逐字命令列尾。
    /// 旧实现静默丢弃尾巴 (调用方以为生效实则丢失); 现在必须是明确的配置
    /// 错误 (平台不支持 PrivilegeReduction 时照旧先报隔离未满足)。
    #[test]
    fn raw_arg_on_restricted_token_path_is_a_configuration_error() {
        let capabilities = current_platform_capabilities();
        let requirement = IsolationRequirement::new().require(
            IsolationCapability::PrivilegeReduction,
            EnforcementLevel::Enforced,
        );
        let request = ProcessRequest::new("cmd.exe")
            .with_raw_arg(r#"/D /S /C "echo hi""#)
            .with_isolation(requirement)
            .with_limits(ProcessLimits::default());

        match capabilities.privilege_reduction {
            EnforcementLevel::Enforced => {
                let error = execute(&request).unwrap_err();
                assert!(
                    matches!(error, ProcessError::InvalidConfiguration(ref m) if m.contains("raw_arg")),
                    "expected InvalidConfiguration about raw_arg, got {error:?}"
                );
            }
            _ => {
                let error = execute(&request).unwrap_err();
                assert!(
                    matches!(error, ProcessError::IsolationRequirementUnsatisfied { .. }),
                    "expected IsolationRequirementUnsatisfied, got {error:?}"
                );
            }
        }
    }

    /// P3-10 审计 (2026-10-31): `StructuredSpawn` 的口径是「可执行文件 + argv 以
    /// 结构化数组传递, 永不是命令串」。Windows 的 raw_arg 逐字尾 (shell 的
    /// `cmd /D /S /C "<script>"`) 恰恰是命令串拼接 ⇒ 该次执行的结构化生成只有
    /// Partial, 不得沿用平台的无条件 Enforced。
    #[test]
    fn raw_arg_request_reports_partial_structured_spawn() {
        let request = ProcessRequest::new("cmd.exe")
            .with_raw_arg(r#"/D /S /C "echo hi""#)
            .with_limits(ProcessLimits::default());

        let result = execute(&request).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        assert_eq!(
            result.enforcement.capabilities.structured_spawn,
            EnforcementLevel::Partial,
            "a verbatim command-line tail must not be reported as a structured spawn"
        );
    }

    fn assert_process_terminated(pid: u32) {
        unsafe {
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                // The process no longer exists; that is a terminated process.
                return;
            }
            let wait = WaitForSingleObject(handle, 5000);
            CloseHandle(handle);
            assert_eq!(
                wait, WAIT_OBJECT_0,
                "descendant process {pid} should have been terminated by Job Object timeout"
            );
        }
    }

    /// P3-11 审计 (2026-10-31): AppContainer profile 名原先是**固定常量** ⇒ SID 由
    /// 名字**确定性**派生 ⇒ 全机全账号同一 SID, 且任何进程都能同名抢占。
    /// 现在名字掺了用户标签 + 随机后缀 ⇒ SID 不再确定。
    #[test]
    fn appcontainer_profile_name_is_not_the_fixed_constant() {
        use apeireth_tools_canonical::process::appcontainer::AppContainerSandbox;

        let Ok(sandbox) = AppContainerSandbox::acquire() else {
            eprintln!("SKIP: 本机不支持 AppContainer profile (环境不支持)");
            return;
        };
        let name = sandbox.profile_name();
        assert!(
            name.starts_with("Apeireth.Shell.Sandbox."),
            "profile name must keep the prefix and add scope: {name}"
        );
        assert!(
            name.len() > "Apeireth.Shell.Sandbox".len(),
            "profile name must not be the bare fixed constant: {name}"
        );
        // 同一进程内两次 acquire 必须拿到同一个 profile (幂等) ⇒ 名字稳定。
        let Ok(second) = AppContainerSandbox::acquire() else {
            return;
        };
        assert_eq!(
            second.profile_name(),
            name,
            "the profile name must be stable within one process"
        );
    }
}

#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;

    #[test]
    fn timeout_terminates_the_whole_process_group_tree() {
        let mut limits = ProcessLimits::default();
        limits.max_runtime = Duration::from_secs(2);
        let request = ProcessRequest::new(helper())
            .with_args(["spawn-child", "sleep", "60"])
            .with_limits(limits);

        let result = execute(&request).unwrap();
        assert!(
            result.timed_out(),
            "expected TimedOut, got {:?}",
            result.termination
        );

        let stdout = text(&result.stdout);
        let grandchild_pid = stdout
            .lines()
            .find_map(|line| line.strip_prefix("SPAWN_OK "))
            .map(|pid| pid.parse::<i32>().unwrap())
            .unwrap_or_else(|| panic!("expected SPAWN_OK <pid> in child stdout, got {stdout:?}"));

        assert_process_gone(grandchild_pid);
    }

    #[test]
    fn privilege_reduction_requirement_sets_no_new_privs() {
        let request = ProcessRequest::new(helper())
            .with_args(["print-no-new-privs"])
            .with_isolation(IsolationRequirement::new().require(
                IsolationCapability::PrivilegeReduction,
                EnforcementLevel::Partial,
            ))
            .with_limits(ProcessLimits::default());

        let result = execute(&request).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        assert_eq!(text(&result.stdout), "NO_NEW_PRIVS:1\n");
    }

    #[test]
    fn process_count_limit_is_reported_partial_not_enforced() {
        let capabilities = current_platform_capabilities();
        assert_eq!(
            capabilities.process_count_limit,
            EnforcementLevel::Partial,
            "Linux backend advertises RLIMIT_NPROC as PARTIAL only"
        );
    }

    fn assert_process_gone(pid: i32) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let ret = unsafe { libc::kill(pid, 0) };
            if ret == -1 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                if errno == libc::ESRCH {
                    return;
                }
            }
            if Instant::now() >= deadline {
                panic!("descendant process {pid} survived process-group termination");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// P3-11 审计 (2026-10-31): AppContainer profile 名原先是**固定常量** ⇒ SID 由名字
/// **确定性**派生 ⇒ ① 全机全账号同一 SID (共享盘上的跨账号授权互相命中);
/// ② 任何进程都能**同名抢占** (抢先注册同名 profile ⇒ 我们 acquire 时拿到
/// `ALREADY_EXISTS` ⇒ 沙箱用的是对方注册的 profile)。
///
/// 拼装规则抽成**平台无关**函数 ⇒ 这条规则在 Linux 上就能测 (不必等 CI 矩阵)。
#[test]
fn appcontainer_profile_name_is_scoped_and_sanitized() {
    use apeireth_tools_canonical::process::appcontainer_profile_name;

    let name = appcontainer_profile_name("Apeireth.Shell.Sandbox", "abc123", "0123456789abcdef");
    assert!(
        name.starts_with("Apeireth.Shell.Sandbox."),
        "prefix lost: {name}"
    );
    assert!(name.contains(".abc123."), "user tag lost: {name}");
    assert!(
        name.ends_with(".0123456789abcdef"),
        "random tag lost: {name}"
    );

    // 随机后缀不同 ⇒ 名字不同 ⇒ 派生出的 SID 不同 (SID 不再确定)。
    let other = appcontainer_profile_name("Apeireth.Shell.Sandbox", "abc123", "fedcba9876543210");
    assert_ne!(
        name, other,
        "a different random tag must yield a different profile name"
    );

    // 非法字符被剔除 (profile 名只允许安全字符), 每段封顶 24 字符。
    let hostile =
        appcontainer_profile_name("Apeireth.Shell.Sandbox", "us er/../\\", &"x".repeat(64));
    assert!(!hostile.contains('/'), "slash must be stripped: {hostile}");
    assert!(!hostile.contains(' '), "space must be stripped: {hostile}");
    for segment in hostile.split('.') {
        assert!(segment.len() <= 24, "segment too long: {segment}");
    }
}

#[cfg(target_os = "macos")]
mod macos_tests {
    use super::*;
    use std::process::{Command, Stdio};

    /// 角色进程的环境标记: 重跑本测试二进制时带上它 ⇒ 走「executor 角色」分支。
    const ROLE_ENV: &str = "APEIRETH_TEST_MACOS_EXECUTOR_ROLE";
    /// 长睡时长的唯一标记 (避免与并发跑的其它测试撞同一个命令行)。
    const SLEEP_MARKER: &str = "987654";

    #[test]
    fn timeout_terminates_the_whole_process_group_tree() {
        let mut limits = ProcessLimits::default();
        limits.max_runtime = Duration::from_secs(2);
        let request = ProcessRequest::new(helper())
            .with_args(["spawn-child", "sleep", "60"])
            .with_limits(limits);

        let result = execute(&request).unwrap();
        assert!(
            result.timed_out(),
            "expected TimedOut, got {:?}",
            result.termination
        );

        let stdout = text(&result.stdout);
        let grandchild_pid = stdout
            .lines()
            .find_map(|line| line.strip_prefix("SPAWN_OK "))
            .map(|pid| pid.parse::<i32>().unwrap())
            .unwrap_or_else(|| panic!("expected SPAWN_OK <pid> in child stdout, got {stdout:?}"));

        assert_process_gone(grandchild_pid);
    }

    #[test]
    fn privilege_reduction_is_honestly_unsupported() {
        let capabilities = current_platform_capabilities();
        assert_eq!(
            capabilities.privilege_reduction,
            EnforcementLevel::Unsupported,
            "macOS backend must not invent a Windows RestrictedToken equivalent"
        );
    }

    /// P3-8 (2026-10-31): macOS 侧必须验证「子进程自任进程组长」—— 父死亡守护
    /// 靠 `kill(-pgid, SIGKILL)` 杀整个组, 组边界不成立则守护形同虚设。
    #[test]
    fn macos_child_leads_its_own_process_group() {
        let result = execute(&request_for("platform-security-info", "")).unwrap();
        assert!(result.success(), "{}", text(&result.stderr));
        let stdout = text(&result.stdout);
        let pgid = stdout
            .lines()
            .find_map(|line| line.strip_prefix("PGRP:"))
            .map(str::trim)
            .expect("PGRP line missing");
        let pid = stdout
            .lines()
            .find_map(|line| line.strip_prefix("PID:"))
            .map(str::trim)
            .expect("PID line missing");
        assert_eq!(
            pgid, pid,
            "the child must lead its own process group (the macOS guard kills -pgid)"
        );
    }

    /// P3-8 (2026-10-31): **executor 被 SIGKILL ⇒ 子进程必须跟着死**。
    ///
    /// 这是 macOS 父死亡守护 (`kqueue(EVFILT_PROC/NOTE_EXIT)`) 的端到端证明:
    /// 角色进程用 ProcessExecutor 起一个长睡子进程, 然后被 SIGKILL (模拟 executor
    /// 崩溃) ⇒ 守护进程必须把子进程所在的进程组一起带走 (否则子进程逃出
    /// `max_runtime` 约束 —— 正是 P3-8 的原始后果)。
    #[test]
    fn macos_child_dies_when_its_executor_is_killed() {
        let exe = std::env::current_exe().expect("current_exe");
        let mut role = Command::new(&exe)
            .arg("--exact")
            .arg("macos_executor_role_process")
            .arg("--nocapture")
            .env(ROLE_ENV, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the executor-role process");

        // 等 executor 起好的子进程出现在进程表里 (命令行含唯一 sleep 标记)。
        let child_pid = match wait_for_pid_containing(
            &format!("sleep {SLEEP_MARKER}"),
            Duration::from_secs(30),
        ) {
            Some(pid) => pid,
            None => {
                // **失败时把证据打出来**（进程表快照 + 角色进程是否还活着）
                // ⇒ 下次 CI 红了不用再猜「是没起来、还是没匹配上」。
                let role_state = match role.try_wait() {
                    Ok(Some(status)) => format!("角色进程已退出: {status}"),
                    Ok(None) => "角色进程仍在跑（⇒ 问题在子进程侧）".to_string(),
                    Err(error) => format!("角色进程状态取不到: {error}"),
                };
                panic!(
                        "the executor's child must show up in the process table\n                         角色进程: {role_state}\n进程表快照:\n{}",
                        process_table_snapshot()
                    );
            }
        };

        // 模拟 executor 崩溃: SIGKILL (不给它任何清理机会)。
        unsafe { libc::kill(role.id() as i32, libc::SIGKILL) };
        let _ = role.wait();

        // 守护进程必须把子进程带走。
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if !process_alive(child_pid) {
                break;
            }
            if Instant::now() >= deadline {
                unsafe { libc::kill(child_pid, libc::SIGKILL) };
                panic!("the child survived its executor's death (macOS parent-death guard failed)");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// 角色进程: 用 ProcessExecutor 起一个长睡子进程并一直阻塞 (由父测试 SIGKILL)。
    #[test]
    fn macos_executor_role_process() {
        if std::env::var_os(ROLE_ENV).is_none() {
            return; // 正常跑测试时本测是空转
        }
        let mut limits = ProcessLimits::default();
        // 阻塞到被 SIGKILL 为止 (默认 30s 会先超时 ⇒ 子进程被 executor 自己杀掉,
        // 那样就测不到「父死亡」这条路径了)。
        limits.max_runtime = Duration::from_secs(600);
        let request = ProcessRequest::new(helper())
            .with_args(["sleep", SLEEP_MARKER])
            .with_limits(limits);
        let _ = execute(&request);
    }

    /// 在进程表里找命令行含 `needle` 的 pid (macOS `ps` 扫描)。
    fn wait_for_pid_containing(needle: &str, timeout: Duration) -> Option<i32> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(pid) = pid_containing(needle) {
                return Some(pid);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn pid_containing(needle: &str) -> Option<i32> {
        let output = Command::new("ps")
            // ⚠️ **`-ww` 是必需的**（2026-11-01 CI 实测修正）：BSD/macOS 的 `ps`
            // **默认把命令行截断到终端宽度** ⇒ 长的命令行尾部被切掉 ⇒ 匹配不到
            // ⇒ 2026-11-01 macOS CI 上本测试真的红了
            // （"the executor's child must show up in the process table"）。
            // Linux 的 `ps` 默认不截断 ⇒ 本地绿 / macOS 红 ⇒ **平台差异被掩盖**。
            // ⇒ 加 `-ww`（BSD ps 的「不截断」选项，Linux 也支持）。
            .arg("-Aww")
            .arg("-o")
            .arg("pid=,command=")
            .output()
            .ok()?;
        let listing = String::from_utf8_lossy(&output.stdout);
        for line in listing.lines() {
            let line = line.trim();
            let Some((pid, command)) = line.split_once(' ') else {
                continue;
            };
            if command.contains(needle) {
                if let Ok(pid) = pid.trim().parse::<i32>() {
                    return Some(pid);
                }
            }
        }
        None
    }

    /// 进程表快照（**失败时打印** ⇒ 下次 CI 不用再猜）。
    fn process_table_snapshot() -> String {
        Command::new("ps")
            .arg("-Aww")
            .arg("-o")
            .arg("pid=,command=")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .unwrap_or_else(|error| format!("<ps 取不到: {error}>"))
    }

    /// 进程是否还活着 (signal 0 探测; EPERM 也算活着 —— 只是没权限发信号)。
    fn process_alive(pid: i32) -> bool {
        let ret = unsafe { libc::kill(pid, 0) };
        if ret == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    fn assert_process_gone(pid: i32) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let ret = unsafe { libc::kill(pid, 0) };
            if ret == -1 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                if errno == libc::ESRCH {
                    return;
                }
            }
            if Instant::now() >= deadline {
                panic!("descendant process {pid} survived process-group termination");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
