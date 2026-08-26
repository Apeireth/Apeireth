//! Fallback process execution for platforms without a dedicated backend.
//!
//! This backend enforces the common contract (timeout, bounded stdout/stderr,
//! working directory and environment policy) and reports every containment
//! capability as unsupported. It exists so the crate compiles on exotic
//! platforms, but it is intentionally weak and says so.

use std::io::Read;
use std::process::{Child, Command, Stdio};

use super::{
    apply_request_to_command, supervise, ChildStatus, EnforcementLevel, IsolationCapabilities,
    ManagedChild, PlatformEnforcement, ProcessError, ProcessRequest, ProcessResult,
};

pub(crate) fn capabilities() -> IsolationCapabilities {
    let mut caps = IsolationCapabilities::default();
    caps.set(
        super::IsolationCapability::StructuredSpawn,
        EnforcementLevel::Enforced,
    );
    caps.set(
        super::IsolationCapability::ExplicitCwd,
        EnforcementLevel::Enforced,
    );
    caps.set(
        super::IsolationCapability::Timeout,
        EnforcementLevel::Enforced,
    );
    caps.set(
        super::IsolationCapability::StdoutLimit,
        EnforcementLevel::Enforced,
    );
    caps.set(
        super::IsolationCapability::StderrLimit,
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

    let child = command.spawn().map_err(|e| ProcessError::SpawnFailed {
        executable: request.executable.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    supervise(GenericChild { child }, request, enforcement)
}

struct GenericChild {
    child: Child,
}

impl ManagedChild for GenericChild {
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
        self.child.kill().map_err(io_error)
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

fn io_error(e: std::io::Error) -> ProcessError {
    ProcessError::Io(e.to_string())
}
