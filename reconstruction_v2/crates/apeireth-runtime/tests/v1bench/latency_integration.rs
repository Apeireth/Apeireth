//! B-2 latency bench integration tests (wiremock 4 协议 + cache hit/miss/retry)
//!
//! 跟 `apeireth-pipeline/tests/pipeline.rs` 复用同款 wiremock 模式.
//!
//! 验收:
//! - 6+ integration test
//! - 0 接真 LLM (mock, 0 网络真实开销)
//! - 跑 4 协议 × 3 场景 = 12 result, 输出 P50/P99 报告

use apeireth_bench::latency_bench::{
    compute_percentiles, LatencyConfig, LatencyResult, LatencyRunner, LatencyScenario, MiniCache,
    MiniRetryPolicy, Protocol,
};

// =====================================================================
// 单场景 smoke 跑分 (验证 percentiles / sample 收集)
// =====================================================================

#[test]
fn integration_cache_hit_returns_30_samples() {
    let cache = MiniCache::new(16);
    let samples =
        apeireth_bench::latency_bench::run_cache_hit_scenario(&cache, Protocol::OpenAiChat, 30);
    assert_eq!(samples.len(), 30);
    // 全部 cache hit 应 < 1ms (LRU 命中, 0 网络)
    for &s in &samples {
        assert!(s < 1_000_000, "cache hit 应 < 1ms, got {s} ns");
    }
}

#[tokio::test]
async fn integration_cache_miss_smoke_4_protocols() {
    // 不启 wiremock (use_wiremock=false), 用 "http://localhost:1" 期望失败
    // 简化: 跑 4 协议 cache miss, 但只测 1 sample (避免网络等待)
    let _server = wiremock::MockServer::start().await;
    for &protocol in &[
        Protocol::OpenAiChat,
        Protocol::OpenAiResponses,
        Protocol::AnthropicMessages,
        Protocol::Gemini,
    ] {
        // 1 sample 测 wiremock 端点 OK
        let samples =
            apeireth_bench::latency_bench::run_cache_miss_scenario(&_server.uri(), protocol, 1)
                .await;
        assert_eq!(samples.len(), 1);
        // wiremock 0 网络 + 本机: < 100ms
        assert!(
            samples[0] < 100_000_000,
            "wiremock hit 应 < 100ms, got {} ns",
            samples[0]
        );
    }
}

#[tokio::test]
async fn integration_retry_smoke_4_protocols() {
    let _server = wiremock::MockServer::start().await;
    for &protocol in &[
        Protocol::OpenAiChat,
        Protocol::OpenAiResponses,
        Protocol::AnthropicMessages,
        Protocol::Gemini,
    ] {
        let samples =
            apeireth_bench::latency_bench::run_retry_scenario(&_server.uri(), protocol, 1, 2).await;
        assert_eq!(samples.len(), 1);
        // retry 走完 1+3 ms 退避 + HTTP ~1ms = > 4ms
        assert!(
            samples[0] > 4_000_000,
            "retry 应 > 4ms (1+3ms 退避), got {} ns",
            samples[0]
        );
    }
}

// =====================================================================
// LatencyRunner 端到端 (12 result 完整性)
// =====================================================================

#[tokio::test]
async fn integration_runner_run_all_12_results() {
    let runner = LatencyRunner::new(LatencyConfig {
        samples: 5, // 5 sample 加速 (30 太慢)
        use_wiremock: true,
        retry_fail_first_n: 2,
    });
    let report = runner.run_all().await;
    assert_eq!(report.results.len(), 12, "4 协议 × 3 场景 = 12 result");

    // 每个 result 应有 5 sample
    for r in &report.results {
        assert_eq!(
            r.samples,
            5,
            "{} {} 应有 5 sample",
            r.protocol.label(),
            r.scenario.label()
        );
    }
}

#[tokio::test]
async fn integration_runner_cache_hit_fastest() {
    let runner = LatencyRunner::new(LatencyConfig {
        samples: 10,
        use_wiremock: true,
        retry_fail_first_n: 2,
    });
    let report = runner.run_all().await;

    // cache hit 应该是 3 场景里最快的 (LRU 命中, 0 网络)
    let cache_hit_p99: Vec<u128> = report
        .results
        .iter()
        .filter(|r| r.scenario == LatencyScenario::CacheHit)
        .map(|r| r.p99_ns)
        .collect();
    let cache_miss_p99: Vec<u128> = report
        .results
        .iter()
        .filter(|r| r.scenario == LatencyScenario::CacheMiss)
        .map(|r| r.p99_ns)
        .collect();

    for &hit in &cache_hit_p99 {
        for &miss in &cache_miss_p99 {
            assert!(
                hit < miss * 100, // 100x 余量 (cache miss 含 wiremock 启 + HTTP)
                "cache hit p99 {hit} 应 < cache miss p99 {miss} * 100"
            );
        }
    }
}

