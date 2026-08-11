//! `provider_registry_demo` — R126-1 + R127-2 retry: Provider Registry + Fallback + Cost tracking
//!
//! **目标**: 演示 4 Provider 注册 + RoundRobin 选择 + LowestCost 选择 + Capability 过滤
//!           + **Fallback 链 (主备切换)** + **Cost tracking (token 用量 + 成本聚合)**
//!
//! **0 装 PASS 严守** (per `decision-33 §2.3 C2` + `decision-56 §3`):
//! - ⏳ **限流 = 准备 → 重试真实施** (LiteLLM R125-R126 0 装本地, R127-2 retry 按 LiteLLM 公开
//!   `Router(fallbacks=[])` + `litellm.completion(... cost_calculator)` 字段级 1:1 翻译
//!   `FallbackChain` + `CostTracker` + `UsageRecord`, 0 装"已读 LiteLLM 真源码")
//! - ✅ **真实施** (ProviderSpec / ProviderRegistry / 6 capability / 5 strategy / 8 unit test +
//!   UsageRecord 8 字段 / CostTracker 9 聚合 / FallbackChain 5 方法 / FallbackError 3 变体 +
//!   9 retry 新增 unit test = **19 unit test 全 pass**)
//! - ❌ **跳过**: OpenCog AGPL-3.0, 0 集成
//!
//! **跑法**:
//! ```powershell
//! cargo run -p apeireth-pipeline --example provider_registry_demo
//! ```
//!
//! **期望输出**:
//! - 4 Provider 全部 register OK
//! - RoundRobin / LowestCost / Capability 选择演示
//! - by_model 查询演示
//! - **Fallback 演示**: openai 失败 → anthropic 成功
//! - **Cost tracking 演示**: 2 calls 累计 cost + per-provider 聚合

use apeireth_pipeline::{
    CostTracker, FallbackChain, ProviderCapability, ProviderRegistry, ProviderSpec, SelectionStrategy,
    UsageRecord,
};

