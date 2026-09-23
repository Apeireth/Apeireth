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

use apeireth_tools_canonical::process::{
    IsolationCapability, IsolationRequirement, ProcessExecutor, ProcessRequest,
};

/// 可达探针: 工作区外的系统文件读得到 = 非沙箱实锤。
///
/// **历史证据保留** (2026-10-10 W1 落地批注): win.ini 自带
/// `ALL APPLICATION PACKAGES (I)(RX)` 继承 ACL (系统文件自我放行) —— 沙箱开启后
/// 本探针可能**依然绿**, 不再是"非沙箱"的充分证据; 边界证据见下方对偶探针
/// `shell_sandbox_probe_walls_user_space` (用户空间零访问 = 墙成立)。
#[test]
#[ignore = "live reach probe (real machine, no key); run manually"]
fn shell_reach_reads_outside_workspace() {
    let request = {
        #[cfg(windows)]
        {
            ProcessRequest::new("cmd.exe").with_raw_arg("/D /S /C \"type %SystemRoot%\\win.ini\"")
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

/// **对偶探针** (W1 沙箱落地, 2026-10-10): 沙箱要求下 —— **用户空间**界外文件
/// 零访问 (墙成立) + 工作区内读写成。边界真相 (台账 #49 ACL 验尸): Windows 给
/// 系统文件自带 AAP 继承 (win.ini 沙箱内仍可读, 系统自我放行, 非墙缺口);
/// 墙挡的是用户空间全盘 (C:\Users\* 无 AAP ACE)。2026-10-06 "全盘可读" 实锤在此反转。
#[test]
#[ignore = "live sandbox probe (real machine); run manually"]
fn shell_sandbox_probe_walls_user_space() {
    let outside = tempfile::tempdir().expect("outside dir");
    let secret = outside.path().join("probe_secret.txt");
    std::fs::write(&secret, "APEIRETH_PROBE_SECRET_654").expect("write secret");
    let ws = tempfile::tempdir().expect("workspace dir");

    let sandboxed = IsolationRequirement::new()
        .require_enforced(IsolationCapability::FilesystemIsolation)
        .require_enforced(IsolationCapability::NetworkIsolation);

    // ① 界外用户空间文件必须读不到。
    let read_outside = ProcessRequest::new("cmd.exe")
        .with_raw_arg(format!("/D /S /C \"type {}\"", secret.display()))
        .with_working_directory(ws.path().to_path_buf())
        .with_isolation(sandboxed.clone());
    let read_result = ProcessExecutor::new()
        .execute(&read_outside)
        .expect("sandboxed spawn must succeed (墙存在 ≠ 起不来)");
    let read_stdout = String::from_utf8_lossy(&read_result.stdout);
    println!(
        "[sandbox-probe] outside-read exit={:?} stdout={:?}",
        read_result.exit_code(),
        read_stdout.chars().take(200).collect::<String>()
    );
    assert!(
        !read_stdout.contains("APEIRETH_PROBE_SECRET_654"),
        "界外用户空间文件必须零访问 (墙): {read_stdout:?}"
    );

    // ② 工作区内读写必须成 (墙不挡自己人)。
    let write_in = ProcessRequest::new("cmd.exe")
        .with_raw_arg("/D /S /C \"echo sandbox-ok > in_probe.txt && type in_probe.txt\"")
        .with_working_directory(ws.path().to_path_buf())
        .with_isolation(sandboxed);
    let write_result = ProcessExecutor::new()
        .execute(&write_in)
        .expect("sandboxed spawn must succeed");
    let write_stdout = String::from_utf8_lossy(&write_result.stdout);
    println!(
        "[sandbox-probe] inside-write exit={:?} stdout={:?}",
        write_result.exit_code(),
        write_stdout.chars().take(200).collect::<String>()
    );
    assert!(
        write_stdout.contains("sandbox-ok"),
        "工作区内读写应成: {write_stdout:?}"
    );
}
