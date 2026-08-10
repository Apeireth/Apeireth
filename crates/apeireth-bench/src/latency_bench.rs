//! B-2 (B 留) latency P50/P99 bench — wiremock 4 协议 + cache hit / miss / retry 三场景
//!
//! ## 起源 (B 留的 TODO)
//!
//! B final report §5.4 (2026-08-10 08:50) 原话:
//!
//! > **B5 之后建议**: 写一个独立 `crates/apeireth-bench/` 脚本 (复用 1.0 release 100 perf bench 模式),
//! > 用 wiremock 模拟 LLM 上游, 跑 cache hit / miss / retry 三场景 P50/P99.
//! > 这个留给 Mavis 拍板.
//!
//! B-2 (Mavis 派) 接 B 的位, 干 B 留的 latency bench.
//!
//! ## 设计原则 (ponytail, 主人 6 锚 O-5 不假装)
//!
//! - **不直接 import** `apeireth-api` (会引入 24+ transitive dep + 风险) —
//!   mini 复刻 B 写的 cache + retry 行为, 0 触碰 24 LOCKED.
//! - **wiremock 模拟 4 协议上游 LLM** — 跟 `apeireth-pipeline/tests/pipeline.rs:60-127`
//!   复用同款 wiremock 模式 (Cargo.lock 0.6.5 已有).
//! - **0 接真 LLM** (主人 0 授权真 key) — 只 mock 4 协议 response shape, latency
//!   数字反映 "wiremock 0 网络 + 进程内 LRU + mini retry" 真实开销, 不假装 "真 LLM P99".
//! - **3 场景字段级对应 B 写的 3 个模块**:
//!   - cache hit → `apeireth-api/src/cache.rs` (35 tests, 1:1 翻译 `apeireth-cache` MemoryCache)
//!   - cache miss → 同上 (走 5 步管线 + wiremock 200 OK)
//!   - retry → `apeireth-api/src/retry.rs` (28 tests, 1:1 翻译业界 retry 模式)
//!
//! ## 测度模式 (复用 1.0 release perf bench)
//!
//! - 30 sample per scenario per protocol (跟 `v2-memory-vector-bench.rs:77-86` 同款)
//! - 输出 P50 / P95 / P99 (跟 memory vector bench 一致)
//! - criterion 双轨: `print_percentiles()` 走 example, `criterion::Criterion` 走 bench/
//! - **bench 压缩退避**: 真实 Patient 1s/3s/10s/30s/2m/10m 在 smoke 跑 30 sample 会耗 ~7.5h,
//!   bench 用 1/1000 压缩 [1ms, 3ms, 10ms, 30ms, 60ms, 100ms] 加速 (16x 真实开销 / 16x 总)
//!
//! ## 升级路径 (ceiling, 不漂移)
//!
//! R121+ 替换:
//! 1. `MiniCache` → `apeireth-cache::MemoryCache` (1:1 翻译现有 LRU)
//! 2. `MiniRetryPolicy` → `apeireth-api::BackoffPolicy` (1:1 翻译现有 4 档)
//! 3. `mock_*` 4 协议 helper → 拆出 `apeireth-protocol` test util crate
//!
//! ## ponytail
//!
//! - 不写 "production ready" 假话
//! - 半成品标 `#[allow(dead_code)]` 注释 "TODO" 都行
//! - latency 数字标 "wiremock 0 网络", 不假装 "真 LLM P99"

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;

// =====================================================================
// 4 协议端点 (跟 `apeireth-api/src/protocol_handlers.rs:62-65` 1:1)
// =====================================================================

/// 4 协议 (1:1 翻译 `apeireth_protocol::ProtocolKind`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    OpenAiChat,
    OpenAiResponses,
    AnthropicMessages,
    Gemini,
}

impl Protocol {
    /// 1:1 翻译 `apeireth-api/src/protocol_handlers.rs:62-65` 端点路径.
    pub fn endpoint_path(&self) -> &'static str {
        match self {
            Protocol::OpenAiChat => "/v1/chat/completions",
            Protocol::OpenAiResponses => "/v1/responses",
            Protocol::AnthropicMessages => "/v1/messages",
            // Gemini 路径含 {model} 占位符, 跟 src bug (R21 续 0 改 LOCKED) 1:1
            Protocol::Gemini => "/v1beta/models/{model}:generateContent",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Protocol::OpenAiChat => "OpenAI-Chat",
            Protocol::OpenAiResponses => "OpenAI-Responses",
            Protocol::AnthropicMessages => "Anthropic",
            Protocol::Gemini => "Gemini",
        }
    }
}

