//! netstat 受控采集端到端实测 (真实子进程 + 真实解析 + 真实工具面)。
//!
//! 口径 (诚实):
//! * **Windows: 已实测** —— 真跑系统目录里那一个 `netstat.exe` (可信路径闸门
//!   放行的那一个), 真实解析其输出, 并钉住真实调用走的是 5 秒时限与输出上限。
//! * **Linux / macOS: 未实测** —— 非 Windows 上本文件只断言「子进程真实跑通
//!   + 上限生效」, **不**断言解析形状 (net-tools 输出形状与 Windows 不同, 本仓
//!   未实测), 不冒充「跨平台已验证」。
//! * 无管理员权限也能过: 断言只依赖「本机确有监听项」, 不依赖任何需要管理员
//!   才有的视图 (逐进程列名等)。
//! * 工具缺失或输出超上限 ⇒ **显式跳过并留下可判真原因**, 不静默 pass; 空输出
//!   一律判失败 (不写成通过)。

use apeireth_tools_canonical::exposure_audit::listener_surface::{
    capture_netstat, NETSTAT_MAX_RUNTIME, NETSTAT_MAX_STDERR_BYTES, NETSTAT_MAX_STDOUT_BYTES,
};
use apeireth_tools_canonical::exposure_audit::{
    parse_netstat_snapshot, resolve_trusted_tool_path, trusted_netstat_path, trusted_system_root,
    ListenerProbeError, ListenerScope,
};
use serde_json::json;

#[cfg(windows)]
use apeireth_plugin::ToolCapability;
#[cfg(windows)]
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
#[cfg(windows)]
use apeireth_tools_canonical::exposure_audit::{
    ExposureAuditConfig, ExposureAuditTool, ItemStatus,
};

/// 平台事实: 本机是否具备可信 netstat —— 可判真 (缺工具 ⇒ 采集显式不可用)。
#[test]
fn platform_reports_the_trusted_netstat_fact() {
    match trusted_netstat_path() {
        Some(path) => {
            let root = trusted_system_root().expect("有可信 netstat ⇒ 必有信任根");
            assert_eq!(
                path.parent(),
                Some(root.as_path()),
                "可信路径必须直接位于信任根内"
            );
            assert!(
                resolve_trusted_tool_path(&root, &path).is_ok(),
                "生产可信路径必须过形状闸门与归属闸门"
            );
            eprintln!("netstat-e2e: 可信系统工具可用 ({})", path.display());
        }
        None => {
            assert!(
                matches!(
                    capture_netstat(),
                    Err(ListenerProbeError::Unavailable { .. })
                ),
                "缺系统工具 ⇒ 采集必须显式不可用 (不静默通过)"
            );
            eprintln!(
                "netstat-e2e: SKIP reason=trusted system netstat unavailable on this host (采集显式不可用)"
            );
        }
    }
}

