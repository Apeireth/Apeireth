//! macOS process backend.
//!
//! macOS has no public, stable, arbitrary-subprocess sandbox primitive for
//! filesystem/network isolation in this phase. This backend therefore reports
//! those capabilities as unsupported and never invents a private-API sandbox.
//!
//! It does provide:
//!
//! * a real process-group boundary: the child is made a process-group leader
//!   before `exec`, and timeout termination kills the entire group
//! * a **parent-death guard**: macOS has no `prctl(PR_SET_PDEATHSIG)`, so a
//!   dedicated guardian process watches the executor with
//!   `kqueue(EVFILT_PROC/NOTE_EXIT)` and kills the whole process group when
//!   the executor dies (P3-8, 2026-10-31)
//! * a `getppid()` re-check in `pre_exec` (the macOS twin of
//!   `linux.rs:109`) so a child never `exec`s after its parent already died
//! * `setrlimit` for address-space (`RLIMIT_AS`), CPU (`RLIMIT_CPU`), file
//!   size (`RLIMIT_FSIZE`), and process count (`RLIMIT_NPROC`, UID-scoped)
//!
//! All setup happens in `pre_exec`, before the child image runs.
#![allow(unsafe_code)]

use std::io;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use super::{
    apply_request_to_command, supervise, ChildStatus, EnforcementLevel, IsolationCapabilities,
    IsolationCapability, ManagedChild, PlatformEnforcement, ProcessError, ProcessLimits,
    ProcessRequest, ProcessResult,
};

pub(crate) fn capabilities() -> IsolationCapabilities {
    let mut caps = IsolationCapabilities::default();
    caps.set(
        IsolationCapability::StructuredSpawn,
        EnforcementLevel::Enforced,
    );
    caps.set(IsolationCapability::ExplicitCwd, EnforcementLevel::Enforced);
    caps.set(IsolationCapability::Timeout, EnforcementLevel::Enforced);
    caps.set(IsolationCapability::StdoutLimit, EnforcementLevel::Enforced);
    caps.set(IsolationCapability::StderrLimit, EnforcementLevel::Enforced);
    caps.set(
        IsolationCapability::EnvironmentIsolation,
        EnforcementLevel::Enforced,
    );
    caps.set(
        IsolationCapability::ProcessTreeContainment,
        EnforcementLevel::Partial,
    );
    caps.set(IsolationCapability::MemoryLimit, EnforcementLevel::Partial);
    caps.set(
        IsolationCapability::ProcessCountLimit,
        EnforcementLevel::Partial,
    );
    caps.set(IsolationCapability::CpuLimit, EnforcementLevel::Enforced);
    caps.set(
        IsolationCapability::FileSizeLimit,
        EnforcementLevel::Enforced,
    );
    caps.set(
        IsolationCapability::PrivilegeReduction,
        EnforcementLevel::Unsupported,
    );
    caps.set(
        IsolationCapability::FilesystemIsolation,
        EnforcementLevel::Unsupported,
    );
    caps.set(
        IsolationCapability::NetworkIsolation,
        EnforcementLevel::Unsupported,
    );
    caps.set(
        IsolationCapability::FailClosedPreExecutionContainment,
        EnforcementLevel::Enforced,
    );
    caps
}

