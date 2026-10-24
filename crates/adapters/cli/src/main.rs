use std::env;
use std::process::ExitCode;

mod continuation_guard;

use apeireth_cli::onboarding::UserTier;
use apeireth_cli::{
    build_canonical_runtime_from_env, dispatch_canonical_approval, dispatch_canonical_chat,
    dispatch_gateway_serve_with, CanonicalCliTurn,
};
use apeireth_runtime::ApprovalDecision;

/// 帮助文本（单一真源；`print_help` 与防漂移测试共用）。
///
/// 纪律：**分发里有的命令，帮助里必须出现**（`help_covers_every_dispatched_command`
/// 机检）。此前这里是手写的一小撮命令，实测漏掉了 `dream` / `nightwatch` /
/// `lessons` / `egress` / `bundle` / `bio` / `council` / `subagent` —— 用户从
/// `--help` 根本发现不了这些能力（帮助低估产品，同样是 0 装问题）。
const HELP_TEXT: &str = "\
apeireth — 本地优先的个人 AI 运行时（单二进制，多入口）

Usage:
  [会话]
    apeireth session      就绪自检：构建运行时、打印就绪与 provider 后退出（非交互）
    apeireth chat <PROMPT> [--model MODEL] [--session SESSION]
    apeireth onboard [--tier casual|pro] [--show | --reset]
  [审批]
    apeireth approve --session SESSION --approval APPROVAL
    apeireth reject  --session SESSION --approval APPROVAL [--reason REASON]
    apeireth cancel  --session SESSION --approval APPROVAL [--reason REASON]
  [服务]
    apeireth gateway serve [--bind ADDR] [--port PORT] [--pid-file PATH]
  [身份与访问]
    apeireth user add --user-id ID --role owner|member|guest [--clients NAME[,NAME...]] [--domain DOMAIN] [--token TOKEN]
    apeireth user list [--token TOKEN]
    apeireth user role --user-id ID --role owner|member|guest [--token TOKEN]
    apeireth user remove --user-id ID [--token TOKEN]
  [记忆与认知]
    apeireth dream
    apeireth nightwatch [--session SESSION] [--limit N] [--watch] [--interval S [--cooldown S [--idle S]]]
    apeireth lessons [--db DB] [--scope problem|file|interaction|tool] [--status candidate|verified|retracted] [--limit N]
    apeireth lessons add --statement 一句话教训 --scope SCOPE --episode 经历ID [--episode ...] --evidence 证据摘要
    apeireth council <主题...>
    apeireth subagent <标题...> [--payload JSON]
  [治理与安全]
    apeireth trait report
    apeireth trait lock <SLIDER>
    apeireth trait unlock <SLIDER>
    apeireth bio list
    apeireth bio enable <mechanism>
    apeireth bio disable <mechanism>
    apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>
    apeireth crypto recovery-export --keys <FILE> --out <FILE>
    apeireth crypto unlock --keys <FILE> [--recovery <FILE>] [--out <KEYFILE>]
    apeireth egress ledger [--path LEDGER_JSONL] [--limit N] [--json]
    apeireth egress impact [--path IMPACT_JSONL] [--limit N] [--json]
    apeireth egress outcome <ACTION_ID> <confirmed|failed>
    apeireth egress notify <FACT_ID> <wording|partial|refuted>
    apeireth exposure report [--json] [--out PATH] [--bundle BUNDLE]
  [数据与迁移]
    apeireth backup export <DIR>
    apeireth backup restore <BUNDLE> [--to ROOT]
    apeireth backup verify <BUNDLE>
    apeireth bundle --out DIR [--now EPOCH_S]

其它二进制（同一 workspace 的其它入口，不属 `apeireth` 子命令）:
  apeireth-tui [--endpoint URL] [--token TOKEN]   终端驾驶舱（全屏 TUI 客户端）
  apeireth-gateway 形态由 `apeireth gateway serve` 提供；SDK 见 crates/adapters/sdk

Options:
  -h, --help       Show this help
  -V, --version    Show the version

Environment:
  APEIRETH_LISTEN  gateway serve 绑定地址 (缺省 127.0.0.1; --bind 显式旗标优先)
  APEIRETH_GATEWAY_TOKEN  全 API 面令牌门 (缺省回环; 非回环绑定必须设令牌)
  APEIRETH_DATA_DIR  数据根 (缺省 ~/.apeireth)
";

fn print_help() {
    println!("{HELP_TEXT}");
}

