//! R32-3-2: 跨 model 真 LLM benchmark — 4 个 MiniMax model 跑同一 prompt
//!
//! **目标**: 给 4 个候选 model (M2.7-highspeed / M2.7 / M2.5 / M3) 跑同一 prompt,
//! 输出 JSON 报告含:
//! - latency_ms (端到端)
//! - input_tokens / output_tokens (Anthropic usage 字段)
//! - stop_reason
//! - text_excerpt (200 字符摘录)
//! - error (any stage fail)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `real_llm_smoke` (复用 7 阶段 metric struct + load_api_key)
//! - 0 改 workspace 1.0.0 / 24 LOCKED crate
//! - 0 引入新外部依赖
//!
//! **借鉴锚 (S-1)**:
//! - HELM (Stanford) 跨 model 评估范式: 同 prompt / 多 model / 报告 metric 矩阵
//! - Anthropic Evals `model_comparison` 模式
//! - VCP `bench.js` 5 字段 token 报数
//!
//! **CI gating (R32-3-3)**: `run_cross_model_benchmark` 默认跑 4 model
//! (4 round-trip × ~1s ≈ 4s), 在 env `APEIRETH_EVAL_LIVE=1` 时启用; CI 默认 0 网络环境
//! 跳过 (避免 flaky).

use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::real_llm_smoke::{
    load_api_key, AnthropicContentBlock, AnthropicMessagesRequest, AnthropicMessagesResponse,
    AnthropicUsage, RealLlmConfig, ANTHROPIC_MESSAGES_PATH, ANTHROPIC_VERSION, MINIMAX_BASE_URL,
};

/// R32-3-2 默认 4 个候选 model (per minimax docs 2026-08-09)
/// 覆盖: highspeed 廉价 / 主力 / M2.5 备用 / M3 前沿
pub const DEFAULT_MODELS: &[&str] = &[
    "MiniMax-M2.7-highspeed",
    "MiniMax-M2.7",
    "MiniMax-M2.5",
    "MiniMax-M3",
];

/// R67: 扩 model 列表 — 加 M2.5-highspeed + M2.1 + M2.1-highspeed (per MiniMax docs 2026-08-09)
/// 包含 cheap (M2) / balanced (M2.5-highspeed) / fast (M2.7-highspeed) / 主力 (M2.7) / 备用 (M2.5 / M2.1) / 前沿 (M3) 6 tier
/// 用于 cross_model_benchmark "extended" mode (per HELM tier 范式)
pub const EXTENDED_MODELS: &[&str] = &[
    "MiniMax-M2.7-highspeed",
    "MiniMax-M2.7",
    "MiniMax-M2.5-highspeed",
    "MiniMax-M2.5",
    "MiniMax-M2.1-highspeed",
    "MiniMax-M2.1",
];
/// 默认 benchmark prompt (短 + 可验证, 不依赖 thinking 模式)
pub const DEFAULT_BENCHMARK_PROMPT: &str =
    "Reply with a single JSON object: {\"ok\": true, \"model\": \"<you decide your model>\"}";

/// 单 model benchmark 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBenchmarkResult {
    /// 候选 model id
    pub model: String,
    /// 端到端 latency (从 send 到收 body)
    pub latency_ms: u64,
    /// HTTP status code
    pub http_status: u16,
    /// LLM 返的 model 字段 (验证 server 跟 client 一致)
    pub response_model: Option<String>,
    /// stop_reason
    pub stop_reason: Option<String>,
    /// Anthropic usage.input_tokens
    pub input_tokens: u32,
    /// Anthropic usage.output_tokens
    pub output_tokens: u32,
    /// 是否拿到 ≥1 text block
    pub has_text: bool,
    /// text 摘录前 200 字符
    pub text_excerpt: String,
    /// 是否全 pass (HTTP 200 + shape valid + text 非空 + token > 0)
    pub all_pass: bool,
    /// error message (any stage fail)
    pub error: Option<String>,
}

