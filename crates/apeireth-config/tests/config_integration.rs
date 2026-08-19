//! Integration tests for apeireth-config (post-1.0.0)
//!
//! src/lib.rs 已有 17 #[test] + organ_kani_proofs. 这里 (tests/) 加跨函数集成 + JSON 边界 + 错误传播测试.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_config::{
    diff, key_is_valid, lookup, merge, merge_three_layers, missing_required, parse_json_layer,
    to_json_layer, validate_all, ConfigEntry, ConfigError,
};

// =============================================================================
// ConfigEntry 边界
// =============================================================================

#[test]
fn entry_validate_required_with_value_ok() {
    let e = ConfigEntry::new("api_key", "secret", true);
    assert!(e.validate().is_ok());
}

#[test]
fn entry_validate_optional_with_empty_ok() {
    let e = ConfigEntry::new("debug_mode", "", false);
    assert!(e.validate().is_ok(), "optional + empty = OK");
}

#[test]
fn entry_validate_whitespace_only_key_rejected() {
    let e = ConfigEntry::new("   ", "x", false);
    assert!(e.validate().is_err(), "whitespace-only key 应拒");
}

#[test]
fn entry_validate_required_empty_value_rejected() {
    let e = ConfigEntry::new("required_key", "", true);
    assert!(e.validate().is_err());
}

#[test]
fn entry_clone_eq() {
    let a = ConfigEntry::new("k", "v", false);
    let b = a.clone();
    assert_eq!(a, b);
}

// =============================================================================
// ConfigError
// =============================================================================

#[test]
fn error_empty_key_display() {
    let e = ConfigError::EmptyKey("".into());
    let s = e.to_string();
    assert!(s.contains("empty"), "{s}");
}

#[test]
fn error_duplicate_key_display() {
    let e = ConfigError::DuplicateKey("api".into());
    let s = e.to_string();
    assert!(s.contains("重复"), "中文 error: {s}");
    assert!(s.contains("api"));
}

// =============================================================================
// validate_all
// =============================================================================

#[test]
fn validate_all_empty_slice_ok() {
    let empty: Vec<ConfigEntry> = vec![];
    assert!(validate_all(&empty).is_ok());
}

#[test]
fn validate_all_first_fails_short_circuits() {
    let v = vec![
        ConfigEntry::new("", "x", false), // fail
        ConfigEntry::new("ok", "y", false),
    ];
    assert!(validate_all(&v).is_err());
}

#[test]
fn validate_all_all_required_with_value() {
    let v = vec![
        ConfigEntry::new("a", "1", true),
        ConfigEntry::new("b", "2", true),
    ];
    assert!(validate_all(&v).is_ok());
}

// =============================================================================
// lookup
// =============================================================================

#[test]
fn lookup_first_match_wins() {
    let v = vec![
        ConfigEntry::new("k", "first", false),
        ConfigEntry::new("k", "second", false),
    ];
    assert_eq!(lookup(&v, "k"), Some("first"));
}

#[test]
fn lookup_empty_slice() {
    let empty: Vec<ConfigEntry> = vec![];
    assert_eq!(lookup(&empty, "any"), None);
}

#[test]
fn lookup_required_ignores_required_flag() {
    let v = vec![ConfigEntry::new("api", "secret", true)];
    assert_eq!(lookup(&v, "api"), Some("secret"));
}

// =============================================================================
// merge
// =============================================================================

#[test]
fn merge_both_empty() {
    let r = merge(&[], &[]).unwrap();
    assert!(r.is_empty());
}

#[test]
fn merge_empty_overlay_preserves_base() {
    let base = vec![ConfigEntry::new("a", "1", false)];
    let r = merge(&base, &[]).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].key, "a");
    assert_eq!(r[0].value, "1");
}

