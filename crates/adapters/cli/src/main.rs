use std::env;
use std::process::ExitCode;

use apeireth_cli::{
    build_canonical_runtime_from_env, dispatch_canonical_approval, dispatch_canonical_chat,
    dispatch_gateway_serve_on, CanonicalCliTurn,
};
use apeireth_runtime::ApprovalDecision;

fn print_help() {
    println!(
        "apeireth\n\nUsage:\n  apeireth session\n  apeireth chat <PROMPT> [--model MODEL] [--session SESSION]\n  apeireth approve --session SESSION --approval APPROVAL\n  apeireth reject --session SESSION --approval APPROVAL [--reason REASON]\n  apeireth cancel --session SESSION --approval APPROVAL [--reason REASON]\n  apeireth gateway serve [--bind ADDR] [--port PORT]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show the version"
    );
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
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(dispatch_canonical_chat(prompt, model, session)) {
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
            ExitCode::FAILURE
        }
    }
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

fn run_gateway(bind: String, port: u16) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(dispatch_gateway_serve_on(&bind, port)) {
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

fn main() -> ExitCode {
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
        "approve" | "reject" | "cancel" => match parse_approval(&args[0], &args[1..]) {
            Ok((session, approval, decision)) => run_approval(session, approval, decision),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "gateway" if args.get(1).map(String::as_str) == Some("serve") => {
            let mut bind = "127.0.0.1".to_string();
            let mut port = 8080;
            let mut index = 2;
            while index < args.len() {
                if args[index] == "--bind" {
                    index += 1;
                    let Some(value) = args.get(index) else {
                        eprintln!("gateway serve --bind requires a value");
                        return ExitCode::FAILURE;
                    };
                    bind = value.clone();
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
                } else {
                    eprintln!("unknown gateway argument: {}", args[index]);
                    print_help();
                    return ExitCode::FAILURE;
                }
                index += 1;
            }
            run_gateway(bind, port)
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
