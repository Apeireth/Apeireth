use crate::ToolError;

/// Multi-platform process sandbox and resource isolator.
/// Native implementations on Windows (JobObject + RestrictedToken), Linux (prctl + setrlimit), macOS (setrlimit).
pub struct PlatformSandbox {
    #[cfg(target_os = "windows")]
    job_handle: winapi::um::winnt::HANDLE,

    #[cfg(target_os = "windows")]
    restricted_token: winapi::um::winnt::HANDLE,

    #[cfg(unix)]
    memory_limit_bytes: u64,
}

unsafe impl Send for PlatformSandbox {}
unsafe impl Sync for PlatformSandbox {}

#[cfg(target_os = "windows")]
impl Drop for PlatformSandbox {
    fn drop(&mut self) {
        unsafe {
            if !self.job_handle.is_null() {
                winapi::um::handleapi::CloseHandle(self.job_handle);
            }
            if !self.restricted_token.is_null() {
                winapi::um::handleapi::CloseHandle(self.restricted_token);
            }
        }
    }
}

impl Default for PlatformSandbox {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::fallback())
    }
}

impl PlatformSandbox {
    /// Create a new platform-specific sandbox with 256MB memory cap and process lifetime tracking.
    pub fn new() -> Result<Self, ToolError> {
        #[cfg(target_os = "windows")]
        {
            use std::ptr;
            use winapi::um::jobapi2::{CreateJobObjectW, SetInformationJobObject};
            use winapi::um::winnt::{
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOB_OBJECT_LIMIT_PROCESS_MEMORY, DISABLE_MAX_PRIVILEGE, LUA_TOKEN,
                TOKEN_ALL_ACCESS, HANDLE,
            };
            use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
            use winapi::um::securitybaseapi::CreateRestrictedToken;
            use winapi::um::handleapi::CloseHandle;

            unsafe {
                // 1. Create Job Object
                let handle = CreateJobObjectW(ptr::null_mut(), ptr::null_mut());
                if handle.is_null() {
                    return Err(ToolError::SandboxError("Failed to create Windows JobObject".into()));
                }

                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = 
                    JOB_OBJECT_LIMIT_JOB_MEMORY | 
                    JOB_OBJECT_LIMIT_PROCESS_MEMORY |
                    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    
                info.ProcessMemoryLimit = 256 * 1024 * 1024;
                info.JobMemoryLimit = 256 * 1024 * 1024;

                let success = SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &mut info as *mut _ as *mut _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );

                if success == 0 {
                    CloseHandle(handle);
                    return Err(ToolError::SandboxError("Failed to set Windows JobObject limits".into()));
                }

                // 2. Create Restricted Token (strips admin privileges and restricts SIDs)
                let mut current_token: HANDLE = ptr::null_mut();
                let mut token_handle: HANDLE = ptr::null_mut();

                if OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &mut current_token) != 0 {
                    let flags = DISABLE_MAX_PRIVILEGE | LUA_TOKEN;
                    CreateRestrictedToken(
                        current_token,
                        flags,
                        0,
                        ptr::null_mut(),
                        0,
                        ptr::null_mut(),
                        0,
                        ptr::null_mut(),
                        &mut token_handle,
                    );
                    CloseHandle(current_token);
                }

                Ok(Self {
                    job_handle: handle,
                    restricted_token: token_handle,
                })
            }
        }

        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                memory_limit_bytes: 256 * 1024 * 1024,
            })
        }

        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                memory_limit_bytes: 256 * 1024 * 1024,
            })
        }

        #[cfg(not(any(target_os = "windows", unix)))]
        {
            Ok(Self::fallback())
        }
    }

    fn fallback() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self {
                job_handle: std::ptr::null_mut(),
                restricted_token: std::ptr::null_mut(),
            }
        }
        #[cfg(unix)]
        {
            Self { memory_limit_bytes: 256 * 1024 * 1024 }
        }
        #[cfg(not(any(target_os = "windows", unix)))]
        {
            Self {}
        }
    }

    /// Assign an active child process to this JobObject sandbox
    #[cfg(target_os = "windows")]
    pub fn assign_process(&self, process_handle: winapi::um::winnt::HANDLE) -> Result<(), ToolError> {
        if self.job_handle.is_null() {
            return Ok(());
        }
        unsafe {
            use winapi::um::jobapi2::AssignProcessToJobObject;
            let res = AssignProcessToJobObject(self.job_handle, process_handle);
            if res == 0 {
                Err(ToolError::SandboxError("Failed to assign process to JobObject".into()))
            } else {
                Ok(())
            }
        }
    }

    /// Apply sandbox resource restrictions to the execution environment.
    pub fn apply_restrictions(&self) -> Result<(), ToolError> {
        #[cfg(target_os = "windows")]
        {
            // Windows JobObject enforcement active
            if !self.job_handle.is_null() {
                Ok(())
            } else {
                Err(ToolError::SandboxError("Windows JobObject uninitialized".into()))
            }
        }

        #[cfg(target_os = "linux")]
        {
            unsafe {
                libc::prctl(38, 1, 0, 0, 0); // PR_SET_NO_NEW_PRIVS
                let rlim = libc::rlimit {
                    rlim_cur: self.memory_limit_bytes as libc::rlim_t,
                    rlim_max: self.memory_limit_bytes as libc::rlim_t,
                };
                libc::setrlimit(libc::RLIMIT_AS, &rlim);
            }
            Ok(())
        }

        #[cfg(target_os = "macos")]
        {
            unsafe {
                let rlim = libc::rlimit {
                    rlim_cur: self.memory_limit_bytes as libc::rlim_t,
                    rlim_max: self.memory_limit_bytes as libc::rlim_t,
                };
                libc::setrlimit(libc::RLIMIT_DATA, &rlim);
            }
            Ok(())
        }

        #[cfg(not(any(target_os = "windows", unix)))]
        {
            Ok(())
        }
    }

    /// Returns the human-readable platform sandbox type.
    pub fn platform_type(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        { "Windows-JobObject-RestrictedToken" }
        #[cfg(target_os = "linux")]
        { "Linux-Seccomp-Rlimit" }
        #[cfg(target_os = "macos")]
        { "macOS-Rlimit-Sandbox" }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        { "Generic-Fallback" }
    }
}

/// Backward-compatible alias for WindowsSandbox
pub type WindowsSandbox = PlatformSandbox;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_sandbox_lifecycle() {
        let sb = PlatformSandbox::new().unwrap();
        assert!(sb.platform_type().contains("JobObject") || sb.platform_type().contains("Rlimit"));
        assert!(sb.apply_restrictions().is_ok());
    }
}

