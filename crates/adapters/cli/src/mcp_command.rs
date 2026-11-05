//! 外部工具（协议桥）主人管理面: `apeireth mcp list|deny|undeny|authorize|unauthorize`。
//!
//! # 面是什么
//!
//! 主人可见的**本地运维出口**: 看一眼外部工具的配置面 (哪些服务器、封了哪些
//! 工具、哪些进了授权面), 以及把某个外部工具**拉黑 / 解封 / 放进授权面 /
//! 移出授权面**。落点是**既有**存储文档 `mcp-servers.json` 的 `deny_tools`
//! 与 `auto_authorize_tools` 两个字段 —— **零新持久化格式、零双写者**
//! (与宿主读的是同一份文件、同一套字段)。
//!
//! # 口径 (0 装诚实)
//!
//! * **改配置 ≠ 当场生效**: `deny_tools` 是桥接**装配期**的播种源 (桥构造时把
//!   该列表折进单调封禁闸)。CLI 改的是**文件** ⇒ 生效点是**宿主下次启动**。
//!   本面输出逐字写明「需重启生效」, **不**声称当场生效。
//! * **只读面零写**: `list` 只读配置面; `--live` 才去探一次服务器 (只读探测,
//!   逐服务器隔离, 不可达如实记为不可达)。
//! * **拒开语义**: 配置文件坏了 / 身份不符 ⇒ 拒 (不覆盖、不回退默认)。
//! * **白名单 ≠ 免人批**: `auto_authorize_tools` 回答的是「这个能力**在不在
//!   授权面里**」, 不是「会不会问人」。桥内播种的形状是「显式 grant **+**
//!   逐次人批标记」, 外部工具族的**族审批地板**让每次调用都停人批 (`full`
//!   预设也不豁免) ⇒ 主人不得从「auto」二字推出「没有人问」。本面的输出
//!   逐字写明这一点。
//!
//! # 两个名单的**名面一致**
//!
//! `deny_tools` 与 `auto_authorize_tools` 记的都是**服务器侧**工具名
//! (目录里的 `remote_name`), 不是模型面名 (`mcp:<server>:<tool>`) ⇒ 四个
//! 写动词的参数文法**完全相同** (`<server>.<tool>`, 同一套名校验函数)。
//!
//! # 两个名单的**优先级**: 封禁胜出
//!
//! 封禁面是**单调闸**: 被封的工具根本不进动态目录 ⇒ 它永远不会落进
//! `auto_authorize_tools` 的授权面, 族授权也救不回来。本面因此在两个名单
//! 同时点名一个工具时**写明**「封禁胜出」(两个名单本身互不干扰: 改一个
//! 不动另一个)。
//!
//! # 为什么不做「运行时当场拉黑」
//!
//! 库 API 有运行时封禁 (`McpToolDenyGuard::deny_capability`: 下一轮刷新即从
//! 目录里撤掉并撤销登记), 但 CLI 是**一次性进程**: 命令跑完进程即退 ⇒ 运行时
//! 改动随进程消失, **当场生效在这条路上没有承载体** (除非把 CLI 做成常驻守护
//! 命令 —— 那要回答「谁持有闸、怎么鉴权、怎么通知在跑的宿主」, 代价远超本批
//! 收益)。故本面走**配置面**: 落点唯一、可复核、跨进程可见; 代价是**需重启
//! 生效**, 且本面明说。
//!
//! # 并发与原子性
//!
//! 改写走**全量**: 读全量文档 → 改一处 → 全量写回 (仓内存储文档只有全量写,
//! 无增量写)。整段临界区在 `storage_atomic::with_file_lock` (锁文件
//! `<目标>.lock`, 跨线程/跨进程互斥) 下, 落盘走 `stored_doc::write_doc`
//! ⇒ 原子替换 + fsync: 并发改写不丢更新, 读者要么读到改前要么读到改后,
//! 不见半写。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::storage_atomic;
use apeireth_core::stored_doc::{self, StoredDoc};
use apeireth_governance::PermissionPolicy;
use apeireth_plugin::mcp::is_valid_mcp_name;
use apeireth_tools_canonical::mcp_bridge::catalog::capability_id_for;
use apeireth_tools_canonical::mcp_bridge::config::{
    doc_compat, is_valid_server_name, server_list_path, McpBridgeOptions, McpServerConfig,
    McpServerSpec, MCP_SERVERS_ENV,
};
use apeireth_tools_canonical::mcp_bridge::{McpBridgeError, McpToolBridge};

