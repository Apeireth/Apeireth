//! 端到端效果验证: 用 apeireth-asi 6 维 LLM judge 真评估
//!
//! 跑法: APEIRETH_API_KEY=*** cargo run -p apeireth-asi --example real_judge_demo

use std::sync::Arc;

use apeireth_api::llm::{providers::scripted::ScriptedLlmProvider, LlmProvider};
use apeireth_asi::llm_judge::JudgeResult;
use apeireth_asi::{judge, LlmJudgeDim};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Apeireth 6 维 LLM judge 真效果验证");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. 先用 minimaxi 生成一段 LLM 输出 (作为被评估的素材)
    let llm: Arc<dyn LlmProvider> = if std::env::var("APEIRETH_API_KEY").is_ok() {
        let config = apeireth_api::llm::ApeirethApiConfig::from_env()?;
        Arc::new(apeireth_api::llm::ApeirethApiProvider::new(config)?)
    } else {
        println!("⚠️  APEIRETH_API_KEY 没设, 用 ScriptedLlmProvider (mock)");
        Arc::new(ScriptedLlmProvider::new("mock"))
    };

    let topic = "Apeireth 通用 API 扩展平台是真能跑还是要等前端";
    let sys_prompt = "你是一个 Rust 工程助手. 简洁回答.";
    let _user_msg = format!("{}\n中文, 1-2 句话.", topic);

    println!("\n📝 阶段 1: LLM 生成被评估的输出 (真 minimaxi)");
    let model = if llm.name() == "apeireth-api" {
        "MiniMax-M3".to_string()
    } else {
        llm.name().to_string()
    };
    let req = apeireth_api::llm::LlmRequest::new(
        &model,
        vec![
            apeireth_api::llm::ChatMessage::system(sys_prompt.to_string()),
            apeireth_api::llm::ChatMessage::user(format!("{}\n中文, 1-2 句话.", topic)),
        ],
    )
    .with_max_tokens(150)
    .with_temperature(0.5);
    let resp = llm.complete(req).await?;
    let output = resp.content.clone();
    println!(
        "✅ 生成 ({}ms, {} tokens):",
        resp.latency_ms, resp.usage.total_tokens
    );
    println!("   {}", &output[..output.len().min(200)]);

    // 2. 用 6 维 LLM judge 真评估 (用同一个 LLM)
    println!("\n📊 阶段 2: apeireth-asi 6 维 LLM judge 真评估 (6 次 minimaxi 调用)");
    let dims = [
        LlmJudgeDim::CoreValuesConsistency,
        LlmJudgeDim::VoiceConsistency,
        LlmJudgeDim::PhilosophyAlignment,
        LlmJudgeDim::ConeOfTruthRate,
        LlmJudgeDim::AbstractionLevel,
        LlmJudgeDim::AnalogyQuality,
    ];

    let mut results: Vec<JudgeResult> = Vec::new();
    for dim in &dims {
        let result = judge(&llm, *dim, &output).await?;
        println!(
            "   维 {:?} ({}): 分数 {:.2} | latency {}ms",
            dim.name(),
            result.model,
            result.score,
            result.latency_ms
        );
        println!(
            "     理由: {}",
            &result.reasoning[..result.reasoning.len().min(120)]
        );
        results.push(result);
    }

    // 3. 输出最终评估报告
    println!("\n📋 阶段 3: 最终评估报告");
    let total_score: f64 = results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64;
    let total_latency: u64 = results.iter().map(|r| r.latency_ms).sum();
    println!("   平均分: {:.2}", total_score);
    println!("   总延迟: {}ms (6 次 LLM 调用)", total_latency);
    println!("   模型:   {}", results[0].model);

    println!("\n✨ 6 维真效果验证完成");

    Ok(())
}
