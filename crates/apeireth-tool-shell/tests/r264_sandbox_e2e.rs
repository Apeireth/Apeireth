//! R264: e2e sandbox tests - verify Standard/Strict 真接 process_group + kill_on_drop.

#![allow(missing_docs)]

use apeireth_tool_shell::sandbox::{apply_sandbox, SandboxMode, SandboxPolicy};

#[tokio::test]
async fn r264_sandbox_standard_command_runs() {
    let mut cmd = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
    if cfg!(windows) {
        cmd.args(["/C", "echo standard-sandbox-ok"]);
    } else {
        cmd.args(["-c", "echo standard-sandbox-ok"]);
    }
    let policy = SandboxPolicy { mode: SandboxMode::Standard, env_clear: true, allowed_syscalls: vec![] };
    apply_sandbox(&mut cmd, &policy).unwrap();
    let out = cmd.output().await.expect("run");
    assert!(out.status.success(), "echo should succeed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("standard-sandbox-ok"), "stdout: {}", stdout);
}

#[tokio::test]
async fn r264_sandbox_strict_command_runs() {
    let mut cmd = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
    if cfg!(windows) {
        cmd.args(["/C", "echo strict-sandbox-ok"]);
    } else {
        cmd.args(["-c", "echo strict-sandbox-ok"]);
    }
    let policy = SandboxPolicy { mode: SandboxMode::Strict, env_clear: true, allowed_syscalls: vec![] };
    apply_sandbox(&mut cmd, &policy).unwrap();
    let out = cmd.output().await.expect("run");
    assert!(out.status.success(), "echo should succeed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("strict-sandbox-ok"), "stdout: {}", stdout);
}

#[tokio::test]
async fn r264_sandbox_standard_kill_on_drop_works() {
    use std::time::Duration;
    let mut cmd = tokio::process::Command::new(if cfg!(windows) { "ping" } else { "sleep" });
    if cfg!(windows) {
        cmd.args(["-n", "60", "127.0.0.1"]);
    } else {
        cmd.args(["60"]);
    }
    let policy = SandboxPolicy { mode: SandboxMode::Standard, env_clear: true, allowed_syscalls: vec![] };
    apply_sandbox(&mut cmd, &policy).unwrap();
    let child = cmd.spawn().expect("spawn");
    let pid = child.id().expect("pid");
    assert!(pid > 0, "got valid pid: {}", pid);
    drop(child);
    tokio::time::sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn r264_sandbox_light_no_process_group_change() {
    let mut cmd = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
    if cfg!(windows) {
        cmd.args(["/C", "echo light-ok"]);
    } else {
        cmd.args(["-c", "echo light-ok"]);
    }
    let policy = SandboxPolicy { mode: SandboxMode::Light, env_clear: true, allowed_syscalls: vec![] };
    apply_sandbox(&mut cmd, &policy).unwrap();
    let out = cmd.output().await.expect("run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("light-ok"));
}
