#![no_main]

//! fuzz: **受控出站管控面**（`crates/capabilities/tools/src/egress.rs`）——
//! 「这个进程对外的连接能不能放出去」的判定层。
//!
//! ## 为什么这个面值得 fuzz（docs/02-guides/fuzz-target-authoring.md §1）
//!
//! - **吃不受控输入**：目的地 URL / 主机名 / 端口 / 请求头都来自 LLM 生成的参数与
//!   运维配置（`tool.fetch`、内容流订阅源拉取都走这条通道）；
//! - **有强承诺**：白名单精确匹配、IP 分类、逐跳重验、凭据面拒绝 —— 任何一条被
//!   绕过 = SSRF / 凭据出境；
//! - **接线了**：`ControlledEgress` 是受控出网的唯一通道（不是「没接线」的待验面）。
//!
//! ## 契约断言的出处（逐条对应被测函数自己写明的契约，不自造期望）
//!
//! | # | 断言 | 出处 |
//! |---|------|------|
//! | 1 | allowlist 命中 ⟺ 同主机（大小写不敏感）∧ 端口合取（无端口入口匹配任意端口） | `egress.rs:140-143`「compared case-insensitively and exactly; subdomains and wildcards are not implied」+ `egress.rs:161-162` |
//! | 2 | `ExplicitAllowList` 只放行**显式列出**的精确 host/port | `egress.rs:181-182`「Only explicitly listed exact hosts/ports are allowed.」 |
//! | 3 | `PublicInternetOnly`：任一解析地址非公网 ⇒ 整体拒绝 | `egress.rs:178-180`、`egress.rs:212-216` |
//! | 4 | 「一个地址都没解析出来」= `ResolutionFailed`，绝不是隐式放行 | `egress.rs:234-239` + 冻结契约 `docs/01-architecture/m2d-network-egress-enforcement.md` §DNS semantics「DNS failure is `ResolutionFailed`, never an implicit allow」 |
//! | 5 | allowlist **主机名**解析出的地址落入 Loopback/LinkLocal/Private/Unspecified/Broadcast ⇒ 拒；**IP 字面量**目的地保持「显式入表即信任」 | `egress.rs:216-223`、`egress.rs:253-276` |
//! | 6 | IPv4-mapped IPv6 按其内嵌 IPv4 地址分类 | `egress.rs:70-72` |
//! | 7 | `is_public_ip` ⟺ 分类为 `Public` | `egress.rs:85-88` |
//! | 8 | 带 userinfo（凭据）的 URL 必须被拒（凭据不得随 URL 出境） | `egress.rs:294-300`「url userinfo is not supported in controlled egress」+ 冻结契约 §3 Pipeline「reject userinfo」 |
//! | 9 | 非 http(s) 协议必须被拒 | `egress.rs:294-296` + 冻结契约 §3 Pipeline「validate scheme (http/https)」 |
//! | 10 | 头名/头值含 CR/LF 必须被拒（CRLF 注入 / 请求走私面） | `egress.rs:441-444`「Header names and values are validated for CRLF injection」+ `egress.rs:564-583` |
//! | 11 | 出境脱敏：检测器命中的真值不得留在出境文本里（**多实例**与**变体形态**同查） | `redaction_bridge.rs:279-283`「输出只含占位族, 永不含真值」+ 模块头 §变体绕过防御面 V1–V5 |
//!
//! 判据 11 走的是**占位族**（出境脱敏双向闸 `RedactionBridge`），与既有 target
//! `guardrail_security.rs` 覆盖的**单向标签族**（`[REDACTED_*]` 绊线）不重叠：
//! 前者是「真值 → 占位符（可还原）」，后者是「真值 → 单向标签」。
//!
//! ## 不做的事（诚实边界）
//!
//! - **不发真网络请求**：async 入口只在 `DenyAll` 策略下调用，判定发生在**任何
//!   await 之前**，故 future 必然一次 poll 即 `Ready`（若实现顺序变了 —— 先 await
//!   再判定 —— 本 target 会 panic 并如实报告，不静默放过）；
//! - **不引入新依赖**：URL 用同 crate 的公开 API `validate_feed_url` 造（不直接
//!   依赖 `url` crate）；变体形态的 base64/百分号/十六进制按公开标准在 harness 侧
//!   独立实现（不读被测实现的内部函数 → 避免同义反复）。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use apeireth_governance::input_security::{PiiDetector, PiiKind};
use apeireth_tools_canonical::content_feed::store::validate_feed_url;
use apeireth_tools_canonical::egress::{
    ControlledEgress, EgressAllowList, EgressDestination, EgressError, EgressIpClass, EgressPolicy,
    classify_ip, is_public_ip,
};
use apeireth_tools_canonical::redaction_bridge::RedactionBridge;
use libfuzzer_sys::fuzz_target;

