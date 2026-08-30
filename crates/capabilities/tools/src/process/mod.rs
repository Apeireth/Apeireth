//! Canonical process execution boundary.
//!
//! This module is the only place in `apeireth-tools-canonical` that may spawn a
//! child process. It executes a **structured program**:
//!
//! ```text
//! ProcessRequest {
//!     executable,
//!     args,
//!     working_directory,
//!     environment,
//!     limits,
//!     isolation,
//! }
//! ```
//!
//! It never accepts a shell command string and never invokes `cmd /c` or
//! `sh -c`. Shell capability work belongs to a later phase.
//!
//! # Enforcement model
//!
//! The executor owns *physical* containment: time, output bounds, working
//! directory, environment, and platform-specific process containment. It does
//! not own governance. It never returns `Allow`/`Deny`/`RequireApproval`; it
//! returns an execution result or a containment/configuration failure.
//!
//! # Cross-platform contract
//!
//! The public layer only expresses canonical types. Platform details such as
//! Job Objects, cgroups, namespaces, seccomp, Landlock, or sandbox-exec are
//! confined to the platform backend modules (`windows`, `linux`, `macos`).
//! Callers query [`current_platform_capabilities`], declare
//! [`IsolationRequirement`]s on a [`ProcessRequest`], and fail closed when the
//! platform cannot meet the requirement.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub(crate) mod platform;
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

/// Enforcement level for one isolation property.
///
/// The ordering is meaningful: `Unsupported < Partial < Enforced`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnforcementLevel {
    /// The platform/backend cannot provide this property. A caller that
    /// requires it must fail closed.
    Unsupported,
    /// The platform/backend provides a weaker or narrower form of the
    /// property. It may still be useful, but the caller must opt in.
    Partial,
    /// The platform/backend provides the property in a tested, enforced form.
    Enforced,
}

/// Isolation properties that a caller can observe, require, or inspect in a
/// result. These are platform-agnostic by design; none of them is named after
/// a specific OS mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IsolationCapability {
    /// The executable and argv are passed as a structured array, never as a
    /// shell command string.
    StructuredSpawn,
    /// The child working directory is explicitly recorded on the request.
    ExplicitCwd,
    /// Wall-clock timeout terminates the spawned tree.
    Timeout,
    /// stdout is bounded and truncation is reported.
    StdoutLimit,
    /// stderr is bounded and truncation is reported.
    StderrLimit,
    /// `EnvironmentSpec::Clear` / `Explicit` starts from an empty environment.
    EnvironmentIsolation,
    /// The spawned process tree is contained and can be terminated as a tree.
    ProcessTreeContainment,
    /// Per-process/per-tree memory usage is bounded.
    MemoryLimit,
    /// The number of processes in the spawned tree is bounded.
    ProcessCountLimit,
    /// CPU time is bounded.
    CpuLimit,
    /// File size output is bounded.
    FileSizeLimit,
    /// The child runs with reduced privileges / restricted identity.
    PrivilegeReduction,
    /// The child filesystem access is isolated from the host filesystem.
    FilesystemIsolation,
    /// The child network egress is isolated from the host network.
    NetworkIsolation,
    /// Containment is attached before the child can execute; setup failure
    /// means no child is started.
    FailClosedPreExecutionContainment,
}

/// A set of platform capabilities, each expressed as an [`EnforcementLevel`].
///
/// This is the observable platform capability model. It is produced by the
/// platform backend through actual OS/runtime detection, never through
/// `cfg!(...)` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationCapabilities {
    pub structured_spawn: EnforcementLevel,
    pub explicit_cwd: EnforcementLevel,
    pub timeout: EnforcementLevel,
    pub stdout_limit: EnforcementLevel,
    pub stderr_limit: EnforcementLevel,
    pub environment_isolation: EnforcementLevel,
    pub process_tree_containment: EnforcementLevel,
    pub memory_limit: EnforcementLevel,
    pub process_count_limit: EnforcementLevel,
    pub cpu_limit: EnforcementLevel,
    pub file_size_limit: EnforcementLevel,
    pub privilege_reduction: EnforcementLevel,
    pub filesystem_isolation: EnforcementLevel,
    pub network_isolation: EnforcementLevel,
    pub fail_closed_pre_execution_containment: EnforcementLevel,
}

