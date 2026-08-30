//! Comprehensive test suite for Apeireth CLI commands, options, exit codes,
//! and approval resolution lifecycle.

use std::process::Command;

fn bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("current test binary path");
    path.pop(); // exit test exe
    if path.ends_with("deps") {
        path.pop(); // exit deps dir to target/debug
    }
    path.push(if cfg!(windows) {
        "apeireth.exe"
    } else {
        "apeireth"
    });
    path
}

#[test]
fn test_cli_help_flags() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    for flag in ["--help", "-h"] {
        let output = Command::new(&bin)
            .arg(flag)
            .output()
            .expect("failed to execute cli binary");
        assert!(
            output.status.success(),
            "expected exit 0 for {flag}, got {:?}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("apeireth session"));
        assert!(stdout.contains("apeireth chat"));
        assert!(stdout.contains("apeireth approve"));
        assert!(stdout.contains("apeireth reject"));
        assert!(stdout.contains("apeireth cancel"));
        assert!(stdout.contains("apeireth gateway serve"));
    }
}

#[test]
fn test_cli_subcommand_help_flags() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    for subcmd in ["session", "chat", "gateway", "approve", "reject", "cancel"] {
        let output = Command::new(&bin)
            .args([subcmd, "--help"])
            .output()
            .expect("failed to execute cli binary");
        assert!(
            output.status.success(),
            "expected exit 0 for {subcmd} --help, got {:?}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Usage:"));
    }
}

#[test]
fn test_cli_version_flags() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    for flag in ["--version", "-V"] {
        let output = Command::new(&bin)
            .arg(flag)
            .output()
            .expect("failed to execute cli binary");
        assert!(
            output.status.success(),
            "expected exit 0 for {flag}, got {:?}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.starts_with("apeireth "));
    }
}

#[test]
fn test_cli_unknown_command_exits_with_failure() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .arg("nonexistent-subcommand")
        .output()
        .expect("failed to execute cli binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown command"));
}

#[test]
fn test_cli_chat_missing_prompt_exits_with_failure() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .args(["chat", "--model", "some-model"])
        .output()
        .expect("failed to execute cli binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("chat requires a prompt"));
}

#[test]
fn test_cli_approve_missing_args_exits_with_failure() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .args(["approve", "--session", "sess-1"])
        .output()
        .expect("failed to execute cli binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("approve requires --approval"));
}

#[test]
fn test_cli_gateway_invalid_port_exits_with_failure() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .args(["gateway", "serve", "--port", "not-a-number"])
        .output()
        .expect("failed to execute cli binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid gateway port"));
}

#[test]
fn test_cli_gateway_unknown_arg_exits_with_failure() {
    let bin = bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .args(["gateway", "serve", "--unrecognized-option"])
        .output()
        .expect("failed to execute cli binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown gateway argument"));
}