#[test]
fn merge_empty_base_appends_overlay() {
    let overlay = vec![ConfigEntry::new("x", "y", false)];
    let r = merge(&[], &overlay).unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn merge_overlay_overrides_required_flag() {
    let base = vec![ConfigEntry::new("k", "old", false)];
    let overlay = vec![ConfigEntry::new("k", "new", true)];
    let r = merge(&base, &overlay).unwrap();
    let m = r.iter().find(|e| e.key == "k").unwrap();
    assert_eq!(m.value, "new");
    assert!(m.required, "overlay 的 required flag 也覆盖");
}

#[test]
fn merge_dup_in_overlay_rejected() {
    let base = vec![ConfigEntry::new("a", "1", false)];
    let overlay = vec![
        ConfigEntry::new("b", "x", false),
        ConfigEntry::new("b", "y", false),
    ];
    assert!(merge(&base, &overlay).is_err());
}

#[test]
fn merge_preserves_base_order() {
    let base = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("b", "2", false),
        ConfigEntry::new("c", "3", false),
    ];
    let overlay = vec![ConfigEntry::new("b", "NEW", false)];
    let r = merge(&base, &overlay).unwrap();
    assert_eq!(r[0].key, "a");
    assert_eq!(r[1].key, "b");
    assert_eq!(r[2].key, "c");
    assert_eq!(r[1].value, "NEW");
}

// =============================================================================
// missing_required
// =============================================================================

#[test]
fn missing_required_empty_input() {
    assert!(missing_required(&[]).is_empty());
}

#[test]
fn missing_required_none_when_all_filled() {
    let v = vec![
        ConfigEntry::new("a", "1", true),
        ConfigEntry::new("b", "2", true),
    ];
    assert!(missing_required(&v).is_empty());
}

#[test]
fn missing_required_optional_empty_excluded() {
    let v = vec![
        ConfigEntry::new("optional", "", false),
        ConfigEntry::new("required", "", true),
    ];
    let m = missing_required(&v);
    assert_eq!(m, vec!["required"]);
}

#[test]
fn missing_required_multiple() {
    let v = vec![
        ConfigEntry::new("a", "", true),
        ConfigEntry::new("b", "x", false),
        ConfigEntry::new("c", "", true),
        ConfigEntry::new("d", "y", true),
    ];
    let m = missing_required(&v);
    assert_eq!(m.len(), 2);
    assert!(m.contains(&"a"));
    assert!(m.contains(&"c"));
}

// =============================================================================
// diff
// =============================================================================

#[test]
fn diff_identical_slices_empty_diff() {
    let v = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("b", "2", false),
    ];
    let (a, r, c) = diff(&v, &v);
    assert!(a.is_empty());
    assert!(r.is_empty());
    assert!(c.is_empty());
}

#[test]
fn diff_empty_before_added() {
    let after = vec![ConfigEntry::new("a", "1", false)];
    let (a, r, c) = diff(&[], &after);
    assert_eq!(a, vec!["a"]);
    assert!(r.is_empty());
    assert!(c.is_empty());
}

#[test]
fn diff_empty_after_removed() {
    let before = vec![ConfigEntry::new("a", "1", false)];
    let (a, r, c) = diff(&before, &[]);
    assert!(a.is_empty());
    assert_eq!(r, vec!["a"]);
    assert!(c.is_empty());
}

#[test]
fn diff_only_required_flag_change_is_not_changed() {
    // diff 只看 value 变化, required flag 不算 changed
    let before = vec![ConfigEntry::new("a", "1", false)];
    let after = vec![ConfigEntry::new("a", "1", true)];
    let (a, r, c) = diff(&before, &after);
    assert!(a.is_empty());
    assert!(r.is_empty());
    assert!(c.is_empty());
}

#[test]
fn diff_comprehensive() {
    let before = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("b", "2", false),
        ConfigEntry::new("c", "3", false),
        ConfigEntry::new("d", "4", false),
    ];
    let after = vec![
        ConfigEntry::new("a", "1", false),       // same
        ConfigEntry::new("b", "CHANGED", false), // changed
        ConfigEntry::new("e", "5", false),       // added
                                                 // c, d removed
    ];
    let (a, r, c) = diff(&before, &after);
    assert_eq!(a, vec!["e"]);
    assert_eq!(r.len(), 2);
    assert!(r.contains(&"c"));
    assert!(r.contains(&"d"));
    assert_eq!(c, vec!["b"]);
}

// =============================================================================
// key_is_valid
// =============================================================================

