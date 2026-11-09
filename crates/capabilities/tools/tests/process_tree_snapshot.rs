//! 进程树快照面（C-B1 · `tool.process_tree`）的真机测试。
//!
//! 覆盖用例：空树 / 单进程 / 多层树 / 逃逸标记 / 幂等取消（取消两次）/ 已退出明确结果 /
//! **越权拒绝（对抗）** / 平台未实现路径显式标注 / 快照面零审批。
//!
//! 全部用例都起**真进程**，不使用任何 mock 进程执行器。

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use apeireth_tools_canonical::process::{
    cancel_process_tree, process_tree_snapshot, process_tree_support, GroupContainment,
    ProcessTreeCancel, ProcessTreeError, ProcessTreeStatus, ProcessTreeSupport,
};

// ===========================================================================
// 辅助
// ===========================================================================

/// 子进程测试用的 helper 可执行文件路径。
///
/// 与既有 `tests/process_executor.rs` 同款做法：`option_env!` + 同目录回退
/// （部分 cargo 版本不注入该编译期变量，`env!` 会直接编译失败）。
fn helper() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_sandbox-test-child") {
        return PathBuf::from(path);
    }
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

/// 起一个子进程，测试结束必收尸（不留僵尸、不留挂着的 sleeper）。
struct ChildGuard(Child);

impl ChildGuard {
    fn pid(&self) -> u32 {
        self.0.id()
    }

    /// 阻塞收尸。
    fn wait(&mut self) -> Option<std::process::ExitStatus> {
        self.0.wait().ok()
    }

