//! 测试共享支持 (D26, 2026-10-21): 回环 bind 重试助手。
//!
//! 背景: Windows 上 `TcpListener::bind("127.0.0.1:0")` 偶发 `WSAEACCES`
//! (`os error 10013`) —— 系统挑中的临时端口恰好落在被 Hyper-V / WinNAT /
//! 其它组件保留的区间内 (失败帧见 `artifacts/wave-bio-w32d-workspace-test.log`:
//! `egress::tests::direct_ip_transport_works` 等)。这是**环境侧瞬时现象**,
//! 不是测试逻辑错误: 重试一次会换到另一个临时端口。
//!
//! 纪律 (D26): **只**对 Windows `WSAEACCES (10013)` 做有界重试 (最多 8 次,
//! 每次间隔 5ms); 其它错误原样返回; 非 Windows 平台无此现象, 直通首个结果。
//! 重试次数耗尽是环境侧故障, 由调用方 `.unwrap()` 如实失败, 不静默 pass。
//!
//! 用法 (单源 helper, 不复制实现):
//! - crate 内单测: `crate::test_support::bind_loopback_ephemeral().await?`
//! - 本 crate 集成测试: `tests/common/mod.rs` 用
//!   `#[path = "../../src/test_support.rs"] mod test_support;` 引入同一文件并
//!   `pub use test_support::bind_loopback_ephemeral;`, 测试文件写 `mod common;`
//!   后调 `common::bind_loopback_ephemeral().await?`。
//!
//! 其它 crate 若出现同形态 flake, 建议按同一模式在各自 crate 内放一份
//! (仓库不新增 test-support crate: 不改 `workspace.members` 口径, 不引外部依赖)。
#![allow(dead_code)]

use std::io;
use std::time::Duration;

/// Windows `WSAEACCES`: 临时端口撞上系统保留区间。
const WSAEACCES: i32 = 10013;

/// 绑定 `127.0.0.1:0`, 仅对 Windows `WSAEACCES (10013)` 做有界重试。
pub async fn bind_loopback_ephemeral() -> io::Result<tokio::net::TcpListener> {
    const MAX_ATTEMPTS: u32 = 8;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match tokio::net::TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => return Ok(listener),
            Err(error) if is_reserved_port_error(&error) && attempt < MAX_ATTEMPTS => {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

/// 仅 Windows `WSAEACCES` 算「临时端口撞保留区间」; 其它错误不重试。
fn is_reserved_port_error(error: &io::Error) -> bool {
    cfg!(windows) && error.raw_os_error() == Some(WSAEACCES)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 助手本身可用: 返回已绑定且端口非 0 的回环 listener。
    #[tokio::test]
    async fn bind_loopback_ephemeral_returns_live_loopback_listener() {
        let listener = bind_loopback_ephemeral()
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local_addr");
        assert!(addr.ip().is_loopback(), "expected loopback, got {addr}");
        assert_ne!(addr.port(), 0, "ephemeral bind must pick a real port");
    }

    /// 重试条件收窄: 仅 Windows WSAEACCES (10013); AddrInUse / 其它错误不重试。
    #[test]
    fn is_reserved_port_error_is_narrow_to_windows_wsaeeacces() {
        let wsaeeacces = io::Error::from_raw_os_error(10013);
        let addrinuse = io::Error::from_raw_os_error(10048);
        let other = io::Error::new(io::ErrorKind::ConnectionRefused, "synthetic");
        assert_eq!(is_reserved_port_error(&wsaeeacces), cfg!(windows));
        assert!(!is_reserved_port_error(&addrinuse));
        assert!(!is_reserved_port_error(&other));
    }
}
