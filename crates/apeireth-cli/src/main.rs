// apeireth-cli binary entry point
// R14 Phase 0: 命令协议稳定, Phase 3 后才接 PyO3 桥
// Fix-16: 提前实现 CliRunner parse + dispatch (解决 #1 #2 困难)
// A1.1: dispatch(Session) 接 core Session API —— 真实构造 Session / HA / PermissionOnion / DefaultPhilosophyGuard

use apeireth_asi::{DimensionTrace, TraceRepository};
use apeireth_cli::{
    build_default_human_authority, build_default_permission_onion, build_sample_measurement,
    create_default_session, describe_verdict, dispatch_asi_calibrate, dispatch_asi_diagnose,
    dispatch_asi_trace, dispatch_asi_trend, dispatch_gateway_serve, handle_input_line,
    welcome_message, AsiSubCommand, CalibrateMode, CliCommand, GatewaySubCommand,
};
use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

/// 解析 CLI 参数为 CliCommand
fn parse_args(args: &[String]) -> Result<CliCommand, String> {
    // args[0] = program name, args[1..] = 用户参数
    if args.len() < 2 {
        return Ok(CliCommand::Session); // 默认 = 启动 session
    }
    match args[1].as_str() {
        "session" => Ok(CliCommand::Session),
        "list-episodes" => Ok(CliCommand::ListEpisodes),
        "run-v1136" => Ok(CliCommand::RunV1136),
        "asi" => {
            // 二级子命令: asi trace / asi trend / asi diagnose
            if args.len() < 3 {
                return Err("asi 子命令需要二级参数 (trace|trend|diagnose)".into());
            }
            match args[2].as_str() {
                "trace" => {
                    let mut n: usize = 10;
                    if let Some(idx) = args.iter().position(|a| a == "--tail") {
                        if let Some(v) = args.get(idx + 1) {
                            n = v
                                .parse()
                                .map_err(|_| format!("invalid --tail value: {v}"))?;
                        }
                    }
                    Ok(CliCommand::Asi(AsiSubCommand::Trace { n }))
                }
                "trend" => {
                    let mut dim = String::new();
                    let mut last: usize = 20;
                    if let Some(idx) = args.iter().position(|a| a == "--dim") {
                        if let Some(v) = args.get(idx + 1) {
                            dim = v.clone();
                        }
                    }
                    if let Some(idx) = args.iter().position(|a| a == "--last") {
                        if let Some(v) = args.get(idx + 1) {
                            last = v
                                .parse()
                                .map_err(|_| format!("invalid --last value: {v}"))?;
                        }
                    }
                    if dim.is_empty() {
                        return Err("asi trend 需要 --dim <dimension_name>".into());
                    }
                    Ok(CliCommand::Asi(AsiSubCommand::Trend { dim, last }))
                }
                "diagnose" => {
                    let mut top: usize = 3;
                    if let Some(idx) = args.iter().position(|a| a == "--top") {
                        if let Some(v) = args.get(idx + 1) {
                            top = v.parse().map_err(|_| format!("invalid --top value: {v}"))?;
                        }
                    }
                    Ok(CliCommand::Asi(AsiSubCommand::Diagnose { top }))
                }
                "calibrate" => {
                    // default = dry-run, every=100, scope=all
                    let mode = if args.iter().any(|a| a == "--apply") {
                        CalibrateMode::Apply
                    } else {
                        CalibrateMode::DryRun
                    };
                    let mut every: usize = 100;
                    if let Some(idx) = args.iter().position(|a| a == "--every") {
                        if let Some(v) = args.get(idx + 1) {
                            every = v
                                .parse()
                                .map_err(|_| format!("invalid --every value: {v}"))?;
                        }
                    }
                    let mut scope = "all".to_string();
                    if let Some(idx) = args.iter().position(|a| a == "--scope") {
                        if let Some(v) = args.get(idx + 1) {
                            scope = v.clone();
                        }
                    }
                    Ok(CliCommand::Asi(AsiSubCommand::Calibrate {
                        mode,
                        every,
                        scope,
                    }))
                }
                unknown => Err(format!("未知 asi 子命令: {unknown}")),
            }
        }
        "quit" | "exit" => Ok(CliCommand::Quit),
        "gateway" => {
            // R16-09 gateway 子命令
            if args.len() < 3 {
                return Err("gateway 子命令需要二级参数 (serve|status|routes)".into());
            }
            match args[2].as_str() {
                "serve" => {
                    let mut port: Option<u16> = None;
                    if let Some(idx) = args.iter().position(|a| a == "--port") {
                        if let Some(v) = args.get(idx + 1) {
                            port = v.parse().ok();
                        }
                    }
                    Ok(CliCommand::Gateway(GatewaySubCommand::Serve { port }))
                }
                // R17 砍掉: status / routes (NewAPI channel 借鉴已砍, 见 lib.rs GatewaySubCommand)
                unknown => Err(format!(
                    "未知 gateway 子命令: {unknown} (R17 只支持 'serve')"
                )),
            }
        }
        "--help" | "-h" => Ok(CliCommand::Quit), // 帮助信息由 main() 输出
        "--version" | "-V" => {
            println!("apeireth-cli {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        unknown => Err(format!(
            "未知命令: {}\n可用命令: session, list-episodes, run-v1136, asi, quit\n\
             试用: apeireth-cli --help",
            unknown
        )),
    }
}