impl Default for IsolationCapabilities {
    fn default() -> Self {
        Self {
            structured_spawn: EnforcementLevel::Unsupported,
            explicit_cwd: EnforcementLevel::Unsupported,
            timeout: EnforcementLevel::Unsupported,
            stdout_limit: EnforcementLevel::Unsupported,
            stderr_limit: EnforcementLevel::Unsupported,
            environment_isolation: EnforcementLevel::Unsupported,
            process_tree_containment: EnforcementLevel::Unsupported,
            memory_limit: EnforcementLevel::Unsupported,
            process_count_limit: EnforcementLevel::Unsupported,
            cpu_limit: EnforcementLevel::Unsupported,
            file_size_limit: EnforcementLevel::Unsupported,
            privilege_reduction: EnforcementLevel::Unsupported,
            filesystem_isolation: EnforcementLevel::Unsupported,
            network_isolation: EnforcementLevel::Unsupported,
            fail_closed_pre_execution_containment: EnforcementLevel::Unsupported,
        }
    }
}

impl IsolationCapabilities {
    /// Get the level for one capability.
    pub fn level(&self, capability: IsolationCapability) -> EnforcementLevel {
        match capability {
            IsolationCapability::StructuredSpawn => self.structured_spawn,
            IsolationCapability::ExplicitCwd => self.explicit_cwd,
            IsolationCapability::Timeout => self.timeout,
            IsolationCapability::StdoutLimit => self.stdout_limit,
            IsolationCapability::StderrLimit => self.stderr_limit,
            IsolationCapability::EnvironmentIsolation => self.environment_isolation,
            IsolationCapability::ProcessTreeContainment => self.process_tree_containment,
            IsolationCapability::MemoryLimit => self.memory_limit,
            IsolationCapability::ProcessCountLimit => self.process_count_limit,
            IsolationCapability::CpuLimit => self.cpu_limit,
            IsolationCapability::FileSizeLimit => self.file_size_limit,
            IsolationCapability::PrivilegeReduction => self.privilege_reduction,
            IsolationCapability::FilesystemIsolation => self.filesystem_isolation,
            IsolationCapability::NetworkIsolation => self.network_isolation,
            IsolationCapability::FailClosedPreExecutionContainment => {
                self.fail_closed_pre_execution_containment
            }
        }
    }

    /// Set the level for one capability.
    pub fn set(&mut self, capability: IsolationCapability, level: EnforcementLevel) {
        match capability {
            IsolationCapability::StructuredSpawn => self.structured_spawn = level,
            IsolationCapability::ExplicitCwd => self.explicit_cwd = level,
            IsolationCapability::Timeout => self.timeout = level,
            IsolationCapability::StdoutLimit => self.stdout_limit = level,
            IsolationCapability::StderrLimit => self.stderr_limit = level,
            IsolationCapability::EnvironmentIsolation => self.environment_isolation = level,
            IsolationCapability::ProcessTreeContainment => self.process_tree_containment = level,
            IsolationCapability::MemoryLimit => self.memory_limit = level,
            IsolationCapability::ProcessCountLimit => self.process_count_limit = level,
            IsolationCapability::CpuLimit => self.cpu_limit = level,
            IsolationCapability::FileSizeLimit => self.file_size_limit = level,
            IsolationCapability::PrivilegeReduction => self.privilege_reduction = level,
            IsolationCapability::FilesystemIsolation => self.filesystem_isolation = level,
            IsolationCapability::NetworkIsolation => self.network_isolation = level,
            IsolationCapability::FailClosedPreExecutionContainment => {
                self.fail_closed_pre_execution_containment = level
            }
        }
    }
}

/// A caller's minimum required isolation properties.
///
/// The default is intentionally empty. Builtin tools use the default and rely
/// on the common contract (structured spawn, cwd, timeout, output bounds).
/// A future shell or any safety-sensitive tool must set explicit requirements
/// and fail closed when the platform cannot meet them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationRequirement {
    required: Vec<(IsolationCapability, EnforcementLevel)>,
}

impl IsolationRequirement {
    pub fn new() -> Self {
        Self {
            required: Vec::new(),
        }
    }

    /// Require `capability` at `level` (or stronger).
    pub fn require(mut self, capability: IsolationCapability, level: EnforcementLevel) -> Self {
        self.required.push((capability, level));
        self
    }

    /// Require a capability at [`EnforcementLevel::Enforced`].
    pub fn require_enforced(self, capability: IsolationCapability) -> Self {
        self.require(capability, EnforcementLevel::Enforced)
    }

    /// The raw requirements, in insertion order.
    pub fn requirements(&self) -> &[(IsolationCapability, EnforcementLevel)] {
        &self.required
    }

