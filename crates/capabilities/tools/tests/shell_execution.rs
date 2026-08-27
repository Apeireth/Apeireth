//! Real one-shot Trusted Shell execution tests.
//!
//! These tests run only harmless platform-native commands: echo/printf, cwd
//! verification, exit codes, and bounded sleep. No filesystem destruction, no
//! public network, no real credentials.

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_tools_canonical::{ShellTool, TrustedShellConfig};
use serde_json::json;
use tempfile::tempdir;

async fn invoke(
    tool: &ShellTool,
    command: &str,
    cwd: Option<&str>,
    timeout_ms: Option<u64>,
) -> serde_json::Value {
    let mut args = json!({ "command": command });
    if let Some(cwd) = cwd {
        args["cwd"] = json!(cwd);
    }
    if let Some(timeout_ms) = timeout_ms {
        args["timeout_ms"] = json!(timeout_ms);
    }
    let call = ToolCall {
        id: "call_shell".into(),
        name: "shell".into(),
        arguments: args,
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "shell call failed: {}", result.render());
    let value = match result.outcome {
        apeireth_protocol::canonical::ToolOutcome::Ok { value } => value,
        other => panic!("expected ok outcome, got {other:?}"),
    };
    value
}

#[tokio::test]
async fn echo_fixture_executes_and_captures_stdout() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "echo shell_fixture_ok";
    #[cfg(not(windows))]
    let command = "printf 'shell_fixture_ok'";

    let value = invoke(&tool, command, None, None).await;
    let stdout = value["stdout"].as_str().unwrap();
    assert!(stdout.contains("shell_fixture_ok"), "stdout was {stdout:?}");
    assert_eq!(value["exit_code"], json!(0));
    assert_eq!(value["timed_out"], json!(false));
}

#[tokio::test]
async fn explicit_cwd_is_used() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "cd";
    #[cfg(not(windows))]
    let command = "pwd";

    let value = invoke(&tool, command, None, None).await;
    let stdout = value["stdout"].as_str().unwrap();
    let canonical_root = tmp.path().canonicalize().unwrap();
    let root_text = canonical_root.to_string_lossy().to_string();

    // macOS may report /private/var vs /var; compare canonicalized tail.
    assert!(
        stdout.contains(&root_text)
            || stdout.contains(tmp.path().to_string_lossy().as_ref())
            || stdout.contains(
                canonical_root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            ),
        "stdout {stdout:?} should reflect the explicit workspace root {root_text:?}"
    );
}

#[tokio::test]
async fn nonzero_exit_is_a_normal_result() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "exit /b 7";
    #[cfg(not(windows))]
    let command = "exit 7";

    let value = invoke(&tool, command, None, None).await;
    assert_eq!(value["exit_code"], json!(7));
    assert_eq!(value["timed_out"], json!(false));
}

#[tokio::test]
async fn bounded_timeout_terminates_long_sleep() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "ping -n 30 127.0.0.1 > nul";
    #[cfg(not(windows))]
    let command = "sleep 30";

    let value = invoke(&tool, command, None, Some(1_000)).await;
    assert_eq!(value["timed_out"], json!(true));
}

#[tokio::test]
async fn cwd_escape_is_rejected_before_execution() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
    let call = ToolCall {
        id: "call_shell".into(),
        name: "shell".into(),
        arguments: json!({ "command": "echo should_not_run", "cwd": "../" }),
    };
    let result = tool.invoke(&call).await;
    assert!(!result.is_ok());
    assert!(result.render().contains("escapes"), "{}", result.render());
}

#[tokio::test]
async fn unicode_script_round_trips_without_normalization() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "powershell -NoProfile -NonInteractive -EncodedCommand WwBDAG8AbgBzAG8AbABlAF0AOgA6AE8AdQB0AHAAdQB0AEUAbgBjAG8AZABpAG4AZwA9AFsAVABlAHgAdAAuAFUAVABGADgARQBuAGMAbwBkAGkAbgBnAF0AOgA6AG4AZQB3ACgAKQA7ACAAVwByAGkAdABlAC0ATwB1AHQAcAB1AHQAIABhAHAAZQBpAHIAZQB0AGgAXwDqlg==";
    #[cfg(not(windows))]
    let command = "printf 'apeireth_雪'";

    let value = invoke(&tool, command, None, Some(120_000)).await;
    let stdout = value["stdout"].as_str().unwrap();
    assert!(stdout.contains("apeireth_雪"), "stdout was {stdout:?}");
}
