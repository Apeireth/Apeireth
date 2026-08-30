use std::env;
use std::process::ExitCode;

use apeireth_cli::{
    build_canonical_runtime_from_env, dispatch_canonical_approval, dispatch_canonical_chat,
    dispatch_gateway_serve, CanonicalCliTurn,
};
use apeireth_runtime::ApprovalDecision;

fn print_help() {
    println!(
        "apeireth\n\nUsage:\n  apeireth session\n  apeireth chat <PROMPT> [--model MODEL] [--session SESSION]\n  apeireth approve --session SESSION --approval APPROVAL\n  apeireth reject --session SESSION --approval APPROVAL [--reason REASON]\n  apeireth cancel --session SESSION --approval APPROVAL [--reason REASON]\n  apeireth gateway serve [--port PORT]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show the version"
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

fn run_gateway(port: u16) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create Tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(dispatch_gateway_serve(port)) {
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
        "approve" | "reject" | "cancel" => match parse_approval(&args[0], &args[1..]) {
            Ok((session, approval, decision)) => run_approval(session, approval, decision),
            Err(error) => {
                eprintln!("{error}");
                print_help();
                ExitCode::FAILURE
            }
        },
        "gateway" if args.get(1).map(String::as_str) == Some("serve") => {
            let mut port = 8080;
            let mut index = 2;
            while index < args.len() {
                if args[index] == "--port" {
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
            run_gateway(port)
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