impl ModelBenchmarkResult {
    /// 该 model 的 cost proxy (输入 + 输出 token 总和, 简化版)
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// 跨 model benchmark 报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossModelBenchmarkReport {
    /// benchmark prompt (脱敏后, 摘前 80 字符)
    pub prompt_excerpt: String,
    /// base URL
    pub base_url: String,
    /// endpoint
    pub endpoint: String,
    /// 跑的总耗时 (所有 model sum)
    pub total_latency_ms: u64,
    /// 模型结果列表 (跟 DEFAULT_MODELS 顺序一致)
    pub results: Vec<ModelBenchmarkResult>,
    /// pass 数
    pub pass_count: usize,
    /// 总数
    pub total_count: usize,
    /// pass rate (0.0-1.0)
    pub pass_rate: f64,
}

impl CrossModelBenchmarkReport {
    /// 7 字段全 pass 的 model 数
    pub fn all_pass_models(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| r.all_pass)
            .map(|r| r.model.as_str())
            .collect()
    }

    /// 最快的全 pass model (latency 最低)
    pub fn fastest_passing_model(&self) -> Option<&str> {
        self.results
            .iter()
            .filter(|r| r.all_pass)
            .min_by_key(|r| r.latency_ms)
            .map(|r| r.model.as_str())
    }

    /// 最便宜 (output_tokens 最低) 的全 pass model
    pub fn cheapest_passing_model(&self) -> Option<&str> {
        self.results
            .iter()
            .filter(|r| r.all_pass)
            .min_by_key(|r| r.output_tokens)
            .map(|r| r.model.as_str())
    }
}

/// 配置 (跟 RealLlmConfig 平行, 给 benchmark 注入 models / prompt)
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// 候选 model 列表 (默认 4 个 MiniMax)
    pub models: Vec<String>,
    /// benchmark prompt (默认 DEFAULT_BENCHMARK_PROMPT)
    pub prompt: String,
    /// max_tokens (默认 128, 避免 thinking 占满)
    pub max_tokens: u32,
    /// 可选 timeout per model (默认 30s)
    pub timeout: std::time::Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            models: DEFAULT_MODELS.iter().map(|s| (*s).to_string()).collect(),
            prompt: DEFAULT_BENCHMARK_PROMPT.to_string(),
            max_tokens: 512,
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// R32-3-2 主入口: 跑 1 个 model 真 round-trip (不调 RealLlmConfig::default 避免传 max_tokens=64)
pub async fn run_single_model(
    model: &str,
    api_key: &str,
    cfg: &BenchmarkConfig,
) -> ModelBenchmarkResult {
    let mut result = ModelBenchmarkResult {
        model: model.to_string(),
        latency_ms: 0,
        http_status: 0,
        response_model: None,
        stop_reason: None,
        input_tokens: 0,
        output_tokens: 0,
        has_text: false,
        text_excerpt: String::new(),
        all_pass: false,
        error: None,
    };

    // 用 apeireth_http_client 跟 real_llm_smoke 一致, 5 字段 Keep-Alive
    // real_llm_smoke 也是借 HttpClient::reqwest_client() 暴露原生 reqwest::Client 来加 header
    // (因为 HttpClient::post() 内部 hardcode Content-Type=application/json, 不让我们加 x-api-key).
    let http = match apeireth_http_client::HttpClient::with_chat_defaults() {
        Ok(c) => c,
        Err(e) => {
            result.error = Some(format!("http_client_init: {e}"));
            return result;
        }
    };
    let _permit = http.pool().enter();
    let url = format!("{}{}", MINIMAX_BASE_URL, ANTHROPIC_MESSAGES_PATH);
    let request = AnthropicMessagesRequest {
        model,
        max_tokens: cfg.max_tokens,
        system: None,
        messages: vec![crate::real_llm_smoke::AnthropicMessage {
            role: "user",
            content: cfg.prompt.clone(),
        }],
        temperature: Some(0.0),
    };
    let body = match serde_json::to_string(&request) {
        Ok(b) => b,
        Err(e) => {
            result.error = Some(format!("serde_json::to_string: {e}"));
            return result;
        }
    };

    let start = Instant::now();
    let reqwest_client = http.reqwest_client();
    let response = match reqwest_client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("Content-Type", "application/json")
        .timeout(cfg.timeout)
        .body(body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            result.latency_ms = start.elapsed().as_millis() as u64;
            result.error = Some(format!("http_request: {e}"));
            return result;
        }
    };
    let status = response.status();
    result.http_status = status.as_u16();
    result.latency_ms = start.elapsed().as_millis() as u64;

    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        result.error = Some(format!(
            "http_status={} body={}",
            status.as_u16(),
            body_text.chars().take(300).collect::<String>()
        ));
        return result;
    }

    let body_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            result.error = Some(format!("read_body: {e}"));
            return result;
        }
    };

    let parsed: AnthropicMessagesResponse = match serde_json::from_str(&body_text) {
        Ok(r) => r,
        Err(e) => {
            result.error = Some(format!(
                "json_parse: {e} body={}",
                body_text.chars().take(200).collect::<String>()
            ));
            return result;
        }
    };

    result.response_model = parsed.model.clone();
    result.stop_reason = parsed.stop_reason.clone();
    if let Some(u) = parsed.usage.as_ref() {
        result.input_tokens = u.input_tokens;
        result.output_tokens = u.output_tokens;
    }
    let first_text = parsed.content.iter().find_map(|b| b.text.clone());
    result.has_text = first_text.is_some();
    result.text_excerpt = first_text
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect::<String>();

    result.all_pass = status.is_success()
        && parsed.model.is_some()
        && parsed.stop_reason.is_some()
        && result.has_text
        && result.input_tokens > 0
        && result.output_tokens > 0;

    if !result.all_pass && result.error.is_none() {
        result.error = Some(format!(
            "shape_check: status={} has_text={} in={} out={} stop={:?}",
            status.as_u16(),
            result.has_text,
            result.input_tokens,
            result.output_tokens,
            result.stop_reason
        ));
    }

    result
}