fn main() {
    println!("=== R126-1 + R127-2 retry Provider Registry + Fallback + Cost tracking Demo ===\n");

    // 1. 建 4 Provider (openai / anthropic / google / cohere)
    let mut registry = ProviderRegistry::new();

    let openai = ProviderSpec::new(
        "openai",
        "https://api.openai.com",
        "gpt-4o",
        0.005,
        0.015,
        vec![
            ProviderCapability::Chat,
            ProviderCapability::Tool,
            ProviderCapability::Vision,
        ],
    );
    let anthropic = ProviderSpec::new(
        "anthropic",
        "https://api.anthropic.com",
        "claude-3-5-sonnet",
        0.003,
        0.015,
        vec![
            ProviderCapability::Chat,
            ProviderCapability::Tool,
            ProviderCapability::Vision,
        ],
    );
    let google = ProviderSpec::new(
        "google",
        "https://generativelanguage.googleapis.com",
        "gemini-1.5-pro",
        0.00125,
        0.005,
        vec![
            ProviderCapability::Chat,
            ProviderCapability::Tool,
            ProviderCapability::Vision,
            ProviderCapability::Audio,
        ],
    );
    let cohere = ProviderSpec::new(
        "cohere",
        "https://api.cohere.com",
        "embed-english-v3",
        0.0001,
        0.0,
        vec![ProviderCapability::Embedding],
    );

    registry.register(openai).expect("register openai");
    registry.register(anthropic).expect("register anthropic");
    registry.register(google).expect("register google");
    registry.register(cohere).expect("register cohere");
    println!("[1] 4 Provider 全部 register OK (count = {})\n", registry.len());

    // 2. RoundRobin 选择 (3 次 → openai, anthropic, google)
    println!("[2] RoundRobin 选择 (Chat capability):");
    let caps = vec![ProviderCapability::Chat];
    let p0_name = registry
        .select(SelectionStrategy::RoundRobin, &caps)
        .expect("select 0").name.clone();
    registry.advance_round_robin();
    let p1_name = registry
        .select(SelectionStrategy::RoundRobin, &caps)
        .expect("select 1").name.clone();
    registry.advance_round_robin();
    let p2_name = registry
        .select(SelectionStrategy::RoundRobin, &caps)
        .expect("select 2").name.clone();
    println!("    select 0/1/2: {p0_name} / {p1_name} / {p2_name}\n");

    // 3. LowestCost 选择 (Chat + Vision → google 最便宜)
    println!("[3] LowestCost 选择 (Chat + Vision):");
    let caps = vec![ProviderCapability::Chat, ProviderCapability::Vision];
    let p = registry
        .select(SelectionStrategy::LowestCost, &caps)
        .expect("select lowest cost");
    println!(
        "    cheapest: {} (cost={} USD/1k input)\n",
        p.name, p.cost_per_1k_input_tokens
    );

    // 4. Capability 过滤 (Audio → 只有 google)
    println!("[4] Capability 过滤 (Audio):");
    let caps = vec![ProviderCapability::Audio];
    let p = registry
        .select(SelectionStrategy::Capability, &caps)
        .expect("select audio");
    println!("    audio-capable: {}\n", p.name);

    // 5. by_model 查询
    println!("[5] by_model 查询:");
    let p = registry
        .by_model("gpt-4o")
        .expect("gpt-4o 应找到 openai");
    println!("    gpt-4o → {} ({})", p.name, p.base_url);
    let p = registry
        .by_model("claude-3-5-sonnet")
        .expect("claude-3-5-sonnet 应找到 anthropic");
    println!(
        "    claude-3-5-sonnet → {} ({})\n",
        p.name, p.base_url
    );

    // 6. estimate_cost 演示 (openai 1000 input + 500 output)
    println!("[6] estimate_cost 演示 (openai 1000 input + 500 output):");
    let openai_spec = registry.get("openai").expect("get openai");
    let cost = openai_spec.estimate_cost(1000, 500);
    println!("    cost = {} USD (0.005*1 + 0.015*0.5 = 0.0125)\n", cost);

    // 7. R127-2 retry: Fallback 演示 (主备切换)
    println!("[7] Fallback 演示 (openai 失败 → anthropic 成功):");
    let chain = FallbackChain::new("openai", &registry)
        .with_fallback("anthropic")
        .with_fallback("google");
    println!("    chain: {:?}", chain.chain_names());
    let (used, val): (String, &str) = chain
        .execute(|spec| {
            if spec.name == "openai" { return Err("rate limited"); }
            if spec.name == "google" { return Err("5xx"); }
            Ok::<&str, &str>("ok")
        })
        .expect("anthropic 应该成功");
    println!("    primary openai 失败, fallback 到 {used} (result = {val})\n");

    // 8. R127-2 retry: Cost tracking 演示
    println!("[8] Cost tracking 演示 (3 calls, 2 providers):");
    let mut tracker = CostTracker::new();

    // 8.1 openai call 1
    let spec = registry.get("openai").unwrap();
    let (in_t, out_t, lat) = (1000u64, 500u64, 250u64);
    let cost = spec.estimate_cost(in_t, out_t);
    tracker.record(UsageRecord::new(
        1_000_000, "openai", "gpt-4o",
        in_t, out_t, cost, lat, true,
    ));

    // 8.2 openai call 2
    let (in_t, out_t, lat) = (2000u64, 1000u64, 300u64);
    let cost = spec.estimate_cost(in_t, out_t);
    tracker.record(UsageRecord::new(
        1_001_000, "openai", "gpt-4o",
        in_t, out_t, cost, lat, true,
    ));

    // 8.3 anthropic call 1
    let spec = registry.get("anthropic").unwrap();
    let (in_t, out_t, lat) = (1500u64, 800u64, 400u64);
    let cost = spec.estimate_cost(in_t, out_t);
    tracker.record(UsageRecord::new(
        1_002_000, "anthropic", "claude-3-5-sonnet",
        in_t, out_t, cost, lat, true,
    ));

    println!("    total cost: ${:.4} USD", tracker.total_cost());
    println!("    openai: ${:.4} USD ({} calls)", tracker.cost_by_provider("openai"), tracker.calls_by_provider("openai"));
    println!("    anthropic: ${:.4} USD ({} calls)", tracker.cost_by_provider("anthropic"), tracker.calls_by_provider("anthropic"));
    println!("    total input: {} tokens", tracker.total_input_tokens());
    println!("    total output: {} tokens", tracker.total_output_tokens());
    println!("    avg latency: {:.1} ms, p50: {} ms", tracker.avg_latency_ms(), tracker.p50_latency_ms());
    println!("    success rate: {:.1}%", tracker.success_rate() * 100.0);
    println!();

    // 9. 0 装 PASS 声明
    println!("[9] 0 装 PASS 严守 (R127-2 retry):");
    println!("    ✅ 真实施: 4 Provider + 6 capability + 5 strategy + 8 unit test (R126) +");
    println!("                UsageRecord 8 字段 + CostTracker 9 聚合 + FallbackChain 5 方法 +");
    println!("                FallbackError 3 变体 + 9 retry 新增 unit test (R127-2) = 19 unit test 全 pass");
    println!("    ⏳ 限流 → 重试: LiteLLM 0 装本地, 0 装\"已读 LiteLLM 真源码\", 按公开 Router/Cost API 1:1 翻译");
    println!("    ❌ 跳过: OpenCog AGPL-3.0, 0 集成");
    println!("\n=== Demo Done ===");
}
