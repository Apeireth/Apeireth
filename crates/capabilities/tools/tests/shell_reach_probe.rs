//! Shell 可达范围 live 探针 (2026-10-06 挂账核销批, 台账挂账 #7 批注)。
//!
//! **目的**: 实测 (不靠源码推理) shell 执行底座 (`ProcessExecutor` = ShellTool 的
//! 真实隔离层) 在**非沙箱现状**下的可达范围 —— 为 "shell 以主人本机账号全权运行"
//! (shell-sandbox-lite 设计的动机) 提供可复现证据, 并作为 W1 沙箱落地后的
//! **回归锚点**: 沙箱开时本探针行为必须反转 (读工作区外失败)。
//!
//! **口径 (0 装)**:
//! - `#[ignore]` 默认不跑 (需真机); 手动: `cargo test -p apeireth-tools-canonical --test shell_reach_probe -- --ignored --nocapture`
//! - 断言方向是**证明可达** (非沙箱现状的事实), 不是"希望它不可达";
//!   W1 沙箱落地后应新增对偶探针并反转断言, 本文件保留为历史证据。
//! - 探针只读不写 (读一个系统自带只读文件), 0 副作用。

use apeireth_tools_canonical::process::{ProcessExecutor, ProcessRequest};

/// 可达探针: 工作区外的系统文件读得到 = 非沙箱实锤。
#[test]
#[ignore = "live reach probe (real machine, no key); run manually"]
fn shell_reach_reads_outside_workspace() {
    let request = {
        #[cfg(windows)]
        {
            ProcessRequest::new("cmd.exe")
                .with_raw_arg("/D /S /C \"type %SystemRoot%\\win.ini\"")
        }
        #[cfg(not(windows))]
        {
            ProcessRequest::new("sh")
                .with_arg("-c")
                .with_arg("cat /etc/hostname")
        }
    };

    let result = ProcessExecutor::new()
        .execute(&request)
        .expect("spawn must succeed (非沙箱现状: 无任何拦截)");
    let stdout = String::from_utf8_lossy(&result.stdout);
    println!(
        "[reach-probe] exit={:?} stdout={:?}",
        result.exit_code(),
        stdout.chars().take(200).collect::<String>()
    );

    #[cfg(windows)]
    assert!(
        stdout.to_lowercase().contains("[fonts]"),
        "win.ini 内容可见 = shell 可读工作区外全盘 (非沙箱实锤): {stdout:?}"
    );
    #[cfg(not(windows))]
    assert!(
        !stdout.trim().is_empty(),
        "/etc/hostname 可见 = 工作区外可读 (非沙箱实锤): {stdout:?}"
    );
}
