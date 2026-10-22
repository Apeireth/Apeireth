//! `net.proto`：pcap 抓包文件的协议还原（经典 pcap，零新依赖）。
//!
//! 支持：Ethernet / 原始 IP、IPv4 / IPv6、TCP / UDP 五元组、DNS 查询名。
//! 不重组 TCP 流、不解密 TLS；未覆盖的部分写进 limitations。

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 一条网络流（四元组 + 协议）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Flow {
    /// 协议（`tcp` / `udp` / `ipv4` / `ipv6`）。
    pub protocol: String,
    /// 源地址:端口（无端口时只有地址）。
    pub source: String,
    /// 目的地址:端口。
    pub destination: String,
}

/// 一次 pcap 解析结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PcapReport {
    /// 容器格式（当前固定 `pcap`）。
    pub format: String,
    /// 链路类型（pcap `network` 字段）。
    pub link_type: u32,
    /// 包数量。
    pub packet_count: usize,
    /// 去重后的流清单（有序）。
    pub flows: Vec<Flow>,
    /// DNS 查询名。
    pub dns_queries: Vec<String>,
    /// 显式局限。
    pub limitations: Vec<String>,
}

/// 解析错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PcapError {
    /// 文件太短。
    #[error("pcap file is too short")]
    TooShort,
    /// 魔数不支持。
    #[error("unsupported pcap magic: {0}")]
    BadMagic(String),
    /// 记录越界。
    #[error("truncated pcap record at packet {0}")]
    Truncated(usize),
    /// 容器不支持（pcapng）。
    #[error("unsupported container: {0}")]
    Unsupported(String),
}

/// 上限：避免大文件把内存撑爆。
const MAX_PACKETS: usize = 100_000;
const MAX_FLOWS: usize = 2_000;
const MAX_DNS: usize = 500;

/// 解析抓包文件：自动识别经典 pcap 与 pcapng。
pub fn parse_capture(bytes: &[u8]) -> Result<PcapReport, PcapError> {
    if bytes.len() >= 4 && bytes.starts_with(&[0x0a, 0x0d, 0x0d, 0x0a]) {
        parse_pcapng(bytes)
    } else {
        parse_pcap(bytes)
    }
}

/// 解析经典 pcap。
pub fn parse_pcap(bytes: &[u8]) -> Result<PcapReport, PcapError> {
    if bytes.len() < 24 {
        return Err(PcapError::TooShort);
    }
    if bytes.starts_with(&[0x0a, 0x0d, 0x0d, 0x0a]) {
        return Err(PcapError::Unsupported("pcapng".into()));
    }
    let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
    let (little, link_type) = match magic {
        [0xd4, 0xc3, 0xb2, 0xa1] | [0x4d, 0x3c, 0xb2, 0xa1] => (
            true,
            u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        ),
        [0xa1, 0xb2, 0xc3, 0xd4] | [0xa1, 0xb2, 0x3c, 0x4d] => (
            false,
            u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        ),
        other => return Err(PcapError::BadMagic(hex(&other))),
    };

    let mut offset = 24usize;
    let mut packet_count = 0usize;
    let mut flows = std::collections::BTreeSet::new();
    let mut dns_queries = Vec::new();

    while offset + 16 <= bytes.len() && packet_count < MAX_PACKETS {
        let header = &bytes[offset..offset + 16];
        let incl_len = if little {
            u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize
        } else {
            u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize
        };
        offset += 16;
        if offset + incl_len > bytes.len() {
            return Err(PcapError::Truncated(packet_count));
        }
        let packet = &bytes[offset..offset + incl_len];
        offset += incl_len;
        packet_count += 1;
        parse_packet(packet, link_type, &mut flows, &mut dns_queries);
    }

    let mut flows: Vec<Flow> = flows.into_iter().take(MAX_FLOWS).collect();
    flows.sort();
    dns_queries.sort();
    dns_queries.dedup();
    dns_queries.truncate(MAX_DNS);

    Ok(PcapReport {
        format: "pcap".into(),
        link_type,
        packet_count,
        flows,
        dns_queries,
        limitations: vec![
            "no TCP stream reassembly".into(),
            "no TLS decryption".into(),
            "only classic pcap (pcapng unsupported)".into(),
        ],
    })
}