// ───────────────────────────── 极简执行器 ─────────────────────────────

/// 只 poll 一次的执行器：本 target 只在 `DenyAll` 下调用 async 入口，判定在任何
/// await 之前返回，故 future 必然一次 poll 即完成。
struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn poll_once<F: std::future::Future>(future: F) -> Poll<F::Output> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    future.as_mut().poll(&mut context)
}

/// 在 `DenyAll` 下跑一次受控 GET —— 只可能得到「判定层的拒绝」。
///
/// `DenyAll` 的契约（`egress.rs:176-177`「No destination is allowed.」）保证不会
/// 触网；判定顺序（头 → URL 解析 → 策略）保证不会进入任何 await。
fn denied_by_policy(url: &str, headers: &[(&str, &str)]) -> EgressError {
    let transport = ControlledEgress::new(EgressPolicy::DenyAll);
    match poll_once(transport.get_with_headers(url, headers)) {
        Poll::Ready(Err(error)) => error,
        Poll::Ready(Ok(response)) => {
            panic!("DenyAll 下不得放行任何目的地: url={url:?} response={response:?}")
        }
        Poll::Pending => panic!("判定必须发生在任何 await 之前（实现顺序变了？）: url={url:?}"),
    }
}

// ───────────────────────────── 输入拆分 ─────────────────────────────

fn field<'a>(fields: &'a [&'a [u8]], index: usize) -> &'a [u8] {
    fields.get(index).copied().unwrap_or(&[])
}

/// 任意字节 → 合法 UTF-8 文本（非法字节按 U+FFFD 替换 → 永不 panic）。
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// 主机名形状的文本：空白/控制字符换成 `'x'`；其余（**含 Unicode 与零宽字符**）
/// 原样保留 —— 那些正是要验的「变体」。
fn host_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .map(|c| {
            if c.is_whitespace() || c.is_control() {
                'x'
            } else {
                c
            }
        })
        .collect()
}

/// URL 安全的路径片段：只保留无歧义字符，其余换成 `'x'`（保证拼出来的 URL 一定
/// 能被解析 —— 这样「非 http(s) 协议 ⇒ `UnsupportedScheme`」才是强断言）。
fn path_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-._~!$&'()*+,;=:@/%".contains(c) {
                c
            } else {
                'x'
            }
        })
        .collect()
}

/// URL 安全的 userinfo 片段（**不含** `@`，否则就不是 userinfo 了）。
fn userinfo_text(bytes: &[u8]) -> String {
    text(bytes)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-._~!$&'()*+,;=:".contains(c) {
                c
            } else {
                'x'
            }
        })
        .collect()
}

fn port_of(bytes: &[u8]) -> u16 {
    let high = bytes.first().copied().unwrap_or(0);
    let low = bytes.get(1).copied().unwrap_or(0);
    u16::from_le_bytes([high, low])
}

fn optional_port(bytes: &[u8]) -> Option<u16> {
    if bytes.is_empty() {
        None
    } else {
        Some(port_of(bytes))
    }
}

/// 构造一个与 `EgressDestination::parse` 同口径的目的地（host 小写；host 是 IP
/// 字面量时 `address = Some`）。
fn destination_of(host: &str, port: u16) -> EgressDestination {
    let host = host.to_ascii_lowercase();
    EgressDestination {
        scheme: "https".to_string(),
        address: host.parse::<IpAddr>().ok(),
        host,
        port,
    }
}

// ───────────────────────── 判据 1+2：allowlist 精确匹配 ─────────────────────────

/// 契约：`EgressAllowList` 是**精确匹配**表 —— 主机名（或 IP 字面量）**大小写不敏感
/// 且完全相等**地比较；**不隐含子域、不隐含通配符**（`egress.rs:140-143`）。
/// 端口语义：**入口无端口 ⇒ 任意端口命中；入口有端口 ⇒ 必须相等**
/// （`egress.rs:161-162`）。
///
/// 由此得到充要条件：命中 ⟺ 同主机（大小写不敏感）∧ 端口合取。
/// 「子域 / 通配 / 后缀」类绕过（`evil-example.com`、`a.example.com`、
/// `*.example.com`、`example.com.evil.net`）在这条充要条件下**必须不命中**。
fn expected_match(
    entry_host: &str,
    entry_port: Option<u16>,
    probe_host: &str,
    probe_port: u16,
) -> bool {
    let same_host = entry_host.to_ascii_lowercase() == probe_host.to_ascii_lowercase();
    let port_ok = entry_port.map(|port| port == probe_port).unwrap_or(true);
    same_host && port_ok
}