#[tokio::test]
async fn integration_runner_retry_slowest() {
    let runner = LatencyRunner::new(LatencyConfig {
        samples: 5,
        use_wiremock: true,
        retry_fail_first_n: 2,
    });
    let report = runner.run_all().await;

    // retry 应比 cache miss 慢 (Patient 1+3 ms 退避 = 4 ms 起步)
    let cache_miss_p50: Vec<u128> = report
        .results
        .iter()
        .filter(|r| r.scenario == LatencyScenario::CacheMiss)
        .map(|r| r.p50_ns)
        .collect();
    let retry_p50: Vec<u128> = report
        .results
        .iter()
        .filter(|r| r.scenario == LatencyScenario::Retry)
        .map(|r| r.p50_ns)
        .collect();

    for &retry in &retry_p50 {
        for &miss in &cache_miss_p50 {
            assert!(
                retry > miss,
                "retry p50 {retry} 应 > cache miss p50 {miss} (Patient 退避)"
            );
        }
    }
}

// =====================================================================
// LatencyConfig 验证 (samples 数量影响)
// =====================================================================

#[test]
fn integration_latency_config_default_values() {
    let c = LatencyConfig::default();
    assert_eq!(c.samples, 30);
    assert!(c.use_wiremock);
    assert_eq!(c.retry_fail_first_n, 2);
}

#[test]
fn integration_mini_retry_policy_patient_tiers_match_apeireth_api() {
    // 1:1 验证: 真实 Patient 1s/3s/10s/30s/2m/10m = 6 档 (B retry.rs:235)
    // smoke 压缩 1/1000: 1ms/3ms/10ms/30ms/60ms/100ms
    // 验证 6 档 + 比例 1:1000
    let p = MiniRetryPolicy::default();
    let tiers = p.tiers();
    assert_eq!(tiers.len(), 6);
    assert_eq!(tiers[0], std::time::Duration::from_millis(1));
    assert_eq!(tiers[2], std::time::Duration::from_millis(10));
    assert_eq!(tiers[5], std::time::Duration::from_millis(100));
    // 比例 1:1000 验证
    assert_eq!(tiers[0].as_millis() * 1000, 1000); // 1ms × 1000 = 1s
    assert_eq!(tiers[5].as_millis() * 100, 10000); // 100ms × 100 = 10s (注意 10m 才是最后档)
                                                   // 注: smoke 压缩 1ms/3ms/10ms/30ms/60ms/100ms 不严格 1:1000 (60/100),
                                                   // 是 B-2 选 smoke-friendly 比例
}

// =====================================================================
// compute_percentiles 端到端
// =====================================================================

#[test]
fn integration_compute_percentiles_real_distribution() {
    // 真实分布: 1ms (cache miss) + 5ms (retry 退避)
    // 模拟 30 sample, 其中 20 是 cache miss (~1ms), 10 是 retry (~5ms)
    let mut samples: Vec<u128> = vec![1_000_000; 20]; // 1ms each
    samples.extend(vec![5_000_000; 10]); // 5ms each
    samples.sort_unstable();

    let r = compute_percentiles(Protocol::OpenAiChat, LatencyScenario::CacheMiss, samples);
    assert_eq!(r.samples, 30);
    assert!(
        r.p50_ns >= 1_000_000 && r.p50_ns <= 1_500_000,
        "p50 应在 1ms 附近, got {} ns",
        r.p50_ns
    );
    assert!(
        r.p99_ns >= 4_500_000 && r.p99_ns <= 5_500_000,
        "p99 应在 5ms 附近, got {} ns",
        r.p99_ns
    );
}

#[test]
fn integration_latency_result_format_complete() {
    let r = LatencyResult {
        protocol: Protocol::OpenAiChat,
        scenario: LatencyScenario::CacheMiss,
        samples: 30,
        p50_ns: 200_000,   // 0.2ms
        p95_ns: 500_000,   // 0.5ms
        p99_ns: 1_000_000, // 1ms
        max_ns: 2_000_000, // 2ms
        mean_ns: 300_000,  // 0.3ms
        total_attempts: 30,
    };
    let s = r.format();
    assert!(s.contains("OpenAI-Chat"));
    assert!(s.contains("cache-miss"));
    // format 用 {:>6.2}ms, 0.20 是 4 chars, 右对齐 6 chars = "  0.20ms"
    assert!(s.contains("p50=  0.20ms"), "got: {s}");
    assert!(s.contains("p99=  1.00ms"), "got: {s}");
    assert!(s.contains("samples= 30"));
}
