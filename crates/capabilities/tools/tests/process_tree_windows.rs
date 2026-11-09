//! 进程树快照面 · **Windows 真机**验证用例 (C-B1 · W3 单元 3)。
//!
//! # 口径 (0 装诚实)
//!
//! 本文件是**真机**用例: 在 Windows 上真起进程、真取消进程、真断言进程已死。
//! CI 走 `.github/workflows/windows-process-tree.yml` (windows-latest)。
//!
//! ⚠️ **「CI 已布线」≠「跑过了」**: 本文件在非 Windows 平台上**不产生任何断言**
//! (只有一条「本套件在本平台不适用」的诚实占位) ⇒ 本机 (Linux) 无法实测本文件 ⇒
//! 口径升级以**首次 CI 绿灯**为准 (由 lead 据实改 claim 等级 + 记日期)。
//!
//! # 六条必查 (逐条)
//!
//! ① 快照**自己**（本进程）；② 真 spawn 子进程 ⇒ 快照看得见**父子树**；
//! ③ 真取消 ⇒ 子进程**真的没了**；④ **不误杀**：重复取消零副作用、无关进程不受影响；
//! ⑤ 收容组归属在 Windows 上**必须**是 `Undetermined`（收容容器句柄不对外暴露 ⇒
//! **不许**谎报 `Contained`）；⑥ 平台支持档必须**显式**（本平台 = 支持；未实现
//! 平台 = 显式 `Unsupported`，那条由跨平台矩阵钉死，见 `process_tree_snapshot.rs`）。
//!
//! # 追加的三条 (W3 复核追加)
//!
//! ⑦ **多层树**：取消要**真的**杀掉孙进程 ⇒ 用**平台进程句柄**判「已终止」
//! （**强信号**；「快照里 pid 消失」只是弱信号 ⇒ 不用它下结论）；
//! ⑧ **系统进程受保护**：`cancel_process_tree(4)` ⇒ 系统进程**绝不被动手**；
//! ⑨ **静态契约**：平台机制名**只**落后端块；未实现平台**显式** `Unsupported`；
//! Windows 后端**不提供**进程启动时刻（⇒ 台账退化为 pid 级，方向是「宁可不动也
//! 不误杀」）而 Linux **提供**（⇒ 复用防护可闭环）。

/// Windows 真机用例 (只在 Windows 上编译与执行)。
#[cfg(windows)]
mod windows_only {
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use apeireth_tools_canonical::process::{
        cancel_process_tree, process_tree_snapshot, process_tree_support, GroupContainment,
        ProcessTreeCancel, ProcessTreeError, ProcessTreeSupport,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    };

    /// 子进程测试用的 helper 可执行文件路径。
    ///
    /// 与既有 `tests/process_executor.rs` 同款做法: `option_env!` + 同目录回退
    /// (部分 cargo 版本不注入该编译期变量, `env!` 会直接编译失败)。
    fn helper() -> PathBuf {
        if let Some(path) = option_env!("CARGO_BIN_EXE_sandbox-test-child") {
            return PathBuf::from(path);
        }
        let mut dir = std::env::current_exe().expect("current_exe");
        dir.pop(); // deps/
        dir.pop(); // <profile>/
        dir.push("sandbox-test-child.exe");
        dir
    }

    /// 子进程句柄: 测试结束必收尸 (不留悬挂进程)。
    struct ChildGuard(Child);

    impl ChildGuard {
        fn pid(&self) -> u32 {
            self.0.id()
        }

        /// 进程是否还活着 (已退出则收尸)。
        fn is_alive(&mut self) -> bool {
            matches!(self.0.try_wait(), Ok(None))
        }

