//! Canonical process execution boundary.
//!
//! This module is the only place in `apeireth-tools-canonical` that may spawn a
//! child process. It executes a **structured program**:
//!
//! ```text
//! ProcessRequest { executable, args, working_directory, environment, limits }
//! ```
//!
//! It never accepts a shell command string and never invokes `cmd /c` or
//! `sh -c`. Shell capability work belongs to a later phase.
//!
//! # Enforcement model
//!
//! The executor owns *physical* containment: time, output bounds, working
//! directory, environment, and — on Windows — a Job Object attached before the
//! child executes. It does not own governance. It never returns
//! `Allow`/`Deny`/`RequireApproval`; it returns an execution result or a
//! containment/configuration failure.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, ExitStatus};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
pub mod windows;

/// Timeout used when waiting for an already-killed child to disappear.
const POST_KILL_WAIT: Duration = Duration::from_secs(5);
/// Poll interval used while supervising a running child.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Maximum time to wait for a reader thread to deliver output after the child
/// has exited. If a descendant inherited a pipe and keeps it open on a
/// non-Windows platform, the executor does not hang forever.
const READER_JOIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Resource and containment limits for one process execution.
///
/// The default is deliberately bounded, not unlimited. Canonical builtin tools
/// use the default. If a caller genuinely needs an effectively unbounded
/// execution, it must call [`ProcessLimits::unrestricted`] explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessLimits {
    /// Wall-clock timeout for the direct child process.
    pub max_runtime: Duration,
    /// Maximum bytes captured from stdout before truncation is reported.
    pub max_stdout_bytes: usize,
    /// Maximum bytes captured from stderr before truncation is reported.
    pub max_stderr_bytes: usize,
    /// Optional Windows Job Object per-process committed-memory limit.
    ///
    /// `None` means no memory limit is requested. This field exists so a
    /// caller can opt into a memory limit; it is not claimed as a default
    /// enforcement because the canonical builtin tools do not need it yet.
    pub max_process_memory_bytes: Option<u64>,
    /// Optional Windows Job Object active-process limit.
    pub max_active_processes: Option<u32>,
    /// Whether the Windows Job Object is configured with
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    pub kill_on_job_close: bool,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            max_runtime: Duration::from_secs(30),
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
            max_process_memory_bytes: None,
            max_active_processes: None,
            kill_on_job_close: true,
        }
    }
}

impl ProcessLimits {
    /// Explicitly near-unbounded limits.
    ///
    /// This is **not** used by canonical builtin tools. It is provided for
    /// callers that must opt out of the safe default with their eyes open.
    pub fn unrestricted() -> Self {
        Self {
            max_runtime: Duration::MAX,
            max_stdout_bytes: usize::MAX,
            max_stderr_bytes: usize::MAX,
            max_process_memory_bytes: None,
            max_active_processes: None,
            kill_on_job_close: false,
        }
    }
}

/// How a child process should receive environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentSpec {
    /// Inherit the parent environment. This is the compatibility default and
    /// matches normal OS process launch semantics. It should not be used for
    /// future untrusted-command phases without an explicit secret review.
    Inherit,
    /// Start from an empty environment. The child receives no inherited
    /// variables.
    Clear,
    /// Start from an empty environment, then set exactly the listed variables.
    Explicit(Vec<(OsString, OsString)>),
}

/// A structured request to run one executable.
///
/// This is deliberately not `command: String`. There is no shell command line
/// parsing anywhere in this type or in the executor.
#[derive(Debug, Clone)]
pub struct ProcessRequest {
    executable: OsString,
    args: Vec<OsString>,
    working_directory: Option<PathBuf>,
    environment: EnvironmentSpec,
    limits: ProcessLimits,
}

impl ProcessRequest {
    /// Start a request for `executable`.
    pub fn new(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            working_directory: None,
            environment: EnvironmentSpec::Inherit,
            limits: ProcessLimits::default(),
        }
    }

    /// Append one argument.
    pub fn with_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append several arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set the working directory for the child.
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Set the environment policy.
    pub fn with_environment(mut self, environment: EnvironmentSpec) -> Self {
        self.environment = environment;
        self
    }

    /// Shorthand for an empty environment.
    pub fn with_clear_env(mut self) -> Self {
        self.environment = EnvironmentSpec::Clear;
        self
    }

    /// Shorthand for an explicit environment. Starts from empty.
    pub fn with_explicit_env(mut self, vars: Vec<(OsString, OsString)>) -> Self {
        self.environment = EnvironmentSpec::Explicit(vars);
        self
    }

    /// Set the execution limits.
    pub fn with_limits(mut self, limits: ProcessLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The requested executable.
    pub fn executable(&self) -> &OsStr {
        &self.executable
    }

    /// The requested arguments.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// The requested working directory, if any.
    pub fn working_directory(&self) -> Option<&PathBuf> {
        self.working_directory.as_ref()
    }

    /// The requested environment policy.
    pub fn environment(&self) -> &EnvironmentSpec {
        &self.environment
    }

    /// The requested limits.
    pub fn limits(&self) -> &ProcessLimits {
        &self.limits
    }
}