/// 解析 pcapng（SHB / IDB / EPB / SPB）。
fn parse_pcapng(bytes: &[u8]) -> Result<PcapReport, PcapError> {
    let mut offset = 0usize;
    let mut little: Option<bool> = None;
    let mut interfaces: Vec<u32> = Vec::new();
    let mut packet_count = 0usize;
    let mut flows = std::collections::BTreeSet::new();
    let mut dns_queries = Vec::new();

    while offset + 12 <= bytes.len() {
        let header_type = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        if header_type == 0x0a0d_0d0a {
            // Section Header Block：由 byte-order magic 决定字节序。
            let bom = bytes
                .get(offset + 8..offset + 12)
                .ok_or(PcapError::Truncated(0))?;
            little = Some(match bom {
                [0x4d, 0x3c, 0x2b, 0x1a] => true,
                [0x1a, 0x2b, 0x3c, 0x4d] => false,
                _ => return Err(PcapError::BadMagic(hex(bom))),
            });
        }
        let little = little.ok_or_else(|| PcapError::BadMagic("pcapng without SHB".into()))?;
        let block_type = read_u32(bytes, offset, little)?;
        let total_len = read_u32(bytes, offset + 4, little)? as usize;
        if total_len < 12 || total_len % 4 != 0 || offset + total_len > bytes.len() {
            return Err(PcapError::Truncated(packet_count));
        }
        let body = &bytes[offset + 8..offset + total_len - 4];
        match block_type {
            0x0000_0001 => {
                // Interface Description Block：linktype + snaplen。
                let link = u32::from(read_u16(body, 0, little)?);
                interfaces.push(link);
            }
            0x0000_0006 => {
                // Enhanced Packet Block。
                let interface_id = read_u32(body, 0, little)? as usize;
                let captured_len = read_u32(body, 12, little)? as usize;
                let data = body
                    .get(20..20 + captured_len)
                    .ok_or(PcapError::Truncated(packet_count))?;
                if let Some(link) = interfaces.get(interface_id) {
                    parse_packet(data, *link, &mut flows, &mut dns_queries);
                    packet_count += 1;
                }
            }
            0x0000_0003 => {
                // Simple Packet Block：只有长度 + 数据，接口固定为 0。
                let data = body.get(4..).ok_or(PcapError::Truncated(packet_count))?;
                if let Some(link) = interfaces.first() {
                    parse_packet(data, *link, &mut flows, &mut dns_queries);
                    packet_count += 1;
                }
            }
            _ => {}
        }
        offset += total_len;
    }

    let mut flows: Vec<Flow> = flows.into_iter().take(MAX_FLOWS).collect();
    flows.sort();
    dns_queries.sort();
    dns_queries.dedup();
    dns_queries.truncate(MAX_DNS);

    Ok(PcapReport {
        format: "pcapng".into(),
        link_type: interfaces.first().copied().unwrap_or(0),
        packet_count,
        flows,
        dns_queries,
        limitations: vec![
            "no TCP stream reassembly".into(),
            "no TLS decryption".into(),
            "only SHB/IDB/EPB/SPB blocks are interpreted".into(),
        ],
    })
}

fn read_u16(bytes: &[u8], offset: usize, little: bool) -> Result<u16, PcapError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(PcapError::Truncated(0))?;
    Ok(if little {
        u16::from_le_bytes([raw[0], raw[1]])
    } else {
        u16::from_be_bytes([raw[0], raw[1]])
    })
}

fn read_u32(bytes: &[u8], offset: usize, little: bool) -> Result<u32, PcapError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(PcapError::Truncated(0))?;
    let arr = [raw[0], raw[1], raw[2], raw[3]];
    Ok(if little {
        u32::from_le_bytes(arr)
    } else {
        u32::from_be_bytes(arr)
    })
}