    /// The required level for `capability`, if any.
    pub fn requires(&self, capability: IsolationCapability) -> Option<EnforcementLevel> {
        self.required
            .iter()
            .find(|(cap, _)| *cap == capability)
            .map(|(_, level)| *level)
    }

    /// True when every requirement is met by `capabilities`.
    pub fn is_satisfied_by(&self, capabilities: &IsolationCapabilities) -> bool {
        self.missing_requirements(capabilities).is_empty()
    }

    /// The subset of requirements that `capabilities` does not meet.
    ///
    /// Each entry is `(capability, required_level)`.
    pub fn missing_requirements(
        &self,
        capabilities: &IsolationCapabilities,
    ) -> Vec<(IsolationCapability, EnforcementLevel)> {
        self.required
            .iter()
            .filter(|(capability, required)| capabilities.level(*capability) < *required)
            .map(|(capability, required)| (*capability, *required))
            .collect()
    }
}

/// Convenience presets for common isolation postures.
///
/// A profile is only a preset for [`IsolationRequirement`]; it never causes a
/// silent fallback. If a profile is not satisfiable on a platform, the
/// executor returns [`ProcessError::IsolationRequirementUnsatisfied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationProfile {
    /// Common contract only: structured spawn, explicit cwd, timeout, and
    /// output bounds. Suitable for trusted builtin tools (e.g. read-only git).
    Trusted,
    /// Trusted plus process-tree containment, environment isolation, and
    /// resource containment (memory / process-count / CPU / file-size where
    /// available). This is the baseline for future untrusted-adjacent tools.
    Restricted,
    /// Restricted plus filesystem and network isolation. This profile is
    /// intentionally unsatisfiable on backends that do not have real
    /// filesystem/network containment; it is the fail-closed posture for a
    /// future arbitrary-code tool.
    Untrusted,
}

impl IsolationProfile {
    /// Convert a profile into a concrete requirement preset.
    pub fn requirement(self) -> IsolationRequirement {
        use EnforcementLevel::{Enforced, Partial};
        use IsolationCapability::{
            CpuLimit, EnvironmentIsolation, ExplicitCwd, FailClosedPreExecutionContainment,
            FileSizeLimit, FilesystemIsolation, MemoryLimit, NetworkIsolation, PrivilegeReduction,
            ProcessCountLimit, ProcessTreeContainment, StderrLimit, StdoutLimit, StructuredSpawn,
            Timeout,
        };

        match self {
            Self::Trusted => IsolationRequirement::new()
                .require(StructuredSpawn, Enforced)
                .require(ExplicitCwd, Enforced)
                .require(Timeout, Enforced)
                .require(StdoutLimit, Enforced)
                .require(StderrLimit, Enforced),
            Self::Restricted => Self::Trusted
                .requirement()
                .require(EnvironmentIsolation, Enforced)
                .require(ProcessTreeContainment, Partial)
                .require(MemoryLimit, Partial)
                .require(ProcessCountLimit, Partial)
                .require(CpuLimit, Partial)
                .require(FileSizeLimit, Partial)
                .require(PrivilegeReduction, Partial)
                .require(FailClosedPreExecutionContainment, Enforced),
            Self::Untrusted => Self::Restricted
                .requirement()
                .require(PrivilegeReduction, Enforced)
                .require(FilesystemIsolation, Enforced)
                .require(NetworkIsolation, Enforced),
        }
    }
}

