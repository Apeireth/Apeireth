//! Streaming server process containment using the existing platform primitives.
#![allow(unsafe_code)]
use super::transport::transport_error;
use crate::mcp::McpError;

pub(super) struct ProcessOwner {
    #[cfg(unix)]
    pid: i32,
    #[cfg(windows)]
    job: crate::process::windows::JobObject,
}
// Job handles are transferable kernel objects; this owner is not Clone and all
// access from a streaming channel is protected by its ownership mutex.
#[cfg(windows)]
unsafe impl Send for ProcessOwner {}

impl ProcessOwner {
    pub(super) fn configure(command: &mut tokio::process::Command) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }
        #[cfg(not(unix))]
        let _ = command;
    }
    pub(super) fn attach(child: &tokio::process::Child) -> Result<Self, McpError> {
        #[cfg(unix)]
        {
            let pid = child.id().ok_or_else(|| {
                McpError::Transport("server exited before ownership binding".into())
            })?;
            let pid =
                i32::try_from(pid).map_err(|e| transport_error("server process identity", e))?;
            Ok(Self { pid })
        }
        #[cfg(windows)]
        {
            let job = crate::process::windows::JobObject::create(
                &crate::process::ProcessLimits::default(),
            )
            .map_err(|e| transport_error("creating server job", e))?;
            let handle = child
                .raw_handle()
                .ok_or_else(|| McpError::Transport("server has no process handle".into()))?;
            job.assign(handle)
                .map_err(|e| transport_error("binding server job", e))?;
            Ok(Self { job })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = child;
            Err(McpError::Transport(
                "owned streaming processes unsupported on this platform".into(),
            ))
        }
    }
    pub(super) fn terminate(&self) -> Result<(), McpError> {
        #[cfg(unix)]
        {
            if unsafe { libc::kill(-self.pid, libc::SIGKILL) } == -1 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(transport_error("closing server process group", error));
                }
            }
        }
        #[cfg(windows)]
        self.job
            .terminate()
            .map_err(|e| transport_error("closing server job", e))?;
        Ok(())
    }
}
impl Drop for ProcessOwner {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
