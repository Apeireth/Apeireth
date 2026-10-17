//! 本地数据面自查 · 项三: 开放端口面 (本机监听枚举, 受控采集)。
//!
//! 防御语自查: 本机**监听**面是自己暴露了什么的第一张网络清单。采集口径 =
//! 受控系统表快照 (`netstat` 固定参数, 无用户输入拼接); 解析器只认监听态
//! (TCP `LISTENING`/`LISTEN`) 与已绑定 UDP, 连接态行不算监听面 (防误报)。
//! 诚实口径: 只列本机监听枚举; 入站可达性、外网暴露面**未测**。

use crate::exposure_audit::report::{ExposureItem, Finding, RiskLevel};

/// 单个体检项的发现条数上限 (超出部分聚合)。
const MAX_FINDINGS: usize = 50;

/// 监听地址的作用域 (暴露面宽窄的分级依据)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerScope {
    /// 回环监听: 仅本机可触达 (暴露面最窄)。
    Loopback,
    /// 全接口监听 (`0.0.0.0` / `::` / `*`): 入站面最宽。
    Wildcard,
    /// 指定接口监听: 介于两者之间。
    Interface,
}

/// 一个本机监听面 (来自受控系统表快照)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listener {
    /// 协议词面 (`tcp` / `tcp6` / `udp` / `udp6`)。
    pub proto: String,
    /// 本地监听地址 (不含端口)。
    pub local_addr: String,
    /// 本地监听端口。
    pub port: u16,
    /// 作用域分级。
    pub scope: ListenerScope,
    /// 持有进程 id (快照里取不到 = `None`, 不编数)。
    pub pid: Option<u32>,
}

/// 一次受控快照的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerSnapshot {
    /// 全部监听面 (按端口、地址排序, 输出确定; 同一 (proto, 地址, 端口) 只留一行)。
    pub listeners: Vec<Listener>,
    /// 形状可识别但地址/端口解析失败的行数 (如实计数, 不静默丢)。
    pub unparsed_rows: usize,
    /// 去重合并的重复行数 (同一监听面在系统表里的多行, 如实计数)。
    pub duplicate_rows: usize,
}

/// 地址:端口 → (地址, 端口); IPv6 花括号与裸冒号两形都认。
fn split_addr_port(raw: &str) -> Option<(String, u16)> {
    if let Some(rest) = raw.strip_prefix('[') {
        let close = rest.find(']')?;
        let addr = &rest[..close];
        let port = rest[close + 1..].strip_prefix(':')?.parse().ok()?;
        return Some((addr.to_string(), port));
    }
    let (addr, port) = raw.rsplit_once(':')?;
    Some((addr.to_string(), port.parse().ok()?))
}

/// 监听作用域分级 (回环 / 全接口 / 指定接口)。
fn classify_scope(addr: &str) -> ListenerScope {
    match addr {
        "0.0.0.0" | "::" | "*" | "" => ListenerScope::Wildcard,
        "::1" | "localhost" => ListenerScope::Loopback,
        other if other.starts_with("127.") => ListenerScope::Loopback,
        _ => ListenerScope::Interface,
    }
}

/// 受控快照文本 → 监听面清单 (只认监听态与已绑定 UDP; 连接态行不算)。
pub fn parse_netstat_snapshot(text: &str) -> ListenerSnapshot {
    let mut listeners = Vec::new();
    let mut unparsed_rows = 0usize;
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(proto) = tokens.first() else {
            continue;
        };
        let proto = proto.to_ascii_lowercase();
        if !matches!(proto.as_str(), "tcp" | "tcp6" | "udp" | "udp6") {
            continue;
        }
        // 本地地址: Windows 形是第 2 列, net-tools 形前面多 recv-q/send-q 两列。
        let local_raw = if tokens.len() >= 2 && tokens[1].contains(':') {
            Some(tokens[1])
        } else {
            tokens
                .iter()
                .skip(2)
                .find(|token| token.contains(':'))
                .copied()
        };
        let Some(local_raw) = local_raw else {
            unparsed_rows += 1;
            continue;
        };
        let is_tcp = proto.starts_with("tcp");
        let listening = tokens.iter().any(|token| {
            token.eq_ignore_ascii_case("listening") || token.eq_ignore_ascii_case("listen")
        });
        if is_tcp && !listening {
            // 连接态行 (ESTABLISHED 等) 不是监听面 —— 防误报的分界线。
            continue;
        }
        let Some((local_addr, port)) = split_addr_port(local_raw) else {
            unparsed_rows += 1;
            continue;
        };
        let pid = tokens
            .last()
            .and_then(|token| token.split('/').next())
            .and_then(|pid| pid.parse::<u32>().ok());
        listeners.push(Listener {
            proto,
            local_addr: local_addr.clone(),
            port,
            scope: classify_scope(&local_addr),
            pid,
        });
    }
    listeners.sort_by(|left, right| {
        (left.port, &left.local_addr, &left.proto).cmp(&(
            right.port,
            &right.local_addr,
            &right.proto,
        ))
    });
    // 去重: 同一 (proto, 地址, 端口) 是同一张暴露面 (系统表里常因多 socket 重复
    // 列行), 逐行报会虚增发现数 —— 合并后行数如实记 `duplicate_rows`。
    let mut unique: Vec<Listener> = Vec::new();
    let mut duplicate_rows = 0usize;
    for listener in listeners {
        let duplicate = unique.iter().any(|kept| {
            kept.proto == listener.proto
                && kept.local_addr == listener.local_addr
                && kept.port == listener.port
        });
        if duplicate {
            duplicate_rows += 1;
        } else {
            unique.push(listener);
        }
    }
    ListenerSnapshot {
        listeners: unique,
        unparsed_rows,
        duplicate_rows,
    }
}