        /// 等进程**真的**退出 (终止送达 ⇒ 进程被系统收掉需要一点时间)。
        fn wait_for_exit(&mut self, timeout_ms: u64) -> bool {
            let deadline = Instant::now() + Duration::from_millis(timeout_ms);
            loop {
                if matches!(self.0.try_wait(), Ok(Some(_))) {
                    return true;
                }
                if Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// 真起一个子进程 (helper 的固定模式, 不接受任何自由命令串)。
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

    /// 轮询直到探针给出结果 (进程枚举是**时点快照** ⇒ 刚起的进程可能还没被枚举到)。
    fn wait_until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(value) = probe() {
                return Some(value);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// 平台进程句柄 (RAII: `Drop` 关句柄 ⇒ 任何提前返回都不漏句柄)。
    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// 开一个进程句柄 (`None` = 平台拒绝查询 ⇒ 如实说「查不到」, 不猜)。
    fn open_process_handle(pid: u32, access: u32) -> Option<OwnedHandle> {
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            None
        } else {
            Some(OwnedHandle(handle))
        }
    }

    /// 等某个进程对象变成「已终止」(句柄 signaled = **强信号**)。
    ///
    /// 对比「快照里 pid 消失」—— 那是**弱信号** (可能是枚举时序问题) ⇒ 不用它下结论。
    fn wait_process_exit(handle: &OwnedHandle, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let wait = unsafe { WaitForSingleObject(handle.0, 20) };
            if wait == WAIT_OBJECT_0 {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
        }
    }

    /// ① 快照**自己**（本进程）: 本进程必须出现在自己的快照里。
    #[test]
    fn the_snapshot_captures_the_current_process() {
        let self_pid = std::process::id();
        let snapshot = process_tree_snapshot(self_pid).expect("self snapshot must succeed");

        assert_eq!(snapshot.root_pid, self_pid);
        let entry = snapshot
            .find(self_pid)
            .expect("the current process must be listed");
        assert_eq!(entry.pid, self_pid);
        assert!(
            !entry.command_line.is_empty(),
            "the current process must report a command line"
        );
        assert_ne!(
            entry.status,
            apeireth_tools_canonical::process::ProcessTreeStatus::Unknown,
            "a live process must not report an unknown status"
        );
    }

    /// ② 真 spawn 子进程 ⇒ 快照看得见**父子树**（子进程的父 = 本进程）。
    #[test]
    fn a_real_child_appears_in_the_parent_child_tree() {
        let child = spawn_child(&["sleep", "30"]);
        let pid = child.pid();

        // 以子进程为根: 它必须被列为本进程的子。
        let subtree = process_tree_snapshot(pid).expect("child snapshot must succeed");
        let entry = subtree.find(pid).expect("the child must be listed");
        assert_eq!(
            entry.parent_pid,
            std::process::id(),
            "the child must be reported as a child of this session"
        );

        // 以本进程为根: 子进程必须出现在这棵树里。
        let whole = process_tree_snapshot(std::process::id()).expect("self snapshot must succeed");
        assert!(
            whole.find(pid).is_some(),
            "the real child must be visible in the session tree"
        );
    }

    /// ③ 真取消 ⇒ 子进程**真的没了**（不是「记了个数」而已）。
    #[test]
    fn cancelling_a_real_child_actually_terminates_it() {
        let mut child = spawn_child(&["sleep", "30"]);
        let pid = child.pid();

        let result = cancel_process_tree(pid).expect("cancel must succeed");
        match result {
            ProcessTreeCancel::Cancelled {
                root_pid,
                terminated,
                failed,
                ..
            } => {
                assert_eq!(root_pid, pid);
                assert!(terminated.contains(&pid), "the child must be terminated");
                assert!(failed.is_empty(), "no member should fail: {failed:?}");
            }
            other => panic!("expected Cancelled, got {other:?}"),
        }

        assert!(
            child.wait_for_exit(10_000),
            "the cancelled child must really be gone"
        );
        assert!(!child.is_alive(), "the cancelled child must not be alive");
    }

    /// ④ **不误杀**: 取消一棵 ⇒ 别的进程不受影响; 重复取消**零副作用**。
    ///
    /// 台账按 pid 认树 ⇒ 同一 pid 的第二次取消直接回放首次结果, **不再发信号**
    /// ⇒ 既不会误杀无关进程, 也不会二次动同一棵树。
    #[test]
    fn cancelling_never_kills_an_unrelated_process_and_stays_idempotent() {
        let victim = spawn_child(&["sleep", "30"]);
        let mut bystander = spawn_child(&["sleep", "30"]);

        let first = cancel_process_tree(victim.pid()).expect("cancel must succeed");
        assert!(
            first.caused_termination(),
            "the first cancel must terminate the tree"
        );
        assert!(
            bystander.is_alive(),
            "cancelling one tree must not touch an unrelated process"
        );

        let second = cancel_process_tree(victim.pid()).expect("repeat cancel must succeed");
        assert!(
            matches!(second, ProcessTreeCancel::AlreadyCancelled { .. }),
            "a repeated cancel must be an idempotent replay: {second:?}"
        );
        assert!(
            !second.caused_termination(),
            "a repeated cancel must produce no termination side effect"
        );
        assert!(
            bystander.is_alive(),
            "a repeated cancel must not touch an unrelated process either"
        );
    }

    /// ⑤ 收容组归属: Windows 上**必须**是 `Undetermined`。
    ///
    /// 执行侧的收容容器句柄不对外暴露 ⇒ 本后端拿不到「某进程是否仍在该容器内」的
    /// 判据 ⇒ 一律记 `Undetermined` ⇒ **不许**谎报 `Contained`（也不许谎报 `Escaped`）。
    #[test]
    fn the_containment_group_verdict_is_undetermined_on_windows() {
        let child = spawn_child(&["sleep", "30"]);
        let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");

        assert!(!snapshot.entries.is_empty());
        for entry in &snapshot.entries {
            assert_eq!(
                entry.group_containment,
                GroupContainment::Undetermined,
                "windows cannot decide containment membership; pid {} must not claim it",
                entry.pid
            );
            assert!(
                !entry.escaped_group(),
                "windows must not claim an escape it cannot decide (pid {})",
                entry.pid
            );
        }
    }

    /// ⑥ 平台支持档**显式**: Windows = 支持 (快照与取消都真跑), 不返回空快照冒充。
    ///
    /// 未实现平台 (macOS / 其他) 那一侧由跨平台矩阵钉死
    /// (`tests/process_tree_snapshot.rs::the_platform_support_matrix_matches_the_implementation`),
    /// 那个用例在本仓的三 OS 工作流里**真机**跑 (含 macOS runner) ⇒ 不许回归。
    #[test]
    fn the_windows_platform_reports_the_face_as_supported() {
        assert_eq!(
            process_tree_support(),
            ProcessTreeSupport::Enumerable,
            "windows implements the snapshot face"
        );

        let child = spawn_child(&["sleep", "30"]);
        let snapshot = process_tree_snapshot(child.pid()).expect("snapshot must succeed");
        assert!(
            !snapshot.is_empty(),
            "a supported platform must yield a real tree, not an empty one"
        );
    }

    /// ⑦ 多层树取消 ⇒ **孙进程确实退出** (平台句柄 signaled = **强信号**)。
    ///
    /// 「快照里 pid 消失」只是**弱信号** (枚举时序) ⇒ 不用它下结论; 这里握住的
    /// 是**平台进程句柄**: 句柄变成 signaled ⇒ 进程**确实**终止了。
    #[test]
    fn cancelling_a_multi_generation_tree_really_kills_the_grandchild() {
        let mut child = spawn_child(&["spawn-child", "sleep", "30"]);
        let child_pid = child.pid();

        // 进程枚举是时点快照 ⇒ 刚起的进程可能还没被枚举到 ⇒ 轮询。
        let grandchild_pid = wait_until(Duration::from_secs(15), || {
            let snapshot = process_tree_snapshot(std::process::id()).ok()?;
            snapshot
                .entries
                .iter()
                .find(|entry| entry.parent_pid == child_pid)
                .map(|entry| entry.pid)
        })
        .expect("the grandchild must show up in the snapshot");

        // **取消前**握住孙进程的句柄 (终止后 pid 可能已被回收 ⇒ 先握住才查得到)。
        let grandchild_handle = open_process_handle(grandchild_pid, PROCESS_SYNCHRONIZE)
            .expect("the grandchild handle must open");

        let result = cancel_process_tree(child_pid).expect("cancel must succeed");
        match &result {
            ProcessTreeCancel::Cancelled {
                terminated, failed, ..
            } => {
                assert!(
                    terminated.contains(&child_pid),
                    "the child must be terminated: {terminated:?}"
                );
                assert!(
                    terminated.contains(&grandchild_pid),
                    "the whole tree must be terminated: {terminated:?}"
                );
                assert!(failed.is_empty(), "no member should fail: {failed:?}");
            }
            other => panic!("expected Cancelled, got {other:?}"),
        }

        assert!(
            child.wait_for_exit(10_000),
            "the cancelled child must actually be gone"
        );
        assert!(
            wait_process_exit(&grandchild_handle, Duration::from_secs(10)),
            "the grandchild must really have exited (process handle signaled)"
        );
    }

    /// ⑧ 系统进程受保护: `cancel_process_tree(4)` ⇒ 系统进程**绝不被动手**。
    ///
    /// Windows 上系统进程的父 pid 为 0 ⇒ 归属证明先挡下 (`NotControlled`); 若平台
    /// 路径不同, 也可能是平台保护闸 (`Refused`) 或「不在进程表」(`NotRunning`)。
    /// ⇒ **唯一必须成立的不变式**: **绝不**把它报成「已取消」。
    #[test]
    fn the_system_process_is_never_signalled() {
        match cancel_process_tree(4) {
            Err(ProcessTreeError::NotControlled { pid }) => assert_eq!(pid, 4),
            Err(ProcessTreeError::Refused { pid, .. }) => assert_eq!(pid, 4),
            Ok(ProcessTreeCancel::NotRunning { root_pid }) => assert_eq!(root_pid, 4),
            other => panic!("pid 4 must never be reported as cancelled: {other:?}"),
        }

        // 系统进程必须**还在** (查得到才下结论; 查不到就如实说「查不到」)。
        match open_process_handle(4, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE) {
            Some(handle) => assert!(
                !wait_process_exit(&handle, Duration::from_millis(0)),
                "the system process must still be running"
            ),
            None => eprintln!(
                "note: the platform refused a query handle for pid 4 ⇒ \
                 the 'still running' half of this case is not observable here"
            ),
        }
    }

    /// ⑨ 静态契约 (跨平台都成立, 这里再钉一遍 Windows 侧):
    ///
    /// - 公共契约面**不得**出现平台机制名 (机制名**只**落平台后端块);
    /// - 未实现平台 (macOS / 其它) 必须**显式** `Unsupported`, 且**不得**返回空
    ///   快照冒充成功 (**macOS 那条不许回归**);
    /// - Windows 后端**不提供**进程启动时刻 ⇒ 取消台账的「(pid + 启动时刻)」
    ///   **退化为** pid 级 ⇒ 方向是「宁可不动也不误杀」(代价: 漏杀 ⇒ 台账过期兜底);
    ///   Linux 后端**提供**启动时刻 ⇒ 归属判定可逐跳校验 (复用防护可闭环)。
    #[test]
    fn the_backend_contract_holds_on_windows() {
        let source = include_str!("../src/process/snapshot.rs");
        let contract = source
            .split("mod backend")
            .next()
            .expect("the contract half must exist");

        for forbidden in [
            "OpenProcess",
            "TerminateProcess",
            "CreateToolhelp32Snapshot",
            "Process32FirstW",
            "Process32NextW",
            "GetExitCodeProcess",
            "JobObject",
            "CreateProcessW",
        ] {
            assert!(
                !contract.contains(forbidden),
                "the public contract must not name the platform mechanism {forbidden}"
            );
        }

        let windows_block = backend_block(&source, "#[cfg(windows)]");
        for owned in [
            "OpenProcess",
            "TerminateProcess",
            "CreateToolhelp32Snapshot",
            "Process32FirstW",
            "Process32NextW",
        ] {
            assert!(
                windows_block.contains(owned),
                "the windows backend must own the platform mechanism {owned}"
            );
        }
        for other in [
            "#[cfg(target_os = \"linux\")]",
            "#[cfg(target_os = \"macos\")]",
            "#[cfg(not(any(windows, target_os = \"linux\", target_os = \"macos\")))]",
        ] {
            let block = backend_block(&source, other);
            for leaked in [
                "OpenProcess",
                "TerminateProcess",
                "CreateToolhelp32Snapshot",
                "Process32FirstW",
                "Process32NextW",
            ] {
                assert!(
                    !block.contains(leaked),
                    "{other}: the windows platform mechanism {leaked} must not leak into another backend"
                );
            }
        }

        for other in [
            "#[cfg(target_os = \"macos\")]",
            "#[cfg(not(any(windows, target_os = \"linux\", target_os = \"macos\")))]",
        ] {
            let block = backend_block(&source, other);
            assert!(
                block.contains("ProcessTreeSupport::Unsupported"),
                "{other}: an unimplemented platform must stay explicitly unsupported"
            );
            assert!(
                block.contains("ProcessTreeError::Unsupported"),
                "{other}: an unimplemented platform must fail explicitly"
            );
            assert!(
                !block.contains("Ok(Vec::new())") && !block.contains("Ok(vec![])"),
                "{other}: an unimplemented platform must never return an empty snapshot pretending success"
            );
        }

        // 平台边界 (如实钉住): Windows **不提供**启动时刻; Linux **提供**。
        assert!(
            windows_block.contains("start_time: None"),
            "the windows backend supplies no process start time ⇒ the ledger degenerates to pid level"
        );
        let linux_block = backend_block(&source, "#[cfg(target_os = \"linux\")]");
        assert!(
            linux_block.contains("start_time: Some("),
            "the linux backend supplies the process start time ⇒ the reuse guard can close the loop"
        );
    }

    /// 取源码里某个平台后端块 (`#[cfg(...)]` 标记之后、下一个 `#[cfg(` 之前)。
    fn backend_block<'a>(source: &'a str, marker: &str) -> &'a str {
        let start = source.find(marker).expect("the backend marker must exist");
        let rest = &source[start..];
        let end = rest[marker.len()..]
            .find("#[cfg(")
            .map(|offset| marker.len() + offset)
            .unwrap_or(rest.len());
        &rest[..end]
    }
}

/// 非 Windows 平台: 本套件**不适用** (诚实占位, 不冒充通过)。
#[cfg(not(windows))]
#[test]
fn the_windows_only_suite_does_not_run_off_windows() {
    assert!(
        !cfg!(windows),
        "this suite is windows-only; the real assertions run on the windows runner"
    );
    // 本平台的支持档仍由跨平台矩阵用例钉死 (macOS = 显式 `Unsupported`)。
    let support = apeireth_tools_canonical::process::process_tree_support();
    if cfg!(target_os = "linux") {
        assert_eq!(
            support,
            apeireth_tools_canonical::process::ProcessTreeSupport::Enumerable,
            "linux implements the snapshot face"
        );
    }
}
