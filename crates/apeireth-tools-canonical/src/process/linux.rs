//! Linux process backend.
//!
//! The Linux backend does **not** promise cgroup/seccomp/namespace sandboxing in
//! this phase. It provides:
//!
//! * a real process-group boundary: the child is made a process-group leader
//!   before `exec`, and timeout termination kills the entire group
//! * `PR_SET_PDEATHSIG` so an orphaned child is killed if the executor parent
//!   dies
//! * `PR_SET_NO_NEW_PRIVS` when the caller requires privilege reduction
//! * `setrlimit` for address-space (`RLIMIT_AS`), CPU (`RLIMIT_CPU`), file
//!   size (`RLIMIT_FSIZE`), and process count (`RLIMIT_NPROC`, UID-scoped)
//!
//! Every containment primitive is applied in `pre_exec`, before the child
//! image runs. If pre-exec setup fails, spawn fails and no child is returned.
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
        EnforcementLevel::Partial,
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

    let parent_pid = std::process::id();
    let limits = request.limits.clone();
    let reduce_privileges = request
        .isolation()
        .requires(IsolationCapability::PrivilegeReduction)
        .is_some();

    unsafe {
        command.pre_exec(move || {
            // New process group: the child's pid becomes the pgid. Descendants
            // inherit the group unless they explicitly create their own.
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }

            // If the executor parent dies, kill the child immediately. The
            // getppid check closes the fork/prctl race: if the parent already
            // died, the child refuses to exec.
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent_pid as i32 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "parent died before pre-exec containment completed",
                ));
            }

            if reduce_privileges {
                // PR_SET_NO_NEW_PRIVS is not a seccomp sandbox and is not
                // advertised as one. It is a privilege-escalation barrier for
                // restricted/untrusted postures.
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }

            set_rlimits(&limits)?;
            Ok(())
        });
    }

    let child = command.spawn().map_err(|e| ProcessError::SpawnFailed {
        executable: request.executable.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    supervise(LinuxChild { child }, request, enforcement)
}

unsafe fn set_rlimits(limits: &ProcessLimits) -> io::Result<()> {
    if let Some(bytes) = limits.max_process_memory_bytes {
        set_rlimit(libc::RLIMIT_AS, bytes)?;
    }
    if let Some(active) = limits.max_active_processes {
        // RLIMIT_NPROC is UID-scoped, not tree-scoped. It is advertised as
        // PARTIAL only and tests must avoid depending on global UID state.
        set_rlimit(libc::RLIMIT_NPROC, active as libc::rlim_t)?;
    }
    if let Some(seconds) = limits.max_cpu_seconds {
        set_rlimit(libc::RLIMIT_CPU, seconds as libc::rlim_t)?;
    }
    if let Some(bytes) = limits.max_file_size_bytes {
        set_rlimit(libc::RLIMIT_FSIZE, bytes as libc::rlim_t)?;
    }
    Ok(())
}

unsafe fn set_rlimit(resource: libc::__rlimit_resource_t, value: libc::rlim_t) -> io::Result<()> {
    let rlim = libc::rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    if libc::setrlimit(resource, &rlim) != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

struct LinuxChild {
    child: Child,
}

impl LinuxChild {
    fn group_signal(&self, signal: libc::c_int) -> io::Result<()> {
        let pid = self.child.id() as i32;
        if unsafe { libc::kill(-pid, signal) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl ManagedChild for LinuxChild {
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