pub(crate) fn spawn_and_supervise(
    request: &ProcessRequest,
    enforcement: PlatformEnforcement,
) -> Result<ProcessResult, ProcessError> {
    let mut command = Command::new(&request.executable);
    command.args(&request.args);
    apply_request_to_command(&mut command, request)?;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let limits = request.limits.clone();
    // executor 自身的 pid: 父死亡守护盯的就是它 (macOS 没有 prctl(PDEATHSIG)).
    let parent_pid = std::process::id();

    // 父死亡守护: 必须在 spawn **之前** 起 (守护进程要盯的是 executor 本身,
    // 与目标子进程无关); 起不来 ⇒ fail-closed (拒绝执行, 不裸跑)。
    let guard = ParentDeathGuard::start(parent_pid as i32).map_err(|e| {
        ProcessError::ContainmentFailed(format!("parent-death guard setup failed: {e}"))
    })?;

    // 守护进程的管道写端 (子进程在 pre_exec 里也用它把 pgid 交出去)。
    let guard_write_fd = guard.write_fd;
    unsafe {
        command.pre_exec(move || {
            // New process group: the child's pid becomes the pgid. Descendants
            // inherit the group unless they explicitly create their own.
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }

            // 父进程复查 (抄 linux.rs:109): executor 已死 ⇒ 拒绝 exec。
            // 原先 macOS 侧**没有**这一步 ⇒ executor 在 fork/exec 之间被杀时,
            // 子进程照样 exec 出来 (逃出 max_runtime 约束)。
            if libc::getppid() != parent_pid as i32 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "parent died before pre-exec containment completed",
                ));
            }

            // 子进程在 exec **之前**把自己的 pid (= 进程组 leader) 交给守护
            // 进程: 否则「executor 在 spawn 返回后、publish 之前被杀」的 ~1ms
            // 窗口里, 守护进程还不知道该杀哪个组 ⇒ 子进程会逃出去。
            write_pid(guard_write_fd, libc::getpid());

            set_rlimits(&limits)?;
            Ok(())
        });
    }

    let child = command.spawn().map_err(|e| ProcessError::SpawnFailed {
        executable: request.executable.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    // 目标进程组 leader = 子进程 pid (pre_exec 里 setpgid(0,0) 让它自任组长).
    guard.publish_group(child.id() as libc::pid_t);

    supervise(MacOsChild { child }, request, enforcement)
}

/// **父死亡守护** (macOS 没有 `prctl(PR_SET_PDEATHSIG)`)。
///
/// 原先错在: `macos.rs` 只装 `setpgid` + `setrlimit` ⇒ executor 崩溃/被杀时
/// 子进程**存活**, 逃出 `max_runtime` 约束 (Linux 兄弟用
/// `prctl(PR_SET_PDEATHSIG)` + `getppid` 复查把这个洞关掉了)。
///
/// 现在: spawn 之前 fork 一个**守护进程**, 它用
/// `kqueue(EVFILT_PROC/NOTE_EXIT)` 盯着 executor; executor 一死就
/// `kill(-pgid, SIGKILL)` 杀掉整个进程组 (目标可能已 fork 出后代)。
/// 守护进程同时盯着目标进程 —— 目标先退出时守护进程自行退出, 不留常驻进程。
///
/// 为什么是「进程」而不是「线程」: 线程随 executor 一起死, 救不了 executor。
/// 为什么不用 `procctl(PROC_PDEATHSIG_CTL)`: 那是 FreeBSD 的接口 —— 本仓锁定的
/// `libc` (0.2.190) 的 apple 目标里**没有** `libc::procctl` / `PROC_PDEATHSIG_CTL`
/// 这两个符号 (只有 freebsd / dragonfly 分支有), 而改 `Cargo.toml` 不在本任务
/// 的可改文件白名单内 ⇒ 只能用 kqueue。
struct ParentDeathGuard {
    /// 管道写端: 把目标进程组 leader 的 pid 交给守护进程。
    write_fd: libc::c_int,
    /// 守护进程 pid (executor 的子进程 ⇒ 退出后必须 waitpid 回收)。
    pid: libc::pid_t,
}

impl ParentDeathGuard {
    /// 起守护进程 (fork + 管道)。
    fn start(executor_pid: i32) -> io::Result<Self> {
        let mut fds: [libc::c_int; 2] = [0, 0];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let (read_fd, write_fd) = (fds[0], fds[1]);
        // 两个 fd 都设 CLOEXEC (macOS 没有 pipe2, 只能 fcntl): 目标子进程
        // exec 之后不继承管道写端。
        for fd in [read_fd, write_fd] {
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        }

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            // —— 守护进程 (fork 的子进程) ——
            // 只用 async-signal-safe 调用 (无分配、无锁): 读 pid → kqueue 等事件。
            unsafe {
                guardian_main(executor_pid, read_fd);
                libc::_exit(0);
            }
        }
        // —— executor 侧 ——
        unsafe { libc::close(read_fd) };
        Ok(Self { write_fd, pid })
    }

    /// 把目标进程组 leader 的 pid 告诉守护进程 (子进程在 pre_exec 里也写过
    /// 一份 —— 值相同, 守护进程只读前 4 个字节 ⇒ 幂等)。
    ///
    /// 失败 (管道写端已坏) 不致命: 守护进程读不到 pid 就直接退出, 此时
    /// executor 仍活着 ⇒ 没有父死亡场景需要处理。
    fn publish_group(&self, pgid: libc::pid_t) {
        unsafe {
            write_pid(self.write_fd, pgid);
        }
    }
}

impl Drop for ParentDeathGuard {
    fn drop(&mut self) {
        // 关掉写端: 守护进程若还在等 pid, 会读到 EOF 并自行退出。
        unsafe { libc::close(self.write_fd) };
        // 守护进程是 executor 的子进程 ⇒ 退出后必须 waitpid 回收 (否则僵尸堆积)。
        // 不能在这里阻塞等: 守护进程要等目标进程退出才退出 ⇒ 交给后台线程。
        let pid = self.pid;
        // `let _ =` : JoinHandle 是 must_use; 这里要的是「脱离式」回收线程。
        let _ = std::thread::spawn(move || {
            let mut status = 0;
            unsafe { libc::waitpid(pid, &mut status, 0) };
        });
    }
}