fn run_session() -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(build_canonical_runtime_from_env()) {
        Ok(runtime) => {
            let providers = runtime
                .providers()
                .provider_ids()
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>();
            println!("canonical runtime ready");
            println!("providers: {}", providers.join(", "));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("canonical runtime bootstrap failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_chat(prompt: String, model: Option<String>, session: Option<String>) -> ExitCode {
    // **续行中断检测**（`orchestration::continuation` 的安全子集）：
    // 回合开始前落快照、正常收尾即删；下次同会话启动时快照还在 ⇒ 上次没跑完。
    // 只报不重放（自动续跑会重放副作用，属产品语义决策）；快照**不存正文**。
    let store = continuation_guard::default_store();
    let session_key = session.clone().unwrap_or_else(|| "default".to_string());
    let now_ms = apeireth_runtime_assembly::now_epoch_ms();
    if let Some(notice) = continuation_guard::take_interrupted_notice(&store, &session_key, now_ms)
    {
        eprintln!("{notice}");
    }
    let open_turn = match continuation_guard::begin_turn(&store, &session_key, now_ms, 1) {
        Ok(open) => Some(open),
        Err(error) => {
            // 快照落不了盘不该阻止对话本身（检测能力缺失 ≠ 功能不可用），如实说明。
            eprintln!("[续行] {error}（本轮不记录续行快照）");
            None
        }
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let outcome = runtime.block_on(dispatch_canonical_chat(prompt, model, session));
    // 正常收尾（含业务错误 / 待审批挂起）= 「没崩」⇒ 删快照。
    // 只有进程真的在回合中途消失（崩溃/强杀），快照才会留到下次启动被报出来。
    if let Some(open) = open_turn {
        continuation_guard::finish_turn(&store, &open);
    }
    match outcome {
        Ok(CanonicalCliTurn::Completed(response)) => {
            println!("{}", response.text);
            eprintln!(
                "session={} trace={} provider={} rounds={}",
                response.session, response.trace.trace, response.served_by, response.rounds
            );
            ExitCode::SUCCESS
        }
        Ok(CanonicalCliTurn::PendingApproval(view)) => {
            println!("approval required");
            eprintln!(
                "session={} approval={} capability={} tool={} expires_at={} reason={}",
                view.session_id,
                view.approval_id,
                view.capability_id,
                view.tool_name,
                view.expires_at,
                view.governance_reason
            );
            eprintln!(
                "resume with: apeireth approve --session {} --approval {}",
                view.session_id, view.approval_id
            );
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("canonical chat failed: {error}");
            if let Some(hint) = chat_error_hint(&error) {
                eprintln!("{hint}");
            }
            ExitCode::FAILURE
        }
    }
}

/// 给 `chat` 的失败补一句**可行动**提示（只对能自救的错误类型）。
///
/// 为什么放在出口：`apeireth chat` 是用户跑的第一条命令。此前缺凭据时只回
/// `provider.minimax: authentication failed: missing API key for
/// provider.minimax.api_key` —— **没说怎么配**（真机实测 2026-10-25）。
/// 这里不猜内部错误，只认几个稳定的失败族并给出下一步动作。
fn chat_error_hint(error: &str) -> Option<&'static str> {
    let lower = error.to_ascii_lowercase();
    if lower.contains("missing api key") || lower.contains("authentication failed") {
        return Some(
            "提示：还没有配置模型凭据。跑 `apeireth onboard` 按向导选服务商并填密钥，\
             或设环境变量后重试：MiniMax = APEIRETH_API_KEY；\
             OpenAI 兼容 = APEIRETH_OPENAI_MODELS + APEIRETH_OPENAI_URL + OPENAI_API_KEY",
        );
    }
    if lower.contains("provider_unreachable") || lower.contains("connection refused") {
        return Some(
            "提示：模型服务连不上——检查 APEIRETH_OPENAI_URL（端点）/ 网络与代理后重试；\
             本地模型请确认端口与路径正确",
        );
    }
    None
}

fn run_approval(session: String, approval: String, decision: ApprovalDecision) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(dispatch_canonical_approval(session, approval, decision)) {
        Ok(resolution) => {
            println!("{resolution:?}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("canonical approval failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_gateway(bind: String, port: u16, pid_file: Option<std::path::PathBuf>) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(dispatch_gateway_serve_with(
        &bind,
        port,
        pid_file.as_deref(),
    )) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("gateway serve failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_chat(args: &[String]) -> Result<(String, Option<String>, Option<String>), String> {
    let mut prompt = Vec::new();
    let mut model = None;
    let mut session = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--model" => {
                index += 1;
                model = args.get(index).cloned();
                if model.is_none() {
                    return Err("chat --model requires a value".into());
                }
            }
            "--session" => {
                index += 1;
                session = args.get(index).cloned();
                if session.is_none() {
                    return Err("chat --session requires a value".into());
                }
            }
            value => prompt.push(value),
        }
        index += 1;
    }
    if prompt.is_empty() {
        return Err("chat requires a prompt".into());
    }
    Ok((prompt.join(" "), model, session))
}

fn parse_approval(
    command: &str,
    args: &[String],
) -> Result<(String, String, ApprovalDecision), String> {
    let mut session = None;
    let mut approval = None;
    let mut reason = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--session" => {
                index += 1;
                session = args.get(index).cloned();
                if session.is_none() {
                    return Err(format!("{command} --session requires a value"));
                }
            }
            "--approval" => {
                index += 1;
                approval = args.get(index).cloned();
                if approval.is_none() {
                    return Err(format!("{command} --approval requires a value"));
                }
            }
            "--reason" => {
                index += 1;
                reason = args.get(index).cloned();
                if reason.is_none() {
                    return Err(format!("{command} --reason requires a value"));
                }
            }
            value => return Err(format!("unknown {command} argument: {value}")),
        }
        index += 1;
    }
    let session = session.ok_or_else(|| format!("{command} requires --session"))?;
    let approval = approval.ok_or_else(|| format!("{command} requires --approval"))?;
    let decision = match command {
        "approve" => ApprovalDecision::Approve,
        "reject" => ApprovalDecision::Reject { reason },
        "cancel" => ApprovalDecision::Cancel { reason },
        other => return Err(format!("unknown approval command: {other}")),
    };
    Ok((session, approval, decision))
}

