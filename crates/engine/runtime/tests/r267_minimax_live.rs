//! R267: live MiniMax API dispatch tests (gated by APEIRETH_MINIMAX_LIVE_TEST=1).
//!
//! 与 examples/r257_minimax_real_api.rs 端到端等价, 但写成 #[tokio::test] 让
//! cargo test 能跑. 默认 ignored (CI 不发真请求), 设 env 变量 APEIRETH_MINIMAX_LIVE_TEST=1 启用.
//!
//! 运行: APEIRETH_MINIMAX_LIVE_TEST=1 cargo test -p apeireth-runtime --test r267_minimax_live -- --nocapture

#![allow(missing_docs)]

use apeireth_runtime::{LlmWorker, Runtime, RuntimeConfig};
use apeireth_tool_registry::TaskStatus;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn read_api_key() -> Option<String> {
    if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.is_empty() {
            return Some(k);
        }
    }
    let path = PathBuf::from(if cfg!(windows) {
        r".openclaw\apikey.txt"
    } else {
        ".openclaw/apikey.txt"
    });
    if !path.exists() {
        // try home
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let alt = PathBuf::from(home).join(".openclaw").join("apikey.txt");
            if alt.exists() {
                return std::fs::read_to_string(&alt)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
            }
        }
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn build_runtime_with_llm(api_key: String) -> Arc<Runtime> {
    let mut config = RuntimeConfig::default();
    config.tick_interval = Duration::from_secs(60);
    config.arbitration_path = None;
    let rt = Arc::new(Runtime::with_config(config));
    rt.bootstrap().expect("bootstrap");
    let worker: Arc<dyn apeireth_runtime::AsyncWorker> = Arc::new(LlmWorker::new("llm", api_key));
    rt.register_worker("llm", worker);
    rt
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r267_live_minimax_returns_completed() {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").ok().as_deref() != Some("1") {
        eprintln!("skipped: set APEIRETH_MINIMAX_LIVE_TEST=1 to run live test");
        return;
    }
    let api_key = read_api_key().expect("API key required");
    let rt = build_runtime_with_llm(api_key);

    let params = serde_json::json!({"prompt": "Reply with exactly: hello from MiniMax", "system": "be terse"});
    let task_id = rt.dispatch_async_task("llm", &params.to_string()).await;

    let rec = rt
        .task_store
        .wait_for_completion(task_id, Duration::from_secs(45))
        .await
        .expect("wait_for_completion");
    assert_eq!(
        rec.status,
        TaskStatus::Completed,
        "expected Completed, got {:?}",
        rec.status
    );
    let result = rec.result_json.expect("result_json");
    assert!(
        result.contains("hello from MiniMax"),
        "result must contain expected text: {}",
        result
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r267_live_minimax_dispatch_llm_task_helper() {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").ok().as_deref() != Some("1") {
        eprintln!("skipped: set APEIRETH_MINIMAX_LIVE_TEST=1 to run live test");
        return;
    }
    let api_key = read_api_key().expect("API key required");

    let rt = Arc::new(Runtime::with_config(RuntimeConfig::default()));
    rt.bootstrap().expect("bootstrap");

    // dispatch_llm_task 内部 spawn 一个 wait_for_completion 后台 task 拿 metrics,
    // 所以外面不能再 wait_for_completion (ReceiverDropped race).
    // 用 self-poll get() 替代.
    let task_id = rt
        .dispatch_llm_task(
            "Reply with exactly: dispatch-llm-task-ok",
            Some("be terse"),
            None,
            None,
            &api_key,
        )
        .await;

    let mut last = None;
    for _ in 0..150 {
        if let Some(rec) = rt.task_store.get(task_id).await {
            if matches!(rec.status, TaskStatus::Completed | TaskStatus::Failed) {
                last = Some(rec);
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let rec = last.expect("task should complete within 45s");
    assert_eq!(
        rec.status,
        TaskStatus::Completed,
        "expected Completed, got {:?}",
        rec.status
    );
    let result = rec.result_json.expect("result_json");
    assert!(
        result.contains("dispatch-llm-task-ok"),
        "result must contain expected text: {}",
        result
    );
}
