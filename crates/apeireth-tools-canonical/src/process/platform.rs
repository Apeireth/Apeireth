//! Non-Windows process execution: guardrails only.
//!
//! This platform implementation enforces timeout, bounded stdout/stderr,
//! working directory and environment policy. It does **not** provide OS
//! privilege isolation (no namespaces, seccomp, cgroups, sandbox-exec) and it
//! does not claim to be a sandbox. Timeout termination kills the direct child;
//! process-tree termination depends on the child not leaving descendants.

use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};

use super::{
    apply_request_to_command, supervise, ContainmentKind, ManagedChild, PlatformEnforcement,
    PlatformKind, ProcessError, ProcessRequest, ProcessResult,
};

pub(crate) fn spawn_and_supervise(request: &ProcessRequest) -> Result<ProcessResult, ProcessError> {
    let mut command = Command::new(&request.executable);
    command.args(&request.args);
    apply_request_to_command(&mut command, request)?;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let child = command.spawn().map_err(|e| ProcessError::SpawnFailed {
        executable: request.executable.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    supervise(
        GenericChild { child },
        request,
        PlatformEnforcement {
            platform: PlatformKind::NonWindows,
            containment: ContainmentKind::GuardrailsOnly,
            fail_closed_spawn: false,
        },
    )
}

struct GenericChild {
    child: Child,
}

impl ManagedChild for GenericChild {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, ProcessError> {
        self.child.try_wait().map_err(io_error)
    }

    fn wait(&mut self) -> Result<ExitStatus, ProcessError> {
        self.child.wait().map_err(io_error)
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        self.child.kill().map_err(io_error)
    }

    fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
    }
}

fn io_error(e: std::io::Error) -> ProcessError {
    ProcessError::Io(e.to_string())
}