#[cfg(windows)]
#[test]
fn real_netstat_capture_runs_with_pinned_limits_and_parses_listeners() {
    let Some(path) = trusted_netstat_path() else {
        eprintln!(
            "netstat-e2e: SKIP reason=trusted system netstat unavailable on this host (采集显式不可用)"
        );
        assert!(
            matches!(
                capture_netstat(),
                Err(ListenerProbeError::Unavailable { .. })
            ),
            "缺系统工具 ⇒ 采集必须显式不可用 (不静默通过)"
        );
        return;
    };
    assert!(path.is_file(), "可信路径必须指向真实文件");

    let capture = match capture_netstat() {
        Ok(capture) => capture,
        Err(ListenerProbeError::Failed { reason }) if reason.contains("truncated") => {
            // 输出触到上限 ⇒ 采集按上限显式失败 (上限真实生效), 但本次没有
            // 可解析快照 ⇒ 显式跳过并留下原因, 不冒充通过。
            eprintln!(
                "netstat-e2e: SKIP reason=真实输出超过 {NETSTAT_MAX_STDOUT_BYTES} 字节上限, 采集按上限显式失败 (上限生效, 本次未取得快照): {reason}"
            );
            return;
        }
        Err(error) => panic!("受控采集必须真实跑通: {error}"),
    };

    // 真实子进程事实。
    assert_eq!(capture.exit_code, Some(0), "真实子进程退出码必须是 0");
    assert!(!capture.stdout_truncated, "标准输出不得被截断");
    assert!(!capture.stderr_truncated, "标准错误不得被截断");
    // 真实调用走的是这些上限 (不是无限等)。
    // 与**字面值**比, 不与同一常量比: 与常量比会恒真 (改常量时两侧一起动,
    // 断言不会红), 那样就钉不住「真实调用用的就是 5 秒 / 256 KiB」。
    assert_eq!(
        capture.max_runtime,
        std::time::Duration::from_secs(5),
        "真实调用必须走 5 秒时限 (不是无限等)"
    );
    assert_eq!(
        capture.max_stdout_bytes,
        256 * 1024,
        "真实调用必须走 256 KiB 标准输出上限"
    );
    assert_eq!(
        capture.max_stderr_bytes,
        16 * 1024,
        "真实调用必须走 16 KiB 标准错误上限"
    );
    // 对外公开的常量与字面值一致 (口径对外钉死)。
    assert_eq!(
        NETSTAT_MAX_RUNTIME,
        std::time::Duration::from_secs(5),
        "公开时限常量必须是 5 秒"
    );
    assert_eq!(
        NETSTAT_MAX_STDOUT_BYTES,
        256 * 1024,
        "公开输出上限常量必须是 256 KiB"
    );
    assert_eq!(
        NETSTAT_MAX_STDERR_BYTES,
        16 * 1024,
        "公开错误输出上限常量必须是 16 KiB"
    );
    assert!(
        capture.elapsed < capture.max_runtime,
        "采集耗时必须落在时限内: {:?} / {:?}",
        capture.elapsed,
        capture.max_runtime
    );
    assert!(
        capture.stdout.len() <= capture.max_stdout_bytes,
        "采集到的输出不得超过上限"
    );
    assert!(
        !capture.stdout.trim().is_empty(),
        "真实输出不得为空 (空输出不写成通过)"
    );

    // 真实解析事实。
    let snapshot = parse_netstat_snapshot(&capture.stdout);
    assert!(
        !snapshot.listeners.is_empty(),
        "真实输出必须解析出至少一个监听项; 输出前 400 字: {:?}",
        &capture.stdout[..capture.stdout.len().min(400)]
    );
    // 行级真值 (独立于被测解析器): 直接从真实输出数出 TCP 监听行及其端口。
    // 没有它, 「解析结果非空」会被 UDP 行兜住 ⇒ 关掉 TCP 监听判定也照样绿。
    let tcp_listening = tcp_listening_ports(&capture.stdout);
    assert!(
        !tcp_listening.is_empty(),
        "真实输出必须含至少一条 TCP 监听行 (否则本条行级断言空转)"
    );
    for (proto, port) in &tcp_listening {
        assert!(
            snapshot
                .listeners
                .iter()
                .any(|listener| &listener.proto == proto && listener.port == *port),
            "真实 TCP 监听行必须在解析结果里有对应条目: {proto} {port}"
        );
    }
    for listener in &snapshot.listeners {
        assert!(
            (1..=65535).contains(&listener.port),
            "端口号必须落在合法区间: {}",
            listener.port
        );
        assert!(
            matches!(listener.proto.as_str(), "tcp" | "tcp6" | "udp" | "udp6"),
            "协议词面必须来自系统表: {}",
            listener.proto
        );
        // 地址列必须真的存在 (环境策略回归的哨兵: 环境全空时系统表工具会以
        // 退出码 0 结束却省略地址列 ⇒ 监听面被清成「零发现」)。
        assert!(
            !listener.local_addr.trim().is_empty(),
            "监听项的本地地址列不得为空: {listener:?}"
        );
    }
    eprintln!(
        "netstat-e2e: 解析出监听项 {} ({})",
        snapshot.listeners.len(),
        scope_histogram(&snapshot)
    );
}

