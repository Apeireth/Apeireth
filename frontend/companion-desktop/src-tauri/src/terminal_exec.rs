// 舵室 · 终端执行面（星港 I 批原生侧，I-starport-builtins.md「舵室」第一批真实落地）
//
// 契约：
//   1. 用户直操面 —— 本命令由用户在终端面手动发起；智能体驱动的命令执行
//      必须走审批族（审批桥→审批管理→权限包链路），那是后续批的接线缝，
//      本面不代批、不代办；
//   2. cwd 钉死 —— 工作目录必须是绝对路径且真实存在为目录（相对/越界/
//      不存在一律拒绝），执行不得逃出工作区；
//   3. 执行语义 —— Windows `cmd /C`、其他平台 `/bin/sh -c`（用户直操面的
//      常规语义），空命令（含纯空白）拒绝；
//   4. 超时强杀 —— 30 秒未结束强杀子进程（显式 kill + kill_on_drop 兜底）；
//      未取得退出码时 code=None 如实外显（不假成功）；Windows 下孙进程
//      存活风险为后续批接线缝（树杀待接）；
//   5. 输出纪律 —— 合计 256KB 截断且 truncated 如实标注；管道排水带 2s
//      宽限，超限即视为截断，不装全量、不憋死管道。
//
// 纯逻辑（可单测）：validate_command / validate_workspace_arg /
// shell_invocation / duration_ms / truncate_output；IO 面走 fs_err（错误带路径）。

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncRead;

/// 输出合计预算（超出截断并如实标注）。
const OUTPUT_BUDGET_BYTES: usize = 256 * 1024;
/// 命令超时：超时强杀。
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// 管道排水宽限：子进程退出后余量排不完/孙进程占管道时的弃取线。
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// 终端执行回执（Serialize 供前端消费）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalResult {
    /// None = 未取得退出码（超时强杀/信号终了），如实外显非正常终了。
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// 输出超预算被截断（含排水宽限超限），如实标注。
    pub truncated: bool,
}

/// 命令校验：空/纯空白拒绝（0 装）。
pub fn validate_command(command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err("命令不能为空".to_string());
    }
    Ok(())
}

/// 工作目录参数校验：非空 + 绝对路径（存在性另验，IO 面带路径报错）。
pub fn validate_workspace_arg(workspace_dir: &str) -> Result<PathBuf, String> {
    let raw = workspace_dir.trim();
    if raw.is_empty() {
        return Err("工作目录不能为空".to_string());
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(format!("工作目录必须是绝对路径（拒绝越界）：{raw}"));
    }
    Ok(path)
}

/// 平台壳语义：Windows `cmd /C`，其余 `/bin/sh -c`（用户直操面常规语义）。
pub fn shell_invocation(command: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), command.to_string()],
        )
    } else {
        (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), command.to_string()],
        )
    }
}

