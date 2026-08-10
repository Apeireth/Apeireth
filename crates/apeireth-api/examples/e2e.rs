//! `e2e` — 端到端演示 (Week 4)
//!
//! 流程: LLM 推理 → Council 7 advisor 真接入 → V1+V2+V3 AND 门 → verdict
//!
//! 跑法: cargo run -p apeireth-api --example e2e
//!
//! 不需要任何 key (用 ScriptedLlmProvider)

use std::sync::Arc;

use apeireth_api::llm::{
    providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
    ChatMessage, LlmProvider, LlmRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 通用 API 扩展平台 — e2e 端到端演示 (Week 4)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📚 流程: LLM chat → Council 7 advisor → V1+V2+V3 AND 门 → verdict");

    // 1. ScriptedLlmProvider (避免 key 依赖)
    let llm: Arc<dyn LlmProvider> = Arc::new(
        ScriptedLlmProvider::new("e2e-mock")
            .with_script(
                "安全",
                ScriptedResponse::new("approve: 议题安全, 通过 12 键哲学守门"),
            )
            .with_script(
                "performance",
                ScriptedResponse::new("approve: 性能影响可接受"),
            )
            .with_script("哲学", ScriptedResponse::new("approve: 12 键哲学守门通过"))
            .with_script("历史", ScriptedResponse::new("neutral: 暂无历史相似案例"))
            .with_script(
                "strategy",
                ScriptedResponse::new("approve: 长期价值 > 短期成本"),
            )
            .with_script("伦理", ScriptedResponse::new("approve: 符合实事求是原则"))
            .with_script(
                "legal",
                ScriptedResponse::new("approve: 未触发 L0 HA 司法边界"),
            ),
    );

    println!(
        "\n✅ LLM provider: {} (scripted mock, 无 key 依赖)",
        llm.name()
    );

    // 2. LLM 推理 (用 LLMProvider trait)
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 1: LLM 推理 (用 LlmProvider trait)");
    let topic = "Apeireth 项目下一阶段开发计划: 真接入 LLM + 端到端 e2e 测试";
    println!("议题: {topic}");

    let req = LlmRequest::new(llm.name(), vec![ChatMessage::user(topic)]);
    let resp = llm.complete(req).await?;
    println!("✅ LLM 响应 (latency={}ms):", resp.latency_ms);
    println!("   {}", &resp.content[..resp.content.len().min(120)]);

    // 3. Council 7 advisor 真接入 (Week 3 完整版)
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 2: Council 7 advisor 真接入 (Week 3)");

    let advisor_prompts = [
        ("safety", "你是 Council safety advisor. 评估议题安全. 给出立场 (approve/reject/neutral) + 推理. 中文."),
        ("performance", "你是 Council performance advisor. 评估议题性能影响. 给出立场 + 推理. 中文."),
        ("philosophy", "你是 Council philosophy advisor. 评估议题哲学一致性. 给出立场 + 推理. 中文."),
        ("history", "你是 Council history advisor. 评估历史相似性. 给出立场 + 推理. 中文."),
        ("strategy", "你是 Council strategy advisor. 评估长期价值 vs 短期成本. 给出立场 + 推理. 中文."),
        ("ethics", "你是 Council ethics advisor. 评估伦理. 给出立场 + 推理. 中文."),
        ("legal", "你是 Council legal advisor. 评估 L0 HA 司法边界. 给出立场 + 推理. 中文."),
    ];

    let mut approvals = 0;
    let mut rejects = 0;
    let mut neutrals = 0;

    for (domain, sys_prompt) in &advisor_prompts {
        let usr = format!("议题: {topic}");
        let req = LlmRequest::new(
            llm.name(),
            vec![
                ChatMessage::system((*sys_prompt).to_string()),
                ChatMessage::user(usr),
            ],
        )
        .with_temperature(0.3)
        .with_max_tokens(150);
        let resp = llm.complete(req).await?;

        // 简单解析: 含 "approve" / "reject" / "neutral"
        let stance = if resp.content.contains("reject") {
            "reject"
        } else if resp.content.contains("neutral") {
            "neutral"
        } else {
            "approve"
        };
        match stance {
            "approve" => approvals += 1,
            "reject" => rejects += 1,
            _ => neutrals += 1,
        }
        println!(
            "   [{}] stance={} (latency={}ms)",
            domain, stance, resp.latency_ms
        );
    }

    println!("\n📊 Council 投票结果:");
    println!("   approve: {approvals}/7");
    println!("   reject:  {rejects}/7");
    println!("   neutral: {neutrals}/7");

    // 4. V1+V2+V3 AND 门 (Week 1 verdict)
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 3: V1+V2+V3 AND 门 (Week 1 verdict)");

    let v1_pass = approvals >= 5; // 哲学: 12 键守门 (至少 5/7 advisor 同意)
    let v2_pass = rejects == 0; // 权限: 无强反对
    let v3_pass = neutrals <= 2; // 默认: 反对/中性太多不通过

    let all_pass = v1_pass && v2_pass && v3_pass;
    let final_verdict = if all_pass { "allow" } else { "block" };

    println!(
        "   V1 哲学守门:  {} (12 键)",
        if v1_pass { "✓ pass" } else { "✗ block" }
    );
    println!(
        "   V2 权限守门:  {} (无强反对)",
        if v2_pass { "✓ pass" } else { "✗ block" }
    );
    println!(
        "   V3 默认守门:  {} (中性 ≤ 2)",
        if v3_pass { "✓ pass" } else { "✗ block" }
    );
    println!();
    println!("🏛️  最终 verdict: {}", final_verdict.to_uppercase());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ e2e 端到端跑通 — LLM → Council 7 → V1+V2+V3 → verdict");
    println!();
    println!("📊 端到端耗时: 见各阶段 latency_ms");
    println!("📊 协议层全通: OpenAI-compatible chat + Council 真接入 + V1+V2+V3 AND 门");
    println!("📊 真实 LLM 接入: 替换 ScriptedLlmProvider → ApeirethApiProvider 即可真跑");

    Ok(())
}