/// 守护进程主体: 读目标进程组 → kqueue 盯 executor/目标 → executor 死则杀组。
///
/// # Safety
/// 只允许在 fork 出来的子进程里调用 (fork 之后只有调用线程存活); 内部只使用
/// async-signal-safe 的系统调用, 不碰分配器与任何锁。
unsafe fn guardian_main(executor_pid: i32, pipe_read_fd: libc::c_int) {
    // 1) 把继承来的 fd 收干净: 只留管道读端。
    //    - 必须关 0/1/2: 守护进程不产出任何输出, 攥着 executor 的 stdout 会让
    //      对端迟迟等不到 EOF。
    //    - 必须关其余 fd: 守护进程不该替 executor 攥着套接字/文件句柄。
    //    注意: 守护进程是**在 spawn 之前** fork 的 ⇒ 它**没有**继承 std 的
    //    exec 错误管道 (那根管道在 fork 之后才创建) ⇒ 不存在"提前 EOF 把
    //    exec 失败吞掉"的竞态。
    let pipe_fd = move_to(pipe_read_fd, GUARD_PIPE_FD);
    let kq = libc::kqueue();
    let kq_fd = move_to(kq, GUARD_KQ_FD);
    close_other_fds(pipe_fd, kq_fd);

    // 2) 先登记 executor 的退出事件 (守护进程必须在"来不及武装"的窗口里也
    //    不漏看 ⇒ 先 arm 再等)。这一步**不阻塞** (零超时): 零超时只登记不等待,
    //    事件仍会排队给下一次 kevent。
    let changes = [proc_exit_event(executor_pid as libc::uintptr_t)];
    let mut events = [zero_kevent(), zero_kevent()];
    let no_wait: libc::timespec = std::mem::zeroed();
    let armed = libc::kevent(
        kq_fd,
        changes.as_ptr(),
        changes.len() as libc::c_int,
        events.as_mut_ptr(),
        events.len() as libc::c_int,
        &no_wait,
    );
    if armed < 0 {
        return;
    }
    // EV_CLEAR: 事件被取出即清零 ⇒ 这里取到的 executor 事件必须就地处理,
    // 不能指望下一次 kevent 再报一次。
    let executor_already_dead = events
        .iter()
        .take(armed as usize)
        .any(|event| event.ident == executor_pid as libc::uintptr_t);

    // 3) 读目标进程组 leader 的 pid (executor 在 spawn 成功后写入)。
    let Some(pgid) = read_pgid(pipe_fd) else {
        return;
    };
    if executor_already_dead {
        // executor 在守护进程武装之前就死了 ⇒ 立刻杀掉整个进程组。
        libc::kill(-pgid, libc::SIGKILL);
        return;
    }

    // 4) 补登记目标进程的退出事件并等待: 目标先退出 ⇒ 守护进程自行退出 (不
    //    留常驻进程); executor 先退出 ⇒ 杀掉整个进程组。
    let target_change = [proc_exit_event(pgid as libc::uintptr_t)];
    let mut target_events = [zero_kevent(), zero_kevent()];
    let n = libc::kevent(
        kq_fd,
        target_change.as_ptr(),
        target_change.len() as libc::c_int,
        target_events.as_mut_ptr(),
        target_events.len() as libc::c_int,
        std::ptr::null(),
    );
    if n <= 0 {
        return;
    }
    for event in target_events.iter().take(n as usize) {
        if event.ident == executor_pid as libc::uintptr_t {
            // executor 死了 ⇒ 杀掉整个进程组 (目标可能已 fork 出后代)。
            libc::kill(-pgid, libc::SIGKILL);
            break;
        }
    }
}

/// 守护进程保留的管道读端 fd 号 (固定 ⇒ 关 fd 循环好写)。
const GUARD_PIPE_FD: libc::c_int = 4;
/// 守护进程保留的 kqueue fd 号。
const GUARD_KQ_FD: libc::c_int = 3;

/// 把 pid 写进管道 (只用 write(2) ⇒ async-signal-safe, 可以在 pre_exec 里调)。
///
/// # Safety
/// `fd` 必须是管道写端; 写失败 (管道已关) 无害 (调用方据此放弃守护)。
unsafe fn write_pid(fd: libc::c_int, pid: libc::pid_t) {
    let bytes = pid.to_ne_bytes();
    libc::write(fd, bytes.as_ptr().cast::<libc::c_void>(), bytes.len());
}

/// 把 `fd` 复制到 `target` (target 上的旧 fd 在守护进程里被关掉, 无副作用)。
///
/// # Safety
/// 只在 fork 出来的守护进程里调用。
unsafe fn move_to(fd: libc::c_int, target: libc::c_int) -> libc::c_int {
    if fd == target {
        return fd;
    }
    if libc::dup2(fd, target) < 0 {
        return fd;
    }
    libc::close(fd);
    target
}