/// 输出帮助信息
fn print_help() {
    println!("apeireth-cli R14 - Apeireth 命令行接口\n");
    println!("用法: apeireth-cli <COMMAND>\n");
    println!("命令:");
    println!("  session         启动一次 session（默认，接 core Session API）");
    println!("  list-episodes   列出最近 N 个 episode");
    println!("  run-v1136       运行 V1136 真测");
    println!("  asi             ASI 测量子命令 (trace / trend / diagnose)");
    println!("  quit, exit      退出\n");
    println!("ASI 子命令:");
    println!("  asi trace --tail N              最近 N 条 DimensionTrace 详细表 (默认 10)");
    println!("  asi trend --dim X --last N     X 维最近 N 个值的 sparkline (默认 20)");
    println!("  asi diagnose --top N            定位最弱 N 维度 (默认 3)");
    println!("  asi calibrate [--apply] [--every M] [--scope X]  ML 在线校准 (默认 dry-run + M=100 + scope=all)\n");
    println!("选项:");
    println!("  -h, --help      显示帮助");
    println!("  -V, --version   显示版本\n");
    println!("示例:");
    println!("  apeireth-cli session");
    println!("  apeireth-cli list-episodes");
    println!("  apeireth-cli run-v1136");
}

/// 真实构造 Session + 跑 stdin 对话循环（A1.1 主实现）
fn run_session() -> ExitCode {
    // 1) 真实构造（不再是 println! 硬编码）
    let session = create_default_session();
    let ha = build_default_human_authority();
    let po = build_default_permission_onion();
    // DefaultPhilosophyGuard 已封装在 handle_input_line_default 内部（ADR 0002: main.rs 不直接 use apeireth_core::*）

    // 2) 欢迎信息从 Session 字段动态生成
    println!("{}", welcome_message(&session, &ha, &po));
    println!();
    println!("📥 输入一行文本 → 自动构造 Action → 走 V1+V2+V3 AND 门");
    println!("   输入 \":quit\" / \":exit\" 或 Ctrl-D / Ctrl-Z 退出");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut handle = stdin.lock();
    let mut buf = String::new();

    loop {
        // ponytail: prompt 写到 stderr 风格更地道，但这里保持 println 兼容非 TTY
        print!("> ");
        if stdout.flush().is_err() {
            break;
        }
        buf.clear();
        let n = match handle.read_line(&mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            // EOF (Ctrl-D on Unix / Ctrl-Z on Windows)
            println!();
            break;
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":quit" || line == ":exit" {
            println!("👋 退出 session ({})", session.id);
            break;
        }

        // 3) 跑完整 V1+V2+V3 AND 门（lib.rs 封装：run_session_action + DefaultPhilosophyGuard）
        let verdict = handle_input_line(line, &session);
        println!("  {}", describe_verdict(&verdict));
    }

    ExitCode::SUCCESS
}

/// 执行 CliCommand
fn dispatch(cmd: CliCommand) -> ExitCode {
    match cmd {
        CliCommand::Session => run_session(),
        CliCommand::ListEpisodes => {
            println!("📜 列出最近 episode...");
            println!("   (待 A11 apeireth-memory SQLite 实装后真正查询)");
            println!("   目前 placeholder 返回空列表");
            ExitCode::SUCCESS
        }
        CliCommand::RunV1136 => {
            println!("🔬 运行 V1136 真测...");
            println!("   (待 A5 apeireth-asi 真测引擎实装后真正跑)");
            println!("   目前 placeholder: 7 子测度 baseline = 0.9063");
            ExitCode::SUCCESS
        }
        CliCommand::Asi(sub) => run_asi(sub),
        CliCommand::Quit => {
            println!("👋 Apeireth-cli 退出");
            ExitCode::SUCCESS
        }
        CliCommand::Gateway(gw) => run_gateway(gw),
    }
}

/// R17 简化: gateway 只支持 serve (status / routes 砍掉, NewAPI channel 借鉴已砍)
fn run_gateway(gw: GatewaySubCommand) -> ExitCode {
    match gw {
        GatewaySubCommand::Serve { port } => {
            let p = port.unwrap_or(8080);
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("tokio runtime 创建失败: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let result = rt.block_on(dispatch_gateway_serve(p));
            match result {
                Ok(msg) => {
                    println!("{msg}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("❌ gateway serve 失败: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// round10-12 (qa_engineer): ASI 子命令 dispatch
fn run_asi(sub: AsiSubCommand) -> ExitCode {
    // Ponytail: in-memory repo 演示, 真实生产可接 SQLite backend
    let mut repo = TraceRepository::new();

    // 预填 5 条 trace (1.0 → 0.6) 让 trend / diagnose 有数据
    for i in 0..5 {
        let rate = 1.0 - (f64::from(i) * 0.1);
        let sample = build_sample_measurement(rate, 10);
        let trace = DimensionTrace::from_sample(0, 0, 1_700_000_000 + i64::from(i), &sample, None);
        repo.append(trace);
    }

    match sub {
        AsiSubCommand::Trace { n } => {
            let out = dispatch_asi_trace(&repo, n);
            print!("{out}");
        }
        AsiSubCommand::Trend { dim, last } => {
            let out = dispatch_asi_trend(&repo, &dim, last);
            print!("{out}");
        }
        AsiSubCommand::Diagnose { top } => {
            let tail = repo.tail(1);
            if tail.is_empty() {
                println!("TraceRepository is empty.");
                return ExitCode::SUCCESS;
            }
            let out = dispatch_asi_diagnose(&tail[0], top);
            print!("{out}");
        }
        AsiSubCommand::Calibrate { mode, every, scope } => {
            let out = dispatch_asi_calibrate(&repo, mode, every, &scope);
            print!("{out}");
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    // --help / -h 单独处理
    if args.len() >= 2 && (args[1] == "--help" || args[1] == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    match parse_args(&args) {
        Ok(cmd) => dispatch(cmd),
        Err(e) => {
            eprintln!("❌ {}", e);
            print_help();
            ExitCode::FAILURE
        }
    }
}