fn section_allowlist_exactness(fields: &[&[u8]]) {
    let entry_host = host_text(field(fields, 0));
    let entry_port = optional_port(field(fields, 1));
    let probe_host = host_text(field(fields, 2));
    let probe_port = port_of(field(fields, 3));

    let list = EgressAllowList::new().allow(entry_host.clone(), entry_port);
    let matched = list.contains(&probe_host, probe_port);
    let expected = expected_match(&entry_host, entry_port, &probe_host, probe_port);

    if entry_host.is_ascii() && probe_host.is_ascii() {
        // ASCII 主机：契约给出完整充要条件。
        assert_eq!(
            matched, expected,
            "allowlist 判定与契约不符（精确匹配/端口语义）: entry={entry_host:?}/{entry_port:?} \
             probe={probe_host:?}/{probe_port} matched={matched} expected={expected}"
        );
    } else {
        // 非 ASCII 主机：大小写折叠口径在文档里不唯一（实现用 ASCII 小写），
        // 故只断言**不误放行**方向 —— 命中就必须同一主机。
        assert!(
            !matched || expected,
            "allowlist 放行了非同一主机（精确匹配被绕过）: entry={entry_host:?} probe={probe_host:?}"
        );
    }
}

// ───────────────────── 判据 2：`ExplicitAllowList` 的放行口径 ─────────────────────

/// 契约：`ExplicitAllowList` ——「Only explicitly listed exact hosts/ports are
/// allowed.」（`egress.rs:181-182`）。判定入口 `validate_destination`
/// （`egress.rs:189-210`）。
fn section_policy_destination(fields: &[&[u8]]) {
    let entry_host = host_text(field(fields, 0));
    let entry_port = optional_port(field(fields, 1));
    let probe_host = host_text(field(fields, 2));
    let probe_port = port_of(field(fields, 3));

    let list = EgressAllowList::new().allow(entry_host.clone(), entry_port);
    let policy = EgressPolicy::ExplicitAllowList(list);
    let destination = destination_of(&probe_host, probe_port);

    let allowed = policy.validate_destination(&destination).is_ok();
    let expected = expected_match(&entry_host, entry_port, &probe_host, probe_port);

    if entry_host.is_ascii() && probe_host.is_ascii() {
        assert_eq!(
            allowed, expected,
            "显式白名单的放行口径与契约不符（只放行显式列出的精确 host/port）: \
             entry={entry_host:?}/{entry_port:?} probe={probe_host:?}/{probe_port} \
             allowed={allowed} expected={expected}"
        );
    } else {
        assert!(
            !allowed || expected,
            "显式白名单放行了未列出的目的地: entry={entry_host:?} probe={probe_host:?}/{probe_port}"
        );
    }

    // `DenyAll` —— 「No destination is allowed.」（`egress.rs:176-177`）：
    // 任何目的地都必须被拒，且拒绝必须**成帧**（结构化 `DestinationDenied`）。
    assert!(
        matches!(
            EgressPolicy::DenyAll.validate_destination(&destination),
            Err(EgressError::DestinationDenied { .. })
        ),
        "DenyAll 必须拒掉一切目的地"
    );

    // `Unrestricted` —— 「Every http/https destination is allowed.」
    // （`egress.rs:183-185`）：显式退出闸，本层不拒。
    assert!(
        EgressPolicy::Unrestricted
            .validate_destination(&destination)
            .is_ok(),
        "Unrestricted 本层不得拒"
    );
}

// ─────────────── 判据 3+4+5：解析地址的策略判定（多地址 = 多实例） ───────────────

