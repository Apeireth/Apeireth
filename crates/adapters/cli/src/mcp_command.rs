//! 外部工具（协议桥）主人管理面: `apeireth mcp list|deny|undeny`。
//!
//! # 面是什么
//!
//! 主人可见的**本地运维出口**: 看一眼外部工具的配置面 (哪些服务器、封了哪些
//! 工具、哪些进了授权面), 以及把某个外部工具**拉黑 / 解封**。落点是**既有**
//! 存储文档 `mcp-servers.json` 的 `deny_tools` 字段 —— **零新持久化格式、
//! 零双写者** (与宿主读的是同一份文件、同一套字段)。
//!
//! # 口径 (0 装诚实)
//!
//! * **改配置 ≠ 当场生效**: `deny_tools` 是桥接**装配期**的播种源 (桥构造时把
//!   该列表折进单调封禁闸)。CLI 改的是**文件** ⇒ 生效点是**宿主下次启动**。
//!   本面输出逐字写明「需重启生效」, **不**声称当场生效。
//! * **只读面零写**: `list` 只读配置面; `--live` 才去探一次服务器 (只读探测,
//!   逐服务器隔离, 不可达如实记为不可达)。
//! * **拒开语义**: 配置文件坏了 / 身份不符 ⇒ 拒 (不覆盖、不回退默认)。
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
    MCP_SERVERS_ENV,
};
use apeireth_tools_canonical::mcp_bridge::{McpBridgeError, McpToolBridge};

use crate::flags;

/// 本面的用法帧 (错误即帧: 参数错 = 带用法的正式通道)。
pub const USAGE: &str = "Usage:\n  apeireth mcp list [--live]\n  apeireth mcp deny <server>.<tool>\n  apeireth mcp undeny <server>.<tool>";

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
}

impl McpVerb {
    /// 动词名 (用法帧与错误行用)。
    fn name(self) -> &'static str {
        match self {
            McpVerb::List => "list",
            McpVerb::Deny => "deny",
            McpVerb::Undeny => "undeny",
        }
    }

    /// 命令路径 (错误帧点名用): `mcp list` / `mcp deny` / `mcp undeny`。
    fn command_path(self) -> &'static str {
        match self {
            McpVerb::List => "mcp list",
            McpVerb::Deny => "mcp deny",
            McpVerb::Undeny => "mcp undeny",
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
            // `deny` / `undeny` 只收一个位置参数 `<server>.<tool>`。
            McpVerb::Deny | McpVerb::Undeny => flags::FlagSpec {
                value: &[],
                boolean: &[],
                meta_as_positional: false,
            },
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
        other => {
            return Err(format!(
                "unknown mcp command: {other} (expected list, deny, or undeny)\n{USAGE}"
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
        McpVerb::Deny | McpVerb::Undeny => {
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
            match verb {
                McpVerb::Deny => mutate_deny_list(data_dir, &server, &tool, Mutation::Add),
                _ => mutate_deny_list(data_dir, &server, &tool, Mutation::Remove),
            }
        }
    }
}

/// 拆 `<server>.<tool>`: 服务器名与工具名各按**自己的**文法校验 (fail-closed)。
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

/// 封禁名单的改动方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mutation {
    /// 追加一条 (拉黑)。
    Add,
    /// 摘掉一条 (解封)。
    Remove,
}

/// 一次改写的结果: 改了 (落盘) 还是没改 (零写)。
#[derive(Debug, Clone, PartialEq, Eq)]
enum MutationOutcome {
    /// 名单变了, 文件已写回。
    Changed(Vec<String>),
    /// 名单没变 (已在名单里 / 本就不在名单里) ⇒ **一个字节都没写**。
    Unchanged(Vec<String>),
}

/// 改写 `deny_tools` (读全量 → 改一处 → 全量写回, 临界区在文件锁下)。
fn mutate_deny_list(
    data_dir: &Path,
    server: &str,
    tool: &str,
    mutation: Mutation,
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
                    if spec.deny_tools.iter().any(|name| name == tool) {
                        false
                    } else {
                        spec.deny_tools.push(tool.to_string());
                        true
                    }
                }
                Mutation::Remove => {
                    let before = spec.deny_tools.len();
                    spec.deny_tools.retain(|name| name != tool);
                    before != spec.deny_tools.len()
                }
            };
            let list = spec.deny_tools.clone();
            if !changed {
                return Ok(MutationOutcome::Unchanged(list));
            }
            stored_doc::write_doc(&path, &doc, stored_doc::DEFAULT_DOC_MODE)
                .map_err(|error| refuse(format!("external server list write failed: {error}")))?;
            Ok(MutationOutcome::Changed(list))
        })
        .map_err(|error| refuse(format!("external server list lock failed: {error}")))?;
    let outcome = outcome?;
    let capability = capability_id_for(server, tool)
        .map(|id| id.as_str().to_string())
        .unwrap_or_else(|_| format!("{server}.{tool}"));
    let mut lines = Vec::with_capacity(3);
    match (&outcome, mutation) {
        (MutationOutcome::Changed(_), Mutation::Add) => lines.push(format!(
            "denied {capability} (external server list updated)"
        )),
        (MutationOutcome::Unchanged(_), Mutation::Add) => {
            lines.push(format!("already denied: {capability} (no change written)"))
        }
        (MutationOutcome::Changed(_), Mutation::Remove) => lines.push(format!(
            "un-denied {capability} (external server list updated)"
        )),
        (MutationOutcome::Unchanged(_), Mutation::Remove) => {
            lines.push(format!("not denied: {capability} (no change written)"))
        }
    }
    let list = match &outcome {
        MutationOutcome::Changed(list) | MutationOutcome::Unchanged(list) => list.clone(),
    };
    lines.push(format!(
        "  server {server} deny_tools: {}",
        list_or_dash(&list)
    ));
    lines.push(
        "takes effect after the host restarts (需重启生效) — the running host's deny set is \
         unchanged"
            .to_string(),
    );
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
        let error = mutate_deny_list(&dir, "demo", "lookup", Mutation::Add)
            .expect_err("env 在场时改文件必须拒");
        assert!(error.contains(MCP_SERVERS_ENV), "{error}");
        std::env::remove_var(MCP_SERVERS_ENV);
        let _ = fs_err::remove_dir_all(&dir);
    }
}