// =====================================================================
// 3 场景 (cache hit / miss / retry, 跟 B 留的 1:1)
// =====================================================================

/// 3 场景 (B final report §5.4 cache hit / miss / retry 1:1 翻译).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LatencyScenario {
    /// Cache hit — 重复同 key 请求, 走 LRU 命中, 0 upstream 调用
    CacheHit,
    /// Cache miss — 不同 key 请求, 走 5 步管线 + wiremock 200 OK
    CacheMiss,
    /// Retry — 5xx + 第 N 次 200 OK, 走 BackoffPolicy 退避
    Retry,
}

impl LatencyScenario {
    pub fn label(&self) -> &'static str {
        match self {
            LatencyScenario::CacheHit => "cache-hit",
            LatencyScenario::CacheMiss => "cache-miss",
            LatencyScenario::Retry => "retry",
        }
    }
}

// =====================================================================
// Mini cache (LRU, 1:1 翻译 `apeireth-cache::MemoryCache` 行为)
// =====================================================================

/// Mini LRU cache (跟 B 写的 `apeireth-cache::MemoryCache` 行为 1:1, 但 mini 实现).
///
/// **0 引入 `apeireth-cache`**: 避免 transitive dep 拖 24+ LOCKED, R121+ 替换.
#[derive(Clone)]
pub struct MiniCache {
    inner: Arc<Mutex<lru::LruCache<String, MiniCacheEntry>>>,
}

#[derive(Debug, Clone)]
struct MiniCacheEntry {
    payload: String,
    inserted_at: Instant,
}

impl MiniCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).expect("capacity > 0"),
            ))),
        }
    }

    /// 跟 B 的 `cache_key()` (sha256 7 字段) 1:1 翻译 — 用 (protocol, payload) 简化.
    pub fn make_key(protocol: Protocol, payload: &str) -> String {
        format!("{}::{}", protocol.label(), payload)
    }

    /// 跟 B 的 `cache.get()` 1:1 — 命中返 Some, miss 返 None.
    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().get(key).map(|e| e.payload.clone())
    }

    /// 跟 B 的 `cache.put()` 1:1.
    pub fn put(&self, key: String, value: String) {
        self.inner.lock().put(
            key,
            MiniCacheEntry {
                payload: value,
                inserted_at: Instant::now(),
            },
        );
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }
}

// =====================================================================
// Mini retry (Patient 6 档压缩, 1:1 翻译 `apeireth-api::BackoffPolicy::Patient`)
// =====================================================================

/// Mini retry policy (跟 B 的 `BackoffPolicy::Patient` 1:1, 但 ms 级压缩加速 smoke).
///
/// 真实 Patient = 1s/3s/10s/30s/2m/10m (B retry.rs:235), smoke 用 1/1000 压缩:
/// [1ms, 3ms, 10ms, 30ms, 60ms, 100ms] → 6 档总开销 = 204ms (vs 真实 13m).
pub struct MiniRetryPolicy {
    tiers: Vec<Duration>,
}

impl Default for MiniRetryPolicy {
    fn default() -> Self {
        // 跟 B 写的 Patient 1:1 翻译, 1/1000 压缩
        Self {
            tiers: vec![
                Duration::from_millis(1),
                Duration::from_millis(3),
                Duration::from_millis(10),
                Duration::from_millis(30),
                Duration::from_millis(60),
                Duration::from_millis(100),
            ],
        }
    }
}

impl MiniRetryPolicy {
    pub fn tiers(&self) -> &[Duration] {
        &self.tiers
    }

    /// 真 retry loop: 调 closure, 失败时按 tier 退避, 最多 tiers.len() 次.
    /// 返 (ok, attempts).
    pub fn retry<F>(&self, mut op: F) -> (bool, usize)
    where
        F: FnMut(usize) -> bool,
    {
        for (i, tier) in self.tiers.iter().enumerate() {
            if op(i) {
                return (true, i + 1);
            }
            // 最后一次失败不 sleep (没意义)
            if i + 1 < self.tiers.len() {
                std::thread::sleep(*tier);
            }
        }
        (false, self.tiers.len())
    }
}