/// 受控采集: 固定参数系统表快照 (无用户输入拼接; 失败即帧, 不猜)。
pub fn probe_netstat() -> Result<String, String> {
    let mut command = std::process::Command::new("netstat");
    #[cfg(windows)]
    command.args(["-ano"]);
    #[cfg(not(windows))]
    command.args(["-an"]);
    let output = command
        .output()
        .map_err(|error| format!("受控采集启动失败: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "受控采集退出码非零: {}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 项三: 开放端口面体检项 (从快照结果渲染; 采集失败 = 显式未测)。
pub fn listener_item(snapshot: Result<ListenerSnapshot, String>) -> ExposureItem {
    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(reason) => {
            return ExposureItem::not_measured(
                "listening_ports",
                "开放端口面 (本机监听)",
                format!("本机监听枚举不可用 (受控采集失败): {reason}"),
                "修复受控采集通道后复检; 先按最小权限核对本机服务清单",
            )
        }
    };
    let mut findings = Vec::new();
    let mut hidden = 0usize;
    for listener in &snapshot.listeners {
        if findings.len() >= MAX_FINDINGS {
            hidden += 1;
            continue;
        }
        let (risk, phrase) = match listener.scope {
            ListenerScope::Wildcard => (RiskLevel::Medium, "全接口监听, 入站面最宽"),
            ListenerScope::Interface => (RiskLevel::Low, "指定接口监听"),
            ListenerScope::Loopback => (RiskLevel::None, "回环监听, 仅本机可触达"),
        };
        let pid = listener
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "未知".to_string());
        findings.push(Finding::new(
            format!(
                "{} {}:{} (PID {})",
                listener.proto, listener.local_addr, listener.port, pid
            ),
            risk,
            format!("{phrase}; 外部可达性未测"),
        ));
    }
    if hidden > 0 {
        findings.push(Finding::new(
            "(聚合)",
            RiskLevel::Medium,
            format!("另有 {hidden} 个监听面超出条目上限未逐条列出"),
        ));
    }
    if snapshot.unparsed_rows > 0 {
        findings.push(Finding::new(
            "(解析失败)",
            RiskLevel::Low,
            format!(
                "{} 行系统表输出形状可识别但解析失败, 未计入清单 (如实计数)",
                snapshot.unparsed_rows
            ),
        ));
    }
    ExposureItem::measured(
        "listening_ports",
        "开放端口面 (本机监听)",
        findings,
        "开放端口面收口: 非必要监听改回环绑定, 全接口监听逐项确认必要性并配防火墙收窄入站",
    )
    .with_note(format!(
        "受控系统表快照 (只读; 重复行合并 {} 行, 如实计数); 只列本机监听枚举, 入站可达性与外网暴露面未测",
        snapshot.duplicate_rows
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addr_port_shapes_parse() {
        assert_eq!(
            split_addr_port("0.0.0.0:135"),
            Some(("0.0.0.0".to_string(), 135))
        );
        assert_eq!(split_addr_port("[::]:8080"), Some(("::".to_string(), 8080)));
        assert_eq!(split_addr_port("no-port"), None);
    }

    #[test]
    fn scope_classification_separates_loopback_wildcard_and_interface() {
        assert_eq!(classify_scope("0.0.0.0"), ListenerScope::Wildcard);
        assert_eq!(classify_scope("::"), ListenerScope::Wildcard);
        assert_eq!(classify_scope("127.0.0.1"), ListenerScope::Loopback);
        assert_eq!(classify_scope("::1"), ListenerScope::Loopback);
        assert_eq!(classify_scope("192.168.1.5"), ListenerScope::Interface);
    }

    #[test]
    fn connection_state_rows_are_not_listening_surface() {
        let snapshot = parse_netstat_snapshot(
            "  Proto  Local Address          Foreign Address        State\n\
             TCP    127.0.0.1:3080        127.0.0.1:50000       ESTABLISHED     42\n",
        );
        assert!(snapshot.listeners.is_empty(), "连接态不算监听面");
        assert_eq!(snapshot.unparsed_rows, 0);
    }
}
