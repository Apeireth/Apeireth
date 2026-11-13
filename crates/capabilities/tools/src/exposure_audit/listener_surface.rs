//! 本地数据面自查 · 项三: 开放端口面 (本机监听枚举, 受控采集)。
//!
//! 防御语自查: 本机**监听**面是自己暴露了什么的第一张网络清单。采集口径 =
//! 受控系统表快照 (`netstat` 固定参数, 无用户输入拼接); 解析器只认监听态
//! (TCP `LISTENING`/`LISTEN`) 与已绑定 UDP, 连接态行不算监听面 (防误报)。
//! 诚实口径: 只列本机监听枚举; 入站可达性、外网暴露面**未测**。
//!
//! 可执行文件口径: 受控采集只走 OS 自带系统工具目录里的那一个文件
//! ([`trusted_netstat_path`]), 路径先过形状闸门 ([`validate_trusted_path_shape`])
//! 再判归属 ([`resolve_trusted_tool_path`]); 找不到 ⇒ 采集显式未测, 不猜路径。
//!
//! 采集事实口: [`capture_netstat`] 返回一次**真实**采集的执行事实 (退出码、
//! 截断标记、实际生效的时限与输出上限), [`probe_netstat_outcome`] 只取其中的
//! 输出文本 —— 审计与测试据此核对「真实那一次」, 不从错误文案反推。
//!
//! 环境口径: 子进程环境**清空**, 只按白名单显式给系统工具自己需要的系统路径
//! 变量 (Windows: `SystemRoot`) —— 系统表工具在环境全空时会以退出码 0 结束却
//! 静默省略地址列, 退出码不能当监听面的证据。

use crate::exposure_audit::report::{ExposureItem, Finding, RiskLevel};
use crate::process::{ProcessExecutor, ProcessLimits, ProcessRequest};
#[cfg(windows)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 单个体检项的发现条数上限 (超出部分聚合)。
const MAX_FINDINGS: usize = 50;

/// 受控采集的墙钟时限: 超时即终止子进程并判失败 (不是无限等)。
pub const NETSTAT_MAX_RUNTIME: Duration = Duration::from_secs(5);

/// 受控采集的标准输出上限 (字节): 触到即截断并判失败。
pub const NETSTAT_MAX_STDOUT_BYTES: usize = 256 * 1024;

/// 受控采集的标准错误上限 (字节): 触到即截断并判失败。
pub const NETSTAT_MAX_STDERR_BYTES: usize = 16 * 1024;

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

/// 受控采集失败的分类（结构化错误：测试可判真，不靠错误文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenerProbeError {
    /// 调用级取消：采集**未开始**，没有子进程被拉起。
    Cancelled,
    /// 调用级期限已到：采集**未开始**，没有子进程被拉起。
    DeadlineExceeded,
    /// 平台缺少可信系统工具：采集**不可用**，不猜路径。
    Unavailable {
        /// 不可用原因。
        reason: String,
    },
    /// 采集已启动但**未完成**（启动失败 / 超时 / 输出截断 / 非零退出码）。
    Failed {
        /// 失败原因。
        reason: String,
    },
}

impl std::fmt::Display for ListenerProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(formatter, "受控采集未开始: call cancelled"),
            Self::DeadlineExceeded => write!(formatter, "受控采集未开始: call deadline exceeded"),
            Self::Unavailable { reason } => write!(formatter, "受控采集不可用: {reason}"),
            Self::Failed { reason } => write!(formatter, "{reason}"),
        }
    }
}

/// 受控采集: 固定参数系统表快照 (无用户输入拼接; 失败即帧, 不猜)。
///
/// 错误分类见 [`ListenerProbeError`]：调用级治理终止 ⇒ 未开始；平台无该系统工具
/// ⇒ 不可用；已启动但未完成 ⇒ 失败。三者互斥，调用方可按类处理。
pub fn probe_netstat() -> Result<String, String> {
    probe_netstat_outcome().map_err(|error| error.to_string())
}