/// 关掉除 `keep` 之外的全部继承 fd (0/1/2 也关)。
///
/// # Safety
/// 只在 fork 出来的守护进程里调用。
unsafe fn close_other_fds(keep_a: libc::c_int, keep_b: libc::c_int) {
    for fd in 0..3 {
        libc::close(fd);
    }
    // 上限兜底 4096: executor 的 fd 通常远少于这个数, 而 rlim 可能被调得极大
    // (逐个 close 上百万个 fd 会拖慢守护进程武装)。
    for fd in 5..=4096 {
        if fd != keep_a && fd != keep_b {
            libc::close(fd);
        }
    }
}

/// 从管道读端读出目标进程组 leader 的 pid (EOF ⇒ executor 没来得及写 ⇒ 放弃)。
///
/// # Safety
/// 只在 fork 出来的守护进程里调用。
unsafe fn read_pgid(fd: libc::c_int) -> Option<i32> {
    let mut buf = [0u8; 4];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = libc::read(
            fd,
            buf[filled..].as_mut_ptr().cast::<libc::c_void>(),
            buf.len() - filled,
        );
        if n <= 0 {
            return None;
        }
        filled += n as usize;
    }
    Some(i32::from_ne_bytes(buf))
}

/// 构造一条 "进程退出" 的 kevent 登记项 (`EVFILT_PROC` + `NOTE_EXIT`)。
fn proc_exit_event(pid: libc::uintptr_t) -> libc::kevent {
    // `libc::kevent` 是 `#[repr(packed(4))] #[exhaustive]` ⇒ 不能用结构体字面量,
    // 只能 zeroed 之后逐字段赋值 (packed 字段赋值合法, 只是不能取引用)。
    let mut event: libc::kevent = unsafe { std::mem::zeroed() };
    event.ident = pid;
    event.filter = libc::EVFILT_PROC;
    event.flags = libc::EV_ADD | libc::EV_CLEAR;
    event.fflags = libc::NOTE_EXIT;
    event
}

/// 一个全零的 kevent (事件缓冲区用)。
fn zero_kevent() -> libc::kevent {
    // SAFETY: `kevent` 只含标量与一个可空裸指针 ⇒ 全零是合法初值。
    unsafe { std::mem::zeroed() }
}

unsafe fn set_rlimits(limits: &ProcessLimits) -> io::Result<()> {
    if let Some(bytes) = limits.max_process_memory_bytes {
        set_rlimit(libc::RLIMIT_AS, libc::rlim_t::from(bytes))?;
    }
    if let Some(active) = limits.max_active_processes {
        // RLIMIT_NPROC is UID-scoped, not tree-scoped. Advertised as PARTIAL.
        set_rlimit(libc::RLIMIT_NPROC, libc::rlim_t::from(active))?;
    }
    if let Some(seconds) = limits.max_cpu_seconds {
        set_rlimit(libc::RLIMIT_CPU, libc::rlim_t::from(seconds))?;
    }
    if let Some(bytes) = limits.max_file_size_bytes {
        set_rlimit(libc::RLIMIT_FSIZE, libc::rlim_t::from(bytes))?;
    }
    Ok(())
}

unsafe fn set_rlimit(resource: libc::c_int, value: libc::rlim_t) -> io::Result<()> {
    let rlim = libc::rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    if libc::setrlimit(resource, &rlim) != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

struct MacOsChild {
    child: Child,
}

impl MacOsChild {
    fn group_signal(&self, signal: libc::c_int) -> io::Result<()> {
        let pid = self.child.id() as i32;
        if unsafe { libc::kill(-pid, signal) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl ManagedChild for MacOsChild {
    fn try_wait(&mut self) -> Result<Option<ChildStatus>, ProcessError> {
        self.child
            .try_wait()
            .map(|o| o.map(status))
            .map_err(io_error)
    }

    fn wait(&mut self) -> Result<ChildStatus, ProcessError> {
        self.child.wait().map(status).map_err(io_error)
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        // Kill the entire process group. Descendants that created their own
        // process group can escape; hence process_tree_containment is PARTIAL.
        match self.group_signal(libc::SIGKILL) {
            Ok(()) => Ok(()),
            Err(_) => self.child.kill().map_err(io_error),
        }
    }

    fn terminate_remaining(&mut self) -> Result<(), ProcessError> {
        match self.group_signal(libc::SIGKILL) {
            Ok(()) => Ok(()),
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }

    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>)
    }
}

fn status(status: std::process::ExitStatus) -> ChildStatus {
    ChildStatus {
        code: status.code(),
    }
}

fn io_error(e: io::Error) -> ProcessError {
    ProcessError::Io(e.to_string())
}