use crate::flags;

/// 本面的用法帧 (错误即帧: 参数错 = 带用法的正式通道)。
pub const USAGE: &str = "Usage:\n  apeireth mcp list [--live]\n  apeireth mcp deny <server>.<tool>\n  apeireth mcp undeny <server>.<tool>\n  apeireth mcp authorize <server>.<tool>\n  apeireth mcp unauthorize <server>.<tool>";

/// 只读探测的时限 (逐服务器隔离之外的**总**时限: 探不通就如实说探不通)。
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 本面的动词。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpVerb {
    /// 只读列表面。
    List,
    /// 把工具拉黑 (写配置面)。
    Deny,
    /// 把工具从封禁名单里摘掉 (写配置面)。
    Undeny,
    /// 把工具放进授权白名单 (写配置面)。
    Authorize,
    /// 把工具从授权白名单里摘掉 (写配置面)。
    Unauthorize,
}

impl McpVerb {
    /// 动词名 (用法帧与错误行用)。
    fn name(self) -> &'static str {
        match self {
            McpVerb::List => "list",
            McpVerb::Deny => "deny",
            McpVerb::Undeny => "undeny",
            McpVerb::Authorize => "authorize",
            McpVerb::Unauthorize => "unauthorize",
        }
    }

    /// 命令路径 (错误帧点名用): `mcp list` / `mcp deny` / `mcp undeny` /
    /// `mcp authorize` / `mcp unauthorize`。
    fn command_path(self) -> &'static str {
        match self {
            McpVerb::List => "mcp list",
            McpVerb::Deny => "mcp deny",
            McpVerb::Undeny => "mcp undeny",
            McpVerb::Authorize => "mcp authorize",
            McpVerb::Unauthorize => "mcp unauthorize",
        }
    }

    /// 逐动词旗标白名单 —— **走 crate 级共享判据** ([`crate::flags`]),
    /// 本面零自写扫描。
    fn spec(self) -> flags::FlagSpec {
        match self {
            // `--live` 是布尔旗标: 不带值、不吞后面的词。
            McpVerb::List => flags::FlagSpec {
                value: &[],
                boolean: &["--live"],
                meta_as_positional: false,
            },
            // 四个写动词只收一个位置参数 `<server>.<tool>`。
            McpVerb::Deny | McpVerb::Undeny | McpVerb::Authorize | McpVerb::Unauthorize => {
                flags::FlagSpec {
                    value: &[],
                    boolean: &[],
                    meta_as_positional: false,
                }
            }
        }
    }
}

/// `apeireth mcp` 入口 (数据根取自 env)。
pub fn run_mcp(args: &[String]) -> Result<String, String> {
    run_mcp_at(&crate::default_panel_data_dir(), args)
}

/// 在指定数据目录执行管理命令 (测试与运维同入口; 语义与 [`run_mcp`] 逐字一致)。
pub fn run_mcp_at(data_dir: &Path, args: &[String]) -> Result<String, String> {
    let Some((raw_verb, rest)) = args.split_first() else {
        return Err(format!("mcp requires a subcommand\n{USAGE}"));
    };
    let verb = match raw_verb.as_str() {
        "list" => McpVerb::List,
        "deny" => McpVerb::Deny,
        "undeny" => McpVerb::Undeny,
        "authorize" => McpVerb::Authorize,
        "unauthorize" => McpVerb::Unauthorize,
        other => {
            return Err(format!(
                "unknown mcp command: {other} (expected list, deny, undeny, authorize, or \
                 unauthorize)\n{USAGE}"
            ))
        }
    };
    let spec = verb.spec();
    let known = spec.known();
    let context = flags::FrameContext::new(verb.command_path(), flags::UnknownNoun::Flag)
        .with_usage(USAGE)
        .with_known(&known);
    let scanned =
        flags::scan(rest, &spec).map_err(|error| error.frame(flags::Style::En, &context))?;
    match verb {
        McpVerb::List => {
            // 本动词不收位置参数 (只读列表面, 没有「多余的话」这一说)。
            if let Some(error) = scanned.extra_positional() {
                return Err(error.frame(flags::Style::En, &context));
            }
            list_servers(data_dir, scanned.has("--live"))
        }
        McpVerb::Deny => write_target(data_dir, &scanned, verb, Mutation::Add, ToolList::Deny),
        McpVerb::Undeny => write_target(data_dir, &scanned, verb, Mutation::Remove, ToolList::Deny),
        McpVerb::Authorize => {
            write_target(data_dir, &scanned, verb, Mutation::Add, ToolList::Allow)
        }
        McpVerb::Unauthorize => {
            write_target(data_dir, &scanned, verb, Mutation::Remove, ToolList::Allow)
        }
    }
}