/// 一次**真实**系统表采集的执行事实 (可核验记录)。
///
/// 字段全部取自**实际那一次**子进程执行 (退出码、截断标记取自执行结果;
/// 时限与输出上限取自真实请求), 不是配置文档上的抄写 —— 审计与测试据此
/// 核对「真实那一次」, 不从错误文案反推。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetstatCapture {
    /// 子进程标准输出原文 (UTF-8 有损转写)。
    pub stdout: String,
    /// 子进程退出码 (`None` = 被信号终止)。
    pub exit_code: Option<i32>,
    /// 标准输出是否触到上限被截断。
    pub stdout_truncated: bool,
    /// 标准错误是否触到上限被截断。
    pub stderr_truncated: bool,
    /// 本次采集的实际耗时。
    pub elapsed: Duration,
    /// 本次采集实际生效的墙钟时限。
    pub max_runtime: Duration,
    /// 本次采集实际生效的标准输出上限 (字节)。
    pub max_stdout_bytes: usize,
    /// 本次采集实际生效的标准错误上限 (字节)。
    pub max_stderr_bytes: usize,
}

/// 受控采集的结构化入口: 返回一次**真实**采集的执行事实。
///
/// 与 [`probe_netstat_outcome`] 同一条路径, 但额外带回退出码、截断标记与实际
/// 生效的时限/输出上限。错误分类见 [`ListenerProbeError`]。
pub fn capture_netstat() -> Result<NetstatCapture, ListenerProbeError> {
    if let Some(context) = apeireth_plugin::tool_execution::native_execution_context() {
        if context.cancellation.is_cancelled() {
            return Err(ListenerProbeError::Cancelled);
        }
        if std::time::Instant::now() >= context.deadline {
            return Err(ListenerProbeError::DeadlineExceeded);
        }
    }
    let request = netstat_request().map_err(|reason| ListenerProbeError::Unavailable { reason })?;
    execute_capture(&request).map_err(|reason| ListenerProbeError::Failed { reason })
}

/// 受控采集的结构化入口 (同 [`probe_netstat`], 错误可判真)。
pub fn probe_netstat_outcome() -> Result<String, ListenerProbeError> {
    capture_netstat().map(|capture| capture.stdout)
}

fn netstat_request() -> Result<ProcessRequest, String> {
    let executable =
        trusted_netstat_path().ok_or_else(|| "trusted system netstat unavailable".to_string())?;
    let request = ProcessRequest::new(executable).with_limits(ProcessLimits {
        max_runtime: NETSTAT_MAX_RUNTIME,
        max_stdout_bytes: NETSTAT_MAX_STDOUT_BYTES,
        max_stderr_bytes: NETSTAT_MAX_STDERR_BYTES,
        ..ProcessLimits::default()
    });
    #[cfg(windows)]
    let request = request
        .with_args(["-ano"])
        .with_explicit_env(system_tool_environment()?);
    #[cfg(not(windows))]
    let request = request.with_args(["-an"]).with_clear_env();
    Ok(request)
}

/// 系统工具的**最小环境白名单** (Windows)。
///
/// 实测口径 (2026-11-13, Windows): 环境完全清空时, 系统 `netstat.exe -ano`
/// 仍以退出码 0 结束, 但**地址列整列为空** ⇒ 监听面被解析成"零发现" (退出码
/// 不能当证据)。只显式给 `SystemRoot` 一个系统路径变量, 地址列即恢复; 其余
/// 环境仍为空 (不把调用方的环境变量泄给子进程)。
#[cfg(windows)]
fn system_tool_environment() -> Result<Vec<(OsString, OsString)>, String> {
    let system_root = crate::process::windows::system_directory()
        .and_then(|directory| directory.parent().map(Path::to_path_buf))
        .filter(|parent| parent.is_absolute())
        .map(|parent| parent.into_os_string())
        .or_else(|| std::env::var_os("SystemRoot"))
        .ok_or_else(|| "trusted system netstat unavailable: 未知系统根目录".to_string())?;
    Ok(vec![(OsString::from("SystemRoot"), system_root)])
}

/// 受控采集用的系统工具文件名 (netstat)。
fn system_tool_file_name() -> &'static str {
    if cfg!(windows) {
        "netstat.exe"
    } else {
        "netstat"
    }
}

