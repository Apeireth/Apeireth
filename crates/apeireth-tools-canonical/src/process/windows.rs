//! Windows process containment: Job Object + fail-closed suspended spawn.
//!
//! The normal launch sequence is:
//!
//! ```text
//! CreateProcessW(..., CREATE_SUSPENDED, ...)  // through std::process::Command
//!   -> AssignProcessToJobObject
//!   -> ResumeThread
//! ```
//!
//! The child therefore cannot execute before it belongs to the Job Object.
//! This module uses `std::process::Command` for command-line quoting, Unicode
//! conversion, working directory, environment and pipe creation, so no manual
//! `CreateProcessW` quoting is invented on the normal path.
//!
//! When the caller requires [`IsolationCapability::PrivilegeReduction`], the
//! backend additionally attempts to launch the child with a restricted token
//! via `CreateProcessWithTokenW`. If the current account cannot hold the
//! required privilege (ordinary non-admin users typically cannot), capability
//! detection reports `Unsupported` and requirement checks fail closed.
#![allow(unsafe_code)]

use std::ffi::{c_void, OsStr, OsString};
use std::fs::File;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{
    CloseHandle, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::{
    CreateRestrictedToken, CreateWellKnownSid, IsTokenRestricted, WinBuiltinUsersSid,
    DISABLE_MAX_PRIVILEGE, PSID, SANDBOX_INERT, SID_AND_ATTRIBUTES, TOKEN_ASSIGN_PRIMARY,
    TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_QUERY,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CreateProcessWithTokenW, GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, OpenThread,
    ResumeThread, TerminateProcess, WaitForSingleObject, CREATE_SUSPENDED, INFINITE,
    PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW, THREAD_SUSPEND_RESUME,
};

use super::{
    apply_request_to_command, supervise, ChildStatus, EnforcementLevel, IsolationCapabilities,
    IsolationCapability, ManagedChild, PlatformEnforcement, ProcessError, ProcessLimits,
    ProcessRequest, ProcessResult,
};

const RESTRICTED_TOKEN_ACCESS: u32 =
    TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_IMPERSONATE;

/// RAII Job Object handle.
pub struct JobObject {
    handle: HANDLE,
}

impl std::fmt::Debug for JobObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobObject")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                CloseHandle(self.handle);
            }
        }
    }
}

impl JobObject {
    /// Create a Job Object configured from `limits`.
    pub fn create(limits: &ProcessLimits) -> Result<Self, ProcessError> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(ProcessError::ContainmentFailed(
                    "CreateJobObjectW failed".into(),
                ));
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            // The executor always kills the whole tree when the last Job
            // Object handle closes. This is the Windows backend's fail-closed
            // cleanup guarantee and is not optional.
            let mut flags: u32 = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            if let Some(memory) = limits.max_process_memory_bytes {
                flags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_JOB_MEMORY;
                info.ProcessMemoryLimit = memory as usize;
                info.JobMemoryLimit = memory as usize;
            }

            if let Some(active_processes) = limits.max_active_processes {
                flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
                info.BasicLimitInformation.ActiveProcessLimit = active_processes;
            }

            info.BasicLimitInformation.LimitFlags = flags;

            let ret = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ret == 0 {
                CloseHandle(handle);
                return Err(ProcessError::ContainmentFailed(
                    "SetInformationJobObject failed".into(),
                ));
            }