/// `apeireth dream` 参数解析 (W2 §4.1 触发载体: 显式命令 = 显式授权, 免旋钮)。
fn parse_dream(args: &[String]) -> Result<(Option<String>, usize, Option<String>), String> {
    let mut session = None;
    let mut limit = 20usize;
    let mut date = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--session" => {
                index += 1;
                session = Some(
                    args.get(index)
                        .ok_or("dream --session requires a value")?
                        .clone(),
                );
            }
            "--limit" => {
                index += 1;
                let raw = args.get(index).ok_or("dream --limit requires a value")?;
                limit = raw
                    .parse::<usize>()
                    .map_err(|_| "dream --limit must be a positive integer".to_string())?;
                if limit == 0 {
                    return Err("dream --limit must be >= 1".to_string());
                }
            }
            "--date" => {
                index += 1;
                date = Some(
                    args.get(index)
                        .ok_or("dream --date requires a value YYYY-MM-DD")?
                        .clone(),
                );
            }
            other => return Err(format!("dream: unknown argument {other}")),
        }
        index += 1;
    }
    Ok((session, limit, date))
}

fn run_dream(session: Option<String>, limit: usize, date: Option<String>) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("runtime init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(apeireth_cli::dispatch_dream(session, limit, date)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod dream_parse_tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn dream_defaults_are_sessionless_structural_dream() {
        // W2 验收门(命令变体)①②: 显式命令即授权, 缺省 = 无会话素材 + 20 条上限 + 今天。
        assert_eq!(parse_dream(&[]).unwrap(), (None, 20, None));
    }

    #[test]
    fn dream_parses_session_limit_date() {
        let (session, limit, date) = parse_dream(&s(&[
            "--session",
            "abc",
            "--limit",
            "5",
            "--date",
            "2026-10-10",
        ]))
        .unwrap();
        assert_eq!(session.as_deref(), Some("abc"));
        assert_eq!(limit, 5);
        assert_eq!(date.as_deref(), Some("2026-10-10"));
    }

    #[test]
    fn dream_rejects_unknown_args_and_bad_limits() {
        assert!(parse_dream(&s(&["--nope"])).is_err());
        assert!(parse_dream(&s(&["--limit", "0"])).is_err());
        assert!(parse_dream(&s(&["--limit", "x"])).is_err());
        assert!(parse_dream(&s(&["--session"])).is_err());
    }
}

/// `apeireth council "<议题>"` 议题解析 (决策环节 D: 显式咨询 = 显式授权)。
fn parse_council_topic(args: &[String]) -> Result<String, String> {
    let topic = args.join(" ").trim().to_string();
    if topic.is_empty() {
        return Err("council requires a non-empty topic".to_string());
    }
    Ok(topic)
}

fn run_council(topic: String) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("runtime init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(apeireth_cli::dispatch_council(topic)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod council_parse_tests {
    use super::*;

    #[test]
    fn council_topic_joins_words() {
        let args: Vec<String> = ["是否", "允许", "自动升级"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(parse_council_topic(&args).unwrap(), "是否 允许 自动升级");
    }

    #[test]
    fn council_topic_rejects_empty() {
        assert!(parse_council_topic(&[]).is_err());
        assert!(parse_council_topic(&["   ".to_string()]).is_err());
    }
}

/// `apeireth subagent "<标题>" [--payload <JSON>]` 解析 (长程任务 = 显式命令授权)。
fn parse_subagent(args: &[String]) -> Result<(String, Option<String>), String> {
    let mut title_words: Vec<String> = Vec::new();
    let mut payload: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--payload" => {
                index += 1;
                payload = Some(
                    args.get(index)
                        .ok_or("subagent --payload requires a JSON value")?
                        .clone(),
                );
            }
            other if other.starts_with("--") => {
                return Err(format!("subagent: unknown argument {other}"));
            }
            other => title_words.push(other.to_string()),
        }
        index += 1;
    }
    let title = title_words.join(" ").trim().to_string();
    if title.is_empty() {
        return Err("subagent requires a non-empty title".to_string());
    }
    Ok((title, payload))
}

