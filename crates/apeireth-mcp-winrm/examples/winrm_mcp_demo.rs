//! # apeireth-mcp-winrm demo
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main/mcp/WinRMMcpServer.js` 的 5 工具最小演示:
//! 1. `winrm_connect` — 建 WinRM 连接 (HTTP, port 5985, Negotiate auth)
//! 2. `winrm_run_command` — 跑 `Get-Process | Select-Object -First 3` PowerShell
//! 3. `winrm_get_command_output` — 取命令输出
//! 4. `winrm_cleanup_command` — 清理命令
//! 5. `winrm_disconnect` — 断开 + 从连接池清理
//!
//! ## 状态
//!
//! ⚠️ skeleton — 当前所有 trait 方法返回占位, **不真连 WinRM**.
//! 完整 1:1 翻译放 R20 阶段 1 实施期 (per `v09021-commercial-extract §6` 5 阶段重写).
//!
//! ## 跑法
//!
//! ```bash
//! cargo run -p apeireth-mcp-winrm --example winrm_mcp_demo
//! ```
//!
//! ## 引用
//!
//! - `docs/stage4/v09021-commercial-extract-2026-08-05.md` §3.2 #10
//! - v0.9.21 商业版 `out/main/mcp/WinRMMcpServer.js` server name `winrm-remote`

use std::time::Duration;

use apeireth_mcp_winrm::{
    SecretString, WinRmAuthMethod, WinRmAuthMethodKind, WinRmCommandStatus, WinRmEndpoint,
    WinRmMcpConfig, WinRmMcpServer, WinRmMcpServerTrait, WinRmTransport,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("apeireth-mcp-winrm demo (skeleton, per v0.9.21 WinRMMcpServer.js 1:1 翻译)");

    // 默认配置: HTTP 5985 + Negotiate auth (跟 v0.9.21 估 default 一致)
    let config = WinRmMcpConfig::default();
    assert_eq!(config.default_port_http, 5985);
    assert_eq!(config.default_port_https, 5986);
    assert_eq!(config.default_auth_kind, WinRmAuthMethodKind::Negotiate);

    let server = WinRmMcpServer::new(config)?;
    server.start().await?;

    // 1) winrm_connect: 建 WinRM endpoint (skeleton, 不真连)
    let endpoint = WinRmEndpoint {
        host: "192.168.1.10".to_string(),
        port: 5985,
        transport: WinRmTransport::Http,
        auth: WinRmAuthMethod::Negotiate {
            username: "Administrator".to_string(),
            password: SecretString::new("REDACTED-IN-DEMO"),
        },
        skip_ca_check: false,
        skip_cn_check: false,
        skip_revocation_check: false,
        configuration_name: Some("Microsoft.PowerShell".to_string()),
        operation_timeout: Duration::from_secs(30),
    };

    let connection_id = server.connect(endpoint).await?;
    eprintln!("[1/5] winrm_connect → connection_id={}", connection_id);

    // 2) winrm_run_command: 跑 PowerShell, skeleton 返回 placeholder command_id
    let powershell = "Get-Process | Select-Object -First 3 | Format-Table";
    let command_id = server
        .run_command(&connection_id, powershell, Some(Duration::from_secs(10)))
        .await?;
    eprintln!("[2/5] winrm_run_command → command_id={}", command_id);

    // 3) winrm_get_command_output: skeleton 返回 placeholder 输出
    let output = server
        .get_command_output(&connection_id, &command_id)
        .await?;
    eprintln!(
        "[3/5] winrm_get_command_output → stdout={}B stderr={}B exit={}",
        output.stdout.len(),
        output.stderr.len(),
        output.exit_code
    );

    // 4) winrm_cleanup_command: skeleton 返回 Done
    let status = server
        .cleanup_command(&connection_id, &command_id)
        .await?;
    assert_eq!(status, WinRmCommandStatus::Done);
    eprintln!("[4/5] winrm_cleanup_command → status={:?}", status);

    // 5) winrm_disconnect: skeleton 清理连接池
    server.disconnect(&connection_id).await?;
    eprintln!("[5/5] winrm_disconnect → connection_id={}", connection_id);

    server.stop().await?;
    eprintln!("apeireth-mcp-winrm demo done (5 tools invoked, all skeleton)");

    Ok(())
}