            Ok(Self { handle })
        }
    }

    /// Attach an existing process to this Job Object.
    pub fn assign(&self, process_handle: HANDLE) -> Result<(), ProcessError> {
        if self.handle.is_null() {
            return Err(ProcessError::ContainmentFailed(
                "Job Object handle is null".into(),
            ));
        }
        unsafe {
            let ret = AssignProcessToJobObject(self.handle, process_handle);
            if ret == 0 {
                return Err(ProcessError::ContainmentFailed(
                    "AssignProcessToJobObject failed".into(),
                ));
            }
        }
        Ok(())
    }

    /// Terminate every process in this Job Object.
    pub fn terminate(&self) -> Result<(), ProcessError> {
        if self.handle.is_null() {
            return Err(ProcessError::ContainmentFailed(
                "Job Object handle is null".into(),
            ));
        }
        unsafe {
            let ret = TerminateJobObject(self.handle, 1);
            if ret == 0 {
                return Err(ProcessError::ContainmentFailed(
                    "TerminateJobObject failed".into(),
                ));
            }
        }
        Ok(())
    }

    /// The raw Windows handle. Test-only and advanced integration use.
    pub fn as_raw_handle(&self) -> HANDLE {
        self.handle
    }
}

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
        EnforcementLevel::Enforced,
    );
    caps.set(IsolationCapability::MemoryLimit, EnforcementLevel::Enforced);
    caps.set(
        IsolationCapability::ProcessCountLimit,
        EnforcementLevel::Enforced,
    );
    caps.set(IsolationCapability::CpuLimit, EnforcementLevel::Unsupported);
    caps.set(
        IsolationCapability::FileSizeLimit,
        EnforcementLevel::Unsupported,
    );
    caps.set(
        IsolationCapability::PrivilegeReduction,
        restricted_launch_capability(),
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
    let job = JobObject::create(&request.limits)?;

    let child: WindowsChild = if request
        .isolation()
        .requires(IsolationCapability::PrivilegeReduction)
        .is_some()
    {
        WindowsChild::Raw(spawn_restricted_child(request, job)?)
    } else {
        WindowsChild::Std(spawn_std_child(request, job)?)
    };

    supervise(child, request, enforcement)
}

enum WindowsChild {
    Std(WindowsStdChild),
    Raw(WindowsRawChild),
}

impl ManagedChild for WindowsChild {
    fn try_wait(&mut self) -> Result<Option<ChildStatus>, ProcessError> {
        match self {
            Self::Std(child) => child.try_wait(),
            Self::Raw(child) => child.try_wait(),
        }
    }

    fn wait(&mut self) -> Result<ChildStatus, ProcessError> {
        match self {
            Self::Std(child) => child.wait(),
            Self::Raw(child) => child.wait(),
        }
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        match self {
            Self::Std(child) => child.terminate(),
            Self::Raw(child) => child.terminate(),
        }
    }

    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        match self {
            Self::Std(child) => child.take_stdout(),
            Self::Raw(child) => child.take_stdout(),
        }
    }

    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        match self {
            Self::Std(child) => child.take_stderr(),
            Self::Raw(child) => child.take_stderr(),
        }
    }
}

struct WindowsStdChild {
    child: Child,
    job: JobObject,
}

impl WindowsStdChild {
    fn spawn(request: &ProcessRequest, job: JobObject) -> Result<Self, ProcessError> {
        let mut command = Command::new(&request.executable);
        command.args(&request.args);
        command.creation_flags(CREATE_SUSPENDED);
        apply_request_to_command(&mut command, request)?;
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = command.spawn().map_err(|e| ProcessError::SpawnFailed {
            executable: request.executable.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;

        let process_handle = child.as_raw_handle() as HANDLE;

        if let Err(e) = job.assign(process_handle) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }

        if let Err(e) = resume_main_thread(child.id()) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }

        Ok(Self { child, job })
    }
}

impl ManagedChild for WindowsStdChild {
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
        // Job Object termination kills the whole tree. Fall back to direct
        // child kill if the job is somehow unusable.
        match self.job.terminate() {
            Ok(()) => Ok(()),
            Err(_) => self.child.kill().map_err(io_error),
        }
    }

    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)
    }
}

fn spawn_std_child(
    request: &ProcessRequest,
    job: JobObject,
) -> Result<WindowsStdChild, ProcessError> {
    WindowsStdChild::spawn(request, job)
}

/// Raw Windows child created through `CreateProcessWithTokenW`.
struct WindowsRawChild {
    process_handle: HANDLE,
    thread_handle: HANDLE,
    job: JobObject,
    stdout: Option<File>,
    stderr: Option<File>,
}

impl Drop for WindowsRawChild {
    fn drop(&mut self) {
        unsafe {
            if !self.thread_handle.is_null() {
                CloseHandle(self.thread_handle);
            }
            if !self.process_handle.is_null() {
                CloseHandle(self.process_handle);
            }
        }
    }
}

