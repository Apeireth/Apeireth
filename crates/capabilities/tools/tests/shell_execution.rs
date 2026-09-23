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

/// Regression for 2026-10-06 真机: 双引号命令 (powershell -Command "...") 曾被
/// cmd /S 的引号规则吃掉, 只回显不执行。外层引号包裹后内层双引号必须存活。
#[cfg(windows)]
#[tokio::test]
async fn double_quoted_powershell_command_executes() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
    let value = invoke(
        &tool,
        r#"powershell -NoProfile -Command "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; Write-Output wrapped-doublequote-ok""#,
        None,
        None,
    )
    .await;
    let stdout = value["stdout"].as_str().unwrap();
    assert!(
        stdout.contains("wrapped-doublequote-ok"),
        "double-quoted command must execute, stdout was {stdout:?}"
    );
    assert_eq!(value["exit_code"], json!(0));
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

    // 2026-10-10: 原用 `ping -n 30 127.0.0.1` 当长命令 —— 沙箱断网后 ping 秒死
    // (连 loopback 都不开), 超时语义前提失效。换无网络长命令 (powershell 休眠)。
    #[cfg(windows)]
    let command = "powershell -NoProfile -Command \"Start-Sleep -Seconds 30\"";
    #[cfg(not(windows))]
    let command = "sleep 30";

    let value = invoke(&tool, command, None, Some(1_000)).await;
    assert_eq!(value["timed_out"], json!(true));
}

/// W1 §2.5 E2E「断网」: AppContainer 零网络 capability → 连 loopback 都建不起
/// socket。`ping` 必须**快速失败** (不是超时) —— 这就是沙箱断网的行为证据。
#[tokio::test]
async fn sandbox_denies_network_even_loopback() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    let value = invoke(&tool, "ping -n 2 127.0.0.1", None, Some(5_000)).await;
    assert_eq!(
        value["timed_out"],
        json!(false),
        "ping 应立即失败而非运行到超时 (网络能力缺失, socket 起不来)"
    );
    assert_ne!(
        value["exit_code"],
        json!(0),
        "ping 不应成功 —— 沙箱断网含 loopback: {value}"
    );
}

/// W1 §2.5 E2E「界外读败」(断言反转锚点): 未授权的**用户空间**路径零访问。
///
/// 真实边界 (2026-10-10 ACL 验尸, 台账 #49): Windows 给系统文件自带
/// `ALL APPLICATION PACKAGES` 继承读执行 (win.ini 在列, 沙箱内仍可读 —— 系统
/// 自我放行, 非本墙缺口); 墙挡的是**用户空间全盘** (C:\Users\* 无任何 AAP ACE)。
/// 2026-10-06 真机 "全盘可读" 的非沙箱实锤, 在此以用户空间界外文件反转。
#[tokio::test]
async fn sandbox_denies_workspace_external_reads() {
    let outside = tempdir().unwrap();
    let secret_path = outside.path().join("secret.txt");
    std::fs::write(&secret_path, "APEIRETH_OUTSIDE_SECRET_987").unwrap();

    let ws = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(ws.path().to_path_buf()));

    let command = format!("type {}", secret_path.display());
    let value = invoke(&tool, &command, None, None).await;
    let stdout = value["stdout"].as_str().unwrap_or_default();
    assert!(
        !stdout.contains("APEIRETH_OUTSIDE_SECRET_987"),
        "界外用户空间文件必须零访问 (沙箱墙): {value}"
    );
}

/// W1 §2.5 E2E「工作区写成」: 授予的工作区目录读写正常 —— 墙不挡自己人。
#[tokio::test]
async fn sandbox_allows_workspace_writes() {
    let tmp = tempdir().unwrap();
    let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));

    #[cfg(windows)]
    let command = "echo sandbox-ok > probe.txt && type probe.txt";
    #[cfg(not(windows))]
    let command = "echo sandbox-ok > probe.txt && cat probe.txt";

    let value = invoke(&tool, command, None, None).await;
    assert_eq!(value["exit_code"], json!(0), "{value}");
    let stdout = value["stdout"].as_str().unwrap_or_default();
    assert!(stdout.contains("sandbox-ok"), "{value}");
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