/// 契约：
/// - `PublicInternetOnly`：「Every resolved address must be public.」
///   （`egress.rs:178-180`）+「if any resolved address is not public, the whole
///   destination is denied」（`egress.rs:212-216`）—— **混合公私 DNS 结果 → 拒**；
/// - 一个地址都没解析出来 ⇒ `ResolutionFailed`，**绝不是隐式放行**
///   （`egress.rs:234-239` + 冻结契约 §DNS semantics）；
/// - `ExplicitAllowList`：**主机名**目的地解析出的地址落入
///   Loopback/LinkLocal/Private/Unspecified/Broadcast 一律拒绝；**IP 字面量**
///   目的地保持「显式入表即信任」（`egress.rs:216-223`、`egress.rs:253-276`）。
fn section_resolved_addresses(fields: &[&[u8]]) {
    let host = host_text(field(fields, 0));
    let port = port_of(field(fields, 1));
    let destination = destination_of(&host, port);

    // DNS 回程可能是**多条** A/AAAA 记录 —— 这一段就是「多实例」面。
    let count = (field(fields, 2).first().copied().unwrap_or(1) as usize % 4) + 1;
    let mut addresses: Vec<SocketAddr> = Vec::with_capacity(count);
    for index in 0..count {
        let ip = pick_ip(field(fields, 3 + index), index);
        addresses.push(SocketAddr::new(ip, port));
    }

    // ── PublicInternetOnly ──
    let public = EgressPolicy::PublicInternetOnly;
    let result = public.validate_resolved(&destination, &addresses);
    if addresses.is_empty() {
        assert!(
            matches!(result, Err(EgressError::ResolutionFailed { .. })),
            "解析不出地址必须是 ResolutionFailed（绝不是隐式放行）: host={host:?}"
        );
    } else {
        let all_public = addresses.iter().all(|address| is_public_ip(address.ip()));
        assert_eq!(
            result.is_ok(),
            all_public,
            "PublicInternetOnly 判定与契约不符（任一解析地址非公网即整体拒绝）: \
             host={host:?} addresses={addresses:?}"
        );
        if !all_public {
            assert!(
                matches!(result, Err(EgressError::DestinationDenied { .. })),
                "非公网解析地址必须成帧为 DestinationDenied: host={host:?} addresses={addresses:?}"
            );
        }
    }

    // ── ExplicitAllowList（M12 口径）──
    let allow = EgressPolicy::ExplicitAllowList(
        EgressAllowList::new().allow(destination.host.clone(), None),
    );
    let result = allow.validate_resolved(&destination, &addresses);
    let denied_class = |ip: IpAddr| {
        matches!(
            classify_ip(ip),
            EgressIpClass::Loopback
                | EgressIpClass::LinkLocal
                | EgressIpClass::Private
                | EgressIpClass::Unspecified
                | EgressIpClass::Broadcast
        )
    };
    // 契约：只有「主机名目的地 + 解析出上述五类地址之一」才拒；IP 字面量目的地
    // 「显式入表即信任」，不查解析地址。
    let expected = destination.address.is_some()
        || !addresses.iter().any(|address| denied_class(address.ip()));
    assert_eq!(
        result.is_ok(),
        expected,
        "ExplicitAllowList 的解析地址判定与契约不符: host={host:?} addresses={addresses:?} \
         literal={:?}",
        destination.address
    );
    if !expected {
        assert!(
            matches!(result, Err(EgressError::DestinationDenied { .. })),
            "allowlist 主机名解析到内网/回环/链路本地必须成帧为 DestinationDenied: \
             host={host:?} addresses={addresses:?}"
        );
    }

    // ── DenyAll / Unrestricted ──
    assert!(
        matches!(
            EgressPolicy::DenyAll.validate_resolved(&destination, &addresses),
            Err(EgressError::DestinationDenied { .. })
        ),
        "DenyAll 必须拒掉一切解析地址"
    );
    assert!(
        EgressPolicy::Unrestricted
            .validate_resolved(&destination, &addresses)
            .is_ok(),
        "Unrestricted 本层不得拒"
    );
}

/// 构造被测地址：一张「有意思」的表（回环/私网/链路本地/元数据/组播/广播/未指定/
/// 公网/IPv6/mapped-IPv6）+ fuzz 派生的随机 IPv4。
const INTERESTING_IPS: &[&str] = &[
    "0.0.0.0",
    "10.0.0.1",
    "100.64.0.1",
    "127.0.0.1",
    "169.254.169.254",
    "192.168.1.10",
    "172.16.0.1",
    "224.0.0.1",
    "255.255.255.255",
    "8.8.8.8",
    "93.184.216.34",
    "::",
    "::1",
    "fc00::1",
    "fd12:3456::1",
    "fe80::1",
    "ff02::1",
    "2001:db8::1",
    "::ffff:127.0.0.1",
    "::ffff:10.0.0.1",
    "::ffff:8.8.8.8",
    "::ffff:169.254.169.254",
];