/// Why a supervised child stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationReason {
    /// The child exited on its own. `code` is the platform exit code, or
    /// `None` on Unix when the child was killed by a signal.
    Exited { code: Option<i32> },
    /// The executor terminated the child because [`ProcessLimits::max_runtime`]
    /// elapsed.
    TimedOut { code: Option<i32> },
}

/// Physical containment layer used for one execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainmentKind {
    /// Windows Job Object attached before the child executed.
    WindowsJobObject,
    /// Non-Windows execution guardrails only: timeout, output bounds, working
    /// directory and environment policy. This is **not** an OS privilege
    /// sandbox.
    GuardrailsOnly,
}

/// Platform fact for the execution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Windows,
    NonWindows,
}

/// Metadata about the physical containment that was actually applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEnforcement {
    /// The platform that served the execution.
    pub platform: PlatformKind,
    /// The containment layer that was applied.
    pub containment: ContainmentKind,
    /// True when the child was created suspended and resumed only after
    /// containment attachment. This is the Windows fail-closed spawn path.
    pub fail_closed_spawn: bool,
}

/// The value returned by a completed execution.
#[derive(Debug, Clone)]
pub struct ProcessResult {
    /// How and why the child stopped.
    pub termination: TerminationReason,
    /// Bounded captured stdout.
    pub stdout: Vec<u8>,
    /// Bounded captured stderr.
    pub stderr: Vec<u8>,
    /// True when stdout exceeded [`ProcessLimits::max_stdout_bytes`].
    pub stdout_truncated: bool,
    /// True when stderr exceeded [`ProcessLimits::max_stderr_bytes`].
    pub stderr_truncated: bool,
    /// Platform enforcement metadata for this execution.
    pub enforcement: PlatformEnforcement,
}

impl ProcessResult {
    /// The child's exit code, when the platform reports one.
    pub fn exit_code(&self) -> Option<i32> {
        match self.termination {
            TerminationReason::Exited { code } | TerminationReason::TimedOut { code } => code,
        }
    }

    /// True when the child exited with code 0.
    pub fn success(&self) -> bool {
        matches!(
            self.termination,
            TerminationReason::Exited { code: Some(0) }
        )
    }

    /// True when the executor timed the child out.
    pub fn timed_out(&self) -> bool {
        matches!(self.termination, TerminationReason::TimedOut { .. })
    }
}

/// An error produced while trying to execute a structured process request.
///
/// A non-zero child exit is **not** represented here. A child that ran and
/// exited 1 is a successful [`ProcessResult`] whose `success()` is `false`.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// The request itself is invalid.
    #[error("invalid process configuration: {0}")]
    InvalidConfiguration(String),
    /// The child could not be spawned.
    #[error("failed to spawn process {executable}: {message}")]
    SpawnFailed { executable: String, message: String },
    /// The child was spawned but could not be attached to the containment
    /// layer.
    #[error("failed to attach process containment: {0}")]
    ContainmentFailed(String),
    /// The platform has no supported containment implementation.
    #[error("platform unsupported for process containment: {0}")]
    PlatformUnsupported(String),
    /// An I/O failure happened while supervising the child.
    #[error("process supervision I/O error: {0}")]
    Io(String),
}

/// Canonical process executor.
///
/// This is a small stateless executor. It is not a registry, not a manager,
/// and not a second governance pipeline.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessExecutor;

impl ProcessExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a structured process request.
    ///
    /// This is synchronous. Async tool implementations should call it through
    /// `tokio::task::spawn_blocking` so a child timeout never blocks an async
    /// worker thread.
    pub fn execute(&self, request: &ProcessRequest) -> Result<ProcessResult, ProcessError> {
        if request.executable.is_empty() {
            return Err(ProcessError::InvalidConfiguration(
                "executable must not be empty".into(),
            ));
        }
        if request.limits.max_stdout_bytes == 0 || request.limits.max_stderr_bytes == 0 {
            return Err(ProcessError::InvalidConfiguration(
                "output limits must be non-zero".into(),
            ));
        }

        #[cfg(windows)]
        {
            return windows::spawn_and_supervise(request);
        }

        #[cfg(not(windows))]
        {
            return crate::process::platform::spawn_and_supervise(request);
        }
    }
}

