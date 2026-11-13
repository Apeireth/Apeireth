//! `net.recon`：TCP connect 扫描 + 被动服务指纹。
//!
//! 边界：只做 connect（不碰 raw socket / 不发畸形包）；目标地址必须落在
//! `Scope` 授权网段内；端口数受预算上限约束；每个连接有超时。
//! 只读取对端主动发送的 banner，不构造协议请求——未覆盖的探测写进 limitations。
//!
//! # 三个入口的契约
//!
//! | 入口 | 需要 reactor | 调用级治理 | 定位 |
//! |---|---|---|---|
//! | [`scan`]（对外别名 `recon::scan_ports`） | 否 | 仅 Scope + 端口预算 | 历史同步入口，**行为不变** |
//! | [`scan_controlled_blocking`] | 否 | Scope + 预算 + 整体期限 + 取消 + 模块任务租约 | 同步调用方的受控入口 |
//! | [`scan_controlled`] | 是（Tokio） | 同上 | 生产异步入口 |
//!
//! 治理口径差异只有一处，且是**刻意**的：历史入口 [`scan`] 不承接调用级期限与
//! 取消（保持原行为），受控入口一律承接。两者对同一目标的**观测口径**一致。
//!
//! # 无 reactor 兼容性
//!
//! [`scan_controlled`] 依赖 Tokio 的 timer 与 IO 驱动。调用方**没有** Tokio
//! 运行时时（自写 `block_on`、同步脚本、fuzz harness），它不会 panic，而是自动
//! 回落到不依赖 reactor 的 [`scan_controlled_blocking`]；需要显式同步入口的调用方
//! 直接调用 [`scan_controlled_blocking`] 即可。两条受控路径的治理口径相同。
//!
//! # `elapsed_ms` 口径
//!
//! `elapsed_ms` = **发起 connect → connect 返回** 的耗时，**不含** banner 读取
//! 等待（banner 读取另有预算 [`BANNER_PROBE_BUDGET`]，不摊进端口耗时）。
//! 同步与异步两条路径口径一致，便于跨入口比较。

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
    /// 连接耗时（毫秒）：**connect 发起 → connect 返回**，不含 banner 读取等待。
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
    /// 整体调用被取消；部分结果不作为完整证据返回。
    #[error("scan cancelled; partial observations discarded")]
    Cancelled,
    /// 整体截止时间已到；不伪造完整端口清单。
    #[error("scan deadline exceeded; partial observations discarded")]
    DeadlineExceeded,
}

/// 单次扫描的整体期限上限（与调用级期限取短）。
pub const SCAN_DEADLINE: Duration = Duration::from_secs(30);
/// 被动 banner 读取的单端口预算（**不计入** `PortFinding::elapsed_ms`）。
pub const BANNER_PROBE_BUDGET: Duration = Duration::from_millis(150);
/// 取消轮询间隔。
const CANCELLATION_POLL: Duration = Duration::from_millis(10);

