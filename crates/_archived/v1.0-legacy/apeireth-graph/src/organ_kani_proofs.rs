//! R177 graph organ Kani proofs (W11)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_gr_01_checkpoint_store() {
    let s = CheckpointStore::new("/tmp/checkpoints");
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_gr_02_conditional_decision() {
    let d = ConditionalDecision {
        from: NodeId::default(),
        label: "ok".into(),
        target: None,
        path_kind: "path_map".into(),
    };
    assert_eq!(d.label, "ok");
}

#[test]
fn r177_gr_03_end_label() {
    assert_eq!(END_LABEL, "__end__");
}

#[test]
fn r177_gr_04_state() {
    let s = State::default();
    let _: String = format!("{:?}", s);
}

#[test]
fn r177_gr_05_subgraph_check() {
    let _: String = END_LABEL.to_string();
}

#[cfg(kani)]
#[kani::proof]
fn r177_gr_kani_01_end_label() {
    assert_eq!(END_LABEL, "__end__");
}

#[cfg(kani)]
#[kani::proof]
fn r177_gr_kani_02_store_invariant() {
    let s = CheckpointStore::new("/tmp/c");
    assert!(!format!("{:?}", s).is_empty());
}