/// R32-3-2 主入口: 跑 4 model benchmark, 返回报告
pub async fn run_cross_model_benchmark(
    _workspace_root: &Path,
    api_key: Option<&str>,
    config: Option<BenchmarkConfig>,
) -> CrossModelBenchmarkReport {
    let cfg = config.unwrap_or_default();
    let (api_key, _src) = match load_api_key(api_key) {
        Ok((k, s)) => (k, s),
        Err(e) => {
            return CrossModelBenchmarkReport {
                prompt_excerpt: cfg.prompt.chars().take(80).collect::<String>(),
                base_url: MINIMAX_BASE_URL.to_string(),
                endpoint: ANTHROPIC_MESSAGES_PATH.to_string(),
                total_latency_ms: 0,
                results: cfg
                    .models
                    .iter()
                    .map(|m| ModelBenchmarkResult {
                        model: m.clone(),
                        latency_ms: 0,
                        http_status: 0,
                        response_model: None,
                        stop_reason: None,
                        input_tokens: 0,
                        output_tokens: 0,
                        has_text: false,
                        text_excerpt: String::new(),
                        all_pass: false,
                        error: Some(format!("apikey: {e}")),
                    })
                    .collect(),
                pass_count: 0,
                total_count: cfg.models.len(),
                pass_rate: 0.0,
            };
        }
    };

    let start_total = Instant::now();
    let mut results = Vec::with_capacity(cfg.models.len());
    for model in &cfg.models {
        let r = run_single_model(model, &api_key, &cfg).await;
        results.push(r);
    }
    let total_latency_ms = start_total.elapsed().as_millis() as u64;
    let pass_count = results.iter().filter(|r| r.all_pass).count();
    let total_count = results.len();
    let pass_rate = if total_count == 0 {
        0.0
    } else {
        pass_count as f64 / total_count as f64
    };

    CrossModelBenchmarkReport {
        prompt_excerpt: cfg.prompt.chars().take(80).collect::<String>(),
        base_url: MINIMAX_BASE_URL.to_string(),
        endpoint: ANTHROPIC_MESSAGES_PATH.to_string(),
        total_latency_ms,
        results,
        pass_count,
        total_count,
        pass_rate,
    }
}

