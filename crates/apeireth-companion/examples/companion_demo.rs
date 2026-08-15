//! apeireth-companion 全功能 demo
//!
//! 展示长期跨 session 用户关系的工程化承载.

use apeireth_companion::{
    Companion, BondStage, MilestoneKind, MilestonePayload, PartnerId, PartnerPreferences,
};

#[tokio::main]
async fn main() {
    let companion = Companion::new();

    // 1. 创建一个伙伴
    let id = PartnerId::new();
    let prefs = PartnerPreferences {
        address: Some("你".into()),
        style: Some("哲学密度".into()),
        topics: vec!["AI 与意识".into(), "生态构建".into()],
        avoid: vec![],
        notes: Default::default(),
        privacy: Default::default(),
    };

    let partner = companion.register_partner(id, "主人".to_string(), prefs).await.unwrap();
    println!("[+] registered partner: {} (id={})", partner.display_name(), partner.id());

    // 2. 记录里程碑
    let m1 = companion
        .record_milestone(id, MilestoneKind::FirstMeeting, MilestonePayload::Text("第一次对话".into()))
        .await
        .unwrap();
    println!("[+] milestone: {:?}", m1.kind());

    let m2 = companion
        .record_milestone(id, MilestoneKind::FirstEmotion, MilestonePayload::Text("首次表达情绪".into()))
        .await
        .unwrap();
    println!("[+] milestone: {:?}", m2.kind());

    // 3. 关系演进
    let bond = companion.evolve_bond(id, BondStage::Trusted, 0.3).await.unwrap();
    println!("[+] bond evolved to: {:?} (depth={})", bond.stage(), bond.depth());

    // 4. 注入情感 (per consciousness bridge)
    let mut partner = companion.get_partner(id).await.unwrap();
    partner.bond_mut().apply_emotion(0.6, 0.7, 0.1, 0.3, 0.0, 0.0, 0.0, 0.5);
    println!("[+] emotion applied: {:?}", partner.bond().character().serialize());

    // 5. 查询轨迹
    let timeline = companion.get_timeline(id).await.unwrap();
    println!("[+] timeline entries: {}", timeline.len());
    for entry in timeline.iter() {
        println!("    - {} @ {}: {:?}", entry.milestone.kind().label(), entry.at, entry.milestone.payload());
    }

    let count = companion.count_partners().await;
    println!("[+] total partners: {}", count);
}
