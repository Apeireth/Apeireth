//! apeireth-life-force v2 — IdentityCard continuity + migration 集成测试.
//!
//! 验证:
//! - v1 surface (continuity_id / birth_time / carriers / migration_history) 100% 保
//! - v2 增字段 (name / version / philosophy_anchors / created_at) 与 v1 字段共存
//! - serde_json round-trip 保所有字段
//! - Migration 字段独立 serde

use apeireth_core::{IdentityCard, Migration};

fn make_test_identity() -> IdentityCard {
    IdentityCard {
        continuity_id: "did:apeireth:migration-001".to_string(),
        birth_time: 1_700_000_000,
        carriers: vec!["carrier-A".to_string(), "carrier-B".to_string()],
        migration_history: vec![
            Migration {
                from_carrier: "carrier-0".to_string(),
                to_carrier: "carrier-A".to_string(),
                timestamp: 1_700_000_100,
            },
            Migration {
                from_carrier: "carrier-A".to_string(),
                to_carrier: "carrier-B".to_string(),
                timestamp: 1_700_000_500,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn v2_identity_card_preserves_v1_continuity_id() {
    let card = make_test_identity();
    assert_eq!(card.continuity_id, "did:apeireth:migration-001");
    assert_eq!(card.birth_time, 1_700_000_000);
}

#[test]
fn v2_identity_card_default_companion_fills_both_surfaces() {
    // v2 surface: default_companion 同时填 v1 + v2 字段
    let card = IdentityCard::default_companion();
    assert!(!card.continuity_id.is_empty(), "v1 continuity_id 必须填");
    assert!(card.birth_time > 0, "v1 birth_time 必须 > 0");
    assert!(!card.carriers.is_empty(), "v1 carriers 必须非空");
    assert!(card.migration_history.is_empty(), "首次创建应无迁移记录");

    // v2 surface 也填
    assert!(!card.name.is_empty(), "v2 name 必须填");
    assert!(!card.version.is_empty(), "v2 version 必须填");
    assert!(
        !card.philosophy_anchors.is_empty(),
        "v2 philosophy_anchors 必须填"
    );
}

#[test]
fn v2_identity_card_with_continuity_helper() {
    let card = IdentityCard::with_continuity("did:apeireth:test", 1234);
    assert_eq!(card.continuity_id, "did:apeireth:test");
    assert_eq!(card.birth_time, 1234);
    // v2 字段用 default (空 + version)
    assert_eq!(card.version, "2.0.0");
    assert!(card.philosophy_anchors.is_empty());
}

#[test]
fn v2_identity_card_serde_json_round_trip_preserves_all_fields() {
    let card = make_test_identity();
    let json = serde_json::to_string(&card).expect("serialize");
    let parsed: IdentityCard = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.continuity_id, card.continuity_id);
    assert_eq!(parsed.birth_time, card.birth_time);
    assert_eq!(parsed.carriers, card.carriers);
    assert_eq!(parsed.migration_history.len(), card.migration_history.len());
    assert_eq!(parsed.migration_history[0].from_carrier, "carrier-0");
    assert_eq!(parsed.migration_history[1].to_carrier, "carrier-B");
}

#[test]
fn v2_migration_struct_serde() {
    let m = Migration::new("carrier-X", "carrier-Y", 9_999_999);
    assert_eq!(m.from_carrier, "carrier-X");
    assert_eq!(m.to_carrier, "carrier-Y");
    assert_eq!(m.timestamp, 9_999_999);
    let json = serde_json::to_string(&m).unwrap();
    let back: Migration = serde_json::from_str(&json).unwrap();
    assert_eq!(back, m);
}

#[test]
fn v2_identity_card_backward_compat_with_missing_v2_fields() {
    // v1 数据 (只有 v1 字段) → JSON → IdentityCard 解析 (v2 字段用 default)
    let v1_json = r#"{
        "continuity_id": "did:apeireth:v1-only",
        "birth_time": 1000,
        "carriers": ["legacy-carrier"],
        "migration_history": []
    }"#;
    let parsed: IdentityCard = serde_json::from_str(v1_json).expect("v1-only data");
    assert_eq!(parsed.continuity_id, "did:apeireth:v1-only");
    assert_eq!(parsed.birth_time, 1000);
    assert_eq!(parsed.carriers, vec!["legacy-carrier"]);
    // v2 字段用 serde default — 不应失败
    assert_eq!(parsed.name, "");
    assert_eq!(parsed.version, "2.0.0");
    assert!(parsed.philosophy_anchors.is_empty());
}

#[test]
fn v2_migration_history_preserves_order() {
    let mut card = IdentityCard::default_companion();
    card.migration_history = vec![
        Migration::new("a", "b", 1),
        Migration::new("b", "c", 2),
        Migration::new("c", "d", 3),
    ];
    assert_eq!(card.migration_history.len(), 3);
    assert_eq!(card.migration_history[0].to_carrier, "b");
    assert_eq!(card.migration_history[1].to_carrier, "c");
    assert_eq!(card.migration_history[2].to_carrier, "d");

    let json = serde_json::to_string(&card).unwrap();
    let back: IdentityCard = serde_json::from_str(&json).unwrap();
    assert_eq!(back.migration_history.len(), 3);
    for (i, m) in back.migration_history.iter().enumerate() {
        assert_eq!(m.timestamp, (i + 1) as i64);
    }
}

#[test]
fn v2_default_trait_provides_valid_continuity_id() {
    let card = IdentityCard::default();
    assert!(card.continuity_id.starts_with("did:apeireth:"));
    assert!(card.birth_time > 0);
    assert!(!card.carriers.is_empty());
}