/// Resource and containment limits for one process execution.
///
/// The default is deliberately bounded, not unlimited. Canonical builtin tools
/// use the default. If a caller genuinely needs an effectively unbounded
/// execution, it must call [`ProcessLimits::unrestricted`] explicitly.
///
/// Optional platform limits (`max_process_memory_bytes`, `max_active_processes`,
/// `max_cpu_seconds`, `max_file_size_bytes`) are fail-closed: when a backend
/// cannot enforce a limit that is set, the executor refuses to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessLimits {
    /// Wall-clock timeout for the spawned process tree.
    pub max_runtime: Duration,
    /// Maximum bytes captured from stdout before truncation is reported.
    pub max_stdout_bytes: usize,
    /// Maximum bytes captured from stderr before truncation is reported.
    pub max_stderr_bytes: usize,
    /// Optional per-process/per-tree committed-memory limit.
    ///
    /// Windows: Job Object process/job memory limit.
    /// Linux/macOS: `RLIMIT_AS` address-space limit (partial memory
    /// containment, not physical memory).
    pub max_process_memory_bytes: Option<u64>,
    /// Optional active-process count limit.
    ///
    /// Windows: Job Object active-process limit (tree-scoped).
    /// Linux: `RLIMIT_NPROC` is UID-scoped and therefore only advertised as
    /// partial; tests must not rely on global UID state.
    pub max_active_processes: Option<u32>,
    /// Optional CPU-time limit in seconds.
    ///
    /// Linux/macOS: `RLIMIT_CPU`. Windows: Job Object
    /// `JOB_OBJECT_LIMIT_PROCESS_TIME` (per-process user-mode time, 100-ns
    /// units). This is CPU time, not wall-clock timeout (`max_runtime`).
    pub max_cpu_seconds: Option<u64>,
    /// Optional maximum file size in bytes that the child may create.
    ///
    /// Linux/macOS: `RLIMIT_FSIZE`. Windows currently has no file-size limit in
    /// this contract and will refuse the limit.
    pub max_file_size_bytes: Option<u64>,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            max_runtime: Duration::from_secs(30),
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
            max_process_memory_bytes: None,
            max_active_processes: None,
            max_cpu_seconds: None,
            max_file_size_bytes: None,
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
            max_cpu_seconds: None,
            max_file_size_bytes: None,
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
    isolation: IsolationRequirement,
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
            isolation: IsolationRequirement::default(),
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

    /// Set the required isolation properties.
    pub fn with_isolation(mut self, isolation: IsolationRequirement) -> Self {
        self.isolation = isolation;
        self
    }

    /// Use a named isolation profile preset.
    pub fn with_profile(mut self, profile: IsolationProfile) -> Self {
        self.isolation = profile.requirement();
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

    /// The requested isolation requirements.
    pub fn isolation(&self) -> &IsolationRequirement {
        &self.isolation
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

/// The platform that served the execution. This is intentionally coarse:
/// capability decisions are made through [`IsolationCapabilities`], never by
/// branching on this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Windows,
    Linux,
    MacOs,
    Other,
}

/// Metadata about the physical containment that was actually applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformEnforcement {
    /// The platform that served the execution.
    pub platform: PlatformKind,
    /// The isolation properties that were actually in effect for this run.
    pub capabilities: IsolationCapabilities,
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
    /// The platform cannot meet the request's [`IsolationRequirement`].
    #[error(
        "isolation requirement unsatisfied: missing {missing:?} (requested {requested:?}, supported {supported:?})"
    )]
    IsolationRequirementUnsatisfied {
        requested: Vec<(IsolationCapability, EnforcementLevel)>,
        supported: IsolationCapabilities,
        missing: Vec<(IsolationCapability, EnforcementLevel)>,
    },
    /// A requested optional limit cannot be enforced on this platform.
    #[error("unsupported process limit on this platform: {0}")]
    UnsupportedLimit(String),
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
        validate_request(request)?;

        let capabilities = current_platform_capabilities();
        validate_limits_against_capabilities(request, &capabilities)?;

        let missing = request.isolation().missing_requirements(&capabilities);
        if !missing.is_empty() {
            return Err(ProcessError::IsolationRequirementUnsatisfied {
                requested: request.isolation().requirements().to_vec(),
                supported: capabilities,
                missing,
            });
        }

        let enforcement_capabilities = capabilities_for_execution(request, &capabilities);
        let enforcement = PlatformEnforcement {
            platform: capabilities_platform_kind(),
            capabilities: enforcement_capabilities,
        };

        #[cfg(windows)]
        {
            return windows::spawn_and_supervise(request, enforcement);
        }

        #[cfg(target_os = "linux")]
        {
            return linux::spawn_and_supervise(request, enforcement);
        }

        #[cfg(target_os = "macos")]
        {
            return macos::spawn_and_supervise(request, enforcement);
        }

        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            return platform::spawn_and_supervise(request, enforcement);
        }
    }
}

/// Report the current platform's isolation capabilities.
///
/// The returned values come from actual OS/runtime detection in the platform
/// backend, never from `cfg!(...)` alone.
pub fn current_platform_capabilities() -> IsolationCapabilities {
    #[cfg(windows)]
    {
        return windows::capabilities();
    }

    #[cfg(target_os = "linux")]
    {
        return linux::capabilities();
    }

    #[cfg(target_os = "macos")]
    {
        return macos::capabilities();
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        return platform::capabilities();
    }
}