fn parse_packet(
    packet: &[u8],
    link_type: u32,
    flows: &mut std::collections::BTreeSet<Flow>,
    dns: &mut Vec<String>,
) {
    let payload = match link_type {
        1 => {
            // Ethernet：14 字节头 + EtherType。
            if packet.len() < 14 {
                return;
            }
            let ether_type = u16::from_be_bytes([packet[12], packet[13]]);
            match ether_type {
                0x0800 | 0x86dd => &packet[14..],
                _ => return,
            }
        }
        101 => packet, // RAW IP
        _ => return,
    };
    parse_ip(payload, flows, dns);
}

fn parse_ip(payload: &[u8], flows: &mut std::collections::BTreeSet<Flow>, dns: &mut Vec<String>) {
    let Some(&version) = payload.first() else {
        return;
    };
    match version >> 4 {
        4 => parse_ipv4(payload, flows, dns),
        6 => parse_ipv6(payload, flows, dns),
        _ => {}
    }
}

fn parse_ipv4(payload: &[u8], flows: &mut std::collections::BTreeSet<Flow>, dns: &mut Vec<String>) {
    if payload.len() < 20 {
        return;
    }
    let ihl = ((payload[0] & 0x0f) as usize) * 4;
    if ihl < 20 || payload.len() < ihl {
        return;
    }
    let protocol = payload[9];
    let source = IpAddr::from([payload[12], payload[13], payload[14], payload[15]]);
    let destination = IpAddr::from([payload[16], payload[17], payload[18], payload[19]]);
    let transport = &payload[ihl..];
    parse_transport(protocol, source, destination, transport, flows, dns);
}

fn parse_ipv6(payload: &[u8], flows: &mut std::collections::BTreeSet<Flow>, dns: &mut Vec<String>) {
    if payload.len() < 40 {
        return;
    }
    let next_header = payload[6];
    let mut source = [0u8; 16];
    let mut destination = [0u8; 16];
    source.copy_from_slice(&payload[8..24]);
    destination.copy_from_slice(&payload[24..40]);
    let source = IpAddr::from(source);
    let destination = IpAddr::from(destination);
    parse_transport(next_header, source, destination, &payload[40..], flows, dns);
}

fn parse_transport(
    protocol: u8,
    source: IpAddr,
    destination: IpAddr,
    transport: &[u8],
    flows: &mut std::collections::BTreeSet<Flow>,
    dns: &mut Vec<String>,
) {
    match protocol {
        6 => {
            if transport.len() < 20 {
                return;
            }
            let source_port = u16::from_be_bytes([transport[0], transport[1]]);
            let destination_port = u16::from_be_bytes([transport[2], transport[3]]);
            flows.insert(Flow {
                protocol: "tcp".into(),
                source: format!("{source}:{source_port}"),
                destination: format!("{destination}:{destination_port}"),
            });
        }
        17 => {
            if transport.len() < 8 {
                return;
            }
            let source_port = u16::from_be_bytes([transport[0], transport[1]]);
            let destination_port = u16::from_be_bytes([transport[2], transport[3]]);
            flows.insert(Flow {
                protocol: "udp".into(),
                source: format!("{source}:{source_port}"),
                destination: format!("{destination}:{destination_port}"),
            });
            if source_port == 53 || destination_port == 53 {
                if let Some(name) = parse_dns_question(&transport[8..]) {
                    dns.push(name);
                }
            }
        }
        _ => {
            flows.insert(Flow {
                protocol: format!("ip-proto-{protocol}"),
                source: source.to_string(),
                destination: destination.to_string(),
            });
        }
    }
}

