//! apeireth-sovereignty demo
//!
//! 演示 MEWG 五重治理端到端流程:
//! 1. 构造 Governance (默认 5 重治理 mock)
//! 2. 注册 3 AI provider (mock)
//! 3. 注册 2 真实人类 + 投 approve
//! 4. 注册 2 物理设备 + 签名 (1 witness)
//! 5. process(decision) → Outcome
//!
//! 运行: `cargo run -p apeireth-sovereignty --example sovereignty_demo`

use std::time::Duration;

use apeireth_sovereignty::{
    AiStance, Decision, Governance, HumanId, MockAiProvider, PhysicalSignerId, Vote,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("👑 apeireth-sovereignty demo (MEWG 五重治理)");
    println!();

    // 1) 构造 Governance (反思期长度 = 0 用于 demo)
    let gov = Governance::default().with_reflection_period(Duration::from_millis(0));
    println!("📋 Governance orchestrator created");

    // 2) 注册 3 AI provider
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "gpt4-mock",
        AiStance::Approve,
    )))
    .await?;
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "claude-mock",
        AiStance::Approve,
    )))
    .await?;
    gov.register_ai_provider(Box::new(MockAiProvider::new(
        "local-mock",
        AiStance::Approve,
    )))
    .await?;
    println!("🤖 3 AI providers registered");

    // 3) 注册 2 真实人类 + 投 approve
    {
        let mut voter = gov.multi_human.lock().await;
        voter.register(HumanId::new("alice", "Alice", "owner"));
        voter.register(HumanId::new("bob", "Bob", "co-owner"));
        voter.cast_vote("alice", Vote::Approve, "LGTM".to_string())?;
        voter.cast_vote("bob", Vote::Approve, "LGTM".to_string())?;
    }
    println!("👥 2 humans registered + voted Approve");

    // 4) 注册 2 物理设备 + 签名
    {
        let mut m = gov.physical.lock().await;
        m.register(PhysicalSignerId::new("yubi-001", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("phone-001", "phone", "bob"));
        m.collect_signature("yubi-001", "decision-digest".to_string(), true)?;
        m.collect_signature("phone-001", "decision-digest".to_string(), false)?;
    }
    println!("🔐 2 physical signers (yubikey + phone, 1 witness)");

    // 5) process(decision) → Outcome
    let decision = Decision {
        id: "demo-1".into(),
        title: "Modify monitoring threshold".into(),
        description: "Lower the L1 monitoring threshold from 0.95 to 0.90".into(),
        touches_e_layer: false,
        tags: vec!["monitoring".into(), "tuning".into()],
        submitted_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    let outcome = gov.process(&decision).await?;
    println!();
    println!("⚖️  Outcome:");
    match &outcome {
        apeireth_sovereignty::GovernanceOutcome::Approved {
            mewg_score,
            rationale,
        } => {
            println!("    ✅ APPROVED (MEWG score = {:.3})", mewg_score);
            println!("    Rationale: {rationale}");
        }
        apeireth_sovereignty::GovernanceOutcome::Blocked { failed_at, reason } => {
            println!("    ❌ BLOCKED at {}", failed_at.name());
            println!("    Reason: {reason}");
        }
        apeireth_sovereignty::GovernanceOutcome::PendingReview { waiting_at, state } => {
            println!("    ⏸️  PENDING REVIEW at {}", waiting_at.name());
            println!("    State: {state}");
        }
    }

    Ok(())
}
