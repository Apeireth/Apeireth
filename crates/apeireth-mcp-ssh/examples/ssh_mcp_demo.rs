//! SSH MCP Server demo (1:1 翻译 v0.9.21 SSHMcpServer.js demo 流程).
//!
//! 流程: new → start → connect → exec → disconnect → stop
//! 用 Agent 认证 (无需密码/key, ssh-agent 走标准接口).
//!
//! R20 阶段 1 P0 验证入口; 真实 ssh2 crate 调用放 R20 阶段 1 实施期.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-mcp-ssh --example ssh_mcp_demo
//! ```
//!
//! ## 期望输出 (skeleton 阶段)
//!
//! ```text
//! stdout:
//! exit_code: -1
//! duration: 0ns
//! [ssh_mcp_demo] completed (skeleton — R20 阶段 1 真实 ssh2 实现待补)
//! ```
//!
//! ## 6 哲学 anchor 验证 (per 主人 19:37 "全用 rust" 强调)
//!
//! - S-1 北极星导向: 1:1 翻译 v0.9.21 商业版 SSH MCP Server
//! - S-2 实事求是: 用 SSHMcpServer.js 实查 token 表 (auth/Handshak/exec/sftp/keepalive)
//!   9 方法 + 4 auth 变体实证, 不假装 1:1
//! - O-2 走在前人肩上: 基于 ssh2 crate 标准 API 翻译, 不重设计
//! - O-5 不假装: JumpHost 0 命中 / Agent 命中是 Node.js Symbol 都标登记
//! - O-3 干到底: skeleton 落地, R20 阶段 1 真实 ssh2 实现跟进
//! - O-4 任何人都能接手: 6 § 结构 + 13 错误 + 9 方法 + 7 test 跟主草稿 1:1

use apeireth_mcp_ssh::{SshAuthMethod, SshMcpConfig, SshMcpServer, SshMcpServerTrait};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) 创建 + 启动
    let server = SshMcpServer::new(SshMcpConfig::default())?;
    server.start().await?;

    // 2) 连接 (Agent 认证, ssh-agent 走标准接口)
    let auth = SshAuthMethod::Agent {
        username: "demo".to_string(),
        socket_path: None,
    };
    server.connect("demo-1", "127.0.0.1", 22, auth).await?;

    // 3) 执行命令
    let out = server.exec("demo-1", "echo hello ssh-mcp").await?;
    println!("stdout: {}", out.stdout);
    println!("exit_code: {}", out.exit_code);
    println!("duration: {:?}", out.duration);

    // 4) 列出活跃会话 (skeleton 阶段返回空)
    let sessions = server.list_sessions().await?;
    println!("active sessions: {}", sessions.len());

    // 5) 断开 + 停止
    server.disconnect("demo-1").await?;
    server.stop().await?;

    println!("[ssh_mcp_demo] completed (skeleton — R20 阶段 1 真实 ssh2 实现待补)");
    Ok(())
}