/// 解析 DNS 查询名（只处理单问题、无压缩指针的常见形态）。
fn parse_dns_question(transport: &[u8]) -> Option<String> {
    if transport.len() < 12 {
        return None;
    }
    let questions = u16::from_be_bytes([transport[4], transport[5]]);
    if questions == 0 {
        return None;
    }
    let mut offset = 12usize;
    let mut labels = Vec::new();
    loop {
        let length = *transport.get(offset)? as usize;
        offset += 1;
        if length == 0 {
            break;
        }
        if length & 0xc0 != 0 {
            return None; // 压缩指针不在这里处理
        }
        let label = transport.get(offset..offset + length)?;
        labels.push(String::from_utf8_lossy(label).into_owned());
        offset += length;
        if labels.len() > 16 {
            return None;
        }
    }
    (!labels.is_empty()).then(|| labels.join("."))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个经典 pcap：Ethernet + IPv4 + UDP + DNS 查询 example.com。
    fn dns_pcap() -> Vec<u8> {
        let mut dns = Vec::new();
        dns.extend_from_slice(&[0x12, 0x34]); // id
        dns.extend_from_slice(&[0x01, 0x00]); // flags: standard query
        dns.extend_from_slice(&[0x00, 0x01]); // qdcount
        dns.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // an/ns/ar
        for label in ["example", "com"] {
            dns.push(label.len() as u8);
            dns.extend_from_slice(label.as_bytes());
        }
        dns.push(0);
        dns.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]); // A / IN

        let udp_len = 8 + dns.len();
        let mut udp = Vec::new();
        udp.extend_from_slice(&40000u16.to_be_bytes());
        udp.extend_from_slice(&53u16.to_be_bytes());
        udp.extend_from_slice(&(udp_len as u16).to_be_bytes());
        udp.extend_from_slice(&0u16.to_be_bytes());
        udp.extend_from_slice(&dns);

        let total = 20 + udp.len();
        let mut ip = Vec::new();
        ip.push(0x45);
        ip.push(0);
        ip.extend_from_slice(&(total as u16).to_be_bytes());
        ip.extend_from_slice(&[0, 0, 0, 0]);
        ip.push(64);
        ip.push(17);
        ip.extend_from_slice(&[0, 0]);
        ip.extend_from_slice(&[10, 0, 0, 2]);
        ip.extend_from_slice(&[8, 8, 8, 8]);
        ip.extend_from_slice(&udp);

        let mut ethernet = vec![0u8; 12];
        ethernet.extend_from_slice(&[0x08, 0x00]);
        ethernet.extend_from_slice(&ip);

        let mut pcap = Vec::new();
        pcap.extend_from_slice(&[0xd4, 0xc3, 0xb2, 0xa1]); // LE magic
        pcap.extend_from_slice(&[0x02, 0x00, 0x04, 0x00]); // version 2.4
        pcap.extend_from_slice(&[0u8; 8]); // thiszone/sigfigs
        pcap.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
        pcap.extend_from_slice(&1u32.to_le_bytes()); // linktype Ethernet
        pcap.extend_from_slice(&[0u8; 8]); // ts
        pcap.extend_from_slice(&(ethernet.len() as u32).to_le_bytes());
        pcap.extend_from_slice(&(ethernet.len() as u32).to_le_bytes());
        pcap.extend_from_slice(&ethernet);
        pcap
    }

    #[test]
    fn parses_flows_and_dns_queries() {
        let report = parse_pcap(&dns_pcap()).expect("parse");
        assert_eq!(report.packet_count, 1);
        assert_eq!(report.link_type, 1);
        assert!(report
            .flows
            .iter()
            .any(|flow| flow.protocol == "udp" && flow.destination == "8.8.8.8:53"));
        assert_eq!(report.dns_queries, vec!["example.com".to_string()]);
    }

    #[test]
    fn rejects_bad_container() {
        assert!(matches!(parse_pcap(b"short"), Err(PcapError::TooShort)));
        let mut pcapng = vec![0x0a, 0x0d, 0x0d, 0x0a];
        pcapng.extend_from_slice(&[0u8; 32]);
        assert!(matches!(
            parse_pcap(&pcapng),
            Err(PcapError::Unsupported(_))
        ));

        let mut bad = vec![0u8; 24];
        bad[0] = 0xde;
        assert!(matches!(parse_pcap(&bad), Err(PcapError::BadMagic(_))));
    }

    #[test]
    fn truncated_record_is_reported() {
        let mut pcap = dns_pcap();
        pcap.truncate(pcap.len() - 8);
        assert!(matches!(parse_pcap(&pcap), Err(PcapError::Truncated(0))));
    }
}