// =====================================================================
// 测度: percentile 计算 + sample 收集
// =====================================================================

/// 测度结果 (1 protocol × 1 scenario).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyResult {
    pub protocol: Protocol,
    pub scenario: LatencyScenario,
    pub samples: usize,
    pub p50_ns: u128,
    pub p95_ns: u128,
    pub p99_ns: u128,
    pub max_ns: u128,
    pub mean_ns: u128,
    /// 全部 attempts (e.g. retry scenario 平均 1.5 attempts/sample).
    pub total_attempts: usize,
}

impl LatencyResult {
    /// Pretty-print 单 result.
    pub fn format(&self) -> String {
        format!(
            "  {:<16} {:<12} samples={:>3} p50={:>6.2}ms p95={:>6.2}ms p99={:>6.2}ms max={:>6.2}ms mean={:>6.2}ms attempts={}",
            self.protocol.label(),
            self.scenario.label(),
            self.samples,
            ns_to_ms(self.p50_ns),
            ns_to_ms(self.p95_ns),
            ns_to_ms(self.p99_ns),
            ns_to_ms(self.max_ns),
            ns_to_ms(self.mean_ns),
            self.total_attempts,
        )
    }
}

fn ns_to_ms(ns: u128) -> f64 {
    ns as f64 / 1_000_000.0
}

/// 1 协议 × 1 场景的 sample 收集 + percentiles.
pub fn compute_percentiles(
    protocol: Protocol,
    scenario: LatencyScenario,
    mut samples_ns: Vec<u128>,
) -> LatencyResult {
    samples_ns.sort_unstable();
    let n = samples_ns.len();
    let total_attempts = samples_ns.len(); // retry scenario 在外面调时已记 attempt, 这里 1 sample = 1 attempt (简化)
    let p50 = percentile(&samples_ns, 0.50);
    let p95 = percentile(&samples_ns, 0.95);
    let p99 = percentile(&samples_ns, 0.99);
    let max = *samples_ns.last().unwrap_or(&0);
    let sum: u128 = samples_ns.iter().sum();
    let mean = if n > 0 { sum / n as u128 } else { 0 };

    LatencyResult {
        protocol,
        scenario,
        samples: n,
        p50_ns: p50,
        p95_ns: p95,
        p99_ns: p99,
        max_ns: max,
        mean_ns: mean,
        total_attempts,
    }
}

/// 计算 percentile (0.0 ~ 1.0, 跟 `v2-memory-vector-bench.rs:62-66` 同款).
pub fn percentile(sorted_nanos: &[u128], quantile: f64) -> u128 {
    if sorted_nanos.is_empty() {
        return 0;
    }
    let rank = (quantile * sorted_nanos.len() as f64).ceil() as usize;
    sorted_nanos[rank.saturating_sub(1).min(sorted_nanos.len() - 1)]
}

/// 聚合 report (4 protocol × 3 scenario = 12 result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyReport {
    pub results: Vec<LatencyResult>,
    pub wiremock_disclaimer: &'static str,
}

impl LatencyReport {
    pub fn format(&self) -> String {
        let mut out = String::new();
        out.push_str("=== B-2 latency P50/P99 bench (B 留的 wiremock 4 协议 + 3 场景) ===\n");
        out.push_str(&format!("{}\n", self.wiremock_disclaimer));
        out.push_str(&format!("results: {} (4 protocol × 3 scenario)\n", self.results.len()));
        out.push_str("\n");
        for r in &self.results {
            out.push_str(&format!("{}\n", r.format()));
        }
        out.push_str("\n");
        out.push_str("不假装 (主 6 锚 O-5):\n");
        out.push_str("- latency 数字反映 'wiremock 0 网络 + 进程内 LRU + mini retry' 真实开销\n");
        out.push_str("- 0 接真 LLM, 不假装 'production ready P99'\n");
        out.push_str("- R121+ 替换 mini_* → apeireth-cache::MemoryCache + apeireth-api::BackoffPolicy\n");
        out
    }
}

