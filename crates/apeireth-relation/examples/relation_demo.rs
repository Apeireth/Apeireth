//! apeireth-relation demo — 演示 4 类关系建模 + 决策树 + 注册表.
//!
//! 运行: `cargo run -p apeireth-relation --example relation_demo`

use apeireth_relations::{
    classify, classify_pair, Relation, RelationDecision, RelationKind, RelationRegistry,
};

fn main() {
    println!("=== apeireth-relation demo ===\n");

    // 场景 1: 打印全部 4 关系语义.
    println!("[场景 1] 4 关系总览");
    for k in RelationKind::ALL {
        println!(
            "  {:>14} = {} (binary={})",
            k.semantic_name(),
            k.describe(),
            k.is_binary()
        );
    }
    println!();

    // 场景 2: 关系决策树演示.
    println!("[场景 2] 关系决策树 (v4 §4.3 扩展)");
    println!(
        "  classify(AEqualsB)    = {:?}",
        classify(RelationDecision::AEqualsB)
    );
    println!(
        "  classify(ALosesBDies) = {:?}",
        classify(RelationDecision::ALosesBDies)
    );
    println!(
        "  classify(AIsInnerOfB) = {:?}",
        classify(RelationDecision::AIsInnerOfB)
    );
    println!(
        "  classify(Default)     = {:?}",
        classify(RelationDecision::Default)
    );
    println!();

    println!("[场景 2b] classify_pair 便捷分类");
    println!(
        "  classify_pair(\"cid-x\", \"cid-x\")  = {:?}",
        classify_pair("cid-x", "cid-x")
    );
    println!(
        "  classify_pair(\"perception\", \"cognition\") = {:?}",
        classify_pair("perception", "cognition")
    );
    println!();

    // 场景 3: 构建 4 类关系实例并注册.
    println!("[场景 3] 4 类关系实例 + 注册表");
    let mut reg = RelationRegistry::new();
    let r1 = Relation::new_symbiosis("perception", "cognition")
        .unwrap()
        .with_note("7 维内部强耦合");
    println!(
        "  Symbiosis:    {} <-> {}  (note={:?})",
        r1.party_a, r1.party_b, r1.note
    );
    reg.register(r1);

    let r2 = Relation::new_coordination("constraint", "evolution")
        .unwrap()
        .with_note("7 维之间弱耦合");
    println!(
        "  Coordination: {} <-> {}  (note={:?})",
        r2.party_a, r2.party_b, r2.note
    );
    reg.register(r2);

    let r3 = Relation::new_embedding("user_scenario", "apeireth_agent")
        .unwrap()
        .with_note("智能体嵌入用户场景");
    println!("  Embedding:    host={}, inner={}", r3.party_a, r3.party_b);
    reg.register(r3);

    let r4 = Relation::new_self_relation("cid-main-agent").unwrap();
    println!(
        "  SelfRelation: {} <-> {}  (involved={:?})",
        r4.party_a,
        r4.party_b,
        r4.involved_parties()
    );
    reg.register(r4);

    println!();
    println!("[场景 4] 注册表查询");
    println!("  total = {}", reg.len());
    println!(
        "  Symbiosis count    = {}",
        reg.count_by_kind(RelationKind::Symbiosis)
    );
    println!(
        "  Coordination count = {}",
        reg.count_by_kind(RelationKind::Coordination)
    );
    println!(
        "  Embedding count    = {}",
        reg.count_by_kind(RelationKind::Embedding)
    );
    println!(
        "  SelfRelation count = {}",
        reg.count_by_kind(RelationKind::SelfRelation)
    );

    let found = reg.find_by_party("perception");
    println!("  find_by_party(\"perception\") = {} 条", found.len());

    println!("\n=== demo 完成 ===");
}