/// 生产异步入口：受控 TCP connect 扫描。
///
/// 治理口径：Scope 限域 + 端口预算 + 整体期限（调用级期限与 [`SCAN_DEADLINE`]
/// 取短）+ 取消 + 模块任务租约。取消或超期时**不**返回部分端口清单。
///
/// 需要 Tokio 运行时（timer + IO 驱动）。调用方**没有** Tokio 运行时时不会 panic：
/// 自动回落到不依赖 reactor 的 [`scan_controlled_blocking`]。
pub async fn scan_controlled(
    target: IpAddr,
    ports: &[u16],
    scope: &Scope,
    options: &ScanOptions,
) -> Result<ScanReport, NetError> {
    // 脱离 reactor 的同步调用方（自写 block_on / 脚本 / fuzz harness）：Tokio 的
    // timer 与 IO 驱动都不可用 ⇒ 走不依赖 reactor 的同步兼容入口（不 panic）。
    if tokio::runtime::Handle::try_current().is_err() {
        return scan_controlled_blocking(target, ports, scope, options);
    }
    validate_scan(target, ports, scope, options)?;
    let context = apeireth_plugin::tool_execution::native_execution_context();
    let cancellation = apeireth_plugin::ToolCancellation::joined(
        context
            .iter()
            .map(|context| context.cancellation.clone())
            .collect(),
    );
    let _cancel_on_drop = cancellation.cancel_on_drop();
    let _lease = context
        .as_ref()
        .map(|context| context.tasks.lease_with_cancellation(cancellation.clone()))
        .transpose()
        .map_err(|_| NetError::Cancelled)?;
    let default_deadline = Instant::now() + SCAN_DEADLINE;
    let deadline = context.as_ref().map_or(default_deadline, |context| {
        context.deadline.min(default_deadline)
    });
    let cancelled = async {
        while !cancellation.is_cancelled() {
            tokio::time::sleep(CANCELLATION_POLL).await;
        }
    };
    tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline.into()) => Err(NetError::DeadlineExceeded),
        _ = cancelled => Err(NetError::Cancelled),
        report = scan_async(target, ports, options) => {
            // 收尾复核：扫描刚跑完但期间已被取消 / 已过期限 ⇒ 与同步兼容入口同口径，
            // 不把部分结果当作完整证据返回。
            if cancellation.is_cancelled() {
                Err(NetError::Cancelled)
            } else if Instant::now() >= deadline {
                Err(NetError::DeadlineExceeded)
            } else {
                Ok(report)
            }
        }
    }
}

