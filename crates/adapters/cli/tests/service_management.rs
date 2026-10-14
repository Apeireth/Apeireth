//! Service-management surface tests (W2-① daemon 化): the Windows service
//! script (`packaging/service/apeireth-service.ps1`) and the Linux systemd
//! unit (`packaging/service/linux/apeireth-gateway.service`).
//!
//! The script is exercised on three faces:
//! 1. static structure — lifecycle actions, dry-run gate, crash self-restart,
//!    single-instance registration, log/data anchors, token-face wiring, and
//!    the plaintext-secret hygiene rule (the token never reaches disk or the
//!    task definition);
//! 2. real `dry-run` execution — the plan prints and nothing is mutated
//!    (checked against a throwaway directory and an isolated task name);
//! 3. argument validation — unknown actions and out-of-range ports fail fast
//!    with an explicit error before anything happens.
//!
//! The Linux unit is validated statically (section/key syntax, required
//! fields, absolute paths) — the Linux face itself is explicitly untested
//! (see the unit file header).

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn service_script() -> PathBuf {
    repo_root()
        .join("packaging")
        .join("service")
        .join("apeireth-service.ps1")
}

fn systemd_unit() -> PathBuf {
    repo_root()
        .join("packaging")
        .join("service")
        .join("linux")
        .join("apeireth-gateway.service")
}

/// PowerShell host for executing the service script. Returns `None` when the
/// machine has no PowerShell available (reported, never silently passed).
fn powershell_host() -> Option<PathBuf> {
    for candidate in ["pwsh", "powershell"] {
        if let Ok(output) = Command::new(candidate)
            .arg("-NoProfile")
            .arg("-Command")
            .arg("$PSVersionTable.PSVersion.ToString()")
            .output()
        {
            if output.status.success() {
                return Some(PathBuf::from(candidate));
            }
        }
    }
    None
}

/// Isolated paths for one script run: dummy binary + throwaway state dirs.
fn script_sandbox(label: &str) -> (PathBuf, PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "apeireth-service-script-{}-{label}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&root);
    fs_err::create_dir_all(&root).expect("script sandbox");
    let binary = root.join("apeireth.exe");
    fs_err::write(&binary, b"not a real binary (resolution only)").expect("dummy binary");
    // Isolated scheduled-task name so a real install on this machine can never
    // be observed (or disturbed) by these tests.
    let task_name = format!("ApeirethServiceSelfTest{}", std::process::id());
    (root, binary, task_name)
}

