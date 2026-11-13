//! The entry point of the standalone long-range evaluation and the
//! scale/sustained-run pressure contract.
//!
//! ⭐ 口径：既有题集是**回归集**（不是独立盲测）；隔离是**同账号隔离**（不是强保密）；
//! 本轮**未调用真实模型**（模型侧是确定性离线夹具）；规模与持续运行结果分别报告。

#[path = "memory_longterm_eval/mod.rs"]
mod memory_longterm_eval;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match memory_longterm_eval::run(&args).await {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("memory longterm evaluation: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