    /// 等子进程**真的**退出（终止信号送达 ⇒ 进程被内核收掉需要一点时间）。
    fn wait_for_exit(&mut self, timeout_ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            if self.0.try_wait().ok().flatten().is_some() {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 起一个普通子进程（与本测试进程同组 ⇒ 属于本会话的受控树）。
fn spawn_child(args: &[&str]) -> ChildGuard {
    let child = Command::new(helper())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("helper child must spawn");
    ChildGuard(child)
}

/// 起一个**自建进程组**的子进程（⇒ 它逃出了根进程所在的组）。
#[cfg(unix)]
fn spawn_child_in_own_group(args: &[&str]) -> ChildGuard {
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(helper());
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: pre_exec 只调 setpgid（异步信号安全），失败即让 spawn 报错。
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = command.spawn().expect("helper child must spawn");
    ChildGuard(child)
}

/// 进程是否还活着（信号 0 探针）。
#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// 读 `/proc/<pid>/stat` 里的父 pid（Linux 专有，用于自证测试前提）。
#[cfg(target_os = "linux")]
fn parent_pid_of(pid: u32) -> Option<u32> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = raw.rfind(')')?;
    let rest = raw.get(close + 1..)?;
    rest.split_whitespace().nth(1)?.parse().ok()
}

/// 派生一个**脱离本会话**的进程（double fork ⇒ 被会话初始进程收养）。
///
/// 返回的 pid **不在本会话的树内** ⇒ 取消面必须拒绝它。目标本身无害（只睡一会儿），
/// 因此测试可以安全地断言「拒绝之后它还活着」。
#[cfg(target_os = "linux")]
fn spawn_detached_sleeper(seconds: u64) -> u32 {
    // SAFETY: fork 之后子进程只调用异步信号安全的 libc 调用并 _exit 退出。
    unsafe {
        let mut fds = [0; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe must succeed");
        let (read_fd, write_fd) = (fds[0], fds[1]);

        let first = libc::fork();
        assert!(first >= 0, "fork must succeed");
        if first == 0 {
            // 第一层：派生第二层后立刻退出 ⇒ 第二层被会话初始进程收养。
            libc::close(read_fd);
            // 断开与测试框架输出管道的连接（否则 sleeper 会一直占着管道 ⇒
            // 测试进程要等它退出才收得到输出）。
            libc::close(1);
            libc::close(2);
            let second = libc::fork();
            if second == 0 {
                libc::close(write_fd);
                sleep_in_child(seconds);
                libc::_exit(0);
            }
            let bytes = second.to_le_bytes();
            libc::write(write_fd, bytes.as_ptr().cast::<libc::c_void>(), bytes.len());
            libc::_exit(0);
        }

        libc::close(write_fd);
        let mut buffer = [0u8; 4];
        let mut filled = 0usize;
        while filled < buffer.len() {
            let read = libc::read(
                read_fd,
                buffer[filled..].as_mut_ptr().cast::<libc::c_void>(),
                buffer.len() - filled,
            );
            if read <= 0 {
                break;
            }
            filled += read as usize;
        }
        libc::close(read_fd);

        let mut status = 0;
        libc::waitpid(first, &mut status, 0);
        assert_eq!(filled, 4, "detached child pid must be readable");
        u32::from_le_bytes(buffer)
    }
}

/// 子进程里的睡眠（只用异步信号安全的调用）。
#[cfg(target_os = "linux")]
fn sleep_in_child(seconds: u64) {
    let mut remaining = libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: 0,
    };
    loop {
        let mut leftover = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let result = unsafe { libc::nanosleep(&remaining, &mut leftover) };
        if result == 0 {
            return;
        }
        remaining = leftover;
    }
}

// ===========================================================================
// 快照枚举
// ===========================================================================

/// 空树：根没有子进程 ⇒ 快照只有根自己。
#[test]
fn empty_tree_snapshot_lists_only_the_root() {
    let child = spawn_child(&["sleep", "30"]);
    let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");

    assert_eq!(snapshot.root_pid, child.pid());
    assert_eq!(snapshot.len(), 1, "a childless root is a one-member tree");
    assert_eq!(snapshot.descendants().count(), 0);
    assert!(snapshot.find(child.pid()).is_some());
}

/// 单进程：pid / 父 pid / 命令 / 状态齐备。
#[test]
fn single_process_snapshot_reports_pid_parent_and_command() {
    let child = spawn_child(&["sleep", "30"]);
    let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");

    let entry = snapshot.find(child.pid()).expect("root must be listed");
    assert_eq!(entry.pid, child.pid());
    assert_eq!(
        entry.parent_pid,
        std::process::id(),
        "the child must be reported as a child of this session"
    );
    assert!(
        entry.command_line.contains("sandbox-test-child"),
        "the command line must be reported: {}",
        entry.command_line
    );
    assert_ne!(
        entry.status,
        ProcessTreeStatus::Unknown,
        "a live child must not report an unknown status"
    );
}

/// 多层树：根 → 子 → 孙，每一层都要在快照里。
#[test]
fn multi_level_tree_snapshot_lists_every_generation() {
    let child = spawn_child(&["spawn-child", "sleep", "30"]);
    let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");

    assert!(
        snapshot.len() >= 2,
        "the tree must contain the child and its own child: {snapshot:?}"
    );
    let grandchild = snapshot
        .entries
        .iter()
        .find(|entry| entry.parent_pid == child.pid())
        .expect("a second generation must be listed");
    assert_ne!(grandchild.pid, child.pid());
    assert_eq!(snapshot.descendants().count(), snapshot.len() - 1);
}

/// 逃逸标记：自建进程组的成员必须被标成 `Escaped`，同组成员是 `Contained`。
#[cfg(unix)]
#[test]
fn escaped_group_is_marked_for_a_member_in_its_own_group() {
    let plain = spawn_child(&["sleep", "30"]);
    let escaped = spawn_child_in_own_group(&["sleep", "30"]);

    // 根取本进程自身 ⇒ 覆盖本会话的全部后代。
    let snapshot = process_tree_snapshot(std::process::id()).expect("snapshot must succeed");

    let escaped_entry = snapshot
        .find(escaped.pid())
        .expect("the escaped member must be listed");
    assert_eq!(
        escaped_entry.group_containment,
        GroupContainment::Escaped,
        "a member in its own group must be marked as escaped"
    );
    assert!(escaped_entry.escaped_group());

    let plain_entry = snapshot
        .find(plain.pid())
        .expect("the plain member must be listed");
    assert_eq!(
        plain_entry.group_containment,
        GroupContainment::Contained,
        "a member sharing the root group must not be marked as escaped"
    );
    assert!(!plain_entry.escaped_group());
    assert!(snapshot.escaped().count() >= 1);
}

// ===========================================================================
// 按 pid 幂等取消
// ===========================================================================

/// 幂等取消：第一次真终止，第二次**零副作用**地回放结果。
#[test]
fn cancel_terminates_the_tree_and_is_idempotent() {
    let mut child = spawn_child(&["sleep", "30"]);
    let pid = child.pid();

    let first = cancel_process_tree(pid).expect("cancel must succeed");
    match &first {
        ProcessTreeCancel::Cancelled {
            root_pid,
            terminated,
            failed,
            ..
        } => {
            assert_eq!(*root_pid, pid);
            assert!(terminated.contains(&pid), "the root must be terminated");
            assert!(failed.is_empty(), "no member should fail: {failed:?}");
        }
        other => panic!("expected Cancelled, got {other:?}"),
    }
    assert!(first.caused_termination());
    assert!(
        child.wait_for_exit(5_000),
        "the cancelled child must actually be gone"
    );

    let second = cancel_process_tree(pid).expect("repeat cancel must succeed");
    match &second {
        ProcessTreeCancel::AlreadyCancelled {
            root_pid,
            terminated,
        } => {
            assert_eq!(*root_pid, pid);
            assert!(
                terminated.contains(&pid),
                "the first outcome must be replayed verbatim"
            );
        }
        other => panic!("expected AlreadyCancelled, got {other:?}"),
    }
    assert!(
        !second.caused_termination(),
        "a repeated cancel must not produce termination side effects"
    );

    // 第三次取消依旧是幂等回放（台账不消耗）⇒ 任意次重复取消都零副作用。
    let third = cancel_process_tree(pid).expect("third cancel must succeed");
    assert!(
        matches!(third, ProcessTreeCancel::AlreadyCancelled { .. }),
        "a repeated cancel must stay idempotent: {third:?}"
    );
    assert!(!third.caused_termination());
}

/// 已退出的进程：返回**明确结果**（不是错误）。
#[test]
fn cancel_reports_an_already_exited_process_explicitly() {
    let mut child = spawn_child(&["exit-code", "0"]);
    let pid = child.pid();
    assert!(child.wait().is_some(), "the child must exit and be reaped");

    let result = cancel_process_tree(pid).expect("cancel must not error");
    assert!(
        matches!(result, ProcessTreeCancel::NotRunning { .. }),
        "an exited process must yield an explicit result: {result:?}"
    );
}

/// 含多层的树：取消要覆盖整棵树（子 → 孙）。
#[test]
fn cancel_covers_every_generation_of_the_tree() {
    let child = spawn_child(&["spawn-child", "sleep", "30"]);
    let pid = child.pid();

    let snapshot = process_tree_snapshot(pid).expect("snapshot must succeed");
    let grandchild = snapshot
        .entries
        .iter()
        .find(|entry| entry.parent_pid == pid)
        .expect("a second generation must exist")
        .pid;

    let result = cancel_process_tree(pid).expect("cancel must succeed");
    match result {
        ProcessTreeCancel::Cancelled {
            terminated,
            already_exited,
            failed,
            ..
        } => {
            assert!(terminated.contains(&pid));
            assert!(
                terminated.contains(&grandchild),
                "the whole tree must be terminated: {terminated:?}"
            );
            assert!(failed.is_empty(), "no member should fail: {failed:?}");
            assert!(already_exited.is_empty());
        }
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

/// 拒绝：目标树里含当前进程自身 ⇒ 不许自我了断。
#[test]
fn cancel_refuses_a_tree_that_contains_the_current_process() {
    let result = cancel_process_tree(std::process::id());
    match result {
        Err(ProcessTreeError::Refused { pid, .. }) => assert_eq!(pid, std::process::id()),
        other => panic!("expected Refused, got {other:?}"),
    }
}

// ===========================================================================
// 对抗：不属于本会话的 pid ⇒ 必须拒绝（不得越权杀别人的进程）
// ===========================================================================

#[cfg(target_os = "linux")]
#[test]
fn cancel_refuses_a_pid_outside_this_session() {
    let pid = spawn_detached_sleeper(30);

    // 前提自证：它已被会话初始进程收养 ⇒ 不在本会话树内。
    let parent = parent_pid_of(pid).expect("detached process must be visible");
    if parent == std::process::id() {
        eprintln!("skipped: this session has no separate init to adopt orphans");
        return;
    }

    let result = cancel_process_tree(pid);
    match result {
        Err(ProcessTreeError::NotControlled { pid: refused }) => assert_eq!(refused, pid),
        other => panic!("expected NotControlled, got {other:?}"),
    }
    assert!(
        process_alive(pid),
        "a refused target must still be alive (no cross-session kill)"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn snapshot_refuses_a_pid_outside_this_session() {
    let pid = spawn_detached_sleeper(30);

    let parent = parent_pid_of(pid).expect("detached process must be visible");
    if parent == std::process::id() {
        eprintln!("skipped: this session has no separate init to adopt orphans");
        return;
    }

    let result = process_tree_snapshot(pid);
    match result {
        Err(ProcessTreeError::NotControlled { pid: refused }) => assert_eq!(refused, pid),
        other => panic!("expected NotControlled, got {other:?}"),
    }
    assert!(
        process_alive(pid),
        "a refused target must still be alive (no cross-session interference)"
    );
}

// ===========================================================================
// 平台未实现路径：显式标注，不许静默通过
// ===========================================================================

/// 平台支持档必须**显式**：未实现的平台必须显式报错，不许返回空快照冒充成功。
#[test]
fn platform_support_is_reported_explicitly() {
    match process_tree_support() {
        ProcessTreeSupport::Enumerable => {
            let child = spawn_child(&["sleep", "30"]);
            let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");
            assert!(
                !snapshot.is_empty(),
                "an implemented platform yields a real tree"
            );
        }
        ProcessTreeSupport::Unsupported => {
            let snapshot_error = process_tree_snapshot(std::process::id())
                .expect_err("an unimplemented platform must fail explicitly");
            assert!(
                matches!(snapshot_error, ProcessTreeError::Unsupported { .. }),
                "expected Unsupported, got {snapshot_error:?}"
            );
            let cancel_error = cancel_process_tree(std::process::id())
                .expect_err("an unimplemented platform must fail explicitly");
            assert!(
                matches!(cancel_error, ProcessTreeError::Unsupported { .. }),
                "expected Unsupported, got {cancel_error:?}"
            );
        }
    }
}

/// 平台支持矩阵必须与实现矩阵**逐格一致** (不许某一格偷偷变)。
///
/// 实现的矩阵: Linux / Windows = 真实现; macOS 与其它平台 = **显式未实现**
/// (`Unsupported`, 绝不返回空快照冒充成功)。本用例在本仓的三 OS 工作流里
/// **真机**跑 (含 macOS runner) ⇒ macOS 那条不许回归。
#[test]
fn the_platform_support_matrix_matches_the_implementation() {
    let expected = if cfg!(any(windows, target_os = "linux")) {
        ProcessTreeSupport::Enumerable
    } else {
        ProcessTreeSupport::Unsupported
    };
    assert_eq!(
        process_tree_support(),
        expected,
        "the platform support level must match the implementation matrix"
    );
}

// ===========================================================================
// 快照面零审批 + 不反向依赖
// ===========================================================================

/// 快照面是**只读 + 受控取消**：不得长出人工确认闸，也不得反向依赖运行时/网关/供给面。
#[test]
fn snapshot_face_is_read_only_and_carries_no_approval_gate() {
    let source = include_str!("../src/process/snapshot.rs");
    let lowered = source.to_lowercase();

    for forbidden in ["approval", "approve", "human_review", "confirmation"] {
        assert!(
            !lowered.contains(forbidden),
            "the snapshot face must stay approval-free: {forbidden}"
        );
    }
    for forbidden in [
        "apeireth_runtime",
        "apeireth_gateway",
        "apeireth_provider",
        "ToolRegistry",
        "GovernancePipeline",
        "SandboxRegistry",
    ] {
        assert!(
            !source.contains(forbidden),
            "the snapshot face must not depend on or recreate {forbidden}"
        );
    }
    // 公共契约面不得出现平台机制名（机制名只落平台后端）。
    let contract = source
        .split("mod backend")
        .next()
        .expect("the contract half must exist");
    for forbidden in [
        "JobObject",
        "RestrictedToken",
        "CreateProcessW",
        "OpenProcess",
        "TerminateProcess",
        "CreateToolhelp32Snapshot",
        "Process32FirstW",
        "Process32NextW",
        "GetExitCodeProcess",
        "cgroup",
        "seccomp",
        "Landlock",
        "namespace",
        "setrlimit",
        "Seatbelt",
        "sandbox-exec",
    ] {
        assert!(
            !contract.contains(forbidden),
            "the public contract must not expose {forbidden}"
        );
    }

    // 对外导出的公共契约面 (process/mod.rs) 同样不得出现进程 API 名。
    let public_contract = include_str!("../src/process/mod.rs");
    for forbidden in [
        "CreateProcessW",
        "OpenProcess",
        "TerminateProcess",
        "CreateToolhelp32Snapshot",
        "Process32FirstW",
        "Process32NextW",
        "GetExitCodeProcess",
    ] {
        assert!(
            !public_contract.contains(forbidden),
            "the exported public contract must not expose {forbidden}"
        );
    }

    // 平台机制名**只**允许出现在对应平台的后端块里（不外溢到别的后端）。
    for (marker, owned) in [
        ("#[cfg(target_os = \"linux\")]", &["getpgid", "kill"][..]),
        (
            "#[cfg(windows)]",
            &[
                "OpenProcess",
                "TerminateProcess",
                "CreateToolhelp32Snapshot",
                "Process32FirstW",
                "Process32NextW",
            ][..],
        ),
    ] {
        let block = backend_block(&source, marker);
        for name in owned {
            assert!(
                block.contains(name),
                "{marker}: the backend block must own its platform mechanism {name}"
            );
        }
    }
    // Windows 的机制名不得出现在别的后端块里（机制名只落本平台后端）。
    let windows_only = [
        "OpenProcess",
        "TerminateProcess",
        "CreateToolhelp32Snapshot",
        "Process32FirstW",
        "Process32NextW",
    ];
    for marker in [
        "#[cfg(target_os = \"linux\")]",
        "#[cfg(target_os = \"macos\")]",
        "#[cfg(not(any(windows, target_os = \"linux\", target_os = \"macos\")))]",
    ] {
        let block = backend_block(&source, marker);
        for name in windows_only {
            assert!(
                !block.contains(name),
                "{marker}: the windows platform mechanism {name} must not leak into another backend"
            );
        }
    }
}

/// 取源码里某个平台后端块（`#[cfg(...)]` 标记之后、下一个 `#[cfg(` 之前）。
fn backend_block<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source.find(marker).expect("the backend marker must exist");
    let rest = &source[start..];
    let end = rest[marker.len()..]
        .find("#[cfg(")
        .map(|offset| marker.len() + offset)
        .unwrap_or(rest.len());
    &rest[..end]
}