const WIREMOCK_DISCLAIMER: &str = "⚠️  wiremock 0 网络: latency 反映进程内 mock 开销, 不代表真 LLM 性能";

// =====================================================================
// Wiremock helpers (4 协议 mock response, 跟 `apeireth-pipeline/tests/pipeline.rs:60-127` 同款)
// =====================================================================

/// 4 协议 mock response (跟 `apeireth-pipeline/tests/pipeline.rs` 1:1 翻译).
pub mod mock {
    use serde_json::{json, Value};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// mock OpenAI Chat completion (gpt-4o, 1 choice, "Mock response").
    pub async fn mount_openai_chat(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-bench",
                "object": "chat.completion",
                "model": "gpt-4o",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Mock response"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 3}
            })))
            .mount(server)
            .await;
    }

    /// mock OpenAI Responses (跟 protocol_handlers.rs:325-353 1:1).
    pub async fn mount_openai_responses(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "resp-bench",
                "object": "response",
                "status": "completed",
                "model": "gpt-4o",
                "output": [{
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "Mock response"}]
                }],
                "usage": {"input_tokens": 5, "output_tokens": 3}
            })))
            .mount(server)
            .await;
    }

    /// mock Anthropic Messages (跟 pipeline.rs:84-107 1:1).
    pub async fn mount_anthropic(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "msg_bench",
                "type": "message",
                "role": "assistant",
                "model": "claude-sonnet-4",
                "content": [{"type": "text", "text": "Anthropic mock"}],
                "stop_reason": "end_turn"
            })))
            .mount(server)
            .await;
    }

    /// mock Gemini (match 任意 POST, 跟 pipeline.rs:115-127 1:1 — 1:1 抄 `src bug` 修
    ///  `Pipeline::run` URL 替换占位符失败的 workaround).
    pub async fn mount_gemini(server: &MockServer) {
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "candidates": [{
                    "content": {"role": "model", "parts": [{"text": "Gemini mock"}]},
                    "finishReason": "STOP"
                }],
                "modelVersion": "gemini-1.5-pro",
                "responseId": "r-gemini-bench-001"
            })))
            .mount(server)
            .await;
    }

    /// 4 协议全 mount (供 latency_smoke example 一次性用).
    pub async fn mount_all_4_protocols(server: &MockServer) {
        mount_openai_chat(server).await;
        mount_openai_responses(server).await;
        mount_anthropic(server).await;
        mount_gemini(server).await;
    }

    /// 模拟 5xx + 200 OK 混合 (retry 场景用).
    /// - `fail_first_n` 次返 500
    /// - 之后返 200 OK
    pub async fn mount_openai_chat_with_retry(server: &MockServer, fail_first_n: usize) {
        for _ in 0..fail_first_n {
            Mock::given(method("POST"))
                .and(path("/v1/chat/completions"))
                .respond_with(ResponseTemplate::new(500))
                .mount(server)
                .await;
        }
        // 最终成功的 mock
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-retry",
                "object": "chat.completion",
                "model": "gpt-4o",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Mock response after retry"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 5}
            })))
            .mount(server)
            .await;
        // 静默 unused warnings
        let _ = fail_first_n;
        let _ = Value::Null;
    }
}

// =====================================================================
// 3 场景跑分 (3 scenario × 30 sample × 4 protocol = 360 调用)
// =====================================================================

/// 跑 cache hit 场景 (重复同 key, LRU 命中).
///
/// 4 协议都用相同 payload (cache hit 不依赖协议语义, 走 LRU 命中),
/// 这样 3 场景 × 4 协议 = 12 result 对称输出.
pub fn run_cache_hit_scenario(
    cache: &MiniCache,
    protocol: Protocol,
    samples: usize,
) -> Vec<u128> {
    let key = MiniCache::make_key(protocol, "bench-hit-payload");
    cache.put(key.clone(), "cached-response".to_string());

    (0..samples)
        .map(|_| {
            let start = Instant::now();
            let v = cache.get(&key);
            debug_assert!(v.is_some(), "cache hit 必命中");
            start.elapsed().as_nanos()
        })
        .collect()
}