/// 四个写动词共用的入口: 取位置参数 → 拆目标 → 改写名单。
///
/// 四个动词的参数文法**逐字相同** (`<server>.<tool>`) —— 两个名单记的
/// 都是服务器侧工具名 ⇒ 同一套名校验, 同一个拆法。
fn write_target(
    data_dir: &Path,
    scanned: &flags::Scan,
    verb: McpVerb,
    mutation: Mutation,
    list: ToolList,
) -> Result<String, String> {
    let target = match scanned.positionals.as_slice() {
        [one] => one.clone(),
        [] => {
            return Err(format!(
                "mcp {} requires <server>.<tool>\n{USAGE}",
                verb.name()
            ))
        }
        many => {
            return Err(format!(
                "mcp {} takes exactly one <server>.<tool> (got {})\n{USAGE}",
                verb.name(),
                many.len()
            ))
        }
    };
    let (server, tool) = split_target(&target)?;
    mutate_tool_list(data_dir, &server, &tool, mutation, list)
}

/// 拆 `<server>.<tool>`: 服务器名与工具名各按**自己的**文法校验 (fail-closed)。
///
/// 工具名是**服务器侧**的名字 (目录里的 `remote_name`) —— `deny_tools` 与
/// `auto_authorize_tools` 两个名单**同一个名面**, 所以封禁面与授权面共用这
/// 一处拆法与这一套名校验 (模型面名 `mcp:<server>:<tool>` 不是本面的入参)。
///
/// 能力 id 由 `capability_id_for` 合成 (`tool.mcp.<server>.<tool>`) —— 与动态
/// 目录同一处合成器, 封禁面与目录面不会对同一身份说法不一。
fn split_target(raw: &str) -> Result<(String, String), String> {
    let Some((server, tool)) = raw.split_once('.') else {
        return Err(format!(
            "mcp target must be <server>.<tool>: {raw}\n{USAGE}"
        ));
    };
    if !is_valid_server_name(server) {
        return Err(format!(
            "mcp target server name is invalid: {server:?} (grammar: [a-z0-9][a-z0-9_-]*)\n{USAGE}"
        ));
    }
    if !is_valid_mcp_name(tool) {
        return Err(format!(
            "mcp target tool name is invalid: {tool:?}\n{USAGE}"
        ));
    }
    capability_id_for(server, tool)
        .map_err(|error| format!("mcp target is invalid: {error}\n{USAGE}"))?;
    Ok((server.to_string(), tool.to_string()))
}

/// 配置面的来源 (env 入口优先于文件入口 —— 与桥接装载口径同源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigSource {
    /// env 变量入口 (`APEIRETH_MCP_SERVERS`)。
    Env,
    /// 数据目录里的存储文档入口 (`mcp-servers.json`)。
    File,
    /// 没有任何入口 (空配置)。
    None,
}

/// 读配置面: env 入口优先, 文件入口次之, 都没有 = 空配置 (不是默认服务器)。
fn load_config(data_dir: &Path) -> Result<(ConfigSource, McpServerConfig), String> {
    let from_env = McpServerConfig::load_from_env()
        .map_err(|error| format!("external server configuration refused to open: {error}"))?;
    if let Some(config) = from_env {
        return Ok((ConfigSource::Env, config));
    }
    let from_file = McpServerConfig::load_from_data_dir(data_dir)
        .map_err(|error| format!("external server configuration refused to open: {error}"))?;
    match from_file {
        Some(config) => Ok((ConfigSource::File, config)),
        None => Ok((ConfigSource::None, McpServerConfig::default())),
    }
}

