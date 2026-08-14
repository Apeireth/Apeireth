//! `r257_minimax_real_api` - real end-to-end smoke against MiniMax Chat Completions API.
//!
//! Reads the api key from `.openclaw\apikey.txt` (default) or
//! the `APEIRETH_API_KEY` env var (override). Falls back to a "fake" key
//! (which will obviously fail) if neither exists.
//!
//! Run: `cargo run --example r257_minimax_real_api -p apeireth-runtime`
//!
//! Demonstrates:
//! 1. Read API key from disk (NEVER embed key in source).
//! 2. Build Runtime + register LlmWorker for the "llm" tool_name.
//! 3. Dispatch a real prompt through dispatch_async_task (which now uses the
//!    registry, not the SimulatedWorker).
//! 4. Wait for the task to complete and print the result.
//! 5. Print metrics_text so you can see the runtime observability surface area.
//!
//! Expected:
//! - With a real key: prints a real LLM response.
//! - With a fake key: prints an error string from the worker (e.g.
//!   "LLM API https://api.minimaxi.com/v1/chat/completions returned 401").
//!   The runtime still records the failure in arbitration + bus.

use apeireth_runtime::{LlmWorker, Runtime, RuntimeConfig};
use apeireth_tool_registry::TaskStatus;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Apeireth R257: real MiniMax API smoke ===\n");

    // 1. Read API key from disk.
    let api_key = read_api_key()?;
    let key_preview = if api_key.len() > 12 {
        format!("{}...{}", &api_key[..6], &api_key[api_key.len() - 6..])
    } else {
        "<too short>".to_string()
    };
    println!("[1] API key loaded: {} (len={})", key_preview, api_key.len());

    // 2. Build Runtime + register LlmWorker.
    let mut config = RuntimeConfig::default();
    config.tick_interval = Duration::from_secs(60); // we drive cycles manually
    config.arbitration_path = None; // in-memory
    let rt = Arc::new(Runtime::with_config(config));
    rt.bootstrap()?;
    let worker: Arc<dyn apeireth_runtime::AsyncWorker> =
        Arc::new(LlmWorker::new("llm", api_key.clone()));
    rt.register_worker("llm", worker);
    println!("[2] LlmWorker registered under tool_name=\"llm\"");

    // 3. Dispatch a real prompt.
    let prompt = "Reply with exactly: hello from MiniMax";
    let params = serde_json::json!({"prompt": prompt, "system": "be terse"});
    let task_id = rt
        .dispatch_async_task("llm", &params.to_string())
        .await;
    println!("[3] Dispatched task_id={} with prompt=\"{}\"", task_id, prompt);

    // 4. Wait for completion (or failure).
    let deadline = Duration::from_secs(30);
    let rec = rt
        .task_store
        .wait_for_completion(task_id, deadline)
        .await?;
    println!("[4] Task status: {:?}", rec.status);
    println!("[4] Task result_json: {:?}", rec.result_json);
    println!("[4] Task error: {:?}", rec.error);

    // 5. Show metrics.
    println!("\n[5] Runtime metrics:\n{}", rt.metrics_text());

    // 6. Show recent arbitration events.
    let n = rt.arbitration.len().unwrap_or(0);
    println!("[6] Arbitration events recorded: {}", n);

    if matches!(rec.status, TaskStatus::Completed) {
        println!("\nSUCCESS: real MiniMax API responded.");
    } else {
        println!("\nFAILURE: see task result_json above.");
    }
    Ok(())
}

fn read_api_key() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.is_empty() {
            return Ok(k);
        }
    }
    let path = PathBuf::from(r".openclaw\apikey.txt");
    let content = std::fs::read_to_string(&path)?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("empty api key in {}", path.display()).into());
    }
    Ok(trimmed)
}
