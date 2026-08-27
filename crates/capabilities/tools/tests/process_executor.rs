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

fn helper() -> &'static str {
    env!("CARGO_BIN_EXE_sandbox-test-child")
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

#[cfg(target_os = "macos")]
mod macos_tests {
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
    fn privilege_reduction_is_honestly_unsupported() {
        let capabilities = current_platform_capabilities();
        assert_eq!(
            capabilities.privilege_reduction,
            EnforcementLevel::Unsupported,
            "macOS backend must not invent a Windows RestrictedToken equivalent"
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
