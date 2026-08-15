//! R177 onion organ Kani proofs (W4)

#![allow(missing_docs)]

use crate::{
    ElectronicRing, OnionAction, PermissionLayerKind, PrincipleLayerKind,
    ELECTRONIC_RING_LEN, PERMISSION_LAYERS_OUTER_IN, PRINCIPLE_LAYERS_OUTER_IN,
};

#[test]
fn r177_oni_01_principle_layers_5() {
    assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len(), 5);
}

#[test]
fn r177_oni_02_principle_layers_distinct() {
    let mut seen = std::collections::HashSet::new();
    for l in &PRINCIPLE_LAYERS_OUTER_IN {
        assert!(seen.insert(*l), "原则层重复: {:?}", l);
    }
    assert_eq!(seen.len(), 5);
}

#[test]
fn r177_oni_03_permission_layers_6() {
    assert_eq!(PERMISSION_LAYERS_OUTER_IN.len(), 6);
}

#[test]
fn r177_oni_04_permission_layers_distinct() {
    let mut seen = std::collections::HashSet::new();
    for l in &PERMISSION_LAYERS_OUTER_IN {
        assert!(seen.insert(*l), "权限层重复: {:?}", l);
    }
    assert_eq!(seen.len(), 6);
}

#[test]
fn r177_oni_05_electronic_ring_11() {
    assert_eq!(ELECTRONIC_RING_LEN, 11);
    assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len() + PERMISSION_LAYERS_OUTER_IN.len(), 11);
}

#[test]
fn r177_oni_06_principle_5_variants() {
    let layers = [
        PrincipleLayerKind::Existence,
        PrincipleLayerKind::Spirit,
        PrincipleLayerKind::Accumulation,
        PrincipleLayerKind::Methodology,
        PrincipleLayerKind::Operational,
    ];
    assert_eq!(layers.len(), 5);
    let mut seen = std::collections::HashSet::new();
    for l in &layers {
        assert!(seen.insert(*l));
    }
}

#[test]
fn r177_oni_07_permission_6_variants() {
    let layers = [
        PermissionLayerKind::L0,
        PermissionLayerKind::L1,
        PermissionLayerKind::L2,
        PermissionLayerKind::L3,
        PermissionLayerKind::L4,
        PermissionLayerKind::L5,
    ];
    assert_eq!(layers.len(), 6);
    let mut seen = std::collections::HashSet::new();
    for l in &layers {
        assert!(seen.insert(*l));
    }
}

#[test]
fn r177_oni_08_electronic_ring_new_empty() {
    let ring = ElectronicRing::new();
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
    assert!(!ring.is_complete(), "空电子环不应 is_complete");
}

#[test]
fn r177_oni_09_onion_action_new() {
    let action = OnionAction::new("act-1", "test action");
    assert_eq!(action.id, "act-1");
    assert_eq!(action.description, "test action");
}

#[test]
fn r177_oni_10_onion_action_touches() {
    let action = OnionAction::new("act-1", "test")
        .touches(PermissionLayerKind::L1)
        .touches(PermissionLayerKind::L2);
    assert!(action.touches_layer.is_some());
}

#[cfg(kani)]
#[kani::proof]
fn r177_oni_kani_01_ring_5_plus_6() {
    assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len() + PERMISSION_LAYERS_OUTER_IN.len(), 11);
    assert_eq!(ELECTRONIC_RING_LEN, 11);
}

#[cfg(kani)]
#[kani::proof]
fn r177_oni_kani_02_layers_distinct() {
    let mut seen = std::collections::HashSet::new();
    for l in &PRINCIPLE_LAYERS_OUTER_IN {
        assert!(seen.insert(*l));
    }
    for l in &PERMISSION_LAYERS_OUTER_IN {
        assert!(seen.insert(*l));
    }
}