/// `list` 动词: 只读列配置面 (`--live` 时附一次只读探测)。
fn list_servers(data_dir: &Path, live: bool) -> Result<String, String> {
    let (source, config) = load_config(data_dir)?;
    let probe = if live {
        Some(probe_live(&config)?)
    } else {
        None
    };
    let mut lines = Vec::with_capacity(config.servers.len() * 4 + 2);
    match source {
        ConfigSource::Env => lines.push(format!("config source: env ({MCP_SERVERS_ENV})")),
        ConfigSource::File => lines.push(format!(
            "config source: file {}",
            server_list_path(data_dir).display()
        )),
        ConfigSource::None => {
            lines.push("config source: none (no external server configured)".to_string())
        }
    }
    if config.servers.is_empty() {
        lines.push("servers: 0".to_string());
    }
    for spec in &config.servers {
        lines.push(format!(
            "server {} enabled={} transport={} target={}",
            spec.name,
            spec.enabled,
            spec.transport.as_str(),
            spec.redacted_target()
        ));
        lines.push(format!("  deny_tools: {}", list_or_dash(&spec.deny_tools)));
        lines.push(format!(
            "  auto_authorize_tools: {}",
            list_or_dash(&spec.auto_authorize_tools)
        ));
        match &probe {
            Some(probe) => {
                let live_tools = probe.get(&spec.name).cloned().unwrap_or_default();
                lines.push(format!(
                    "  live tools: {}",
                    if live_tools.is_empty() {
                        if spec.enabled {
                            "none".to_string()
                        } else {
                            "not probed (server disabled)".to_string()
                        }
                    } else {
                        live_tools.join(", ")
                    }
                ));
            }
            None => lines
                .push("  live tools: not probed (pass --live to probe the servers)".to_string()),
        }
    }
    if probe.is_some() {
        lines.push(
            "live tools: probed from this process (the running host keeps its own registered set; \
             denied tools never enter the probe)"
                .to_string(),
        );
    }
    Ok(lines.join("\n"))
}

/// 只读探测: 逐服务器刷新动态工具目录 (每服务器隔离: 不可达不牵连别的)。
///
/// 探测到的是**这个进程**看到的目录快照, 不是在跑宿主已登记的那一份 (宿主那份
/// 只在宿主自己的日志里可见) —— 本面不混说。
fn probe_live(config: &McpServerConfig) -> Result<BTreeMap<String, Vec<String>>, String> {
    let options = McpBridgeOptions::from_env();
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    let bridge = McpToolBridge::new_with_policy(config.clone(), options, policy)
        .map_err(|error| format!("external tool bridge refused to open: {error}"))?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("failed to create Tokio runtime: {error}"))?;
    let outcome = runtime.block_on(async {
        tokio::time::timeout(PROBE_TIMEOUT, bridge.refresh_outcome())
            .await
            .map_err(|_| "probe timed out".to_string())
    });
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(reason) => {
            let mut per_server: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for spec in &config.servers {
                if spec.enabled {
                    per_server.insert(spec.name.clone(), vec![format!("unreachable ({reason})")]);
                }
            }
            return Ok(per_server);
        }
    };
    let mut per_server: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in &outcome.report.live {
        per_server
            .entry(entry.server.clone())
            .or_default()
            .push(format!(
                "{} (tool={}, read_only={})",
                entry.capability_id.as_str(),
                entry.remote_name,
                entry.declared_read_only
            ));
    }
    for failure in &outcome.failures {
        let server = match failure {
            McpBridgeError::Connect { server, .. } => server.clone(),
            other => other.to_string(),
        };
        per_server
            .entry(server)
            .or_default()
            .push(format!("unreachable ({failure})"));
    }
    Ok(per_server)
}