impl ManagedChild for WindowsRawChild {
    fn try_wait(&mut self) -> Result<Option<ChildStatus>, ProcessError> {
        unsafe {
            let wait = WaitForSingleObject(self.process_handle, 0);
            if wait == WAIT_OBJECT_0 {
                Ok(Some(exit_status(self.process_handle)?))
            } else if wait == WAIT_TIMEOUT {
                Ok(None)
            } else {
                Err(ProcessError::Io(format!(
                    "WaitForSingleObject failed while polling child: {}",
                    std::io::Error::last_os_error()
                )))
            }
        }
    }

    fn wait(&mut self) -> Result<ChildStatus, ProcessError> {
        unsafe {
            let wait = WaitForSingleObject(self.process_handle, INFINITE);
            if wait != WAIT_OBJECT_0 {
                return Err(ProcessError::Io(format!(
                    "WaitForSingleObject failed while waiting for child: {}",
                    std::io::Error::last_os_error()
                )));
            }
            exit_status(self.process_handle)
        }
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        if self.job.terminate().is_ok() {
            return Ok(());
        }
        unsafe {
            let ret = TerminateProcess(self.process_handle, 1);
            if ret == 0 {
                return Err(ProcessError::Io(format!(
                    "TerminateProcess failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        Ok(())
    }

    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)
    }
}

fn exit_status(process_handle: HANDLE) -> Result<ChildStatus, ProcessError> {
    unsafe {
        let mut code: u32 = 0;
        let ret = GetExitCodeProcess(process_handle, &mut code);
        if ret == 0 {
            return Err(ProcessError::Io(format!(
                "GetExitCodeProcess failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(ChildStatus {
            code: Some(code as i32),
        })
    }
}

/// One-time capability detection for restricted-token process launch.
///
/// This is intentionally a real OS probe: we create a restricted token from
/// the current process token, verify that `IsTokenRestricted` is true on the
/// token, and then attempt `CreateProcessWithTokenW` with a benign system
/// command. If any step fails, the backend reports `Unsupported`; it never
/// claims restricted identity by merely creating a token handle.
fn restricted_launch_capability() -> EnforcementLevel {
    static CACHED: OnceLock<EnforcementLevel> = OnceLock::new();
    *CACHED.get_or_init(|| {
        let token = match create_restricted_token() {
            Ok(token) => token,
            Err(_) => return EnforcementLevel::Unsupported,
        };
        let launched = probe_restricted_launch(token);
        unsafe {
            CloseHandle(token);
        }
        if launched {
            EnforcementLevel::Enforced
        } else {
            EnforcementLevel::Unsupported
        }
    })
}

/// Create a restricted token from the current process token.
///
/// The token is restricted by adding `WinBuiltinUsersSid` as a restricting SID
/// and by disabling maximum privileges. `IsTokenRestricted` is verified before
/// the token is returned; a token handle that is not truly restricted is
/// treated as failure.
fn create_restricted_token() -> Result<HANDLE, ProcessError> {
    unsafe {
        let process_token = get_current_token()?;

        let mut sid_size: u32 = 0;
        let _ = CreateWellKnownSid(
            WinBuiltinUsersSid,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sid_size,
        );
        if sid_size == 0 {
            CloseHandle(process_token);
            return Err(ProcessError::ContainmentFailed(
                "CreateWellKnownSid failed to compute SID size".into(),
            ));
        }
        let mut sid = vec![0u8; sid_size as usize];
        let ret = CreateWellKnownSid(
            WinBuiltinUsersSid,
            std::ptr::null_mut(),
            sid.as_mut_ptr() as *mut c_void,
            &mut sid_size,
        );
        if ret == 0 {
            CloseHandle(process_token);
            return Err(ProcessError::ContainmentFailed(
                "CreateWellKnownSid failed".into(),
            ));
        }

        let mut restricted: HANDLE = std::ptr::null_mut();
        let mut restrict_sids = [SID_AND_ATTRIBUTES {
            Sid: sid.as_mut_ptr() as PSID,
            Attributes: 0,
        }];
        let ret = CreateRestrictedToken(
            process_token,
            DISABLE_MAX_PRIVILEGE | SANDBOX_INERT,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            restrict_sids.len() as u32,
            restrict_sids.as_ptr(),
            &mut restricted,
        );
        CloseHandle(process_token);
        if ret == 0 {
            return Err(ProcessError::ContainmentFailed(
                "CreateRestrictedToken failed".into(),
            ));
        }

        if IsTokenRestricted(restricted) == 0 {
            CloseHandle(restricted);
            return Err(ProcessError::ContainmentFailed(
                "CreateRestrictedToken returned a token with IsTokenRestricted == FALSE".into(),
            ));
        }

        Ok(restricted)
    }
}

fn get_current_token() -> Result<HANDLE, ProcessError> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        let ret = OpenProcessToken(GetCurrentProcess(), RESTRICTED_TOKEN_ACCESS, &mut token);
        if ret == 0 {
            return Err(ProcessError::ContainmentFailed(
                "OpenProcessToken failed".into(),
            ));
        }
        Ok(token)
    }
}

/// Attempt one benign restricted-token launch. This is a capability probe, not
/// a product execution path.
fn probe_restricted_launch(token: HANDLE) -> bool {
    unsafe {
        let cmd = OsStr::new("cmd.exe /d /c exit 0")
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>();
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

        let ret = CreateProcessWithTokenW(
            token,
            0,
            std::ptr::null(),
            cmd.as_ptr() as *mut u16,
            CREATE_SUSPENDED,
            std::ptr::null(),
            std::ptr::null(),
            &mut si,
            &mut pi,
        );
        if ret == 0 {
            return false;
        }

        // We only need to prove that a restricted-token child can be created;
        // terminate the suspended probe immediately.
        TerminateProcess(pi.hProcess, 0);
        WaitForSingleObject(pi.hProcess, 5000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        true
    }
}

/// Spawn the actual requested child with a restricted token, suspended, and
/// attach it to the Job Object before resume.
fn spawn_restricted_child(
    request: &ProcessRequest,
    job: JobObject,
) -> Result<WindowsRawChild, ProcessError> {
    let token = create_restricted_token()?;

    let command_line = build_windows_command_line(&request.executable, &request.args);
    let environment_block = build_environment_block(&request.environment)?;
    let cwd_block = build_cwd_block(request.working_directory())?;

    unsafe {
        let (stdout_read, stdout_write) = create_pipe()?;
        let (stderr_read, stderr_write) = create_pipe()?;
        let (stdin_read, stdin_write) = create_pipe()?;

        // The child only needs the write/read handles it actually uses.
        SetHandleInformation(stdout_read, HANDLE_FLAG_INHERIT, 0);
        SetHandleInformation(stderr_read, HANDLE_FLAG_INHERIT, 0);
        SetHandleInformation(stdin_write, HANDLE_FLAG_INHERIT, 0);

        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESTDHANDLES;
        si.hStdInput = stdin_read;
        si.hStdOutput = stdout_write;
        si.hStdError = stderr_write;

        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let ret = CreateProcessWithTokenW(
            token,
            0,
            std::ptr::null(),
            command_line.as_ptr() as *mut u16,
            CREATE_SUSPENDED,
            environment_block
                .as_ref()
                .map(|b| b.as_ptr() as *const c_void)
                .unwrap_or(std::ptr::null()),
            cwd_block
                .as_ref()
                .map(|b| b.as_ptr() as *const u16)
                .unwrap_or(std::ptr::null()),
            &mut si,
            &mut pi,
        );

        // Parent-side pipe write handles are no longer needed.
        CloseHandle(stdout_write);
        CloseHandle(stderr_write);
        CloseHandle(stdin_write);

        if ret == 0 {
            let message = std::io::Error::last_os_error().to_string();
            CloseHandle(stdout_read);
            CloseHandle(stderr_read);
            CloseHandle(stdin_read);
            CloseHandle(token);
            return Err(ProcessError::SpawnFailed {
                executable: request.executable.to_string_lossy().into_owned(),
                message: format!("CreateProcessWithTokenW failed: {message}"),
            });
        }
        CloseHandle(token);

        if let Err(e) = job.assign(pi.hProcess) {
            TerminateProcess(pi.hProcess, 0);
            WaitForSingleObject(pi.hProcess, 5000);
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
            return Err(e);
        }

        if let Err(e) = resume_main_thread(pi.dwProcessId) {
            TerminateProcess(pi.hProcess, 0);
            WaitForSingleObject(pi.hProcess, 5000);
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
            return Err(e);
        }

        Ok(WindowsRawChild {
            process_handle: pi.hProcess,
            thread_handle: pi.hThread,
            job,
            stdout: Some(File::from_raw_handle(stdout_read as _)),
            stderr: Some(File::from_raw_handle(stderr_read as _)),
        })
    }
}

fn create_pipe() -> Result<(HANDLE, HANDLE), ProcessError> {
    unsafe {
        let mut read: HANDLE = std::ptr::null_mut();
        let mut write: HANDLE = std::ptr::null_mut();
        let attrs = windows_sys::Win32::Security::SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>()
                as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: 1,
        };
        let ret = CreatePipe(&mut read, &mut write, &attrs, 0);
        if ret == 0 {
            return Err(ProcessError::ContainmentFailed(format!(
                "CreatePipe failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok((read, write))
    }
}

fn build_windows_command_line(executable: &OsStr, args: &[OsString]) -> Vec<u16> {
    let mut line: Vec<u16> = quote_windows_arg(executable);
    for arg in args {
        line.push(b' ' as u16);
        line.extend(quote_windows_arg(arg));
    }
    line.push(0);
    line
}

/// Quote one argv element using the Windows C runtime parsing rules. This is
/// the same quoting rule used by `std::process::Command` on Windows; it is only
/// used by the restricted-token launch path.
fn quote_windows_arg(arg: &OsStr) -> Vec<u16> {
    let encoded = arg.encode_wide().collect::<Vec<u16>>();
    let mut out: Vec<u16> = Vec::with_capacity(encoded.len() + 2);
    out.push(b'"' as u16);
    let mut backslashes = 0usize;
    for ch in encoded {
        if ch == b'\\' as u16 {
            backslashes += 1;
        } else if ch == b'"' as u16 {
            for _ in 0..backslashes * 2 + 1 {
                out.push(b'\\' as u16);
            }
            out.push(b'"' as u16);
            backslashes = 0;
        } else {
            for _ in 0..backslashes {
                out.push(b'\\' as u16);
            }
            backslashes = 0;
            out.push(ch);
        }
    }
    for _ in 0..backslashes * 2 {
        out.push(b'\\' as u16);
    }
    out.push(b'"' as u16);
    out
}

fn build_environment_block(
    environment: &super::EnvironmentSpec,
) -> Result<Option<Vec<u16>>, ProcessError> {
    match environment {
        super::EnvironmentSpec::Inherit => Ok(None),
        super::EnvironmentSpec::Clear => Ok(Some(vec![0, 0])),
        super::EnvironmentSpec::Explicit(vars) => {
            let mut block = Vec::new();
            for (key, value) in vars {
                let mut entry: Vec<u16> = key.encode_wide().collect();
                entry.push(b'=' as u16);
                entry.extend(value.encode_wide());
                entry.push(0);
                block.extend(entry);
            }
            block.push(0);
            Ok(Some(block))
        }
    }
}

fn build_cwd_block(
    working_directory: Option<&std::path::PathBuf>,
) -> Result<Option<Vec<u16>>, ProcessError> {
    match working_directory {
        None => Ok(None),
        Some(dir) => {
            let mut block: Vec<u16> = dir.as_os_str().encode_wide().collect();
            block.push(0);
            Ok(Some(block))
        }
    }
}

fn resume_main_thread(pid: u32) -> Result<(), ProcessError> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(ProcessError::ContainmentFailed(
                "CreateToolhelp32Snapshot failed; cannot resume suspended child".into(),
            ));
        }

        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

        let mut resumed = false;

        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32OwnerProcessID == pid {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if thread.is_null() {
                        CloseHandle(snapshot);
                        return Err(ProcessError::ContainmentFailed(
                            "OpenThread failed; cannot resume suspended child".into(),
                        ));
                    }
                    let previous = ResumeThread(thread);
                    CloseHandle(thread);
                    // ResumeThread returns the previous suspend count, or
                    // u32::MAX on failure. A newly suspended main thread has
                    // suspend count 1.
                    if previous != u32::MAX {
                        resumed = true;
                    }
                    break;
                }

                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);

        if resumed {
            Ok(())
        } else {
            Err(ProcessError::ContainmentFailed(
                "suspended main thread not found; cannot resume child".into(),
            ))
        }
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