fn pick_ip(bytes: &[u8], index: usize) -> IpAddr {
    let choice = bytes.first().copied().unwrap_or(0) as usize;
    if choice % 2 == 0 {
        let literal = INTERESTING_IPS[(choice / 2 + index) % INTERESTING_IPS.len()];
        literal
            .parse::<IpAddr>()
            .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
    } else {
        let octets = [
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
            bytes.get(4).copied().unwrap_or(1),
        ];
        IpAddr::V4(Ipv4Addr::from(octets))
    }
}

// ───────────────────────── 判据 6+7：IP 分类 ─────────────────────────

/// 契约：
/// - 「IPv4-mapped IPv6 addresses are classified by their embedded IPv4 address so
///   `::ffff:127.0.0.1` cannot bypass IPv4 loopback checks.」（`egress.rs:70-72`）
/// - 「True when `ip` is permitted by a `PublicInternetOnly` policy.」
///   （`egress.rs:85-88`）⇒ `is_public_ip` ⟺ 分类为 `Public`。
fn section_ip_classification(fields: &[&[u8]]) {
    let ip = pick_ip(field(fields, 0), 0);

    // IPv4-mapped IPv6 必须按内嵌 IPv4 分类（mapped 判定不得被绕过）。
    if let IpAddr::V4(v4) = ip {
        let raw = u32::from(v4);
        let mapped = IpAddr::V6(Ipv6Addr::new(
            0,
            0,
            0,
            0,
            0,
            0xffff,
            (raw >> 16) as u16,
            (raw & 0xffff) as u16,
        ));
        assert_eq!(
            classify_ip(mapped),
            classify_ip(ip),
            "IPv4-mapped IPv6 必须按内嵌 IPv4 分类: {mapped} vs {ip}"
        );
        assert_eq!(
            is_public_ip(mapped),
            is_public_ip(ip),
            "IPv4-mapped IPv6 的公网判定必须跟随内嵌 IPv4: {mapped} vs {ip}"
        );
    }

    assert_eq!(
        is_public_ip(ip),
        classify_ip(ip) == EgressIpClass::Public,
        "is_public_ip 必须等价于「分类为 Public」: {ip}"
    );

    // 冻结契约 §Address classes：回环/私网/链路本地/组播/未指定/广播的分类口径
    // （Rust 标准库分类；IPv6 另含 ULA）。这些地址**都不是**公网。
    for (literal, class) in [
        ("127.0.0.1", EgressIpClass::Loopback),
        ("10.0.0.1", EgressIpClass::Private),
        ("192.168.1.10", EgressIpClass::Private),
        ("169.254.169.254", EgressIpClass::LinkLocal),
        ("224.0.0.1", EgressIpClass::Multicast),
        ("0.0.0.0", EgressIpClass::Unspecified),
        ("255.255.255.255", EgressIpClass::Broadcast),
        ("::1", EgressIpClass::Loopback),
        ("fc00::1", EgressIpClass::Private),
        ("fe80::1", EgressIpClass::LinkLocal),
        ("ff02::1", EgressIpClass::Multicast),
        ("::", EgressIpClass::Unspecified),
    ] {
        let parsed: IpAddr = literal.parse().unwrap_or(ip);
        assert_eq!(
            classify_ip(parsed),
            class,
            "IP 分类与冻结契约 §Address classes 不符: {literal}"
        );
        assert!(
            !is_public_ip(parsed),
            "非公网地址不得算公网（PublicInternetOnly 的前提）: {literal}"
        );
    }
}

// ───────────────────────── 判据 8+9：URL 解析面（凭据 / 协议） ─────────────────────────