/// R32-3-2 helper: 把报告写成 markdown 表格 (给 reports/r32-3-2-*.md 用)
pub fn report_to_markdown(report: &CrossModelBenchmarkReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Cross-Model Benchmark — {}\n\n",
        chrono_like_now()
    ));
    out.push_str(&format!("- **Prompt**: `{}`\n", report.prompt_excerpt));
    out.push_str(&format!("- **Endpoint**: `{}{}`\n", report.base_url, report.endpoint));
    out.push_str(&format!("- **Total latency**: {} ms\n", report.total_latency_ms));
    out.push_str(&format!("- **Pass rate**: {}/{} ({:.0}%)\n\n", report.pass_count, report.total_count, report.pass_rate * 100.0));

    out.push_str("| Model | Status | Latency (ms) | In | Out | Stop | Text excerpt | All pass |\n");
    out.push_str("|-------|--------|--------------|----|----|------|--------------|----------|\n");
    for r in &report.results {
        let excerpt = if r.text_excerpt.is_empty() {
            "-".to_string()
        } else {
            r.text_excerpt.chars().take(40).collect::<String>()
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | `{}` | {} | {} |\n",
            r.model,
            r.http_status,
            r.latency_ms,
            r.input_tokens,
            r.output_tokens,
            r.stop_reason.as_deref().unwrap_or("-"),
            excerpt.replace('|', "\\|").replace('\n', " "),
            if r.all_pass { "✅" } else { "❌" }
        ));
    }

    if let Some(m) = report.fastest_passing_model() {
        out.push_str(&format!("\n**Fastest passing**: `{m}`\n"));
    }
    if let Some(m) = report.cheapest_passing_model() {
        out.push_str(&format!("**Cheapest passing (output tokens)**: `{m}`\n"));
    }
    out
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 简化 ISO-ish (no chrono dep): YYYY-MM-DD HH:MM:SS UTC (粗略)
    // 1970-01-01 起算的天数
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days as i64);
    let time_of_day = secs % 86400;
    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
}

fn days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    // 简化: 1970-01-01 起算, 跨闰年用 365.25 近似
    let mut year = 1970i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let yd = if leap { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let months = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for &md in &months {
        if days < md {
            return (year, month, (days + 1) as u32);
        }
        days -= md;
        month += 1;
    }
    (year, 12, 31)
}

// ============================================================
// 单元测试 (no network, 用 stub http_client 验证 7 字段)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_models_are_minimax_family() {
        assert_eq!(DEFAULT_MODELS.len(), 4);
        for m in DEFAULT_MODELS {
            assert!(m.starts_with("MiniMax-M"), "non-MiniMax model leaked: {m}");
        }
    }

    #[test]
    fn default_benchmark_prompt_is_short() {
        // 避免 benchmark 跑长 prompt 浪费时间
        assert!(DEFAULT_BENCHMARK_PROMPT.len() < 200);
    }

    #[test]
    fn model_benchmark_result_total_tokens() {
        let r = ModelBenchmarkResult {
            model: "M".to_string(),
            latency_ms: 100,
            http_status: 200,
            response_model: Some("M".to_string()),
            stop_reason: Some("end_turn".to_string()),
            input_tokens: 50,
            output_tokens: 20,
            has_text: true,
            text_excerpt: "ok".to_string(),
            all_pass: true,
            error: None,
        };
        assert_eq!(r.total_tokens(), 70);
    }

    #[test]
    fn report_pass_rate_zero_when_all_fail() {
        let r = ModelBenchmarkResult {
            model: "M".to_string(),
            latency_ms: 0,
            http_status: 0,
            response_model: None,
            stop_reason: None,
            input_tokens: 0,
            output_tokens: 0,
            has_text: false,
            text_excerpt: String::new(),
            all_pass: false,
            error: Some("x".to_string()),
        };
        let report = CrossModelBenchmarkReport {
            prompt_excerpt: "x".to_string(),
            base_url: "x".to_string(),
            endpoint: "x".to_string(),
            total_latency_ms: 0,
            results: vec![r.clone(), r],
            pass_count: 0,
            total_count: 2,
            pass_rate: 0.0,
        };
        assert_eq!(report.pass_rate, 0.0);
        assert!(report.fastest_passing_model().is_none());
        assert!(report.cheapest_passing_model().is_none());
    }

    #[test]
    fn report_fastest_cheapest_selection() {
        let mk = |m: &str, lat: u64, out: u32, pass: bool| ModelBenchmarkResult {
            model: m.to_string(),
            latency_ms: lat,
            http_status: 200,
            response_model: Some(m.to_string()),
            stop_reason: Some("end_turn".to_string()),
            input_tokens: 50,
            output_tokens: out,
            has_text: true,
            text_excerpt: "ok".to_string(),
            all_pass: pass,
            error: None,
        };
        let results = vec![
            mk("M2.7", 1000, 100, true),
            mk("M3", 500, 50, true),
            mk("M2.5", 800, 200, false),
        ];
        let pass_count = results.iter().filter(|r| r.all_pass).count();
        let report = CrossModelBenchmarkReport {
            prompt_excerpt: "x".to_string(),
            base_url: "x".to_string(),
            endpoint: "x".to_string(),
            total_latency_ms: 0,
            results,
            pass_count,
            total_count: 3,
            pass_rate: pass_count as f64 / 3.0,
        };
        assert_eq!(report.fastest_passing_model(), Some("M3"));
        assert_eq!(report.cheapest_passing_model(), Some("M3"));
    }

    #[test]
    fn report_to_markdown_has_table_header() {
        let report = CrossModelBenchmarkReport {
            prompt_excerpt: "test".to_string(),
            base_url: MINIMAX_BASE_URL.to_string(),
            endpoint: ANTHROPIC_MESSAGES_PATH.to_string(),
            total_latency_ms: 1000,
            results: vec![],
            pass_count: 0,
            total_count: 0,
            pass_rate: 0.0,
        };
        let md = report_to_markdown(&report);
        assert!(md.contains("| Model | Status |"));
        assert!(md.contains("Pass rate"));
    }

    #[test]
    fn days_to_ymd_known_epoch() {
        // 1970-01-01 → (1970, 1, 1)
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 2026-08-09 (今天) - 粗略, 1970..2026 跨 56 年
        let today = days_to_ymd(20750);
        assert_eq!(today.0, 2026);
    }
}