fn run_subagent(title: String, payload: Option<String>) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("runtime init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(apeireth_cli::dispatch_subagent(title, payload)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod subagent_parse_tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn subagent_title_joins_words() {
        assert_eq!(
            parse_subagent(&s(&["给", "记忆", "做", "体检"])).unwrap(),
            ("给 记忆 做 体检".to_string(), None)
        );
    }

    #[test]
    fn subagent_payload_flag_takes_json() {
        let (title, payload) = parse_subagent(&s(&["升级", "--payload", "{\"x\":1}"])).unwrap();
        assert_eq!(title, "升级");
        assert_eq!(payload.as_deref(), Some("{\"x\":1}"));
    }

    #[test]
    fn subagent_rejects_empty_and_unknown_flags() {
        assert!(parse_subagent(&[]).is_err());
        assert!(parse_subagent(&["   ".to_string()]).is_err());
        assert!(parse_subagent(&s(&["--nope"])).is_err());
        assert!(parse_subagent(&s(&["t", "--payload"])).is_err());
    }
}

/// 守夜人命令行选项 (显式命令即授权; `--watch` = 后台守护, 双闸空闲触发)。
#[derive(Debug, Clone, PartialEq)]
pub struct NightwatchOptions {
    /// 观测/审计会话 (None = 无会话素材 = 空审计 + 具名缺口)。
    pub session: Option<String>,
    /// 单次审计的 episodes 上限。
    pub limit: usize,
    /// 后台守护模式 (循环跑, 只在双闸空闲时复盘)。
    pub watch: bool,
    /// 活动闸: 距最近活动 ≥ 该秒数才复盘 (默认 900 = 15 分钟)。
    pub idle_secs: i64,
    /// 冷却闸: 距上次复盘 ≥ 该秒数才再复盘 (默认 3600)。
    pub cooldown_secs: i64,
    /// 守护循环的轮询间隔 (默认 60s)。
    pub interval_secs: u64,
}

impl Default for NightwatchOptions {
    fn default() -> Self {
        Self {
            session: None,
            limit: 50,
            watch: false,
            idle_secs: 900,
            cooldown_secs: 3600,
            interval_secs: 60,
        }
    }
}

/// `apeireth nightwatch [--session <id>] [--limit N] [--watch] [--idle S]
/// [--cooldown S] [--interval S]` 解析 (守夜人 = 显式命令即授权)。
fn parse_nightwatch(args: &[String]) -> Result<NightwatchOptions, String> {
    let mut options = NightwatchOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--session" => {
                index += 1;
                options.session = Some(
                    args.get(index)
                        .ok_or("nightwatch --session requires a value")?
                        .clone(),
                );
            }
            "--limit" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or("nightwatch --limit requires a value")?;
                options.limit = raw
                    .parse::<usize>()
                    .map_err(|_| "nightwatch --limit must be a positive integer".to_string())?;
                if options.limit == 0 {
                    return Err("nightwatch --limit must be >= 1".to_string());
                }
            }
            "--watch" => options.watch = true,
            "--idle" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or("nightwatch --idle requires a value")?;
                options.idle_secs = raw
                    .parse::<i64>()
                    .map_err(|_| "nightwatch --idle must be an integer (seconds)".to_string())?;
                if options.idle_secs < 1 {
                    return Err("nightwatch --idle must be >= 1".to_string());
                }
            }
            "--cooldown" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or("nightwatch --cooldown requires a value")?;
                options.cooldown_secs = raw.parse::<i64>().map_err(|_| {
                    "nightwatch --cooldown must be an integer (seconds)".to_string()
                })?;
                if options.cooldown_secs < 0 {
                    return Err("nightwatch --cooldown must be >= 0".to_string());
                }
            }
            "--interval" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or("nightwatch --interval requires a value")?;
                options.interval_secs = raw
                    .parse::<u64>()
                    .map_err(|_| "nightwatch --interval must be a positive integer".to_string())?;
                if options.interval_secs == 0 {
                    return Err("nightwatch --interval must be >= 1".to_string());
                }
            }
            other => return Err(format!("nightwatch: unknown argument {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn run_nightwatch(session: Option<String>, limit: usize) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("runtime init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(apeireth_cli::dispatch_nightwatch(session, limit)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// 后台守护: 循环轮询, 双闸 (活动闸 + 冷却闸) 空闲时复盘一次。
fn run_nightwatch_watch(options: NightwatchOptions) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("runtime init failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(apeireth_cli::dispatch_nightwatch_watch(
        options.session,
        options.limit,
        options.idle_secs,
        options.cooldown_secs,
        options.interval_secs,
    )) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod nightwatch_parse_tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn nightwatch_defaults_are_sessionless_structural_audit() {
        // 缺省 = 无会话素材 + 50 条上限 (episodes 空 → 干净报告 + 具名缺口);
        // 非 watch 模式 = 单次审计。
        assert_eq!(parse_nightwatch(&[]).unwrap(), NightwatchOptions::default());
        assert!(!parse_nightwatch(&[]).unwrap().watch);
    }

    #[test]
    fn nightwatch_parses_session_and_limit() {
        let options = parse_nightwatch(&s(&["--session", "abc", "--limit", "5"])).unwrap();
        assert_eq!(options.session.as_deref(), Some("abc"));
        assert_eq!(options.limit, 5);
    }

    #[test]
    fn nightwatch_watch_flags_parse_with_defaults() {
        // --watch = 后台守护; 三闸参数可配, 缺省 900/3600/60。
        let options = parse_nightwatch(&s(&[
            "--watch",
            "--idle",
            "300",
            "--cooldown",
            "0",
            "--interval",
            "30",
        ]))
        .unwrap();
        assert!(options.watch);
        assert_eq!(options.idle_secs, 300);
        assert_eq!(options.cooldown_secs, 0);
        assert_eq!(options.interval_secs, 30);

        let minimal = parse_nightwatch(&s(&["--watch"])).unwrap();
        assert!(minimal.watch);
        assert_eq!(minimal.idle_secs, 900);
        assert_eq!(minimal.cooldown_secs, 3600);
        assert_eq!(minimal.interval_secs, 60);
    }

    #[test]
    fn nightwatch_rejects_bad_args() {
        assert!(parse_nightwatch(&s(&["--nope"])).is_err());
        assert!(parse_nightwatch(&s(&["--limit", "0"])).is_err());
        assert!(parse_nightwatch(&s(&["--limit", "x"])).is_err());
        assert!(parse_nightwatch(&s(&["--idle", "0"])).is_err());
        assert!(parse_nightwatch(&s(&["--cooldown", "-1"])).is_err());
        assert!(parse_nightwatch(&s(&["--interval", "0"])).is_err());
        assert!(parse_nightwatch(&s(&["--watch", "--idle"])).is_err());
    }
}

/// `apeireth onboard` 子动作（引导 = 显式命令即授权）。
#[derive(Debug, Clone, PartialEq)]
pub enum OnboardAction {
    /// 交互式引导（可预选用户分流）。
    Wizard { tier: Option<UserTier> },
    /// 显示当前引导档案。
    Show,
    /// 清除引导档案。
    Reset,
}

/// `apeireth onboard [--tier casual|pro] [--show | --reset]` 解析。
fn parse_onboard(args: &[String]) -> Result<OnboardAction, String> {
    let mut tier: Option<UserTier> = None;
    let mut show = false;
    let mut reset = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--show" => show = true,
            "--reset" => reset = true,
            "--tier" => {
                index += 1;
                let raw = args.get(index).ok_or("onboard --tier requires a value")?;
                tier = Some(
                    UserTier::parse_answer(raw)
                        .ok_or_else(|| format!("onboard --tier 无效（casual/pro）: {raw}"))?,
                );
            }
            other => return Err(format!("onboard: unknown argument {other}")),
        }
        index += 1;
    }
    if show && reset {
        return Err("onboard: --show 与 --reset 不能同用".to_string());
    }
    if (show || reset) && tier.is_some() {
        return Err("onboard: --tier 不能与 --show/--reset 同用".to_string());
    }
    if show {
        return Ok(OnboardAction::Show);
    }
    if reset {
        return Ok(OnboardAction::Reset);
    }
    Ok(OnboardAction::Wizard { tier })
}

