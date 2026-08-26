use std::env;
use std::process::ExitCode;

use apeireth_cli::{
    build_canonical_runtime_from_env, dispatch_canonical_chat, dispatch_gateway_serve,
};

fn print_help() {
    println!(
        "apeireth\n\nUsage:\n  apeireth session\n  apeireth chat <PROMPT> [--model MODEL] [--session SESSION]\n  apeireth gateway serve [--port PORT]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show the version"
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
        Ok(response) => {
            println!("{}", response.text);
            eprintln!(
                "session={} trace={} provider={} rounds={}",
                response.session, response.trace.trace, response.served_by, response.rounds
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("canonical chat failed: {error}");
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

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "session" {
        return run_session();
    }
    if args[0] == "--help" || args[0] == "-h" {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("apeireth {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
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
