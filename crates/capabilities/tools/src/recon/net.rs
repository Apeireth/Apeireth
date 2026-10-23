//! `net.recon`：TCP connect 扫描 + 被动服务指纹。
//!
//! 边界：只做 connect（不碰 raw socket / 不发畸形包）；目标地址必须落在
//! `Scope` 授权网段内；端口数受预算上限约束；每个连接有超时。
//! 只读取对端主动发送的 banner，不构造协议请求——未覆盖的探测写进 limitations。

use std::io::Read;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::scope::{Scope, ScopeError};

/// 单端口观察结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortFinding {
    /// 端口。
    pub port: u16,
    /// 状态：`open` / `closed` / `filtered`。
    pub state: String,
    /// 被动读到的 banner（若有）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    /// 连接耗时（毫秒）。
    pub elapsed_ms: u64,
}

/// 一次扫描结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// 目标地址。
    pub target: String,
    /// 逐端口结果（按输入端口顺序）。
    pub ports: Vec<PortFinding>,
    /// 开放端口数。
    pub open: usize,
    /// 显式局限。
    pub limitations: Vec<String>,
}

/// 扫描参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanOptions {
    /// 单次 connect 超时。
    pub timeout: Duration,
    /// 端口数上限（预算）。
    pub max_ports: usize,
    /// 是否被动读取 banner。
    pub banner_probe: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(300),
            max_ports: 1024,
            banner_probe: true,
        }
    }
}

/// 扫描错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NetError {
    /// 目标越界 / 未授权。
    #[error(transparent)]
    Scope(#[from] ScopeError),
    /// 端口数超过预算。
    #[error("requested {requested} ports, budget is {max}")]
    BudgetExceeded {
        /// 请求的端口数。
        requested: usize,
        /// 预算上限。
        max: usize,
    },
    /// 地址不可解析。
    #[error("address {0:?} is not a concrete IP")]
    BadTarget(String),
}

/// 扫描 `target` 的 `ports`。
pub fn scan(
    target: IpAddr,
    ports: &[u16],
    scope: &Scope,
    options: &ScanOptions,
) -> Result<ScanReport, NetError> {
    scope.check_address(target)?;
    if ports.len() > options.max_ports {
        return Err(NetError::BudgetExceeded {
            requested: ports.len(),
            max: options.max_ports,
        });
    }

    let mut findings = Vec::with_capacity(ports.len());
    let mut open = 0usize;
    for &port in ports {
        let address = SocketAddr::new(target, port);
        let started = Instant::now();
        match TcpStream::connect_timeout(&address, options.timeout) {
            Ok(mut stream) => {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                let banner = if options.banner_probe {
                    read_passive_banner(&mut stream)
                } else {
                    None
                };
                open += 1;
                findings.push(PortFinding {
                    port,
                    state: "open".to_string(),
                    banner,
                    elapsed_ms,
                });
            }
            Err(error) => {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                let state = classify(&error);
                findings.push(PortFinding {
                    port,
                    state,
                    banner: None,
                    elapsed_ms,
                });
            }
        }
    }

    Ok(ScanReport {
        target: target.to_string(),
        ports: findings,
        open,
        limitations: vec![
            "connect scan only (no SYN/raw sockets)".to_string(),
            "passive banner only; no protocol-specific probes".to_string(),
        ],
    })
}

fn classify(error: &std::io::Error) -> String {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::ConnectionRefused => "closed",
        ErrorKind::TimedOut | ErrorKind::WouldBlock => "filtered",
        _ => "filtered",
    }
    .to_string()
}

/// 只读取对端主动发送的有限字节，不发送任何请求。
fn read_passive_banner(stream: &mut TcpStream) -> Option<String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(150)))
        .ok()?;
    let mut buffer = [0u8; 256];
    let read = stream.read(&mut buffer).ok()?;
    if read == 0 {
        return None;
    }
    let text: String = String::from_utf8_lossy(&buffer[..read])
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    fn scoped_to(address: &str) -> Scope {
        Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec![format!("{address}/32")],
        }
    }

    #[test]
    fn finds_open_port_and_reads_passive_banner() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let banner = "SSH-2.0-fixture\r\n";
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(banner.as_bytes());
            }
        });

        let scope = scoped_to("127.0.0.1");
        let options = ScanOptions::default();
        let report = scan(IpAddr::from([127, 0, 0, 1]), &[port], &scope, &options).expect("scan");

        assert_eq!(report.open, 1);
        assert_eq!(report.ports[0].state, "open");
        assert!(report.ports[0]
            .banner
            .as_deref()
            .unwrap_or_default()
            .contains("SSH-2.0-fixture"));
        handle.join().ok();
    }

    #[test]
    fn scope_and_budget_are_enforced() {
        let scope = scoped_to("10.0.0.1");
        let options = ScanOptions::default();
        let denied = scan(IpAddr::from([192, 0, 2, 1]), &[80], &scope, &options);
        assert!(matches!(denied, Err(NetError::Scope(_))));

        let scope = scoped_to("127.0.0.1");
        let too_many: Vec<u16> = (1..=8).collect();
        let options = ScanOptions {
            max_ports: 4,
            ..ScanOptions::default()
        };
        let budget = scan(IpAddr::from([127, 0, 0, 1]), &too_many, &scope, &options);
        assert!(matches!(
            budget,
            Err(NetError::BudgetExceeded {
                requested: 8,
                max: 4
            })
        ));
    }

    #[test]
    fn closed_port_is_classified_not_open() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);

        let scope = scoped_to("127.0.0.1");
        let options = ScanOptions {
            timeout: Duration::from_millis(200),
            ..ScanOptions::default()
        };
        let report = scan(IpAddr::from([127, 0, 0, 1]), &[port], &scope, &options).expect("scan");
        assert_eq!(report.open, 0);
        assert_ne!(report.ports[0].state, "open");
    }
}