#[test]
fn key_valid_alphanumeric() {
    assert!(key_is_valid("abc123"));
    assert!(key_is_valid("ABC"));
    assert!(key_is_valid("mixedCase42"));
}

#[test]
fn key_valid_dash_underscore() {
    assert!(key_is_valid("with-dash"));
    assert!(key_is_valid("with_underscore"));
    assert!(key_is_valid("a-b_c-d_e"));
}

#[test]
fn key_invalid_empty() {
    assert!(!key_is_valid(""));
}

#[test]
fn key_invalid_special_chars() {
    assert!(!key_is_valid("with space"));
    assert!(!key_is_valid("with.dot"));
    assert!(!key_is_valid("with/slash"));
    assert!(!key_is_valid("with:colon"));
    assert!(!key_is_valid("with@at"));
    assert!(!key_is_valid("unicodeñ"));
    assert!(!key_is_valid("with\"quote"));
}

// =============================================================================
// merge_three_layers
// =============================================================================

#[test]
fn merge_three_all_empty() {
    let r = merge_three_layers(&[], &[], &[]).unwrap();
    assert!(r.is_empty());
}

#[test]
fn merge_three_only_default() {
    let def = vec![ConfigEntry::new("a", "1", false)];
    let r = merge_three_layers(&def, &[], &[]).unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn merge_three_only_override() {
    let ov = vec![ConfigEntry::new("x", "1", false)];
    let r = merge_three_layers(&[], &[], &ov).unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn merge_three_priority_default_file_override() {
    let def = vec![ConfigEntry::new("k", "default", false)];
    let file = vec![ConfigEntry::new("k", "file", false)];
    let ov = vec![ConfigEntry::new("k", "override", false)];
    let r = merge_three_layers(&def, &file, &ov).unwrap();
    assert_eq!(r[0].value, "override", "override > file > default");
}

#[test]
fn merge_three_file_overrides_default() {
    let def = vec![ConfigEntry::new("k", "default", false)];
    let file = vec![ConfigEntry::new("k", "file", false)];
    let r = merge_three_layers(&def, &file, &[]).unwrap();
    assert_eq!(r[0].value, "file");
}

#[test]
fn merge_three_duplicate_across_layers_allowed() {
    // 跨层重复是 override 语义, 不报错
    let def = vec![ConfigEntry::new("k", "1", false)];
    let file = vec![ConfigEntry::new("k", "2", false)];
    let r = merge_three_layers(&def, &file, &[]).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].value, "2");
}

#[test]
fn merge_three_dup_in_default_rejected() {
    let def = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("a", "2", false),
    ];
    assert!(merge_three_layers(&def, &[], &[]).is_err());
}

#[test]
fn merge_three_dup_in_file_rejected() {
    let file = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("a", "2", false),
    ];
    assert!(merge_three_layers(&[], &file, &[]).is_err());
}

#[test]
fn merge_three_dup_in_override_rejected() {
    let ov = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("a", "2", false),
    ];
    assert!(merge_three_layers(&[], &[], &ov).is_err());
}

// =============================================================================
// parse_json_layer / to_json_layer
// =============================================================================

#[test]
fn parse_json_empty_string_returns_empty() {
    let v = parse_json_layer("");
    assert!(v.is_empty(), "空字符串 -> 空 Vec");
}

#[test]
fn parse_json_object_not_array_returns_empty() {
    let v = parse_json_layer(r#"{"key":"value"}"#);
    assert!(v.is_empty(), "非数组 -> 空 Vec");
}

#[test]
fn parse_json_empty_array() {
    let v = parse_json_layer("[]");
    assert!(v.is_empty());
}

#[test]
fn parse_json_skips_entries_without_key() {
    let json = r#"[{"value":"orphan"},{"key":"ok","value":"1"}]"#;
    let v = parse_json_layer(json);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].key, "ok");
}

#[test]
fn parse_json_required_default_false() {
    let json = r#"[{"key":"a","value":"1"}]"#;
    let v = parse_json_layer(json);
    assert!(!v[0].required, "缺省 required = false");
}