#[cfg(windows)]
#[tokio::test]
async fn exposure_audit_tool_measures_listening_ports_from_the_real_capture() {
    if trusted_netstat_path().is_none() {
        eprintln!(
            "netstat-e2e: SKIP reason=trusted system netstat unavailable on this host (采集显式不可用)"
        );
        assert!(
            matches!(
                capture_netstat(),
                Err(ListenerProbeError::Unavailable { .. })
            ),
            "缺系统工具 ⇒ 采集必须显式不可用 (不静默通过)"
        );
        return;
    }
    let data_root = tempfile::tempdir().expect("temp dir");
    // 默认探测口 = 生产探测口 (真实 netstat 受控采集), 不注入替身。
    let tool = ExposureAuditTool::new(ExposureAuditConfig::new(data_root.path()));
    let call = ToolCall {
        id: "netstat-e2e".into(),
        name: "exposure_audit".into(),
        arguments: json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "工具调用必须成功: {}", result.render());
    let ToolOutcome::Ok { value } = result.outcome else {
        panic!("工具必须返回成功结果: {}", result.render());
    };
    let items = value["items"].as_array().expect("报告 items 数组");
    let item = items
        .iter()
        .find(|item| item["id"] == "listening_ports")
        .expect("listening_ports 体检项");
    assert_eq!(
        item["status"],
        ItemStatus::Measured.as_str(),
        "真实 netstat 采集必须让监听项显式已测 (不得是未测): {item}"
    );
    let finding_count = item["finding_count"].as_u64().expect("finding_count");
    assert!(
        finding_count >= 1,
        "真实采集必须解析出至少一个监听项: {item}"
    );
    let findings = item["findings"].as_array().expect("findings 数组");
    let mut ports = 0usize;
    for finding in findings {
        let surface = finding["surface"].as_str().unwrap_or_default();
        let Some(port) = surface_port(surface) else {
            continue;
        };
        assert!(
            (1..=65535).contains(&port),
            "发现条目里的端口号必须合法: {surface}"
        );
        ports += 1;
    }
    assert!(ports >= 1, "至少一条发现必须带合法端口: {item}");

    // 行级真值 (独立于被测解析器): 真实输出里的 TCP 监听行必须真的出现在工具面
    // 发现里 —— 否则「发现数 >= 1」会被 UDP 行兜住, 关掉 TCP 监听判定也照样绿。
    let capture = capture_netstat().expect("真实采集必须跑通 (用于行级真值)");
    let tcp_rows = tcp_listening_ports(&capture.stdout);
    assert!(
        !tcp_rows.is_empty(),
        "真实输出必须含至少一条 TCP 监听行 (否则本条行级断言空转)"
    );
    let tcp_surfaces = findings
        .iter()
        .filter(|finding| {
            finding["surface"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .starts_with("tcp ")
        })
        .count();
    assert!(
        tcp_surfaces >= 1,
        "真实 TCP 监听行必须在工具面发现里出现 (不得只剩 UDP 行): {item}"
    );
}

#[cfg(not(windows))]
#[test]
fn non_windows_capture_runs_the_child_but_the_parse_shape_is_not_verified() {
    let Some(path) = trusted_netstat_path() else {
        eprintln!(
            "netstat-e2e: SKIP reason=trusted system netstat unavailable on this host (采集显式不可用)"
        );
        assert!(
            matches!(
                capture_netstat(),
                Err(ListenerProbeError::Unavailable { .. })
            ),
            "缺系统工具 ⇒ 采集必须显式不可用 (不静默通过)"
        );
        return;
    };
    assert!(path.is_file(), "可信路径必须指向真实文件");
    let capture = match capture_netstat() {
        Ok(capture) => capture,
        Err(ListenerProbeError::Failed { reason }) if reason.contains("truncated") => {
            eprintln!(
                "netstat-e2e: SKIP reason=真实输出超过 {NETSTAT_MAX_STDOUT_BYTES} 字节上限, 采集按上限显式失败 (上限生效, 本次未取得快照): {reason}"
            );
            return;
        }
        Err(error) => panic!("受控采集必须真实跑通: {error}"),
    };
    assert_eq!(capture.exit_code, Some(0), "真实子进程退出码必须是 0");
    assert!(!capture.stdout_truncated, "标准输出不得被截断");
    assert_eq!(
        capture.max_runtime, NETSTAT_MAX_RUNTIME,
        "必须钉住 5 秒时限"
    );
    assert!(
        capture.elapsed < capture.max_runtime,
        "采集耗时必须落在时限内: {:?} / {:?}",
        capture.elapsed,
        capture.max_runtime
    );
    // Linux / macOS 的 netstat 输出形状本仓未实测 ⇒ 只记录不宣称解析已验证。
    let snapshot = parse_netstat_snapshot(&capture.stdout);
    eprintln!(
        "netstat-e2e: NOT-VERIFIED 平台=非 Windows; 子进程真实跑通, 但本仓未实测该平台的解析形状 (解析项 {} / {})",
        snapshot.listeners.len(),
        scope_histogram(&snapshot)
    );
}

/// 监听作用域分布 (事实记录, 不做跨组相对比较)。
fn scope_histogram(
    snapshot: &apeireth_tools_canonical::exposure_audit::ListenerSnapshot,
) -> String {
    let loopback = snapshot
        .listeners
        .iter()
        .filter(|listener| listener.scope == ListenerScope::Loopback)
        .count();
    let wildcard = snapshot
        .listeners
        .iter()
        .filter(|listener| listener.scope == ListenerScope::Wildcard)
        .count();
    let interface = snapshot
        .listeners
        .iter()
        .filter(|listener| listener.scope == ListenerScope::Interface)
        .count();
    format!("回环 {loopback} / 全接口 {wildcard} / 指定接口 {interface}")
}

/// 工具面的发现词面形如 `tcp 0.0.0.0:135 (PID 1024)` ⇒ 取端口; 不是端口型
/// 词面 (聚合条目 / 解析失败条目) 返回 `None`。
#[cfg(windows)]
fn surface_port(surface: &str) -> Option<u16> {
    let (_, tail) = surface.rsplit_once(':')?;
    let digits: String = tail
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// 行级真值: 直接从系统表输出里数出 TCP 监听行 (协议 + 本地端口)。
///
/// 这是**独立于被测解析器**的判据: 解析器若把 TCP 监听行判漏, 这里仍数得出来,
/// 两者必须对得上 (断言不会因为 UDP 行兜底而恒绿)。
#[cfg(windows)]
fn tcp_listening_ports(stdout: &str) -> Vec<(String, u16)> {
    let mut ports = Vec::new();
    for line in stdout.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(proto) = tokens.first() else {
            continue;
        };
        let proto = proto.to_ascii_lowercase();
        if !proto.starts_with("tcp") {
            continue;
        }
        if !tokens.iter().any(|token| {
            token.eq_ignore_ascii_case("listening") || token.eq_ignore_ascii_case("listen")
        }) {
            continue;
        }
        let Some(local) = tokens.get(1) else {
            continue;
        };
        let Some((_, port)) = local.rsplit_once(':') else {
            continue;
        };
        let Ok(port) = port.parse::<u16>() else {
            continue;
        };
        ports.push((proto, port));
    }
    ports
}