/// 契约（`egress.rs:292-315`）：
/// - scheme 必须是 http/https，否则 `UnsupportedScheme`（`egress.rs:294-296`）；
/// - userinfo 一律拒（`egress.rs:297-301`，错误语义「url userinfo is not supported
///   in controlled egress」；冻结契约 §3 Pipeline「reject userinfo」）；
/// - 无主机 ⇒ `InvalidUrl`（`egress.rs:302-304`）；
/// - IP 字面量 ⇒ `address = Some`（`egress.rs:288-289`、`egress.rs:308`）；
/// - 目的地是**归一化**的（host 小写，`egress.rs:282-283`、`egress.rs:311`）。
fn section_url_parse(fields: &[&[u8]]) {
    // ── (a) 确定性形状：从同 crate 的公开 API 造一个 `url::Url`（不引新依赖）──
    if let Ok(base) = validate_feed_url("http://example.com/", &EgressPolicy::Unrestricted) {
        // ① 凭据面：带 userinfo 的 URL 必须被拒 —— 凭据不得随 URL 出境。
        let mut with_user = base.clone();
        if with_user.set_username("user").is_ok() {
            assert!(
                matches!(
                    EgressDestination::parse(&with_user),
                    Err(EgressError::InvalidUrl(_))
                ),
                "带 userinfo 的 URL 必须被拒（凭据不得出境）: {with_user}"
            );
        }
        let mut with_password = base.clone();
        if with_password.set_username("user").is_ok()
            && with_password.set_password(Some("pass")).is_ok()
        {
            assert!(
                matches!(
                    EgressDestination::parse(&with_password),
                    Err(EgressError::InvalidUrl(_))
                ),
                "带口令的 URL 必须被拒（凭据不得出境）: {with_password}"
            );
        }

        // ② 协议面：非 http(s) 必须被拒。
        let mut other_scheme = base.clone();
        if other_scheme.set_scheme("ftp").is_ok() {
            assert!(
                matches!(
                    EgressDestination::parse(&other_scheme),
                    Err(EgressError::UnsupportedScheme(_))
                ),
                "非 http(s) 协议必须被拒: {other_scheme}"
            );
        }

        // ③ 无主机必须被拒。
        let mut no_host = base.clone();
        if no_host.set_host(None).is_ok() {
            assert!(
                matches!(
                    EgressDestination::parse(&no_host),
                    Err(EgressError::InvalidUrl(_))
                ),
                "无主机的 URL 必须被拒: {no_host}"
            );
        }

        // ④ IP 字面量 ⇒ `address = Some`；端口取协议默认（http → 80）。
        let mut literal = base.clone();
        if literal.set_host(Some("127.0.0.1")).is_ok() {
            let parsed = EgressDestination::parse(&literal).expect("IP 字面量目的地必须可解析");
            assert_eq!(
                parsed.address,
                Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
                "IP 字面量目的地必须带 address"
            );
            assert_eq!(parsed.host, "127.0.0.1");
            assert_eq!(parsed.port, 80, "http 的默认端口必须是 80");
        }

        // ⑤ 主机名归一化（小写）。
        let mut upper = base.clone();
        if upper.set_host(Some("EXAMPLE.COM")).is_ok() {
            let parsed = EgressDestination::parse(&upper).expect("主机名目的地必须可解析");
            assert_eq!(
                parsed.host,
                upper.host_str().unwrap_or_default().to_ascii_lowercase(),
                "目的地主机必须归一化为小写"
            );
        }

        // ⑥ 显式端口必须被采纳。
        let mut ported = base.clone();
        if ported.set_port(Some(8443)).is_ok() {
            let parsed = EgressDestination::parse(&ported).expect("显式端口目的地必须可解析");
            assert_eq!(parsed.port, 8443);
        }
    }

    // ── (b) 模糊输入的 URL：走 async 入口（DenyAll ⇒ 不触网）──
    let path = path_text(field(fields, 0));
    let host = host_text(field(fields, 1));
    let userinfo = userinfo_text(field(fields, 2));
    let scheme = if field(fields, 3).first().copied().unwrap_or(0) % 2 == 0 {
        "http"
    } else {
        "https"
    };

    // ① 凭据面：URL 里带**真凭据**（非空用户名或非空口令）⇒ 必须被拒（无论 URL
    //    本身是否可解析，两条路径都是 `InvalidUrl`：解析失败也是 `InvalidUrl`，
    //    解析成功则被 userinfo 拒）。
    //    例外（**harness 自造期望的坑，已修**）：userinfo 全是 ':'（空用户名 + 空
    //    口令）时，`url` crate 的 `parse_userinfo` 因 has_username/has_password 皆
    //    为 false 而**不写 '@'** → 整段 userinfo 被归一化掉 → URL 里根本没有凭据
    //    （`username()` 为空且 `password()` 为 None）→ 不该被拒。第一版断言写成
    //    「userinfo 非空 ⇒ InvalidUrl」，被 fuzz 用 `http://:@x2/` 打破 —— 那是
    //    harness 的错，不是库的洞。
    let carries_credential = userinfo.chars().any(|c| c != ':');
    if carries_credential {
        let url = format!("{scheme}://{userinfo}@{host}/{path}");
        let error = denied_by_policy(&url, &[]);
        assert!(
            matches!(error, EgressError::InvalidUrl(_)),
            "带凭据的 URL 必须被拒（凭据不得随 URL 出境）: url={url:?} error={error:?}"
        );
    }

    // ② 协议面：固定合法主机 ⇒ URL 一定可解析 ⇒ 非 http(s) 必须成帧为
    //    `UnsupportedScheme`（强断言，不是「大概被拒」）。
    for other in ["ftp", "file", "gopher", "javascript", "data", "ws"] {
        let url = format!("{other}://example.com/{path}");
        let error = denied_by_policy(&url, &[]);
        assert!(
            matches!(error, EgressError::UnsupportedScheme(_)),
            "非 http(s) 协议必须被拒成 UnsupportedScheme: url={url:?} error={error:?}"
        );
    }

    // ③ 正常 URL：若同 crate 的 URL 校验认为它合法（可解析 + http(s) + 无
    //    userinfo + 有主机），则判定必须走到策略层并被 `DenyAll` 拒成
    //    `DestinationDenied`（证明「没在解析层被悄悄放过」）。
    let plain = format!("{scheme}://{host}/{path}");
    if validate_feed_url(&plain, &EgressPolicy::Unrestricted).is_ok() {
        let error = denied_by_policy(&plain, &[]);
        assert!(
            matches!(error, EgressError::DestinationDenied { .. }),
            "合法 URL 必须走到策略层并被 DenyAll 拒: url={plain:?} error={error:?}"
        );
    }
}