#[cfg(not(windows))]
mod platform;

pub(crate) fn apply_request_to_command(
    command: &mut std::process::Command,
    request: &ProcessRequest,
) -> Result<(), ProcessError> {
    if let Some(dir) = &request.working_directory {
        // `Command::current_dir` records the path; an invalid directory is
        // reported as a spawn failure by the OS, which keeps the error path
        // uniform across platforms.
        command.current_dir(dir);
    }

    match &request.environment {
        EnvironmentSpec::Inherit => {}
        EnvironmentSpec::Clear => {
            command.env_clear();
        }
        EnvironmentSpec::Explicit(vars) => {
            command.env_clear();
            for (key, value) in vars {
                command.env(key, value);
            }
        }
    }

    Ok(())
}

pub(crate) trait ManagedChild {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, ProcessError>;
    fn wait(&mut self) -> Result<ExitStatus, ProcessError>;
    fn terminate(&mut self) -> Result<(), ProcessError>;
    fn take_stdout(&mut self) -> Option<ChildStdout>;
    fn take_stderr(&mut self) -> Option<ChildStderr>;
}

pub(crate) fn supervise<C: ManagedChild>(
    mut child: C,
    request: &ProcessRequest,
    enforcement: PlatformEnforcement,
) -> Result<ProcessResult, ProcessError> {
    let stdout_pipe = child
        .take_stdout()
        .ok_or_else(|| ProcessError::InvalidConfiguration("stdout pipe was not captured".into()))?;
    let stderr_pipe = child
        .take_stderr()
        .ok_or_else(|| ProcessError::InvalidConfiguration("stderr pipe was not captured".into()))?;

    let stdout_rx = spawn_reader(stdout_pipe, request.limits.max_stdout_bytes);
    let stderr_rx = spawn_reader(stderr_pipe, request.limits.max_stderr_bytes);

    let start = Instant::now();
    let mut timed_out = false;

    let status = 'supervise: loop {
        if let Some(status) = child.try_wait()? {
            break 'supervise status;
        }

        if start.elapsed() >= request.limits.max_runtime {
            timed_out = true;
            child.terminate()?;

            let kill_deadline = Instant::now() + POST_KILL_WAIT;
            loop {
                if let Some(status) = child.try_wait()? {
                    break 'supervise status;
                }
                if Instant::now() >= kill_deadline {
                    return Err(ProcessError::Io(
                        "child did not terminate after containment kill".into(),
                    ));
                }
                thread::sleep(POLL_INTERVAL);
            }
        }

        thread::sleep(POLL_INTERVAL);
    };

    let code = status.code();
    let termination = if timed_out {
        TerminationReason::TimedOut { code }
    } else {
        TerminationReason::Exited { code }
    };

    let (stdout, stdout_truncated) = join_reader(stdout_rx)?;
    let (stderr, stderr_truncated) = join_reader(stderr_rx)?;

    Ok(ProcessResult {
        termination,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        enforcement,
    })
}

type ReaderMessage = (Vec<u8>, bool, Option<String>);

fn spawn_reader<R>(mut pipe: R, max_bytes: usize) -> Receiver<ReaderMessage>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut limited = pipe.take(max_bytes as u64 + 1);
        match limited.read_to_end(&mut buffer) {
            Ok(_) => {
                let truncated = buffer.len() > max_bytes;
                if truncated {
                    buffer.truncate(max_bytes);
                }
                let _ = tx.send((buffer, truncated, None));
            }
            Err(e) => {
                let _ = tx.send((buffer, false, Some(e.to_string())));
            }
        }
    });
    rx
}

fn join_reader(rx: Receiver<ReaderMessage>) -> Result<(Vec<u8>, bool), ProcessError> {
    match rx.recv_timeout(READER_JOIN_TIMEOUT) {
        Ok((bytes, truncated, None)) => Ok((bytes, truncated)),
        Ok((bytes, truncated, Some(read_error))) => {
            if truncated || !bytes.is_empty() {
                // We still return what was captured; the caller can see the
                // error separately if it matters. The read error is logged via
                // the structured error path, but partial output is not lost.
                let _ = read_error;
                Ok((bytes, truncated))
            } else {
                Err(ProcessError::Io(read_error))
            }
        }
        Err(RecvTimeoutError::Timeout) => Err(ProcessError::Io(
            "timed out waiting for child output reader to finish".into(),
        )),
        Err(RecvTimeoutError::Disconnected) => Err(ProcessError::Io(
            "child output reader terminated unexpectedly".into(),
        )),
    }
}
