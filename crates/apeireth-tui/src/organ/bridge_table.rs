//! 9 organ naming bridge table (ADR-0028)
//!
//! **权威桥接**: TUI R11 LOCKED 旧名 (heart/brain/hand/eye/ear/memory/voice/body/mind)
//!   ↔ crate R23+ 新名 (consciousness/perception/cognition/motivation/life-force/
//!     memory/value/graph-primitive/companion)
//!
//! **不漂移**:
//! - 0 改 organ/{body,brain,ear,eye,hand,heart,memory,mind,voice}.rs R11 LOCKED
//! - 0 改 organ/mod.rs Organ enum R11 LOCKED
//! - 0 改 i18n key organs.{heart,brain,...} R21 G-1 LOCKED
//! - 0 改 ASCII art [♥] [BRAIN] ... R11 LOCKED
//!
//! **唯一权威**: docs/adr/0028-organ-naming-bridge.md §2.4
//!
//! 当前状态: 1.0 release 后, snapshot_all_organs 已覆盖 9 个 crate (perception/cognition/
//! consciousness/memory/motivation/value/graph_primitive/action/life_force), 但
//! TUI Organ enum 仅 9 个 R11 LOCKED 旧名. 桥接表是 backend snapshot 的 1:1 文档化.

#![allow(missing_docs)]

/// TUI 旧名 → crate 新名 桥接表 (权威, 11 项)
pub const ORGAN_BRIDGE_TABLE: &[(&str, &str)] = &[
    // TUI 旧名      crate 新名
    ("heart",         "life_force"),
    ("brain",         "cognition"),
    ("hand",          "action"),
    ("eye",           "perception"),
    ("ear",           "perception"),
    ("memory",        "memory"),
    ("voice",         "voice"),
    ("body",          "body"),
    ("mind",          "consciousness"),
    // crate 新名有但 TUI 旧名无 (NEW crate)
    ("motivation",    "motivation"),
    ("value",         "value"),
    ("graph_primitive", "graph_primitive"),
    ("companion",     "companion"),
];

/// TUI 旧名 → crate 新名
pub fn tui_to_crate(tui: &str) -> Option<&'static str> {
    ORGAN_BRIDGE_TABLE.iter()
        .find(|(t, _)| *t == tui)
        .map(|(_, c)| *c)
}

/// crate 新名 → TUI 旧名 (反向, 仅适用于有 TUI 对应的 9 个)
pub fn crate_to_tui(crate_name: &str) -> Option<&'static str> {
    ORGAN_BRIDGE_TABLE.iter()
        .find(|(_, c)| *c == crate_name)
        .map(|(t, _)| *t)
}

/// TUI 旧名列表 (R11 LOCKED 9 organ)
pub const TUI_ORGAN_NAMES: &[&str] = &[
    "heart", "brain", "hand", "eye", "ear", "memory", "voice", "body", "mind",
];

/// crate 新名列表 (R23+ 9+1 organ)
pub const CRATE_ORGAN_NAMES: &[&str] = &[
    "consciousness", "perception", "cognition", "motivation", "life_force",
    "memory", "value", "graph_primitive", "companion", "action", "voice",
];

// =====================================================================
// 单元测试 (桥接表权威性, 11 项 + 反向 + 列表)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_table_count_is_13() {
        // 9 TUI 旧名 + 4 crate 新名 (motivation/value/graph_primitive/companion 无 TUI 对应)
        assert_eq!(ORGAN_BRIDGE_TABLE.len(), 13);
    }

    #[test]
    fn tui_to_crate_known_mappings() {
        assert_eq!(tui_to_crate("heart"), Some("life_force"));
        assert_eq!(tui_to_crate("brain"), Some("cognition"));
        assert_eq!(tui_to_crate("hand"), Some("action"));
        assert_eq!(tui_to_crate("eye"), Some("perception"));
        assert_eq!(tui_to_crate("ear"), Some("perception"));
        assert_eq!(tui_to_crate("memory"), Some("memory"));
        assert_eq!(tui_to_crate("voice"), Some("voice"));
        assert_eq!(tui_to_crate("body"), Some("body"));
        assert_eq!(tui_to_crate("mind"), Some("consciousness"));
    }

    #[test]
    fn tui_to_crate_unknown_returns_none() {
        assert_eq!(tui_to_crate("unknown_organ"), None);
        assert_eq!(tui_to_crate(""), None);
    }

    #[test]
    fn crate_to_tui_known_mappings() {
        assert_eq!(crate_to_tui("life_force"), Some("heart"));
        assert_eq!(crate_to_tui("cognition"), Some("brain"));
        assert_eq!(crate_to_tui("action"), Some("hand"));
        assert_eq!(crate_to_tui("perception"), Some("eye"));  // first match wins (eye before ear)
        assert_eq!(crate_to_tui("memory"), Some("memory"));
        assert_eq!(crate_to_tui("voice"), Some("voice"));
        assert_eq!(crate_to_tui("consciousness"), Some("mind"));
    }

    #[test]
    fn crate_to_tui_unknown_returns_none() {
        assert_eq!(crate_to_tui("unknown_crate"), None);
    }

    #[test]
    fn tui_organ_names_count_is_9() {
        // R11 LOCKED 9 organ
        assert_eq!(TUI_ORGAN_NAMES.len(), 9);
        // 必须包含全部 9 个
        for n in &["heart", "brain", "hand", "eye", "ear", "memory", "voice", "body", "mind"] {
            assert!(TUI_ORGAN_NAMES.contains(n), "TUI missing: {}", n);
        }
    }

    #[test]
    fn crate_organ_names_count_is_11() {
        // R23+ 9+1+1 (action/voice 是额外 organ crate, 不在 9 organ 内)
        assert_eq!(CRATE_ORGAN_NAMES.len(), 11);
        for n in &[
            "consciousness", "perception", "cognition", "motivation", "life_force",
            "memory", "value", "graph_primitive", "companion", "action", "voice"
        ] {
            assert!(CRATE_ORGAN_NAMES.contains(n), "Crate missing: {}", n);
        }
    }

    #[test]
    fn all_tui_names_in_bridge_table() {
        // TUI 9 organ 全部在桥接表
        for n in TUI_ORGAN_NAMES {
            assert!(tui_to_crate(n).is_some(), "TUI '{}' not in bridge table", n);
        }
    }
}