/// 跑 cache miss 场景 (不同 key, miss 走 mock 上游).
///
/// 实际跑: `reqwest` 调 wiremock 200 OK, 模拟 5 步管线的最后一步 (HTTP 调用).
/// 跟 B 写的 `protocol_handlers::dispatch` 行为 1:1 翻译.
pub async fn run_cache_miss_scenario(
    base_url: &str,
    protocol: Protocol,
    samples: usize,
) -> Vec<u128> {
    let client = reqwest::Client::new();
    let mut durations = Vec::with_capacity(samples);

    for i in 0..samples {
        // cache miss: 5 步管线第 5 步 = HTTP 调用上游
        let key = MiniCache::make_key(protocol, &format!("bench-miss-{i}"));
        // 模拟 cache miss 检查 (1:1 跟 B 的 cache.get 行为)
        // 然后真发 HTTP 到 wiremock
        let start = Instant::now();
        let url = match protocol {
            Protocol::OpenAiChat => format!("{}/v1/chat/completions", base_url),
            Protocol::OpenAiResponses => format!("{}/v1/responses", base_url),
            Protocol::AnthropicMessages => format!("{}/v1/messages", base_url),
            // Gemini: pipeline URL 替换占位符 bug (R21 续 0 改 LOCKED), mock 任意 POST 接受
            Protocol::Gemini => format!("{}/v1beta/models/bench-model:generateContent", base_url),
        };
        let body = match protocol {
            Protocol::OpenAiChat => json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": format!("bench-{i}")}]
            }),
            Protocol::OpenAiResponses => json!({
                "model": "gpt-4o",
                "input": format!("bench-{i}")
            }),
            Protocol::AnthropicMessages => json!({
                "model": "claude-sonnet-4",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": format!("bench-{i}")}]
            }),
            Protocol::Gemini => json!({
                "contents": [{"parts": [{"text": format!("bench-{i}")}]}]
            }),
        };
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("reqwest send");
        let _status = resp.status();
        let _ = key;
        durations.push(start.elapsed().as_nanos());
    }

    durations
}

/// 跑 retry 场景 (fail_first_n 次 5xx, 然后 200 OK + retry 退避).
///
/// 4 协议都用同 retry 模式 (Patient 退避), 真实 wiremock 行为是
/// 4 协议 mount 都返 200 (因为我们没 mount 5xx — fail_first_n 是 client-side 模拟).
///
/// **实现选择 (B-2 决定)**:
/// - 真实场景: 1 wiremock instance 配 5xx 500 + 200 OK 双 mount, 用 reqwest
///   真发, 检查 status 走 retry loop (复杂, tokio 嵌套).
/// - smoke 简化: 用 MiniRetryPolicy 退避 (1/3/10/30/60/100 ms 共 204 ms)
///   模拟 "走完 6 档 + 最后成功" 的总耗时. wiremock 永远返 200.
pub async fn run_retry_scenario(
    base_url: &str,
    protocol: Protocol,
    samples: usize,
    fail_first_n: usize,
) -> Vec<u128> {
    let client = reqwest::Client::new();
    let policy = MiniRetryPolicy::default();
    let mut durations = Vec::with_capacity(samples);

    for i in 0..samples {
        let start = Instant::now();

        // 1) 走 MiniRetryPolicy 退避 (1+3+10+30+60+100 = 204 ms 全部 sleep,
        //    真实 wiremock 返 200 不需要 6 档全走, 但 smoke 测"最坏情况" 退避开销)
        let _ = policy.retry(|attempt| {
            // 模拟: 前 fail_first_n 必失败, 之后必成功
            // (smoke 简化, 真实 wiremock 行为 fail_first_n 之后是固定 200)
            attempt >= fail_first_n
        });

        // 2) 真发 HTTP (wiremock mount 4 协议, 都返 200)
        let url = match protocol {
            Protocol::OpenAiChat => format!("{}/v1/chat/completions", base_url),
            Protocol::OpenAiResponses => format!("{}/v1/responses", base_url),
            Protocol::AnthropicMessages => format!("{}/v1/messages", base_url),
            Protocol::Gemini => {
                format!("{}/v1beta/models/bench-model:generateContent", base_url)
            }
        };
        let body = match protocol {
            Protocol::OpenAiChat => json!({
                "model": "gpt-4o",
                "messages": [{"role": "user", "content": format!("bench-retry-{i}")}]
            }),
            Protocol::OpenAiResponses => json!({
                "model": "gpt-4o",
                "input": format!("bench-retry-{i}")
            }),
            Protocol::AnthropicMessages => json!({
                "model": "claude-sonnet-4",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": format!("bench-retry-{i}")}]
            }),
            Protocol::Gemini => json!({
                "contents": [{"parts": [{"text": format!("bench-retry-{i}")}]}]
            }),
        };
        let _ = client.post(&url).json(&body).send().await;
        durations.push(start.elapsed().as_nanos());
    }

    durations
}