// ───────────────────────── 判据 10：请求头面 ─────────────────────────

/// 契约：「Header names and values are validated for CRLF injection」
/// （`egress.rs:441-444`）+ 头名/头值不得为空（`egress.rs:566-570`）。
fn section_headers(fields: &[&[u8]]) {
    let name = text(field(fields, 0));
    let value = text(field(fields, 1));

    let error = denied_by_policy("http://example.com/", &[(name.as_str(), value.as_str())]);

    if name.contains('\r') || name.contains('\n') || value.contains('\r') || value.contains('\n') {
        assert!(
            matches!(error, EgressError::InvalidHeader(_)),
            "含 CR/LF 的头必须被拒（CRLF 注入 / 请求走私面）: name={name:?} value={value:?} \
             error={error:?}"
        );
    } else if name.is_empty() || value.is_empty() {
        assert!(
            matches!(error, EgressError::InvalidHeader(_)),
            "空头名/空头值必须被拒: name={name:?} value={value:?} error={error:?}"
        );
    } else {
        assert!(
            matches!(error, EgressError::DestinationDenied { .. }),
            "合法头不得被误判为非法头（否则正常请求全被拒）: name={name:?} value={value:?} \
             error={error:?}"
        );
    }
}

// ────────────── 判据 11：出境脱敏后的残留检查（多实例 + 变体形态） ──────────────

/// 真值种子（每条都是 `PiiDetector` 认得的形状；**真值恰好等于检测器命中的区间**，
/// 否则断言会误判）。
const REDACTION_SEEDS: &[&str] = &[
    "test.user@example.com",
    "13812345678",
    "sk-1234567890abcdef1234567890",
    "192.168.1.10",
    "export API_KEY=abcdefgh12345678",
    "https://user:pass@example.com",
];

