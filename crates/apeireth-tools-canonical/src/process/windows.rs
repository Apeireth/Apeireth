//! Windows process containment: Job Object + fail-closed suspended spawn.
//!
//! The launch sequence is:
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
//! `CreateProcessW` quoting is invented here. The suspended launch is achieved
//! with `std::os::windows::process::CommandExt::creation_flags`.
#![allow(unsafe_code)]

use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
};
use windows_sys::Win32::System::Threading::{
    OpenThread, ResumeThread, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME,
};

use super::{
    apply_request_to_command, supervise, ContainmentKind, ManagedChild, PlatformEnforcement,
    PlatformKind, ProcessError, ProcessLimits, ProcessRequest, ProcessResult,
};

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
            let mut flags: u32 = 0;

            if limits.kill_on_job_close {
                flags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            }

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
                &mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const std::ffi::c_void,
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

pub(crate) fn spawn_and_supervise(request: &ProcessRequest) -> Result<ProcessResult, ProcessError> {
    let job = JobObject::create(&request.limits)?;

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

    supervise(
        WindowsChild { child, job },
        request,
        PlatformEnforcement {
            platform: PlatformKind::Windows,
            containment: ContainmentKind::WindowsJobObject,
            fail_closed_spawn: true,
        },
    )
}

struct WindowsChild {
    child: Child,
    job: JobObject,
}

impl ManagedChild for WindowsChild {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, ProcessError> {
        self.child.try_wait().map_err(io_error)
    }

    fn wait(&mut self) -> Result<ExitStatus, ProcessError> {
        self.child.wait().map_err(io_error)
    }

    fn terminate(&mut self) -> Result<(), ProcessError> {
        // Job Object termination kills the whole tree. Fall back to direct
        // child kill if the job is somehow unusable.
        match self.job.terminate() {
            Ok(()) => Ok(()),
            Err(_) => self.child.kill().map_err(io_error),
        }
    }

    fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.stderr.take()
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

fn io_error(e: std::io::Error) -> ProcessError {
    ProcessError::Io(e.to_string())
}