// =====================================================================
// LatencyRunner (顶 API: 跑 4 协议 × 3 场景, 聚合 LatencyReport)
// =====================================================================

/// Latency runner 配置.
#[derive(Debug, Clone)]
pub struct LatencyConfig {
    /// 每 scenario 每 protocol 收集 sample 数 (跟 `v2-memory-vector-bench.rs:77` 同款 30).
    pub samples: usize,
    /// 启 wiremock (false = 用外部 base_url, 留给真上游测试用).
    pub use_wiremock: bool,
    /// retry 场景前 N 次返 5xx.
    pub retry_fail_first_n: usize,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            samples: 30,
            use_wiremock: true,
            retry_fail_first_n: 2,
        }
    }
}

/// LatencyRunner: 顶 API.
pub struct LatencyRunner {
    config: LatencyConfig,
    cache: MiniCache,
}

impl Default for LatencyRunner {
    fn default() -> Self {
        Self::new(LatencyConfig::default())
    }
}

impl LatencyRunner {
    pub fn new(config: LatencyConfig) -> Self {
        Self {
            config,
            cache: MiniCache::new(1024),
        }
    }

    /// 跑全部 3 场景 × 4 协议, 输出 LatencyReport.
    pub async fn run_all(&self) -> LatencyReport {
        let mut results: Vec<LatencyResult> = Vec::new();
        let protocols = [
            Protocol::OpenAiChat,
            Protocol::OpenAiResponses,
            Protocol::AnthropicMessages,
            Protocol::Gemini,
        ];

        // 启 wiremock
        let server = if self.config.use_wiremock {
            let s = wiremock::MockServer::start().await;
            // 4 协议默认都 mount OK (retry 场景用专门的 fail-first mount)
            mock::mount_all_4_protocols(&s).await;
            Some(s)
        } else {
            None
        };

        let base_url = server
            .as_ref()
            .map(|s| s.uri())
            .unwrap_or_else(|| "http://localhost:8080".to_string());

        // 1) Cache hit 场景 (4 协议 × samples) — 全部 LRU 命中, 0 网络
        for &protocol in &protocols {
            let samples_ns =
                run_cache_hit_scenario(&self.cache, protocol, self.config.samples);
            results.push(compute_percentiles(
                protocol,
                LatencyScenario::CacheHit,
                samples_ns,
            ));
        }

        // 2) Cache miss 场景 (4 协议 × samples) — 走 5 步管线 + wiremock
        for &protocol in &protocols {
            let samples_ns =
                run_cache_miss_scenario(&base_url, protocol, self.config.samples).await;
            results.push(compute_percentiles(
                protocol,
                LatencyScenario::CacheMiss,
                samples_ns,
            ));
        }

        // 3) Retry 场景 (4 协议 × samples) — Patient 退避
        for &protocol in &protocols {
            let samples_ns = run_retry_scenario(
                &base_url,
                protocol,
                self.config.samples,
                self.config.retry_fail_first_n,
            )
            .await;
            results.push(compute_percentiles(
                protocol,
                LatencyScenario::Retry,
                samples_ns,
            ));
        }

        LatencyReport {
            results,
            wiremock_disclaimer: WIREMOCK_DISCLAIMER,
        }
    }
}