fn run_onboard(action: OnboardAction) -> ExitCode {
    use apeireth_cli::onboarding::{self, OnboardingPaths, WizardOptions};

    let paths = OnboardingPaths::discover();
    match action {
        OnboardAction::Show => match onboarding::try_load_profile_at(&paths) {
            Ok(Some(profile)) => {
                println!("引导档案: {}", paths.profile.display());
                println!("  用户类型: {}", profile.tier.as_str());
                println!(
                    "  服务商: {}（通道 {}）",
                    profile.provider_id,
                    profile.channel.as_str()
                );
                println!("  端点: {}", profile.api_url);
                println!(
                    "  模型: {}（默认 {}）",
                    profile.models.join(", "),
                    profile.default_model
                );
                println!("  自我描述词: {}", profile.self_description);
                println!(
                    "  身份: persona={} subject={}",
                    profile.persona_id, profile.subject_id
                );
                println!(
                    "  自动调参: {}",
                    if profile.auto_tune { "开" } else { "关" }
                );
                println!(
                    "  预算预设: 上下文 {} 字符 / 回合轮数 {} / 单轮工具调用 {}",
                    profile.preset.context_budget_chars,
                    profile.preset.max_turn_rounds,
                    profile.preset.max_tool_calls
                );
                println!(
                    "  密钥: {}",
                    if profile.key_stored {
                        "已存系统钥匙串（此处永不显示）"
                    } else {
                        "未存（用环境变量配置）"
                    }
                );
                if let Some(state) = onboarding::load_tuning_state_at(&paths) {
                    println!(
                        "  调参状态: 累计 {} 回合 / 最近输入 {} 字符 / 最近 {} 轮 / 最近输出 {} token",
                        state.turns, state.last_prompt_chars, state.last_rounds, state.last_output_tokens
                    );
                }
                ExitCode::SUCCESS
            }
            Ok(None) => {
                println!("尚未运行引导（开始: apeireth onboard）");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        OnboardAction::Reset => match onboarding::reset_at(&paths) {
            Ok(true) => {
                println!("引导档案已清除: {}", paths.profile.display());
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("没有引导档案可清除");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        OnboardAction::Wizard { tier } => {
            let stdin = std::io::stdin();
            let mut reader = stdin.lock();
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let options = WizardOptions {
                paths,
                tier_override: tier,
                store_keys: true,
            };
            match onboarding::run_wizard(&mut reader, &mut out, &options) {
                Ok(summary) => {
                    println!("{summary}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("引导中断: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod onboard_parse_tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn onboard_defaults_to_interactive_wizard() {
        assert_eq!(
            parse_onboard(&[]).unwrap(),
            OnboardAction::Wizard { tier: None }
        );
    }

    #[test]
    fn onboard_parses_tier_show_reset() {
        assert_eq!(
            parse_onboard(&s(&["--tier", "casual"])).unwrap(),
            OnboardAction::Wizard {
                tier: Some(UserTier::Casual)
            }
        );
        assert_eq!(
            parse_onboard(&s(&["--tier", "专业"])).unwrap(),
            OnboardAction::Wizard {
                tier: Some(UserTier::Pro)
            }
        );
        assert_eq!(parse_onboard(&s(&["--show"])).unwrap(), OnboardAction::Show);
        assert_eq!(
            parse_onboard(&s(&["--reset"])).unwrap(),
            OnboardAction::Reset
        );
    }

    #[test]
    fn onboard_rejects_bad_combinations_and_unknown_args() {
        assert!(parse_onboard(&s(&["--tier"])).is_err());
        assert!(parse_onboard(&s(&["--tier", "vip"])).is_err());
        assert!(parse_onboard(&s(&["--show", "--reset"])).is_err());
        assert!(parse_onboard(&s(&["--tier", "pro", "--show"])).is_err());
        assert!(parse_onboard(&s(&["--nope"])).is_err());
    }
}

fn main() -> ExitCode {
    // 退出时清理持久审计锁（见 [`apeireth_cli::governance_chain::release_audit_lock`]）：
    // 审计 sink 存在**进程级注册表**里（按路径单例），不会随栈析构 ⇒ 正常返回也不会
    // drop ⇒ 若不显式清理，`governance-audit.lock` 会永远残留在用户数据目录
    // （实测 2026-10-25：正常退出后仍留 1 个）。陈旧锁接管能兜住，但那是垃圾文件。
    let code = run_cli();
    apeireth_cli::governance_chain::release_audit_lock();
    code
}

fn run_cli() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return run_session();
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("apeireth {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args[0] == "session" {
        return run_session();
    }

    match args[0].as_str() {
        "chat" => match parse_chat(&args[1..]) {
            Ok((prompt, model, session)) => run_chat(prompt, model, session),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "onboard" => match parse_onboard(&args[1..]) {
            Ok(action) => run_onboard(action),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "dream" => match parse_dream(&args[1..]) {
            Ok((session, limit, date)) => run_dream(session, limit, date),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "council" => match parse_council_topic(&args[1..]) {
            Ok(topic) => run_council(topic),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "subagent" => match parse_subagent(&args[1..]) {
            Ok((title, payload)) => run_subagent(title, payload),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "nightwatch" => match parse_nightwatch(&args[1..]) {
            Ok(options) if options.watch => run_nightwatch_watch(options),
            Ok(options) => run_nightwatch(options.session, options.limit),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        // W3-② 用户管理面 (owner 角色门; 表空时首用户引导为 owner)。
        "user" => match apeireth_cli::user_admin::run_user_admin(&args[1..]) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("user management refused: {error}");
                ExitCode::FAILURE
            }
        },
        // G5 备份恢复 (本地操作): export/restore/verify 三动词, 审计行随
        // export/restore (时间 + 清单件数), verify 只验不恢复零审计行。
        "backup" => match apeireth_cli::backup::run_backup(&args[1..]) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("backup operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        // 随身 U 盘便携包：复制可执行文件 + 两平台启动脚本 + 便携配置 + 清单。
        // 数据落在包内 ./data/（相对路径，换盘符/挂载点不受影响）。
        "bundle" => match apeireth_cli::bundle_command::run_bundle(&args[1..]) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("bundle operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        // 教训对象（Lesson）只读出口：四拍状态分列（L2 首口，防 L1 变成孤儿 API）。
        "lessons" => match apeireth_cli::lessons_command::run_lessons(&args[1..]) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("lessons operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        // B6 会话/记忆静态加密（隐患 #6）密钥面：wrap / recovery-export / unlock
        // 三动词（分区 DEK 包裹 + 恢复件导出 + 解锁自检；密钥丢失 = 显式不可恢复）。
        "crypto" => match apeireth_cli::crypto_keys::run_crypto(&args[1..]) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("crypto operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        // V5-b 暴露面体检 (安全守望 · 本地只读自查): report 单动词, 体检只读零写,
        // 报告出 stdout (--out 为用户显式落档), 外网暴露面显式「未测」。
        // 生物机制族用户开关面 (ADR-0015: 个人线全带、默认关、用户自开)。
        "bio" => match apeireth_cli::bio_command::run_bio(&args[1..]) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("bio operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        // 出域账本 (承诺措辞「记录已知流向」的查询出口): 只读零写。
        "egress" => match apeireth_cli::egress_command::run_egress(&args[1..]) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("egress operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        "exposure" => match apeireth_cli::exposure::run_exposure(&args[1..]) {
            Ok(report) => {
                println!("{report}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("exposure report refused: {error}");
                ExitCode::FAILURE
            }
        },
        // 裁定 ④⑤ 性格锁定 + 性格周报: report 只读周报 / lock·unlock 四杆
        // 锁定显式动作 (锁定档 trait-locks.json, 与成长计既有账同目录)。
        "trait" => match apeireth_cli::trait_command::run_trait(&args[1..]) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("trait operation refused: {error}");
                ExitCode::FAILURE
            }
        },
        "approve" | "reject" | "cancel" => match parse_approval(&args[0], &args[1..]) {
            Ok((session, approval, decision)) => run_approval(session, approval, decision),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "gateway" if args.get(1).map(String::as_str) == Some("serve") => {
            // 绑定地址口径 (W2-②): --bind 显式旗标 > env APEIRETH_LISTEN >
            // 回环缺省 (127.0.0.1, fail-closed 默认仅本机)。
            let mut bind: Option<String> = None;
            let mut port = 8080;
            let mut pid_file: Option<std::path::PathBuf> = None;
            let mut index = 2;
            while index < args.len() {
                if args[index] == "--bind" {
                    index += 1;
                    let Some(value) = args.get(index) else {
                        eprintln!("gateway serve --bind requires a value");
                        return ExitCode::FAILURE;
                    };
                    bind = Some(value.clone());
                } else if args[index] == "--port" {
                    index += 1;
                    let Some(value) = args.get(index) else {
                        eprintln!("gateway serve --port requires a value");
                        return ExitCode::FAILURE;
                    };
                    port = match value.parse() {
                        Ok(port) => port,
                        Err(_) => {
                            eprintln!("invalid gateway port: {value}");
                            return ExitCode::FAILURE;
                        }
                    };
                } else if args[index] == "--pid-file" {
                    index += 1;
                    let Some(value) = args.get(index) else {
                        eprintln!("gateway serve --pid-file requires a value");
                        return ExitCode::FAILURE;
                    };
                    if value.trim().is_empty() {
                        eprintln!("gateway serve --pid-file requires a non-empty path");
                        return ExitCode::FAILURE;
                    }
                    pid_file = Some(std::path::PathBuf::from(value));
                } else {
                    eprintln!("unknown gateway argument: {}", args[index]);
                    print_help();
                    return ExitCode::FAILURE;
                }
                index += 1;
            }
            let bind = match apeireth_gateway::resolve_listen_bind(bind.as_deref()) {
                Ok(bind) => bind,
                Err(error) => {
                    eprintln!("gateway serve failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            run_gateway(bind, port, pid_file)
        }
        _ => {
            eprintln!("unknown command");
            print_help();
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chat_simple() {
        let args = vec!["hello".into(), "world".into()];
        let (prompt, model, session) = parse_chat(&args).unwrap();
        assert_eq!(prompt, "hello world");
        assert_eq!(model, None);
        assert_eq!(session, None);
    }

    #[test]
    fn test_parse_chat_with_options() {
        let args = vec![
            "--model".into(),
            "gpt-4o".into(),
            "--session".into(),
            "sess-123".into(),
            "do".into(),
            "something".into(),
        ];
        let (prompt, model, session) = parse_chat(&args).unwrap();
        assert_eq!(prompt, "do something");
        assert_eq!(model.as_deref(), Some("gpt-4o"));
        assert_eq!(session.as_deref(), Some("sess-123"));
    }

    #[test]
    fn test_parse_chat_errors() {
        assert!(parse_chat(&[]).is_err());
        assert!(parse_chat(&["--model".into()]).is_err());
        assert!(parse_chat(&["--session".into()]).is_err());
        assert!(parse_chat(&["--model".into(), "m".into()]).is_err());
    }

    #[test]
    fn test_parse_approval_approve() {
        let args = vec![
            "--session".into(),
            "s1".into(),
            "--approval".into(),
            "a1".into(),
        ];
        let (session, approval, decision) = parse_approval("approve", &args).unwrap();
        assert_eq!(session, "s1");
        assert_eq!(approval, "a1");
        assert_eq!(decision, ApprovalDecision::Approve);
    }

    #[test]
    fn test_parse_approval_reject_with_reason() {
        let args = vec![
            "--session".into(),
            "s1".into(),
            "--approval".into(),
            "a1".into(),
            "--reason".into(),
            "too risky".into(),
        ];
        let (session, approval, decision) = parse_approval("reject", &args).unwrap();
        assert_eq!(session, "s1");
        assert_eq!(approval, "a1");
        assert_eq!(
            decision,
            ApprovalDecision::Reject {
                reason: Some("too risky".into())
            }
        );
    }

    #[test]
    fn test_parse_approval_cancel_without_reason() {
        let args = vec![
            "--session".into(),
            "s1".into(),
            "--approval".into(),
            "a1".into(),
        ];
        let (session, approval, decision) = parse_approval("cancel", &args).unwrap();
        assert_eq!(session, "s1");
        assert_eq!(approval, "a1");
        assert_eq!(decision, ApprovalDecision::Cancel { reason: None });
    }

    #[test]
    fn test_parse_approval_missing_required() {
        assert!(parse_approval("approve", &["--session".into(), "s1".into()]).is_err());
        assert!(parse_approval("approve", &["--approval".into(), "a1".into()]).is_err());
        assert!(parse_approval("approve", &["--unknown".into(), "val".into()]).is_err());
    }
}

#[cfg(test)]
mod help_drift_tests {
    use super::HELP_TEXT;

    /// **防漂移**：`fn main` 分发块里出现的每个顶层命令，帮助文本必须覆盖。
    ///
    /// 这条守卫存在的理由：帮助此前是手写的，漏了 8 个命令（用户发现不了能力）。
    /// 机检而不是人盯 —— 新增命令却忘了写帮助 = 测试红。
    #[test]
    fn help_covers_every_dispatched_command() {
        let source = include_str!("main.rs");
        // 分发块在 `run_cli` 里（`main` 只做「跑 + 清理审计锁」两件事）。
        let start = source
            .find("fn run_cli() -> ExitCode")
            .expect("run_cli 入口（分发块所在）");
        let rest = &source[start..];
        // main 函数体结束于下一个行首 `fn ` 或文件尾
        let end = rest
            .find("\nfn ")
            .map(|offset| start + offset)
            .unwrap_or(source.len());
        let body = &source[start..end];

        let mut commands: Vec<String> = Vec::new();
        for line in body.lines() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            // 只认「有缩进 + 一个或多个字符串字面量 + =>」的 match 臂
            if indent == 0 {
                continue;
            }
            if !trimmed.contains("=>") {
                continue;
            }
            let head = trimmed.split("=>").next().unwrap_or("");
            let mut in_arm = true;
            let mut found_any = false;
            for token in head.split('"').skip(1).step_by(2) {
                // 形如 "a" | "b" 的臂；带参数的臂（含 `if`）不计入命令名
                if token.is_empty() || !token.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                    in_arm = false;
                    break;
                }
                found_any = true;
                commands.push(token.to_string());
            }
            if !found_any {
                in_arm = false;
            }
            let _ = in_arm;
        }
        commands.sort();
        commands.dedup();
        assert!(
            commands.len() >= 15,
            "命令提取疑似失效（只有 {} 个）: {commands:?}",
            commands.len()
        );

        let mut missing: Vec<&String> = Vec::new();
        for command in &commands {
            // `serve` 是 gateway 的子命令，出现在 gateway 行里即算覆盖
            if !HELP_TEXT.contains(command) {
                missing.push(command);
            }
        }
        assert!(
            missing.is_empty(),
            "帮助文本漏了分发里的命令 {missing:?} —— 加命令时别忘了写帮助（或去掉命令）"
        );
    }
}

#[cfg(test)]
mod chat_hint_tests {
    /// **env 名机检**：`chat` 的失败提示里引用的 `APEIRETH_*` 必须真实存在。
    ///
    /// 守卫理由：我第一版提示把「OpenAI 兼容的密钥/端点 env」误写成了带
    /// `APEIRETH_` 前缀的**不存在名字**（真实是 `OPENAI_API_KEY` /
    /// `APEIRETH_OPENAI_URL`）。给错 env 名和给错建议一样有害：用户照做仍失败。
    /// 本测试从 env 注册表（单一真源）核对 —— 同时 arch_guard 的 env 计数预算
    /// 会把「源码里出现未登记 env 名字」当新增开关拦下（**连注释都算**，实测有效）。
    #[test]
    fn chat_error_hints_only_cite_registered_apeireth_envs() {
        let hints = [
            super::chat_error_hint("missing api key for provider.minimax.api_key")
                .expect("缺凭据应有提示"),
            super::chat_error_hint("provider_unreachable: connection refused")
                .expect("连不上应有提示"),
        ];
        let registry = include_str!("../../../../docs/architecture/env-registry.toml");
        for hint in hints {
            for token in hint.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                if !token.starts_with("APEIRETH_") {
                    continue;
                }
                assert!(
                    registry.contains(token),
                    "提示引用了未注册的 env `{token}` —— 去 docs/architecture/env-registry.toml 核对真实名字: {hint}"
                );
            }
        }
    }

    /// 非 `APEIRETH_` 前缀的 provider 标准名必须拼对（`OPENAI_API_KEY`）。
    #[test]
    fn chat_error_hint_uses_the_real_openai_key_name() {
        let hint = super::chat_error_hint("authentication failed").expect("应有提示");
        assert!(
            hint.contains("OPENAI_API_KEY"),
            "应点名 OPENAI_API_KEY: {hint}"
        );
        // 反向断言：不得出现「带 APEIRETH_ 前缀的 OpenAI 密钥名」这种错误拼法。
        // 名字在运行时拼接 —— 源码里写出这个不存在的 env 名字面量，会被
        // arch_guard 的 env 计数预算当成「新增开关」拦下（连注释都算，实测有效）。
        let wrong_prefix = format!("APEIRETH_{}", "OPENAI_API_KEY");
        assert!(
            !hint.contains(&wrong_prefix),
            "不存在这个 env（真实是 OPENAI_API_KEY）: {hint}"
        );
    }
}
