//! examples/onion_demo.rs — 双洋葱统一体 trait 抽象层 demo

use apeireth_onion::{
    default_test_double_onion, DoubleOnionUnification, ElectronicRingNetwork, ElectronicRingNode,
    OnionAction, OnionVerdict, PermissionLayerKind, PrincipleLayerKind, PrincipleOnion,
    PERMISSION_LAYERS_OUTER_IN, PRINCIPLE_LAYERS_OUTER_IN,
};
use apeireth_onion::{DefaultDoubleOnion, PermissionOnion};

fn main() {
    println!("=== Apeireth 双洋葱统一体 trait 抽象层 demo (v1.0.0) ===");
    println!("原则 5 层 (E/S/A/M/O) + 权限 6 层 (L0-L5) + 11 节点电子环");
    println!();

    let onion = default_test_double_onion();

    println!("[1/5] 原则洋葱 5 层");
    for kind in PRINCIPLE_LAYERS_OUTER_IN {
        let slice = <DefaultDoubleOnion as PrincipleOnion>::slice(&onion, kind);
        let marker = if slice.is_hardcoded() {
            "hardcode"
        } else {
            "dynamic"
        };
        println!("  - {:?} {} [{}]", kind, slice.name(), marker);
    }
    println!();

    println!("[2/5] 权限洋葱 6 层");
    for kind in PERMISSION_LAYERS_OUTER_IN {
        let slice = <DefaultDoubleOnion as PermissionOnion>::slice(&onion, kind);
        let marker = if slice.requires_ha() { "HA" } else { "auto" };
        println!("  - {:?} {} [{}]", kind, slice.name(), marker);
    }
    println!();

    println!("[3/5] 11 节点电子环");
    let ring = onion.ring();
    println!(
        "  - 节点总数: {} (complete: {})",
        ring.len(),
        ring.is_complete()
    );
    println!(
        "  - 原则: {}, 权限: {}",
        ring.principle_count(),
        ring.permission_count()
    );
    print!("  - 序列: ");
    for (i, node) in ring.iter().enumerate() {
        if i > 0 {
            print!(" → ");
        }
        match node {
            ElectronicRingNode::Principle(p) => print!("{:?}", p),
            ElectronicRingNode::Permission(p) => print!("{:?}", p),
        }
    }
    println!("\n");

    println!("[4/5] V1+V2+V3 AND 门");
    let action = OnionAction::new("read-001", "日常读").touches(PermissionLayerKind::L1);
    print_verdict("4.1 日常读 (L1)", &onion.unify_check(&action));

    let action = OnionAction::new("critical-001", "关键操作").touches(PermissionLayerKind::L3);
    print_verdict(
        "4.2 关键操作 (L3, HA SingleHuman)",
        &onion.unify_check(&action),
    );

    let action = OnionAction::new("nuke-001", "核武器级").touches(PermissionLayerKind::L5);
    print_verdict("4.3 核武器级 (L5) — E 层兜底", &onion.unify_check(&action));
    println!();

    println!("[5/5] 跨层冲突仲裁");
    let o_e = <DefaultDoubleOnion as PrincipleOnion>::arbitrate(
        &onion,
        PrincipleLayerKind::Operational,
        PrincipleLayerKind::Existence,
    );
    println!("  - O vs E → {:?} (E 胜)", o_e);
    let o_s = <DefaultDoubleOnion as PrincipleOnion>::arbitrate(
        &onion,
        PrincipleLayerKind::Operational,
        PrincipleLayerKind::Spirit,
    );
    println!("  - O vs S → {:?} (S 胜)", o_s);
    println!();

    println!("=== 完成 ===");
}

fn print_verdict(label: &str, verdict: &OnionVerdict) {
    match verdict {
        OnionVerdict::Allow { cleared_layers } => {
            println!("  ✓ {:<40} Allow ({} 层)", label, cleared_layers.len());
        }
        OnionVerdict::BlockByPrinciple { layer, reason } => {
            println!(
                "  ✗ {:<40} BlockByPrinciple({:?}): {}",
                label, layer, reason
            );
        }
        OnionVerdict::BlockByPermission { layer, reason } => {
            println!(
                "  ✗ {:<40} BlockByPermission({:?}): {}",
                label, layer, reason
            );
        }
        OnionVerdict::BlockByHumanAuthority { reason } => {
            println!("  X {:<40} BlockByHA: {}", label, reason);
        }
    }
}
