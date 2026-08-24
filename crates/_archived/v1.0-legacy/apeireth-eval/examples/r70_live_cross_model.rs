//! R70 LIVE: 真接 MiniMax 8 model benchmark example (env-gated `APEIRETH_EVAL_LIVE=1`)
//!
//! **目标**: 跑 cross_model_benchmark on 8 model (DEFAULT_MODELS + EXTENDED_MODELS dedup),
//! 拿 Markdown 报告 + latency 统计. 借鉴 R32-3-2 example (per master 8/9 拍板 LIVE 真接模式).
//!
//! **跑法**:
//! ```bash
//! APEIRETH_EVAL_LIVE=1 cargo run --example r70_live_cross_model --release
//! ```
//!
//! **CI gating**: 默认 `APEIRETH_EVAL_LIVE` 未设置时跳过 (0 网络环境, 0 flaky).

use apeireth_eval::cross_model_benchmark::{
    report_to_markdown, run_cross_model_benchmark, BenchmarkConfig, DEFAULT_BENCHMARK_PROMPT,
    DEFAULT_MODELS, EXTENDED_MODELS,
};
use apeireth_eval::real_llm_smoke::{
    load_api_key, ANTHROPIC_MESSAGES_PATH, ANTHROPIC_VERSION, MINIMAX_BASE_URL,
};

#[tokio::main]
async fn main() {
    if std::env::var("APEIRETH_EVAL_LIVE").ok().as_deref() != Some("1") {
        eprintln!("R70 LIVE example: skip (set APEIRETH_EVAL_LIVE=1 to enable real MiniMax calls)");
        eprintln!(
            "Note: this hits https://api.minimaxi.com with x-api-key from openclaw/apikey.txt"
        );
        std::process::exit(0);
    }

    let (api_key, apikey_source) = match load_api_key(None) {
        Ok((k, s)) => (k, s),
        Err(e) => {
            eprintln!("R70 LIVE example: failed to load apikey: {e}");
            eprintln!("expected: C:\\Users\\REDACTED\\.openclaw\\apikey.txt");
            std::process::exit(1);
        }
    };

    let mut all_models: Vec<String> = DEFAULT_MODELS.iter().map(|s| (*s).to_string()).collect();
    for m in EXTENDED_MODELS {
        if !all_models.contains(&(*m).to_string()) {
            all_models.push((*m).to_string());
        }
    }

    eprintln!(
        "R70 LIVE: base={} path={} version={} apikey_source={}",
        MINIMAX_BASE_URL, ANTHROPIC_MESSAGES_PATH, ANTHROPIC_VERSION, apikey_source
    );
    eprintln!(
        "R70 LIVE: running {} models: {:?}",
        all_models.len(),
        all_models
    );

    let cfg = BenchmarkConfig {
        models: all_models.clone(),
        prompt: DEFAULT_BENCHMARK_PROMPT.to_string(),
        max_tokens: 512,
        timeout: std::time::Duration::from_secs(60),
    };

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let report = run_cross_model_benchmark(&workspace_root, Some(&api_key), Some(cfg)).await;

    let md = report_to_markdown(&report);
    println!("{}", md);

    eprintln!(
        "R70 LIVE summary: {}/{} pass (rate {:.0}%); total {}ms",
        report.pass_count,
        report.total_count,
        report.pass_rate * 100.0,
        report.total_latency_ms
    );
}