#[test]
fn parse_json_value_missing_defaults_empty() {
    let json = r#"[{"key":"a","required":true}]"#;
    let v = parse_json_layer(json);
    assert_eq!(v[0].value, "", "缺 value -> 空字符串");
}

#[test]
fn to_json_basic() {
    let v = vec![ConfigEntry::new("a", "1", false)];
    let s = to_json_layer(&v);
    assert!(s.contains("\"a\""));
    assert!(s.contains("\"1\""));
}

#[test]
fn to_json_empty() {
    let s = to_json_layer(&[]);
    assert_eq!(s, "[]");
}

#[test]
fn json_roundtrip_preserves_fields() {
    let original = vec![
        ConfigEntry::new("api_key", "secret", true),
        ConfigEntry::new("debug", "false", false),
        ConfigEntry::new("mode", "production", false),
    ];
    let s = to_json_layer(&original);
    let parsed = parse_json_layer(&s);
    assert_eq!(parsed.len(), 3);
    for (o, p) in original.iter().zip(parsed.iter()) {
        assert_eq!(o.key, p.key);
        assert_eq!(o.value, p.value);
        assert_eq!(o.required, p.required);
    }
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_diff_after_merge() {
    // 算 "merge 后 vs 原 base" 的差异
    let base = vec![
        ConfigEntry::new("a", "1", false),
        ConfigEntry::new("b", "2", false),
    ];
    let overlay = vec![
        ConfigEntry::new("b", "NEW", false),
        ConfigEntry::new("c", "3", false),
    ];
    let merged = merge(&base, &overlay).unwrap();
    let (added, removed, changed) = diff(&base, &merged);
    assert!(added.contains(&"c"), "新 key c 加入");
    assert!(removed.is_empty(), "无移除");
    assert!(changed.contains(&"b"), "b 值改");
}

#[test]
fn integration_validate_merged_result() {
    // merge 后, validate 看是否满足 required
    let base = vec![ConfigEntry::new("a", "", true)];
    let overlay = vec![ConfigEntry::new("a", "filled", true)];
    let merged = merge(&base, &overlay).unwrap();
    assert!(validate_all(&merged).is_ok(), "merge 后 OK");
    assert!(missing_required(&merged).is_empty());
}

#[test]
fn integration_three_layer_then_diff() {
    let def = vec![ConfigEntry::new("model", "gpt-4", false)];
    let file = vec![ConfigEntry::new("model", "claude", false)];
    let ov = vec![ConfigEntry::new("model", "local-llama", false)];
    let merged = merge_three_layers(&def, &file, &ov).unwrap();
    let (a, r, c) = diff(&def, &merged);
    assert!(a.is_empty());
    assert!(r.is_empty());
    assert!(c.contains(&"model"));
    assert_eq!(merged[0].value, "local-llama");
}

#[test]
fn integration_parse_validate_lookup() {
    let json = r#"[
        {"key":"api_key","value":"abc123","required":true},
        {"key":"model","value":"claude","required":false},
        {"key":"","value":"orphan","required":false}
    ]"#;
    let v = parse_json_layer(json);
    assert_eq!(v.len(), 3, "3 entries (含 empty key, 由 validate 拒绝)");
    assert!(validate_all(&v).is_err(), "empty key 应 fail validate");
    // 但 lookup 仍能查 valid 的:
    assert_eq!(lookup(&v, "api_key"), Some("abc123"));
    assert_eq!(lookup(&v, "model"), Some("claude"));
}

#[test]
fn integration_keys_all_valid_via_naming() {
    let json = r#"[
        {"key":"valid_key","value":"1"},
        {"key":"with-dash","value":"2"},
        {"key":"CamelCase","value":"3"},
        {"key":"BAD key","value":"4"},
        {"key":"bad.dot","value":"5"}
    ]"#;
    let v = parse_json_layer(json);
    // validate 内只查 empty key, key 命名约束是 key_is_valid 单独检查
    let valid_keys: Vec<&str> = v
        .iter()
        .filter(|e| key_is_valid(&e.key))
        .map(|e| e.key.as_str())
        .collect();
    assert_eq!(valid_keys.len(), 3, "3 个 key 通过 key_is_valid");
}