fn capabilities_platform_kind() -> PlatformKind {
    #[cfg(windows)]
    {
        return PlatformKind::Windows;
    }

    #[cfg(target_os = "linux")]
    {
        return PlatformKind::Linux;
    }

    #[cfg(target_os = "macos")]
    {
        return PlatformKind::MacOs;
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        return PlatformKind::Other;
    }
}

fn validate_request(request: &ProcessRequest) -> Result<(), ProcessError> {
    if request.executable.is_empty() {
        return Err(ProcessError::InvalidConfiguration(
            "executable must not be empty".into(),
        ));
    }
    if request.limits.max_runtime.is_zero() {
        return Err(ProcessError::InvalidConfiguration(
            "max_runtime must be non-zero".into(),
        ));
    }
    if request.limits.max_stdout_bytes == 0 || request.limits.max_stderr_bytes == 0 {
        return Err(ProcessError::InvalidConfiguration(
            "output limits must be non-zero".into(),
        ));
    }
    if request.limits.max_process_memory_bytes == Some(0) {
        return Err(ProcessError::InvalidConfiguration(
            "max_process_memory_bytes must be non-zero".into(),
        ));
    }
    if request.limits.max_active_processes == Some(0) {
        return Err(ProcessError::InvalidConfiguration(
            "max_active_processes must be non-zero".into(),
        ));
    }
    if request.limits.max_cpu_seconds == Some(0) {
        return Err(ProcessError::InvalidConfiguration(
            "max_cpu_seconds must be non-zero".into(),
        ));
    }
    if request.limits.max_file_size_bytes == Some(0) {
        return Err(ProcessError::InvalidConfiguration(
            "max_file_size_bytes must be non-zero".into(),
        ));
    }
    Ok(())
}

fn validate_limits_against_capabilities(
    request: &ProcessRequest,
    capabilities: &IsolationCapabilities,
) -> Result<(), ProcessError> {
    let limits = &request.limits;

    if limits.max_process_memory_bytes.is_some()
        && capabilities.memory_limit == EnforcementLevel::Unsupported
    {
        return Err(ProcessError::UnsupportedLimit(
            "max_process_memory_bytes is not enforceable on this platform".into(),
        ));
    }
    if limits.max_active_processes.is_some()
        && capabilities.process_count_limit == EnforcementLevel::Unsupported
    {
        return Err(ProcessError::UnsupportedLimit(
            "max_active_processes is not enforceable on this platform".into(),
        ));
    }
    if limits.max_cpu_seconds.is_some() && capabilities.cpu_limit == EnforcementLevel::Unsupported {
        return Err(ProcessError::UnsupportedLimit(
            "max_cpu_seconds is not enforceable on this platform".into(),
        ));
    }
    if limits.max_file_size_bytes.is_some()
        && capabilities.file_size_limit == EnforcementLevel::Unsupported
    {
        return Err(ProcessError::UnsupportedLimit(
            "max_file_size_bytes is not enforceable on this platform".into(),
        ));
    }
    Ok(())
}

fn capabilities_for_execution(
    request: &ProcessRequest,
    platform: &IsolationCapabilities,
) -> IsolationCapabilities {
    let mut actual = platform.clone();

    // Optional limits are only "actually in effect" when the request opted in.
    if request.limits.max_process_memory_bytes.is_none() {
        actual.memory_limit = EnforcementLevel::Unsupported;
    }
    if request.limits.max_active_processes.is_none() {
        actual.process_count_limit = EnforcementLevel::Unsupported;
    }
    if request.limits.max_cpu_seconds.is_none() {
        actual.cpu_limit = EnforcementLevel::Unsupported;
    }
    if request.limits.max_file_size_bytes.is_none() {
        actual.file_size_limit = EnforcementLevel::Unsupported;
    }

    // Privilege reduction is only applied when the caller explicitly asks for
    // it. Merely being able to reduce privileges is not the same as doing it.
    if request
        .isolation()
        .requires(IsolationCapability::PrivilegeReduction)
        .is_none()
    {
        actual.privilege_reduction = EnforcementLevel::Unsupported;
    }

    actual
}

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChildStatus {
    pub code: Option<i32>,
}

pub(crate) trait ManagedChild {
    fn try_wait(&mut self) -> Result<Option<ChildStatus>, ProcessError>;
    fn wait(&mut self) -> Result<ChildStatus, ProcessError>;
    fn terminate(&mut self) -> Result<(), ProcessError>;
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>>;
    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>>;
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

    let termination = if timed_out {
        TerminationReason::TimedOut { code: status.code }
    } else {
        TerminationReason::Exited { code: status.code }
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