struct ScriptRun {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run_service_script(
    host: &PathBuf,
    root: &PathBuf,
    binary: &PathBuf,
    task_name: &str,
    args: &[&str],
) -> ScriptRun {
    let mut command = Command::new(host);
    command
        .arg("-NoProfile")
        .arg("-File")
        .arg(service_script())
        .args(args)
        .arg("-TaskName")
        .arg(task_name)
        .arg("-BinaryPath")
        .arg(binary)
        .arg("-RunDir")
        .arg(root.join("run"))
        .arg("-LogDir")
        .arg(root.join("logs"))
        .arg("-DataDir")
        .arg(root.join("data"));
    let output = command.output().expect("invoke service script");
    ScriptRun {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

// ---------------------------------------------------------------------------
// 静态结构面: 动作面 / dry-run 闸 / 崩溃自拉起 / 单实例 / 日志锚 / 令牌面
// ---------------------------------------------------------------------------

/// The script owns the full service lifecycle and guards every mutation with
/// the dry-run gate; crash self-restart and single-instance semantics are
/// configured on the scheduled task; logs and state land in the app-data tree.
#[test]
fn windows_service_script_covers_the_lifecycle_and_guards_mutations_with_dry_run() {
    let text = fs_err::read_to_string(service_script()).expect("read service script");

    // 动作面: install/start/stop/status/uninstall (+ 内部 run) 全在参数校验集里。
    let validate_set = text
        .lines()
        .find(|line| line.contains("ValidateSet("))
        .expect("action ValidateSet");
    for action in ["install", "start", "stop", "status", "uninstall", "run"] {
        assert!(
            validate_set.contains(&format!("'{action}'")),
            "action {action} missing from ValidateSet: {validate_set}"
        );
    }

    // dry-run 闸: 每个变更动作都走 Invoke-Step, 闸本身在 dry-run 下只打印。
    assert!(text.contains("[switch]$DryRun"), "dry-run switch");
    assert!(
        text.contains("function Invoke-Step"),
        "mutation gate function"
    );
    assert!(text.contains("if ($DryRun)"), "dry-run gate");
    assert!(text.contains("[dry-run]"), "dry-run plan marker");

    // 变更 cmdlet 只允许各出现一次 (单一收口在 Invoke-Step 体内)。
    for (needle, expected) in [
        ("Register-ScheduledTask", 1),
        ("Unregister-ScheduledTask", 1),
    ] {
        assert_eq!(
            text.matches(needle).count(),
            expected,
            "{needle} must be wired exactly once"
        );
    }
    assert!(
        text.contains("Start-ScheduledTask") && text.contains("Stop-ScheduledTask"),
        "start/stop must drive the scheduled task"
    );

    // 崩溃自拉起 + 单实例 (不双开) 在任务设置里。
    assert!(text.contains("RestartCount"), "crash self-restart count");
    assert!(
        text.contains("RestartInterval"),
        "crash self-restart interval"
    );
    assert!(
        text.contains("MultipleInstances IgnoreNew"),
        "single-instance task setting"
    );
    assert!(text.contains("-AtLogOn"), "boot/logon autostart trigger");

    // 常驻占位与优雅停机接线: PID 文件 + 令牌门停机端点 + 健康探测。
    assert!(text.contains("--pid-file"), "resident slot file wiring");
    assert!(
        text.contains("/v1/admin/shutdown"),
        "graceful stop endpoint wiring"
    );
    assert!(text.contains("/health"), "health probe wiring");

    // 日志/数据锚 (%LOCALAPPDATA%\Apeireth\...)。
    assert!(
        text.contains("Apeireth\\logs"),
        "log directory must anchor at the app-data tree"
    );
    assert!(text.contains("Apeireth\\run"), "run directory anchor");
    assert!(text.contains("Apeireth\\data"), "data directory anchor");
    assert!(
        text.contains("gateway-service.log"),
        "service log file name"
    );
}

/// The service token face is a credential-store entry, never a file or a task
/// argument: the mint is 32 random bytes, the read/write/delete go through the
/// credential-store API, and no file-writing line may carry the token value.
#[test]
fn service_script_keeps_the_token_out_of_files_and_task_definitions() {
    let text = fs_err::read_to_string(service_script()).expect("read service script");

    // 令牌面身份 (与桌面壳 / TUI 同源读取面一致)。
    assert!(
        text.contains("$TokenTargetName = 'apeireth.gateway-token'"),
        "credential-store target name"
    );
    assert!(
        text.contains("$TokenUserName = 'apeireth'"),
        "credential-store user name"
    );

    // 生成 = 32 随机字节; 存取 = 凭据库 API (明文不落盘)。
    assert!(
        text.contains("RandomNumberGenerator"),
        "token must be minted from a cryptographic RNG"
    );
    for api in ["CredWriteW", "CredReadW", "CredDeleteW"] {
        assert!(text.contains(api), "credential-store API {api}");
    }

    // 密钥卫生: 任何落盘行 (文件写 cmdlet / 重定向) 不得携带令牌值;
    // 计划任务定义同样不得内嵌令牌 (令牌只走服务 env 面)。
    for line in text.lines() {
        let writes_file = [
            "Set-Content",
            "Out-File",
            "Add-Content",
            "WriteAllText",
            "Tee-Object",
        ]
        .iter()
        .any(|cmd| line.contains(cmd));
        if writes_file {
            assert!(
                !line.to_lowercase().contains("token"),
                "token value must never reach a file write: {line}"
            );
        }
        if line.contains("New-ScheduledTaskAction") {
            assert!(
                !line.contains("$token") && !line.to_lowercase().contains("token"),
                "task definition must not embed the token: {line}"
            );
        }
    }

    // 服务 env 面是唯一的明文通道。
    assert!(
        text.contains("$env:APEIRETH_GATEWAY_TOKEN"),
        "token must reach the gateway through the service environment only"
    );
}

// ---------------------------------------------------------------------------
// dry-run 执行面: 打印计划、零副作用
// ---------------------------------------------------------------------------

/// A real `install -DryRun` run prints the plan and creates nothing: no state
/// directories, no scheduled task, no token. `stop`/`uninstall` dry-runs are
/// equally side-effect free, and `status` (read-only) reports honestly.
#[test]
fn windows_service_script_dry_run_plans_without_touching_the_system() {
    let Some(host) = powershell_host() else {
        eprintln!("SKIP: no PowerShell host available for the dry-run execution face");
        return;
    };
    let (root, binary, task_name) = script_sandbox("dryrun");

    // install -DryRun: 计划可见, 零副作用。
    let install = run_service_script(&host, &root, &binary, &task_name, &["install", "-DryRun"]);
    assert_eq!(install.status, 0, "dry-run install: {}", install.stderr);
    assert!(install.stdout.contains("[dry-run]"), "{}", install.stdout);
    assert!(
        install.stdout.contains("register scheduled task"),
        "plan must show the task registration: {}",
        install.stdout
    );
    assert!(
        install.stdout.contains("service token"),
        "plan must show the token step: {}",
        install.stdout
    );
    for dir in ["run", "logs", "data"] {
        assert!(
            !root.join(dir).exists(),
            "dry-run must not create the {dir} directory"
        );
    }

    // stop / uninstall -DryRun: 同样只打印计划。
    for action in ["stop", "uninstall"] {
        let run = run_service_script(&host, &root, &binary, &task_name, &[action, "-DryRun"]);
        assert_eq!(run.status, 0, "dry-run {action}: {}", run.stderr);
        assert!(run.stdout.contains("[dry-run]"), "{action}: {}", run.stdout);
    }
    assert!(
        !root.join("run").join("gateway.pid").exists(),
        "dry-run must not touch the resident slot file"
    );

    // status (只读真跑): 用隔离任务名, 必须如实报"未安装"。
    let status = run_service_script(&host, &root, &binary, &task_name, &["status"]);
    assert_eq!(status.status, 0, "status: {}", status.stderr);
    assert!(
        status.stdout.contains("installed: False"),
        "isolated task name must report not installed: {}",
        status.stdout
    );
    assert!(
        status.stdout.contains("gateway: stopped"),
        "{}",
        status.stdout
    );
    assert!(
        status.stdout.contains("service token: "),
        "token presence must be reported without any value: {}",
        status.stdout
    );

    // 未安装 start: 显式报错 (dry-run 也照实校验, 不静默假装成功)。
    let start = run_service_script(&host, &root, &binary, &task_name, &["start", "-DryRun"]);
    assert_ne!(start.status, 0, "start on a missing install must fail");
    assert!(
        start.stderr.contains("not installed"),
        "explicit refusal expected: {}",
        start.stderr
    );

    let _ = fs_err::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 参数校验面: 非法动作 / 越界端口 / 空绑定
// ---------------------------------------------------------------------------

/// Unknown actions, out-of-range ports, and a blank bind address all fail fast
/// with a non-zero exit before anything is touched (the parameter validators
/// are the contract pinned by the static face above).
#[test]
fn windows_service_script_rejects_invalid_arguments_before_touching_anything() {
    let Some(host) = powershell_host() else {
        eprintln!("SKIP: no PowerShell host available for the validation face");
        return;
    };
    let (root, binary, task_name) = script_sandbox("badargs");

    // 非法动作: 参数校验集拒绝 (退出码非零 + 显式报错)。
    let bogus = run_service_script(&host, &root, &binary, &task_name, &["bogus-action"]);
    assert_ne!(bogus.status, 0, "unknown action must fail");
    assert!(!bogus.stderr.trim().is_empty(), "must explain the refusal");

    // 越界端口: 0 与 70000 都被 ValidateRange 拒。
    for port in ["0", "70000"] {
        let run = run_service_script(
            &host,
            &root,
            &binary,
            &task_name,
            &["install", "-DryRun", "-Port", port],
        );
        assert_ne!(run.status, 0, "port {port} must be rejected");
        assert!(!run.stderr.trim().is_empty(), "port {port} refusal message");
    }

    // 空绑定地址: 显式英文错误句 (与实现同句, 稳定锚)。
    let blank_bind =
        run_service_script(&host, &root, &binary, &task_name, &["status", "-Bind", ""]);
    assert_ne!(blank_bind.status, 0, "blank bind must fail");
    assert!(
        blank_bind.stderr.contains("bind address must not be empty"),
        "explicit message expected: {}",
        blank_bind.stderr
    );

    // 全程零副作用。
    for dir in ["run", "logs", "data"] {
        assert!(
            !root.join(dir).exists(),
            "invalid arguments must not create the {dir} directory"
        );
    }
    let _ = fs_err::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Linux 面 (未实测): systemd 单元静态校验
// ---------------------------------------------------------------------------

/// The systemd unit parses as valid unit syntax and carries the daemon
/// contract: absolute paths (CWD 无关), crash self-restart (`Restart=`),
/// graceful stop (`KillSignal=SIGTERM`), log destination, and the PID-file
/// single-instance claim. (The Linux face itself is untested; this pins the
/// file's structure so it cannot rot silently.)
#[test]
fn linux_systemd_unit_is_structurally_valid_with_crash_restart_and_sigterm_stop() {
    let text = fs_err::read_to_string(systemd_unit()).expect("read systemd unit");
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            sections.push((name.to_string(), Vec::new()));
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("unit line must be `key=value` or `[section]`: {line}"));
        let section = sections
            .last_mut()
            .expect("unit content must follow a section header");
        assert!(
            !section.1.iter().any(|(existing, _)| existing == key.trim()),
            "duplicate key `{}` in [{}]",
            key.trim(),
            section.0
        );
        section
            .1
            .push((key.trim().to_string(), value.trim().to_string()));
    }

    let names: Vec<&str> = sections.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["Unit", "Service", "Install"], "unit sections");

    let get = |section: &str, key: &str| -> String {
        sections
            .iter()
            .find(|(name, _)| name == section)
            .and_then(|(_, entries)| {
                entries
                    .iter()
                    .find(|(entry_key, _)| entry_key == key)
                    .map(|(_, value)| value.clone())
            })
            .unwrap_or_else(|| panic!("[{section}] must define {key}="))
    };

    // 装配身份与依赖序。
    assert!(!get("Unit", "Description").is_empty());
    assert_eq!(get("Unit", "After"), "network.target");

    // 运行形态: 绝对路径 + 常驻占位 + 数据锚 (工作目录无关)。
    let exec = get("Service", "ExecStart");
    assert!(exec.starts_with('/'), "ExecStart must be absolute: {exec}");
    assert!(
        exec.contains("apeireth gateway serve"),
        "ExecStart must run the canonical serve form: {exec}"
    );
    assert!(
        exec.contains("--pid-file"),
        "ExecStart must claim the resident slot: {exec}"
    );
    let workdir = get("Service", "WorkingDirectory");
    assert!(
        workdir.starts_with('/'),
        "WorkingDirectory must be absolute"
    );
    assert!(
        get("Service", "EnvironmentFile").starts_with('/'),
        "EnvironmentFile must be absolute"
    );
    assert_eq!(get("Service", "Type"), "simple");
    assert_eq!(get("Service", "User"), "apeireth");

    // 崩溃自拉起 + 优雅停机 + 日志落位。
    assert_eq!(get("Service", "Restart"), "on-failure");
    assert!(!get("Service", "RestartSec").is_empty());
    assert_eq!(get("Service", "KillSignal"), "SIGTERM");
    assert!(!get("Service", "TimeoutStopSec").is_empty());
    for log_key in ["StandardOutput", "StandardError"] {
        let sink = get("Service", log_key);
        assert!(
            sink.starts_with("append:/"),
            "{log_key} must append to an absolute log path: {sink}"
        );
    }

    // 安装面。
    assert_eq!(get("Install", "WantedBy"), "multi-user.target");
}