/// 系统工具的**信任根**: OS 自带的系统工具目录。
///
/// Windows = 系统目录 (`GetSystemDirectoryW`); unix = 标准系统工具目录
/// (取含该系统工具的那个)。根目录本身不是可信输入的产物, 由 OS 决定。
pub fn trusted_system_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        crate::process::windows::system_directory()
    }
    #[cfg(unix)]
    {
        ["/usr/bin", "/bin", "/usr/sbin"]
            .into_iter()
            .map(PathBuf::from)
            .find(|root| root.join(system_tool_file_name()).is_file())
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

/// 可信系统工具路径的**形状闸门** (纯函数, 不碰文件系统)。
///
/// 拒绝: 相对路径 · 含 `.` / `..` 路径组件 · 文件名含空白/引号/shell 元字符 ·
/// 含控制字符。剩下的路径必须由 [`resolve_trusted_tool_path`] 再判归属。
pub fn validate_trusted_path_shape(candidate: &Path) -> Result<(), String> {
    if !candidate.is_absolute() {
        return Err("可信路径无效: 必须是绝对路径".to_string());
    }
    for part in raw_path_parts(candidate) {
        if part == "." || part == ".." {
            return Err("可信路径无效: 含 . 或 .. 路径组件".to_string());
        }
    }
    let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
        return Err("可信路径无效: 缺少文件名".to_string());
    };
    if candidate
        .to_string_lossy()
        .chars()
        .any(|character| character.is_control())
    {
        return Err("可信路径无效: 含控制字符".to_string());
    }
    if !is_plain_file_name(name) {
        return Err("可信路径无效: 文件名含空白/引号/元字符".to_string());
    }
    Ok(())
}

/// 可信系统工具路径闸门: 形状合法 + 直接位于信任根内 + 是普通文件。
///
/// 归属判定用**字面**比较父目录与信任根 (不解析符号链接): 解析后的路径可能
/// 指向根外, 而字面比较只放行"根目录里那一个文件"。
pub fn resolve_trusted_tool_path(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
    validate_trusted_path_shape(candidate)?;
    let parent = candidate
        .parent()
        .ok_or_else(|| "可信路径无效: 缺少父目录".to_string())?;
    if parent != root {
        return Err("可信路径无效: 不在系统信任根目录内".to_string());
    }
    if !candidate.is_file() {
        return Err("可信路径无效: 不是普通文件".to_string());
    }
    Ok(candidate.to_path_buf())
}

/// 受控采集用的可信系统工具路径 (netstat); 已过 [`resolve_trusted_tool_path`] 闸门。
///
/// 找不到 (平台没有该系统工具) ⇒ `None` ⇒ 采集显式未测, 不旁路。
pub fn trusted_netstat_path() -> Option<PathBuf> {
    let root = trusted_system_root()?;
    let candidate = root.join(system_tool_file_name());
    resolve_trusted_tool_path(&root, &candidate).ok()
}

/// 路径的原始分段 (按两种分隔符切, 不做规范化) —— 用于判定 `.` / `..` 组件。
fn raw_path_parts(candidate: &Path) -> Vec<String> {
    candidate
        .to_string_lossy()
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

/// 可信工具文件名允许的字符集: ASCII 字母数字 + `.` `_` `-`。
///
/// (无空白、无引号、无 shell 元字符 ⇒ 路径不可能被改写成命令串。)
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

/// 真跑一次受控采集, 带回执行事实 (失败即帧: 超时/截断/非零退出码都判失败)。
fn execute_capture(request: &ProcessRequest) -> Result<NetstatCapture, String> {
    let started = std::time::Instant::now();
    let output = ProcessExecutor::new()
        .execute(request)
        .map_err(|error| format!("受控采集启动失败: {error}"))?;
    let elapsed = started.elapsed();
    let exit_code = output.exit_code();
    let stdout_truncated = output.stdout_truncated;
    let stderr_truncated = output.stderr_truncated;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.timed_out() {
        return Err("受控采集未完成: deadline exceeded".to_string());
    }
    if stdout_truncated || stderr_truncated {
        return Err("受控采集未完成: output truncated".to_string());
    }
    if !output.success() {
        return Err(format!(
            "受控采集退出码非零: {}",
            exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ));
    }
    Ok(NetstatCapture {
        stdout,
        exit_code,
        stdout_truncated,
        stderr_truncated,
        elapsed,
        max_runtime: request.limits().max_runtime,
        max_stdout_bytes: request.limits().max_stdout_bytes,
        max_stderr_bytes: request.limits().max_stderr_bytes,
    })
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
