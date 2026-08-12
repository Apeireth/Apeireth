//! Streaming stdout/stderr capture for long-running commands.
//!
//! Wraps `tokio::io::AsyncBufReadExt::lines()` so consumers can receive
//! command output as it arrives (vs. waiting for completion).
//!
//! Used by EnhancedShell::exec_streaming to feed lines into a tokio mpsc
//! channel.

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout};

/// Read stdout from a child to end, collecting lines into a Vec<String>.
pub async fn collect_stdout(child: &mut Child) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            lines.push(line);
        }
    }
    // Reap to avoid zombie
    let _ = child.wait().await;
    lines
}

/// Read stderr lines.
pub async fn collect_stderr(child: &mut Child) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            lines.push(line);
        }
    }
    lines
}

/// Helper to take stdout reference for streaming to a channel.
pub fn stdout_reader(child: &mut Child) -> Option<BufReader<ChildStdout>> {
    child.stdout.take().map(BufReader::new)
}

/// Helper to take stderr reference.
pub fn stderr_reader(child: &mut Child) -> Option<BufReader<ChildStderr>> {
    child.stderr.take().map(BufReader::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn collect_stdout_short_command() {
        // `echo` is a cmd.exe builtin on Windows; use cmd /c.
        let mut child = if cfg!(windows) {
            tokio::process::Command::new("cmd")
                .args(["/c", "echo", "hello"])
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        } else {
            tokio::process::Command::new("echo")
                .arg("hello")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        };
        let lines = collect_stdout(&mut child).await;
        assert_eq!(lines, vec!["hello".to_string()]);
    }
}