/// 时长换算（饱和，不溢出）。
pub fn duration_ms(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// 合计预算截断：先给 stdout 半额，stderr 吃余量；超额必标 truncated。
pub fn truncate_output(stdout: &[u8], stderr: &[u8], budget: usize) -> (Vec<u8>, Vec<u8>, bool) {
    if stdout.len().saturating_add(stderr.len()) <= budget {
        return (stdout.to_vec(), stderr.to_vec(), false);
    }
    let stdout_cap = budget / 2;
    let out: Vec<u8> = stdout.iter().take(stdout_cap).copied().collect();
    let stderr_cap = budget - out.len();
    let err: Vec<u8> = stderr.iter().take(stderr_cap).copied().collect();
    (out, err, true)
}

/// 有界排水：只存预算内的字节，超限继续排水但不再存储（防憋死管道）。
async fn read_capped<R: AsyncRead + Unpin>(mut reader: R, cap: usize) -> (Vec<u8>, bool) {
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                if buf.len() < cap {
                    let take = (cap - buf.len()).min(n);
                    buf.extend_from_slice(&chunk[..take]);
                    if take < n {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            // 读管道出错即止（进程已终/管道已关），如实收尾不假装读全。
            Err(_) => break,
        }
    }
    (buf, truncated)
}

/// 在工作区内执行用户直操命令（舵室面；智能体驱动走审批族，见契约 §1）。
#[tauri::command]
pub async fn run_workspace_command(
    command: String,
    workspace_dir: String,
) -> Result<TerminalResult, String> {
    validate_command(&command)?;
    let dir = validate_workspace_arg(&workspace_dir)?;
    // fs_err 政策：路径携带型错误带路径外显。
    let meta = fs_err::metadata(&dir).map_err(|e| format!("工作目录不可读（{e}）"))?;
    if !meta.is_dir() {
        return Err(format!("工作目录不是目录：{}", dir.display()));
    }

    let (program, args) = shell_invocation(&command);
    let started = Instant::now();
    let mut child = tokio::process::Command::new(&program)
        .args(&args)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("命令启动失败（{program}）：{e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "拿不到标准输出管道".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "拿不到标准错误管道".to_string())?;
    let out_task = tokio::spawn(read_capped(stdout, OUTPUT_BUDGET_BYTES));
    let err_task = tokio::spawn(read_capped(stderr, OUTPUT_BUDGET_BYTES));

    let mut exit_code: Option<i32> = None;
    let mut timed_out = false;
    match tokio::time::timeout(COMMAND_TIMEOUT, child.wait()).await {
        Ok(Ok(status)) => exit_code = status.code(),
        Ok(Err(e)) => return Err(format!("等待命令结束失败：{e}")),
        Err(_) => {
            timed_out = true;
            let _ = child.kill().await;
        }
    }

    // 排水宽限：孙进程占管道时超限弃取并如实标截断（契约 §5）。
    let (out_raw, out_trunc) = match tokio::time::timeout(DRAIN_GRACE, out_task).await {
        Ok(Ok(v)) => v,
        _ => (Vec::new(), true),
    };
    let (err_raw, err_trunc) = match tokio::time::timeout(DRAIN_GRACE, err_task).await {
        Ok(Ok(v)) => v,
        _ => (Vec::new(), true),
    };

    let (out_capped, err_capped, cap_trunc) =
        truncate_output(&out_raw, &err_raw, OUTPUT_BUDGET_BYTES);

    Ok(TerminalResult {
        // 超时强杀/信号终了未取得退出码：code=None 如实外显（契约 §4）。
        code: if timed_out { None } else { exit_code },
        stdout: String::from_utf8_lossy(&out_capped).into_owned(),
        stderr: String::from_utf8_lossy(&err_capped).into_owned(),
        duration_ms: duration_ms(started.elapsed()),
        truncated: out_trunc || err_trunc || cap_trunc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_small_output_intact() {
        let (out, err, trunc) = truncate_output(b"abc", b"de", 200);
        assert_eq!(out, b"abc");
        assert_eq!(err, b"de");
        assert!(!trunc);
    }

    #[test]
    fn truncate_boundary_exact_budget_is_not_truncated() {
        let (out, err, trunc) = truncate_output(&[b'a'; 100], &[b'b'; 100], 200);
        assert!(!trunc);
        assert_eq!(out.len(), 100);
        assert_eq!(err.len(), 100);
    }

    #[test]
    fn truncate_over_budget_caps_combined() {
        // 超额：stdout 先限半额，stderr 吃余量，truncated 如实标注。
        let (out, err, trunc) = truncate_output(&[b'a'; 300], &[b'b'; 300], 200);
        assert!(trunc);
        assert_eq!(out.len(), 100);
        assert_eq!(err.len(), 100);
    }

    #[test]
    fn truncate_stderr_uses_stdout_leftover() {
        // stdout 不足半额时 stderr 吃满余量。
        let (out, err, trunc) = truncate_output(&[b'a'; 10], &[b'b'; 300], 200);
        assert!(trunc);
        assert_eq!(out.len(), 10);
        assert_eq!(err.len(), 190);
    }

    #[test]
    fn empty_command_rejected() {
        assert!(validate_command("").is_err());
        assert!(validate_command("   \t\n").is_err());
        assert!(validate_command("echo ok").is_ok());
    }

    #[test]
    fn workspace_arg_requires_absolute_path() {
        assert!(validate_workspace_arg("").is_err());
        assert!(validate_workspace_arg("  ").is_err());
        assert!(validate_workspace_arg("relative/dir").is_err());
        let abs = std::env::current_dir().expect("cwd");
        assert!(validate_workspace_arg(&abs.to_string_lossy()).is_ok());
    }

    #[test]
    fn duration_ms_is_saturating_millis() {
        assert_eq!(duration_ms(Duration::from_millis(1500)), 1500);
        assert_eq!(duration_ms(Duration::from_micros(500)), 0);
        assert_eq!(duration_ms(Duration::from_secs(2)), 2000);
    }

    #[test]
    fn shell_invocation_matches_platform_semantics() {
        let (program, args) = shell_invocation("echo hi");
        if cfg!(windows) {
            assert_eq!(program, "cmd");
            assert_eq!(args, vec!["/C".to_string(), "echo hi".to_string()]);
        } else {
            assert_eq!(program, "/bin/sh");
            assert_eq!(args, vec!["-c".to_string(), "echo hi".to_string()]);
        }
    }
}
