//! Explicit, manually invoked memory answer evaluation. No default network call.

#[path = "memory_answer_eval/mod.rs"]
mod memory_answer_eval;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match memory_answer_eval::run(std::env::args().skip(1).collect()).await {
        Ok(passed) if passed => std::process::ExitCode::SUCCESS,
        Ok(_) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("memory answer evaluation: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