/// 错误即帧: 本面的失败一律带用法帧 (参数错 = 带用法的正式通道)。
fn refuse(reason: impl std::fmt::Display) -> String {
    format!("{reason}\n{USAGE}")
}

/// 名单的改动方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mutation {
    /// 加进名单 (拉黑 / 放进授权面)。
    Add,
    /// 摘出名单 (解封 / 移出授权面)。
    Remove,
}

/// 名单字段族: 封禁黑名单 / 授权白名单 (同一份配置、同一个 spec 的两个字段)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolList {
    /// `deny_tools` (封禁黑名单): 拉黑 / 解封。
    Deny,
    /// `auto_authorize_tools` (授权白名单): 放进授权面 / 移出授权面。
    Allow,
}

/// 授权白名单的**语义提醒句** (白名单 ≠ 免人批)。
///
/// 桥内播种的形状是「显式 grant **+** 逐次人批标记」, 外部工具族的**族审批
/// 地板**让每次调用都停人批 (`full` 预设也不豁免) ⇒ 加进白名单只回答
/// 「在不在授权面里」, 不回答「会不会问人」。这句话逐字进输出, 主人不得从
/// 「auto」二字推出「没有人问」。
const ALLOW_SEMANTICS_NOTE: &str = "allowlist entry ≠ approval-free: the tool enters the \
authorization surface, but every call still waits for a human (逐次人批); the full preset does \
not exempt it either";

impl ToolList {
    /// 字段名 (输出行用)。
    fn field(self) -> &'static str {
        match self {
            ToolList::Deny => "deny_tools",
            ToolList::Allow => "auto_authorize_tools",
        }
    }

    /// 该字段的读面 (两个字段**同一个名面**: 服务器侧工具名)。
    fn entries(self, spec: &McpServerSpec) -> &[String] {
        match self {
            ToolList::Deny => &spec.deny_tools,
            ToolList::Allow => &spec.auto_authorize_tools,
        }
    }

    /// 该字段的写面。
    fn entries_mut(self, spec: &mut McpServerSpec) -> &mut Vec<String> {
        match self {
            ToolList::Deny => &mut spec.deny_tools,
            ToolList::Allow => &mut spec.auto_authorize_tools,
        }
    }

    /// 另一个名单 (交叉口径): 封禁面罩着这个工具吗?
    ///
    /// 封禁闸是**单调**的: 被封的工具根本不进动态目录 ⇒ 它永远不会落进授权
    /// 面 ⇒ 封禁胜出, 族授权也救不回来。
    fn shadowed_by_other(self, spec: &McpServerSpec, tool: &str) -> bool {
        match self {
            ToolList::Deny => spec.auto_authorize_tools.iter().any(|name| name == tool),
            ToolList::Allow => spec.deny_tools.iter().any(|name| name == tool),
        }
    }

    /// 落盘后的完成句。
    fn added_message(self, capability: &str) -> String {
        match self {
            ToolList::Deny => format!("denied {capability} (external server list updated)"),
            ToolList::Allow => format!("authorized {capability} (external server list updated)"),
        }
    }

    /// 幂等句: 已经在名单里 ⇒ **一个字节都没写**。
    fn already_message(self, capability: &str) -> String {
        match self {
            ToolList::Deny => format!("already denied: {capability} (no change written)"),
            ToolList::Allow => format!("already authorized: {capability} (no change written)"),
        }
    }

    /// 摘除后的完成句。
    fn removed_message(self, capability: &str) -> String {
        match self {
            ToolList::Deny => format!("un-denied {capability} (external server list updated)"),
            ToolList::Allow => format!("un-authorized {capability} (external server list updated)"),
        }
    }

    /// 幂等句: 本就不在名单里 ⇒ **一个字节都没写**。
    fn absent_message(self, capability: &str) -> String {
        match self {
            ToolList::Deny => format!("not denied: {capability} (no change written)"),
            ToolList::Allow => format!("not authorized: {capability} (no change written)"),
        }
    }

    /// 收尾句的后半截 (宿主侧不变的是什么)。
    fn unchanged_note(self) -> &'static str {
        match self {
            ToolList::Deny => "the running host's deny set is unchanged",
            ToolList::Allow => "the running host's authorization surface is unchanged",
        }
    }

    /// 交叉提醒 (两个名单同时点名一个工具时): 封禁胜出。
    fn shadow_note(self, server: &str, tool: &str) -> String {
        match self {
            ToolList::Deny => format!(
                "note: {server}.{tool} is also on the allowlist (auto_authorize_tools) — the deny \
                 list wins, so the tool stays blocked"
            ),
            ToolList::Allow => format!(
                "note: {server}.{tool} is also on the deny list (deny_tools) — the deny list wins, \
                 so the tool stays blocked"
            ),
        }
    }
}

