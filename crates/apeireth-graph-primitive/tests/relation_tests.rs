//! 集成测试: 4 关系建模 + 注册表 + 决策树联合验证.
use apeireth_graph_primitive::{
    classify, classify_pair, Relation, RelationDecision, RelationKind, RelationRegistry,
};

#[test]
fn integration_full_4_relation_scenario() {
    // 集成: 模拟 Apeireth 主体的完整关系网.
    let mut reg = RelationRegistry::new();

    // 1. 共生: 感知 <-> 认知 (7 维内部)
    reg.register(Relation::new_symbiosis("perception", "cognition").unwrap());
    // 2. 协调: 约束 <-> 演化 (7 维之间)
    reg.register(Relation::new_coordination("constraint", "evolution").unwrap());
    // 3. 嵌入: 用户场景 host 智能体
    reg.register(Relation::new_embedding("user_scenario", "apeireth_agent").unwrap());
    // 4. 与自身: 主体连续性
    reg.register(Relation::new_self_relation("cid-apeireth-main").unwrap());

    assert_eq!(reg.len(), 4);
    assert_eq!(reg.count_by_kind(RelationKind::Symbiosis), 1);
    assert_eq!(reg.count_by_kind(RelationKind::Coordination), 1);
    assert_eq!(reg.count_by_kind(RelationKind::Embedding), 1);
    assert_eq!(reg.count_by_kind(RelationKind::SelfRelation), 1);

    // 主体的关系数: "perception" 出现 1 次, "cid-apeireth-main" 出现 1 次 (SelfRelation).
    assert_eq!(reg.find_by_party("perception").len(), 1);
    assert_eq!(reg.find_by_party("cid-apeireth-main").len(), 1);
    // "apeireth_agent" 作为 inner 出现在 1 个 embedding.
    assert_eq!(reg.find_by_party("apeireth_agent").len(), 1);
}

#[test]
fn integration_decision_tree_end_to_end() {
    // 集成: 决策树 4 类全覆盖 + 优先级.
    assert_eq!(
        classify(RelationDecision::AEqualsB),
        RelationKind::SelfRelation
    );
    assert_eq!(
        classify(RelationDecision::ALosesBDies),
        RelationKind::Symbiosis
    );
    assert_eq!(
        classify(RelationDecision::AIsInnerOfB),
        RelationKind::Embedding
    );
    assert_eq!(
        classify(RelationDecision::Default),
        RelationKind::Coordination
    );

    // 便捷分类: 相同主体 → SelfRelation.
    assert_eq!(classify_pair("x", "x"), RelationKind::SelfRelation);
    // 不同主体 → Coordination (无更多上下文).
    assert_eq!(
        classify_pair("perception", "cognition"),
        RelationKind::Coordination
    );

    // 验证 SelfRelation 的 party_a == party_b 不变量.
    let r = Relation::new_self_relation("cid-x").unwrap();
    assert_eq!(r.party_a, r.party_b);
    assert!(r.is_self_relation());
}

#[test]
fn integration_embedding_with_host_inner_semantics() {
    // 集成: Embedding 的语义 (host != inner) + 双向查询.
    let mut reg = RelationRegistry::new();
    reg.register(Relation::new_embedding("agent_outer", "reflection_inner").unwrap());
    let relations = reg.all();
    assert_eq!(relations.len(), 1);
    let r = &relations[0];
    assert!(r.is_embedding());
    assert_eq!(r.party_a, "agent_outer");
    assert_eq!(r.party_b, "reflection_inner");
    assert_ne!(r.party_a, r.party_b);
    // 双向查询: 任一 party 匹配都应找到.
    assert_eq!(reg.find_by_party("agent_outer").len(), 1);
    assert_eq!(reg.find_by_party("reflection_inner").len(), 1);
}