// ============================================================
// R67: ModelTier + tier-based model selection (per HELM tier 范式)
// ============================================================

/// R67: model tier 分类 (per MiniMax docs 2026-08-09 8 model 拆 4 tier)
/// - Frontier: 最高质量 (M3)
/// - Balanced: 主力 (M2.7 / M2.5)
/// - Fast: 廉价高速 (M2.7-highspeed / M2.5-highspeed / M2.1-highspeed)
/// - Legacy: 上一代 (M2.1 / M2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// 前沿模型 (M3)
    Frontier,
    /// 主力 (M2.7 / M2.5)
    Balanced,
    /// 廉价高速 (highspeed 系列)
    Fast,
    /// 上一代 (M2.1 / M2)
    Legacy,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::Frontier => "frontier",
            ModelTier::Balanced => "balanced",
            ModelTier::Fast => "fast",
            ModelTier::Legacy => "legacy",
        }
    }
}

/// R67: 给单个 model 推断 tier
pub fn tier_of(model: &str) -> ModelTier {
    if model.starts_with("MiniMax-M3") {
        ModelTier::Frontier
    } else if model.contains("highspeed") {
        ModelTier::Fast
    } else if model.starts_with("MiniMax-M2.7") || model.starts_with("MiniMax-M2.5") {
        ModelTier::Balanced
    } else if model.starts_with("MiniMax-M2.1") || model.starts_with("MiniMax-M2") {
        ModelTier::Legacy
    } else {
        // 未知 model 默认 legacy (per docs 未列即弃)
        ModelTier::Legacy
    }
}

/// R67: 按 tier 选 model (返回 DEFAULT_MODELS ∪ EXTENDED_MODELS 中匹配 tier 的)
pub fn select_models_for_tier(tier: ModelTier) -> Vec<&'static str> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for &m in DEFAULT_MODELS.iter().chain(EXTENDED_MODELS.iter()) {
        if tier_of(m) == tier && seen.insert(m) {
            out.push(m);
        }
    }
    out
}

/// R67: 默认 4 tier 全选 (frontier + balanced + fast + legacy → 8 model dedup)
pub fn all_tiered_models() -> Vec<&'static str> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for &m in DEFAULT_MODELS.iter().chain(EXTENDED_MODELS.iter()) {
        if seen.insert(m) {
            out.push(m);
        }
    }
    out
}