/// 一次改写的结果: 改了 (落盘) 还是没改 (零写), 外加两个名单的交叉情况。
#[derive(Debug, Clone, PartialEq, Eq)]
struct MutationOutcome {
    /// 目标字段改写后的名单。
    entries: Vec<String>,
    /// 是否真改了 (false = **一个字节都没写**)。
    changed: bool,
    /// 另一个名单是否也罩着这个工具 (封禁胜出的交叉口径)。
    shadowed: bool,
}

/// 改写一个名单字段 (读全量 → 改一处 → 全量写回, 临界区在文件锁下)。
///
/// 两个名单共用这一条临界区、同一把锁、同一个原子写 ⇒ 同一份文件上的
/// 两次改写不会互相踩 (两个名单的条目**互不干扰**: 改一个不动另一个)。
fn mutate_tool_list(
    data_dir: &Path,
    server: &str,
    tool: &str,
    mutation: Mutation,
    list: ToolList,
) -> Result<String, String> {
    // env 入口在场时, 宿主读的是 env —— 改文件不会有任何效果 ⇒ 拒 (fail-closed,
    // 不写一份永远不被读的名单)。
    if env_entry_present() {
        return Err(refuse(format!(
            "{MCP_SERVERS_ENV} is set: the host reads that entry instead of the server list \
             file, so editing the file would have no effect (unset the variable first)"
        )));
    }
    let path = server_list_path(data_dir);
    let present = fs_err::exists(&path)
        .map_err(|error| refuse(format!("external server list unreadable: {error}")))?;
    if !present {
        return Err(refuse(format!(
            "no external server list at {} (nothing to change)",
            path.display()
        )));
    }
    let lock_path = storage_atomic::lock_path_for(&path);
    let outcome =
        storage_atomic::with_file_lock(&lock_path, || -> Result<MutationOutcome, String> {
            let mut doc: StoredDoc<McpServerConfig> = stored_doc::open_single(&path, &doc_compat())
                .map_err(|error| {
                    refuse(format!("external server list refused to open: {error}"))
                })?;
            doc.body.validate().map_err(|error| {
                refuse(format!("external server list refused to open: {error}"))
            })?;
            let known: Vec<String> = doc
                .body
                .servers
                .iter()
                .map(|spec| spec.name.clone())
                .collect();
            let Some(spec) = doc.body.servers.iter_mut().find(|spec| spec.name == server) else {
                return Err(refuse(format!(
                    "unknown external server {server:?} (configured: {})",
                    known.join(", ")
                )));
            };
            let changed = match mutation {
                Mutation::Add => {
                    let entries = list.entries_mut(spec);
                    if entries.iter().any(|name| name == tool) {
                        false
                    } else {
                        entries.push(tool.to_string());
                        true
                    }
                }
                Mutation::Remove => {
                    let entries = list.entries_mut(spec);
                    let before = entries.len();
                    entries.retain(|name| name != tool);
                    before != entries.len()
                }
            };
            let entries = list.entries(spec).to_vec();
            let shadowed = list.shadowed_by_other(spec, tool);
            if !changed {
                // 幂等路径: **一个字节都没写** (只报状态)。
                return Ok(MutationOutcome {
                    entries,
                    changed: false,
                    shadowed,
                });
            }
            stored_doc::write_doc(&path, &doc, stored_doc::DEFAULT_DOC_MODE)
                .map_err(|error| refuse(format!("external server list write failed: {error}")))?;
            Ok(MutationOutcome {
                entries,
                changed: true,
                shadowed,
            })
        })
        .map_err(|error| refuse(format!("external server list lock failed: {error}")))?;
    let outcome = outcome?;
    let capability = capability_id_for(server, tool)
        .map(|id| id.as_str().to_string())
        .unwrap_or_else(|_| format!("{server}.{tool}"));
    let mut lines = Vec::with_capacity(5);
    match (outcome.changed, mutation) {
        (true, Mutation::Add) => lines.push(list.added_message(&capability)),
        (false, Mutation::Add) => lines.push(list.already_message(&capability)),
        (true, Mutation::Remove) => lines.push(list.removed_message(&capability)),
        (false, Mutation::Remove) => lines.push(list.absent_message(&capability)),
    }
    lines.push(format!(
        "  server {server} {}: {}",
        list.field(),
        list_or_dash(&outcome.entries)
    ));
    if outcome.shadowed {
        lines.push(list.shadow_note(server, tool));
    }
    if let ToolList::Allow = list {
        // 白名单面**必须**带这句: 加进白名单 ≠ 免人批 (外部工具族逐次人批)。
        lines.push(ALLOW_SEMANTICS_NOTE.to_string());
    }
    lines.push(format!(
        "takes effect after the host restarts (需重启生效) — {}",
        list.unchanged_note()
    ));
    Ok(lines.join("\n"))
}