#[cfg(test)]
mod pcapng_tests {
    use super::*;

    fn block(kind: u32, body: &[u8]) -> Vec<u8> {
        let mut padded = body.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        let total = 12 + padded.len();
        let mut out = Vec::new();
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&padded);
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out
    }

    /// 复用经典 pcap 测试里的 Ethernet + IPv4 + UDP + DNS(example.com) 帧。
    fn dns_frame() -> Vec<u8> {
        let mut dns = Vec::new();
        dns.extend_from_slice(&[0x12, 0x34, 0x01, 0x00, 0x00, 0x01]);
        dns.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        for label in ["example", "com"] {
            dns.push(label.len() as u8);
            dns.extend_from_slice(label.as_bytes());
        }
        dns.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);

        let mut udp = Vec::new();
        udp.extend_from_slice(&40000u16.to_be_bytes());
        udp.extend_from_slice(&53u16.to_be_bytes());
        udp.extend_from_slice(&((8 + dns.len()) as u16).to_be_bytes());
        udp.extend_from_slice(&0u16.to_be_bytes());
        udp.extend_from_slice(&dns);

        let mut ip = vec![0x45, 0x00];
        ip.extend_from_slice(&((20 + udp.len()) as u16).to_be_bytes());
        ip.extend_from_slice(&[0, 0, 0, 0, 64, 17, 0, 0]);
        ip.extend_from_slice(&[10, 0, 0, 2]);
        ip.extend_from_slice(&[8, 8, 8, 8]);
        ip.extend_from_slice(&udp);

        let mut ethernet = vec![0u8; 12];
        ethernet.extend_from_slice(&[0x08, 0x00]);
        ethernet.extend_from_slice(&ip);
        ethernet
    }

    fn capture() -> Vec<u8> {
        let mut out = Vec::new();
        // SHB: BOM 0x1A2B3C4D 小端排布 + 版本 1.0 + section length -1。
        let mut shb = Vec::new();
        shb.extend_from_slice(&[0x4d, 0x3c, 0x2b, 0x1a]);
        shb.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        shb.extend_from_slice(&u64::MAX.to_le_bytes());
        out.extend_from_slice(&block(0x0a0d_0d0a, &shb));
        // IDB: Ethernet, snaplen 65535。
        let mut idb = Vec::new();
        idb.extend_from_slice(&1u16.to_le_bytes());
        idb.extend_from_slice(&0u16.to_le_bytes());
        idb.extend_from_slice(&65535u32.to_le_bytes());
        out.extend_from_slice(&block(1, &idb));
        // EPB: interface 0, 时间戳 0, captured/original 长度 = 帧长。
        let frame = dns_frame();
        let mut epb = Vec::new();
        epb.extend_from_slice(&0u32.to_le_bytes());
        epb.extend_from_slice(&0u32.to_le_bytes());
        epb.extend_from_slice(&0u32.to_le_bytes());
        epb.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        epb.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        epb.extend_from_slice(&frame);
        out.extend_from_slice(&block(6, &epb));
        out
    }

    #[test]
    fn parses_enhanced_packet_block() {
        let report = parse_capture(&capture()).expect("parse pcapng");
        assert_eq!(report.format, "pcapng");
        assert_eq!(report.link_type, 1);
        assert_eq!(report.packet_count, 1);
        assert!(report
            .flows
            .iter()
            .any(|flow| flow.protocol == "udp" && flow.destination == "8.8.8.8:53"));
        assert_eq!(report.dns_queries, vec!["example.com".to_string()]);
    }

    #[test]
    fn truncated_pcapng_is_rejected() {
        let mut bytes = capture();
        bytes.truncate(bytes.len() - 12);
        assert!(matches!(
            parse_capture(&bytes),
            Err(PcapError::Truncated(_))
        ));
    }
}