/// 不依赖 Tokio reactor 的同步受控入口（与 [`scan_controlled`] 同治理口径）。
///
/// 承接：Scope 限域 + 端口预算 + 整体期限（调用级期限与 [`SCAN_DEADLINE`] 取短）
/// + 取消 + 模块任务租约。取消或超期时**不**返回部分端口清单。
pub fn scan_controlled_blocking(
    target: IpAddr,
    ports: &[u16],
    scope: &Scope,
    options: &ScanOptions,
) -> Result<ScanReport, NetError> {
    validate_scan(target, ports, scope, options)?;
    let context = apeireth_plugin::tool_execution::native_execution_context();
    let cancellation = apeireth_plugin::ToolCancellation::joined(
        context
            .iter()
            .map(|context| context.cancellation.clone())
            .collect(),
    );
    let _cancel_on_drop = cancellation.cancel_on_drop();
    let _lease = context
        .as_ref()
        .map(|context| context.tasks.lease_with_cancellation(cancellation.clone()))
        .transpose()
        .map_err(|_| NetError::Cancelled)?;
    let default_deadline = Instant::now() + SCAN_DEADLINE;
    let deadline = context.as_ref().map_or(default_deadline, |context| {
        context.deadline.min(default_deadline)
    });

    let mut findings = Vec::with_capacity(ports.len());
    let mut open = 0usize;
    for &port in ports {
        if cancellation.is_cancelled() {
            return Err(NetError::Cancelled);
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(NetError::DeadlineExceeded);
        }
        let connect_timeout = options.timeout.min(deadline - now);
        let (state, banner, elapsed_ms) =
            probe_port_blocking(target, port, options, connect_timeout);
        if state == "open" {
            open += 1;
        }
        findings.push(PortFinding {
            port,
            state,
            banner,
            elapsed_ms,
        });
    }
    if cancellation.is_cancelled() {
        return Err(NetError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(NetError::DeadlineExceeded);
    }
    Ok(ScanReport {
        target: target.to_string(),
        ports: findings,
        open,
        limitations: limitations(),
    })
}

/// 阻塞单端口观察：connect 计时（口径见 [`PortFinding::elapsed_ms`]）+ 可选被动 banner。
fn probe_port_blocking(
    target: IpAddr,
    port: u16,
    options: &ScanOptions,
    connect_timeout: Duration,
) -> (String, Option<String>, u64) {
    let address = SocketAddr::new(target, port);
    let started = Instant::now();
    match TcpStream::connect_timeout(&address, connect_timeout) {
        Ok(mut stream) => {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let banner = if options.banner_probe {
                read_passive_banner(&mut stream)
            } else {
                None
            };
            ("open".to_string(), banner, elapsed_ms)
        }
        Err(error) => {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            (classify(&error), None, elapsed_ms)
        }
    }
}

/// 扫描显式局限（三个入口共用同一口径）。
fn limitations() -> Vec<String> {
    vec![
        "connect scan only (no SYN/raw sockets)".to_string(),
        "passive banner only; no protocol-specific probes".to_string(),
    ]
}

async fn scan_async(target: IpAddr, ports: &[u16], options: &ScanOptions) -> ScanReport {
    use tokio::io::AsyncReadExt;
    let mut findings = Vec::with_capacity(ports.len());
    let mut open = 0;
    for &port in ports {
        let started = Instant::now();
        let connection = tokio::time::timeout(
            options.timeout,
            tokio::net::TcpStream::connect(SocketAddr::new(target, port)),
        )
        .await;
        // 端口耗时口径：connect 发起 → connect 返回（**不含** banner 读取等待）。
        // 必须在 banner 读取之前取值, 否则等待时间会被摊进端口耗时。
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (state, banner) = match connection {
            Ok(Ok(mut stream)) => {
                open += 1;
                let mut buffer = [0; 256];
                let banner = if options.banner_probe {
                    match tokio::time::timeout(BANNER_PROBE_BUDGET, stream.read(&mut buffer)).await
                    {
                        Ok(Ok(read)) => decode_banner(&buffer[..read]),
                        _ => None,
                    }
                } else {
                    None
                };
                ("open".to_string(), banner)
            }
            Ok(Err(error)) => (classify(&error), None),
            Err(_) => ("filtered".to_string(), None),
        };
        findings.push(PortFinding {
            port,
            state,
            banner,
            elapsed_ms,
        });
    }
    ScanReport {
        target: target.to_string(),
        ports: findings,
        open,
        limitations: limitations(),
    }
}

fn validate_scan(
    target: IpAddr,
    ports: &[u16],
    scope: &Scope,
    options: &ScanOptions,
) -> Result<(), NetError> {
    scope.check_address(target)?;
    if ports.len() > options.max_ports {
        return Err(NetError::BudgetExceeded {
            requested: ports.len(),
            max: options.max_ports,
        });
    }
    Ok(())
}

/// 扫描 `target` 的 `ports`（历史同步入口，**行为不变**）。
///
/// 与受控入口 [`scan_controlled`] / [`scan_controlled_blocking`] 的差别只有一处：
/// 本入口**不**承接调用级期限与取消（保持原行为），受控入口一律承接。
/// 观测口径（`elapsed_ms` 只量 connect、被动 banner 读取）两条路径一致。
pub fn scan(
    target: IpAddr,
    ports: &[u16],
    scope: &Scope,
    options: &ScanOptions,
) -> Result<ScanReport, NetError> {
    validate_scan(target, ports, scope, options)?;

    let mut findings = Vec::with_capacity(ports.len());
    let mut open = 0usize;
    for &port in ports {
        let (state, banner, elapsed_ms) =
            probe_port_blocking(target, port, options, options.timeout);
        if state == "open" {
            open += 1;
        }
        findings.push(PortFinding {
            port,
            state,
            banner,
            elapsed_ms,
        });
    }

    Ok(ScanReport {
        target: target.to_string(),
        ports: findings,
        open,
        limitations: limitations(),
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
    stream.set_read_timeout(Some(BANNER_PROBE_BUDGET)).ok()?;
    let mut buffer = [0u8; 256];
    let read = stream.read(&mut buffer).ok()?;
    decode_banner(&buffer[..read])
}

fn decode_banner(buffer: &[u8]) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }
    let text: String = String::from_utf8_lossy(buffer)
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