/// env 入口是否在场 (在场 = 宿主读 env, 文件入口是死的)。
fn env_entry_present() -> bool {
    std::env::var(MCP_SERVERS_ENV).is_ok_and(|raw| !raw.trim().is_empty())
}

/// 名单的打印形态 (空名单 = `-`, 不留空尾)。
fn list_or_dash(names: &[String]) -> String {
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// env 改动的串行闸 (同进程内 env 读改不是线程安全的 ⇒ 本模块的 env 用例
    /// 一律持闸, 避免与并行用例互相看见)。
    static ENV_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn env_gate() -> std::sync::MutexGuard<'static, ()> {
        ENV_GATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn target_split_checks_server_and_tool_grammars() {
        let (server, tool) = split_target("demo.lookup-item").expect("valid target");
        assert_eq!(server, "demo");
        assert_eq!(tool, "lookup-item");
        for bad in [
            "demo",             // 缺工具名
            "Demo.lookup",      // 服务器名大写
            "demo.Lookup",      // 工具名大写
            "demo.lookup item", // 工具名带空格
            "demo..lookup",     // 工具名为空
        ] {
            assert!(split_target(bad).is_err(), "必须拒绝: {bad}");
        }
    }

    #[test]
    fn env_entry_in_the_room_makes_file_edits_refuse() {
        let _guard = env_gate();
        std::env::set_var(
            MCP_SERVERS_ENV,
            r#"[{"name":"demo","transport":"stdio","command":"x","enabled":true}]"#,
        );
        let dir = std::env::temp_dir().join(format!("apeireth-mcp-env-{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        let error = mutate_tool_list(&dir, "demo", "lookup", Mutation::Add, ToolList::Deny)
            .expect_err("env 在场时改文件必须拒");
        assert!(error.contains(MCP_SERVERS_ENV), "{error}");
        std::env::remove_var(MCP_SERVERS_ENV);
        let _ = fs_err::remove_dir_all(&dir);
    }

    /// env 入口在场时**授权白名单**的写面同样拒 (与封禁面同纪律):
    /// 宿主读 env ⇒ 改文件没有效果 ⇒ 不写一份永远不被读的名单。
    #[test]
    fn env_entry_in_the_room_makes_allow_list_edits_refuse() {
        let _guard = env_gate();
        std::env::set_var(
            MCP_SERVERS_ENV,
            r#"[{"name":"demo","transport":"stdio","command":"x","enabled":true}]"#,
        );
        let dir =
            std::env::temp_dir().join(format!("apeireth-mcp-env-allow-{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        let error = mutate_tool_list(&dir, "demo", "lookup", Mutation::Add, ToolList::Allow)
            .expect_err("env 在场时改文件必须拒");
        assert!(error.contains(MCP_SERVERS_ENV), "{error}");
        std::env::remove_var(MCP_SERVERS_ENV);
        let _ = fs_err::remove_dir_all(&dir);
    }
}