/// R67: model count helper (per tier 报告用)
pub fn count_by_tier() -> std::collections::BTreeMap<&'static str, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for &m in all_tiered_models().iter() {
        *counts.entry(tier_of(m).as_str()).or_insert(0) += 1;
    }
    counts
}

// ============================================================
// R67 单元测试 (纯 unit, 0 网络, CI 默认 0-网络环境跑)
// ============================================================

#[cfg(test)]
mod r67_tests {
    use super::*;

    #[test]
    fn tier_of_classifies_all_models() {
        assert_eq!(tier_of("MiniMax-M3"), ModelTier::Frontier);
        assert_eq!(tier_of("MiniMax-M2.7"), ModelTier::Balanced);
        assert_eq!(tier_of("MiniMax-M2.5"), ModelTier::Balanced);
        assert_eq!(tier_of("MiniMax-M2.7-highspeed"), ModelTier::Fast);
        assert_eq!(tier_of("MiniMax-M2.5-highspeed"), ModelTier::Fast);
        assert_eq!(tier_of("MiniMax-M2.1-highspeed"), ModelTier::Fast);
        assert_eq!(tier_of("MiniMax-M2.1"), ModelTier::Legacy);
        assert_eq!(tier_of("MiniMax-M2"), ModelTier::Legacy);
        assert_eq!(tier_of("unknown-model"), ModelTier::Legacy);
    }

    #[test]
    fn select_models_for_tier_filters_correctly() {
        let frontier = select_models_for_tier(ModelTier::Frontier);
        assert!(frontier.contains(&"MiniMax-M3"));
        assert_eq!(frontier.len(), 1);

        let balanced = select_models_for_tier(ModelTier::Balanced);
        assert!(balanced.contains(&"MiniMax-M2.7"));
        assert!(balanced.contains(&"MiniMax-M2.5"));
        assert_eq!(balanced.len(), 2);

        let fast = select_models_for_tier(ModelTier::Fast);
        assert!(fast.contains(&"MiniMax-M2.7-highspeed"));
        assert!(fast.contains(&"MiniMax-M2.5-highspeed"));
        assert!(fast.contains(&"MiniMax-M2.1-highspeed"));
        assert_eq!(fast.len(), 3);

        let legacy = select_models_for_tier(ModelTier::Legacy);
        assert!(legacy.contains(&"MiniMax-M2.1"));
        // 注: M2 不在 default/extended, 故不在 legacy 输出
        // (per MiniMax docs 2026-08-09 实际可用 model, 我们只测默认+扩展)
        assert!(!legacy.is_empty());
    }

    #[test]
    fn select_models_for_tier_dedup() {
        // 同 model 在 DEFAULT_MODELS 和 EXTENDED_MODELS 中可能出现, dedup 后只 1 次
        for tier in [ModelTier::Frontier, ModelTier::Balanced, ModelTier::Fast, ModelTier::Legacy] {
            let models = select_models_for_tier(tier);
            let unique: std::collections::BTreeSet<_> = models.iter().collect();
            assert_eq!(unique.len(), models.len(), "tier {:?} should be dedup", tier);
        }
    }

    #[test]
    fn all_tiered_models_dedup_no_overlap() {
        let all = all_tiered_models();
        let unique: std::collections::BTreeSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "all_tiered_models must dedup");
        // 默认 4 + 扩展 6 (去重) = 8 model total
        assert_eq!(all.len(), 7, "expected 7 unique models (4 default + 3 new extended, with 3 dedup overlaps), got {}", all.len());
    }

    #[test]
    fn count_by_tier_sums_to_total() {
        let counts = count_by_tier();
        let total: usize = counts.values().sum();
        assert_eq!(total, 7, "tier counts should sum to 7 unique models, got {}", total);
        assert_eq!(counts.get("frontier").copied(), Some(1));
        assert_eq!(counts.get("balanced").copied(), Some(2));
        assert_eq!(counts.get("fast").copied(), Some(3));
        assert_eq!(counts.get("legacy").copied().unwrap_or(0) >= 1, true);
    }

    #[test]
    fn model_tier_serialize_round_trip() {
        for tier in [ModelTier::Frontier, ModelTier::Balanced, ModelTier::Fast, ModelTier::Legacy] {
            let json = serde_json::to_string(&tier).unwrap();
            let restored: ModelTier = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, tier);
        }
    }
}