/// 契约（`redaction_bridge.rs:279-283`）：「出境脱敏: 真值面 → 占位符（检测器命中 +
/// 已登记值的裸/变体形态）…… **输出只含占位族, 永不含真值**」；模块头
/// §变体绕过防御面 列出 V1 裸形态 / V2 base64 / V3 百分号 / V4 十六进制 /
/// V5 拆分串（字符间插空白或零宽字符）。
fn section_egress_redaction_residue(fields: &[&[u8]]) {
    let context = text(field(fields, 0));
    let seed_index = field(fields, 1).first().copied().unwrap_or(0) as usize;
    let seed = REDACTION_SEEDS[seed_index % REDACTION_SEEDS.len()];

    // 上下文里若已含该真值 ⇒ 跳过（否则「同值多处」会造成假阳性）。
    if context.contains(seed) {
        return;
    }

    // ① 多实例：同一真值出现两次 ⇒ **两处都要**换占位（历史上「每族只脱敏首处」
    //    就是这类漏网）。
    let doubled = format!("中文 {context} | {seed} | {context} | {seed}");
    let mut bridge = RedactionBridge::with_session_key([7u8; 32]);
    let egress = bridge.redact_for_egress(&doubled);
    assert!(
        !egress.text.contains(seed),
        "出境文本仍含真值（多实例漏网）: seed={seed:?} egress={:?}",
        egress.text
    );

    // ② 任意文本：检测器命中的每一个真值都不得留在出境文本里。
    //    （唯一性前置检查：真值在原文出现多次时无法区分「漏网处」与「未命中处」，
    //    那种情况由上面的多实例用例覆盖。）
    let mut bridge = RedactionBridge::with_session_key([11u8; 32]);
    let egress = bridge.redact_for_egress(&context);
    for finding in PiiDetector::findings(&context) {
        let Some(value) = context.get(finding.start..finding.end) else {
            continue;
        };
        if value.is_empty() || context.matches(value).count() != 1 {
            continue;
        }
        assert!(
            !egress.text.contains(value),
            "出境文本仍含检测器命中的真值（残留）: kind={:?} value={value:?} egress={:?}",
            finding.kind,
            egress.text
        );
    }

    // ③ 变体形态：已登记真值的 V2–V5 形态必须全部换占位，且**不得误伤**正常文本。
    //    固定带一个非 ASCII 前缀（修复前：影子串偏移错位 → 命中区间算不出来 = 真值
    //    原样出境，或算到错误位置 = 误伤正常文本 + 真值碎片残留）。
    //    变体编码按公开标准在 harness 侧独立实现（不读被测实现的私有函数）。
    let secret = "hunter2-plain-value";
    let mut bridge = RedactionBridge::with_session_key([13u8; 32]);
    bridge.register_value(PiiKind::EnvSecret, secret);
    let marker = "结尾标记";
    for variant in variant_forms(secret) {
        let text = format!("中文 {context} | {variant} | {marker}");
        let egress = bridge.redact_for_egress(&text);
        assert!(
            !egress.text.contains(&variant),
            "变体形态漏网（该形态仍明文出境）: variant={variant:?} egress={:?}",
            egress.text
        );
        if !context.contains(marker) {
            assert!(
                egress.text.contains(marker),
                "命中区间错位 → 正常文本被误伤: egress={:?}",
                egress.text
            );
        }
    }
}

// ───────────────────── 变体形态编码（公开标准，独立实现） ─────────────────────

fn base64_standard(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(chunk.get(1).copied().unwrap_or(0));
        let b2 = u32::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(char::from(ALPHABET[((triple >> 18) & 0x3F) as usize]));
        out.push(char::from(ALPHABET[((triple >> 12) & 0x3F) as usize]));
        if chunk.len() > 1 {
            out.push(char::from(ALPHABET[((triple >> 6) & 0x3F) as usize]));
        }
        if chunk.len() > 2 {
            out.push(char::from(ALPHABET[(triple & 0x3F) as usize]));
        }
    }
    out
}

fn base64_unpadded(bytes: &[u8]) -> String {
    base64_standard(bytes).trim_end_matches('=').to_string()
}

fn percent_encode(bytes: &[u8], upper_hex: bool) -> String {
    let mut out = String::new();
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(*byte));
        } else if upper_hex {
            out.push_str(&format!("%{byte:02X}"));
        } else {
            out.push_str(&format!("%{byte:02x}"));
        }
    }
    out
}

fn hex_encode(bytes: &[u8], upper: bool) -> String {
    let mut out = String::new();
    for byte in bytes {
        if upper {
            out.push_str(&format!("{byte:02X}"));
        } else {
            out.push_str(&format!("{byte:02x}"));
        }
    }
    out
}

/// V5 拆分串：真值字符之间插入分隔符（空白 / 零宽字符）。
fn split_form(value: &str, separator: &str) -> String {
    let mut out = String::new();
    for (index, character) in value.chars().enumerate() {
        if index > 0 {
            out.push_str(separator);
        }
        out.push(character);
    }
    out
}

fn variant_forms(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut forms = vec![
        base64_standard(bytes),
        base64_unpadded(bytes),
        percent_encode(bytes, true),
        percent_encode(bytes, false),
        hex_encode(bytes, true),
        hex_encode(bytes, false),
        split_form(value, "\u{200B}"),
        split_form(value, " "),
    ];
    forms.retain(|form| form != value && !form.is_empty());
    forms
}

// ───────────────────────────── 主入口 ─────────────────────────────

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let fields: Vec<&[u8]> = rest.split(|byte| *byte == 0x00).collect();
    match mode % 7 {
        0 => section_allowlist_exactness(&fields),
        1 => section_policy_destination(&fields),
        2 => section_resolved_addresses(&fields),
        3 => section_ip_classification(&fields),
        4 => section_url_parse(&fields),
        5 => section_headers(&fields),
        _ => section_egress_redaction_residue(&fields),
    }
});
