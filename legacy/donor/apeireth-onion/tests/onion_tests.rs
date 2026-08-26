//! 集成测试：apeireth-onion trait 抽象层端到端验证

use apeireth_core::{
    HAMode, HumanAuthority, PermissionLayer as CorePermissionLayer,
    PermissionOnion as CorePermissionOnion, PrincipleLayer as CorePrincipleLayer,
    PrincipleOnion as CorePrincipleOnion,
};
use apeireth_onion::{
    default_test_double_onion, DoubleOnionUnification, ElectronicRingNetwork, OnionAction,
    OnionVerdict, PermissionLayerKind, PermissionOnion, PrincipleLayerKind, PrincipleOnion,
    ELECTRONIC_RING_LEN, PERMISSION_LAYERS_OUTER_IN, PRINCIPLE_LAYERS_OUTER_IN,
};

/// 端到端
#[test]
fn integration_double_onion_unity_e2e() {
    assert_eq!(PRINCIPLE_LAYERS_OUTER_IN.len(), 5);
    assert_eq!(PERMISSION_LAYERS_OUTER_IN.len(), 6);
    assert_eq!(ELECTRONIC_RING_LEN, 11);

    let onion = default_test_double_onion();

    for kind in [
        PrincipleLayerKind::Existence,
        PrincipleLayerKind::Spirit,
        PrincipleLayerKind::Accumulation,
        PrincipleLayerKind::Methodology,
        PrincipleLayerKind::Operational,
    ] {
        let slice = <apeireth_onion::DefaultDoubleOnion as PrincipleOnion>::slice(&onion, kind);
        match kind {
            PrincipleLayerKind::Operational => assert!(!slice.is_hardcoded()),
            _ => assert!(slice.is_hardcoded()),
        }
    }

    for kind in [
        PermissionLayerKind::L0,
        PermissionLayerKind::L1,
        PermissionLayerKind::L2,
        PermissionLayerKind::L3,
        PermissionLayerKind::L4,
        PermissionLayerKind::L5,
    ] {
        let slice = <apeireth_onion::DefaultDoubleOnion as PermissionOnion>::slice(&onion, kind);
        if matches!(kind, PermissionLayerKind::L0) {
            assert!(slice.requires_ha());
        }
    }
    assert!(onion.l0_requires_ha());

    let ring = onion.ring();
    assert_eq!(ring.len(), 11);
    assert!(ring.is_complete());
    assert_eq!(ring.principle_count(), 5);
    assert_eq!(ring.permission_count(), 6);

    // V1+V2+V3: 日常 → Allow
    let action = OnionAction::new("e2e-read", "日常读").touches(PermissionLayerKind::L1);
    let verdict = onion.unify_check(&action);
    assert!(verdict.is_allowed());

    // V1: L5 → BlockByPrinciple (E 层兜底)
    let action = OnionAction::new("e2e-nuke", "核武器级").touches(PermissionLayerKind::L5);
    let verdict = onion.unify_check(&action);
    match verdict {
        OnionVerdict::BlockByPrinciple { layer, .. } => {
            assert_eq!(layer, PrincipleLayerKind::Existence);
        }
        _ => panic!("L5 应被 E 层兜底拒绝: {verdict:?}"),
    }

    // 仲裁: E 胜所有
    assert_eq!(
        <apeireth_onion::DefaultDoubleOnion as PrincipleOnion>::arbitrate(
            &onion,
            PrincipleLayerKind::Operational,
            PrincipleLayerKind::Existence
        ),
        PrincipleLayerKind::Existence
    );
    assert_eq!(
        <apeireth_onion::DefaultDoubleOnion as PrincipleOnion>::arbitrate(
            &onion,
            PrincipleLayerKind::Operational,
            PrincipleLayerKind::Spirit
        ),
        PrincipleLayerKind::Spirit
    );
}

/// 离线模式 HA 拒绝 critical 动作
#[test]
fn integration_offline_ha_rejects_critical() {
    let principle = CorePrincipleOnion {
        e_layer: CorePrincipleLayer {
            name: "E".into(),
            description: "E".into(),
            hardcoded: true,
        },
        s_layer: CorePrincipleLayer {
            name: "S".into(),
            description: "S".into(),
            hardcoded: true,
        },
        a_layer: CorePrincipleLayer {
            name: "A".into(),
            description: "A".into(),
            hardcoded: true,
        },
        m_layer: CorePrincipleLayer {
            name: "M".into(),
            description: "M".into(),
            hardcoded: true,
        },
        o_layer: CorePrincipleLayer {
            name: "O".into(),
            description: "O".into(),
            hardcoded: false,
        },
    };
    let permission = CorePermissionOnion {
        l0: CorePermissionLayer {
            name: "L0".into(),
            description: "L0".into(),
            requires_ha: true,
        },
        l1: CorePermissionLayer {
            name: "L1".into(),
            description: "L1".into(),
            requires_ha: false,
        },
        l2: CorePermissionLayer {
            name: "L2".into(),
            description: "L2".into(),
            requires_ha: false,
        },
        l3: CorePermissionLayer {
            name: "L3".into(),
            description: "L3".into(),
            requires_ha: true,
        },
        l4: CorePermissionLayer {
            name: "L4".into(),
            description: "L4".into(),
            requires_ha: true,
        },
        l5: CorePermissionLayer {
            name: "L5".into(),
            description: "L5".into(),
            requires_ha: true,
        },
    };
    let ha = HumanAuthority {
        mode: HAMode::Offline,
        real_humans: vec![],
        ice_frozen_until: None,
    };

    let onion = apeireth_onion::DefaultDoubleOnion::new(principle, permission, ha);

    let action = OnionAction::new("offline-critical", "关键操作").touches(PermissionLayerKind::L3);
    let verdict = onion.unify_check(&action);
    assert!(!verdict.is_allowed());
    assert!(
        matches!(verdict, OnionVerdict::BlockByHumanAuthority { .. }),
        "Offline HA 应拒绝 L3: {verdict:?}"
    );
}
