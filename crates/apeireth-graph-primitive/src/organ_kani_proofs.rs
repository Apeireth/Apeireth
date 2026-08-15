//! R177 graph-primitive organ Kani proofs (W2)

#![allow(missing_docs)]

use crate::{
    classify, classify_pair, Relation, RelationDecision, RelationKind, RelationRegistry,
};

#[test]
fn r177_gp_01_relation_kind_all_4() {
    assert_eq!(RelationKind::ALL.len(), 4);
}

#[test]
fn r177_gp_02_relation_kind_describe_non_empty() {
    for k in &RelationKind::ALL {
        assert!(!k.describe().is_empty(), "{:?} describe 空", k);
    }
}

#[test]
fn r177_gp_03_new_symbiosis() {
    let r = Relation::new_symbiosis("a", "b").unwrap();
    assert_eq!(r.kind, RelationKind::Symbiosis);
    assert_eq!(r.party_a, "a");
    assert_eq!(r.party_b, "b");
    assert!(!r.is_self_relation());
    assert!(!r.is_embedding());
}

#[test]
fn r177_gp_04_new_coordination() {
    let r = Relation::new_coordination("a", "b").unwrap();
    assert_eq!(r.kind, RelationKind::Coordination);
}

#[test]
fn r177_gp_05_new_embedding() {
    let r = Relation::new_embedding("a", "b").unwrap();
    assert_eq!(r.kind, RelationKind::Embedding);
    assert!(r.is_embedding());
}

#[test]
fn r177_gp_06_embedding_rejects_self_loop() {
    let r = Relation::new_embedding("a", "a");
    assert!(r.is_err(), "embedding 不允许 self-loop");
}

#[test]
fn r177_gp_07_self_relation_ok() {
    let ok = Relation::new_self_relation("cid-1");
    assert!(ok.is_ok());
    let r = ok.unwrap();
    assert_eq!(r.kind, RelationKind::SelfRelation);
    assert!(r.is_self_relation());
    assert_eq!(r.party_a, r.party_b);
}

#[test]
fn r177_gp_08_involved_parties_two() {
    let r = Relation::new_symbiosis("a", "b").unwrap();
    let parties = r.involved_parties();
    assert_eq!(parties.len(), 2);
    assert!(parties.contains(&"a"));
    assert!(parties.contains(&"b"));
}

#[test]
fn r177_gp_09_with_note() {
    let r = Relation::new_symbiosis("a", "b").unwrap().with_note("test note");
    assert_eq!(r.note, Some("test note".to_string()));
}

#[test]
fn r177_gp_10_classify_decision() {
    let kinds = [
        classify(RelationDecision::AEqualsB),
        classify(RelationDecision::ALosesBDies),
        classify(RelationDecision::AIsInnerOfB),
        classify(RelationDecision::Default),
    ];
    assert_eq!(kinds[0], RelationKind::SelfRelation);
    assert_eq!(kinds[1], RelationKind::Symbiosis);
    assert_eq!(kinds[2], RelationKind::Embedding);
    assert_eq!(kinds[3], RelationKind::Coordination);
}

#[test]
fn r177_gp_11_classify_pair() {
    let k_same = classify_pair("a", "a");
    assert_eq!(k_same, RelationKind::SelfRelation);
    let k_diff = classify_pair("a", "b");
    assert_ne!(k_diff, RelationKind::SelfRelation);
}

#[test]
fn r177_gp_12_registry_basic() {
    let mut reg = RelationRegistry::new();
    assert_eq!(reg.len(), 0);
    let r = Relation::new_symbiosis("a", "b").unwrap();
    reg.register(r);
    assert_eq!(reg.len(), 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_gp_kani_01_kind_count() {
    assert_eq!(RelationKind::ALL.len(), 4);
}

#[cfg(kani)]
#[kani::proof]
fn r177_gp_kani_02_classify_pair_self() {
    let k = classify_pair("x", "x");
    assert_eq!(k, RelationKind::SelfRelation);
    let k2 = classify_pair("x", "y");
    assert_ne!(k2, RelationKind::SelfRelation);
}