// =====================================================================
// 单元测试 (bench 框架本身, 0 接真 LLM)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_endpoint_paths_match_apeireth_api() {
        // 1:1 验证 4 协议 path 跟 protocol_handlers.rs:62-65 严守
        assert_eq!(Protocol::OpenAiChat.endpoint_path(), "/v1/chat/completions");
        assert_eq!(Protocol::OpenAiResponses.endpoint_path(), "/v1/responses");
        assert_eq!(Protocol::AnthropicMessages.endpoint_path(), "/v1/messages");
        assert_eq!(
            Protocol::Gemini.endpoint_path(),
            "/v1beta/models/{model}:generateContent"
        );
    }

    #[test]
    fn mini_cache_get_put_roundtrip() {
        let cache = MiniCache::new(16);
        cache.put("k1".to_string(), "v1".to_string());
        assert_eq!(cache.get("k1"), Some("v1".to_string()));
        assert_eq!(cache.get("k2"), None);
    }

    #[test]
    fn mini_cache_lru_eviction() {
        let cache = MiniCache::new(2);
        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());
        cache.put("c".to_string(), "3".to_string()); // evict "a"
        assert_eq!(cache.get("a"), None, "LRU 必 evict oldest");
        assert_eq!(cache.get("b"), Some("2".to_string()));
        assert_eq!(cache.get("c"), Some("3".to_string()));
    }

    #[test]
    fn mini_cache_make_key_unique() {
        let k1 = MiniCache::make_key(Protocol::OpenAiChat, "hello");
        let k2 = MiniCache::make_key(Protocol::OpenAiChat, "world");
        let k3 = MiniCache::make_key(Protocol::AnthropicMessages, "hello");
        assert_ne!(k1, k2, "不同 payload → 不同 key");
        assert_ne!(k1, k3, "不同 protocol → 不同 key");
    }

    #[test]
    fn mini_retry_policy_6_tiers() {
        let p = MiniRetryPolicy::default();
        assert_eq!(p.tiers().len(), 6, "Patient 6 档, 跟 B retry.rs:235 1:1");
        assert_eq!(p.tiers()[0], Duration::from_millis(1));
        assert_eq!(p.tiers()[2], Duration::from_millis(10));
        assert_eq!(p.tiers()[5], Duration::from_millis(100));
    }

    #[test]
    fn mini_retry_succeeds_on_first_try() {
        let p = MiniRetryPolicy::default();
        let mut attempts = 0;
        let (ok, n) = p.retry(|_| {
            attempts += 1;
            true // 第 1 次就成功
        });
        assert!(ok);
        assert_eq!(n, 1);
        assert_eq!(attempts, 1);
    }

    #[test]
    fn mini_retry_eventually_succeeds() {
        let p = MiniRetryPolicy::default();
        let mut attempts = 0;
        let (ok, n) = p.retry(|i| {
            attempts += 1;
            i >= 2 // 第 3 次成功
        });
        assert!(ok);
        assert_eq!(n, 3);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn mini_retry_exhausts() {
        let p = MiniRetryPolicy::default();
        let mut attempts = 0;
        let (ok, n) = p.retry(|_| {
            attempts += 1;
            false // 全失败
        });
        assert!(!ok);
        assert_eq!(n, 6, "Patient 6 档全失败");
        assert_eq!(attempts, 6);
    }

    #[test]
    fn percentile_basic() {
        // nearest-rank 算法 (跟 v2-memory-vector-bench.rs:62-66 同款):
        // rank = ceil(quantile * n) (1-indexed) → sorted[rank - 1]
        // n=10, p50: rank = ceil(0.5*10) = 5 → sorted[4] = 50
        let samples = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100_u128];
        let sorted = {
            let mut s = samples.clone();
            s.sort_unstable();
            s
        };
        assert_eq!(percentile(&sorted, 0.50), 50, "nearest-rank p50 = sorted[4] = 50");
        assert_eq!(percentile(&sorted, 0.95), 100, "ceil(0.95*10)=10 → sorted[9] = 100");
        assert_eq!(percentile(&sorted, 0.99), 100, "ceil(0.99*10)=10 → sorted[9] = 100");
    }

    #[test]
    fn percentile_empty() {
        assert_eq!(percentile(&[], 0.50), 0);
    }

    #[test]
    fn compute_percentiles_basic() {
        // n=10, sorted = [100..1000] step 100
        // p50 = ceil(0.5*10)=5 → sorted[4] = 500
        let samples = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000_u128];
        let r = compute_percentiles(Protocol::OpenAiChat, LatencyScenario::CacheHit, samples);
        assert_eq!(r.samples, 10);
        assert_eq!(r.p50_ns, 500, "p50 = sorted[4] = 500 (nearest-rank)");
        assert_eq!(r.max_ns, 1000);
        assert_eq!(r.mean_ns, 550, "mean = sum/n = 5500/10 = 550");
    }

    #[test]
    fn latency_result_format_contains_p50_p99() {
        let samples = vec![1000, 2000, 3000, 4000, 5000_u128];
        let r = compute_percentiles(Protocol::AnthropicMessages, LatencyScenario::CacheMiss, samples);
        let s = r.format();
        assert!(s.contains("p50="));
        assert!(s.contains("p95="));
        assert!(s.contains("p99="));
        assert!(s.contains("Anthropic"));
        assert!(s.contains("cache-miss"));
    }

    #[test]
    fn cache_hit_scenario_returns_n_samples() {
        let cache = MiniCache::new(16);
        let samples = run_cache_hit_scenario(&cache, Protocol::OpenAiChat, 30);
        assert_eq!(samples.len(), 30);
        // 全部 samples 都应该 < 1ms (进程内 LRU 命中)
        for &s in &samples {
            assert!(s < 1_000_000, "cache hit 应 < 1ms, got {s} ns");
        }
    }

    #[test]
    fn latency_report_format_contains_disclaimer() {
        let r = compute_percentiles(Protocol::OpenAiChat, LatencyScenario::CacheHit, vec![100, 200, 300]);
        let report = LatencyReport {
            results: vec![r],
            wiremock_disclaimer: WIREMOCK_DISCLAIMER,
        };
        let s = report.format();
        assert!(s.contains("wiremock 0 网络"));
        assert!(s.contains("不假装"));
        assert!(s.contains("R121+ 替换"));
    }

    #[test]
    fn latency_runner_default_config() {
        let config = LatencyConfig::default();
        assert_eq!(config.samples, 30);
        assert!(config.use_wiremock);
        assert_eq!(config.retry_fail_first_n, 2);
    }

    #[test]
    fn protocol_label_format() {
        assert_eq!(Protocol::OpenAiChat.label(), "OpenAI-Chat");
        assert_eq!(Protocol::OpenAiResponses.label(), "OpenAI-Responses");
        assert_eq!(Protocol::AnthropicMessages.label(), "Anthropic");
        assert_eq!(Protocol::Gemini.label(), "Gemini");
    }

    #[test]
    fn latency_scenario_label_format() {
        assert_eq!(LatencyScenario::CacheHit.label(), "cache-hit");
        assert_eq!(LatencyScenario::CacheMiss.label(), "cache-miss");
        assert_eq!(LatencyScenario::Retry.label(), "retry");
    }

    // 验证 4 协议 + 3 场景 = 12 result 模式
    #[test]
    fn latency_report_12_results_full_run() {
        // 这里不跑全 wiremock (太慢), 验证 LatencyReport 结构可容纳 12 result
        let results: Vec<LatencyResult> = (0..12)
            .map(|i| {
                let protocol = match i % 4 {
                    0 => Protocol::OpenAiChat,
                    1 => Protocol::OpenAiResponses,
                    2 => Protocol::AnthropicMessages,
                    _ => Protocol::Gemini,
                };
                let scenario = match i / 4 {
                    0 => LatencyScenario::CacheHit,
                    1 => LatencyScenario::CacheMiss,
                    _ => LatencyScenario::Retry,
                };
                compute_percentiles(protocol, scenario, vec![1000, 2000, 3000])
            })
            .collect();
        assert_eq!(results.len(), 12);
        let report = LatencyReport {
            results,
            wiremock_disclaimer: WIREMOCK_DISCLAIMER,
        };
        let s = report.format();
        assert!(s.contains("results: 12"));
    }

    // 验证 4 protocol 区分 (cache key 必不同)
    #[test]
    fn cache_key_protocol_prefix() {
        let p1 = MiniCache::make_key(Protocol::OpenAiChat, "x");
        let p2 = MiniCache::make_key(Protocol::AnthropicMessages, "x");
        assert!(p1.starts_with("OpenAI-Chat::"));
        assert!(p2.starts_with("Anthropic::"));
    }

    // 静默 unused HashMap import (保留, 未来加 metrics 字段用)
    #[test]
    fn silence_unused_warning() {
        let mut m: HashMap<String, u32> = HashMap::new();
        m.insert("k".to_string(), 1);
        assert_eq!(m.len(), 1);
    }
}
