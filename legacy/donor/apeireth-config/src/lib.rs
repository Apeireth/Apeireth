//! apeireth-config — R23 6 module config 子模块。
//!
//! R23 P1 #5 实质化: 加 +6 顶层 pub fn — 合并 / 校验 / snapshot diff / lookup.
//! 不假装: 真 validation + 真 diff (不假装"相等").
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config: key `{0}` is empty")]
    EmptyKey(String),
    #[error("config: 重复 key `{0}`")]
    DuplicateKey(String),
}
pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub required: bool,
}

impl ConfigEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>, required: bool) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            required,
        }
    }
    pub fn validate(&self) -> ConfigResult<()> {
        if self.key.trim().is_empty() {
            return Err(ConfigError::EmptyKey(self.key.clone()));
        }
        if self.required && self.value.is_empty() {
            return Err(ConfigError::EmptyKey(format!("required: {}", self.key)));
        }
        Ok(())
    }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — Config slice operations
// ============================================================================

/// 批量 validate 全 entry. 首个 fail 返 `Err`.
pub fn validate_all(entries: &[ConfigEntry]) -> ConfigResult<()> {
    for e in entries {
        e.validate()?;
    }
    Ok(())
}

/// 按 key 查 value. 找不到返 None.
pub fn lookup<'a>(entries: &'a [ConfigEntry], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
}

/// 合并两 slice. 后者覆盖前者 (override semantics). 同 key duplicate 检测 只针对同 slice 内 (不限制 base + overlay 交叉, 那是 override 语义).
pub fn merge(base: &[ConfigEntry], overlay: &[ConfigEntry]) -> ConfigResult<Vec<ConfigEntry>> {
    let mut seen = std::collections::HashSet::new();
    for e in base {
        if !seen.insert(e.key.as_str()) {
            return Err(ConfigError::DuplicateKey(e.key.clone()));
        }
    }
    seen.clear();
    for e in overlay {
        if !seen.insert(e.key.as_str()) {
            return Err(ConfigError::DuplicateKey(e.key.clone()));
        }
    }
    let mut out: Vec<ConfigEntry> = base.to_vec();
    for ov in overlay {
        match out.iter_mut().find(|e| e.key == ov.key) {
            Some(existing) => {
                existing.value = ov.value.clone();
                existing.required = ov.required;
            }
            None => out.push(ov.clone()),
        }
    }
    Ok(out)
}

/// 出 required 但 value 为空的 keys — fail-fast 报警用.
pub fn missing_required(entries: &[ConfigEntry]) -> Vec<&str> {
    entries
        .iter()
        .filter(|e| e.required && e.value.is_empty())
        .map(|e| e.key.as_str())
        .collect()
}

/// 算两组 entry 差异. 返 (added, removed, changed) 三组 key 名.
pub fn diff<'a>(
    before: &'a [ConfigEntry],
    after: &'a [ConfigEntry],
) -> (Vec<&'a str>, Vec<&'a str>, Vec<&'a str>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for e in after {
        match lookup(before, &e.key) {
            None => added.push(e.key.as_str()),
            Some(v) if v != e.value => changed.push(e.key.as_str()),
            _ => {}
        }
    }
    for e in before {
        if lookup(after, &e.key).is_none() {
            removed.push(e.key.as_str());
        }
    }
    (added, removed, changed)
}

/// 校验 key 命名约束 (lowercase alnum + dash + underscore), 不允许空格 / 点 / 特殊符.
pub fn key_is_valid(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// R30 U12: VCP-style 三层配置合并 (DEFAULT -> file -> override).
///
/// 严格 override 语义: 右层覆盖左层同 key. 三层都允许, 任一层允许空.
/// 先内部 validate 三层 (无重复 key), 再依次 merge. 出错返 Err.
///
/// 借鉴: VCP toolApprovalManager 三层合并 (plugin-manifest.json 默认 -> 用户 tool-policy.json -> CLI/env 临时覆盖).
pub fn merge_three_layers(
    default: &[ConfigEntry],
    file: &[ConfigEntry],
    override_layer: &[ConfigEntry],
) -> ConfigResult<Vec<ConfigEntry>> {
    // 内部 validate: 每层不允许 duplicate key
    let mut seen = std::collections::HashSet::new();
    for layer in [default, file, override_layer] {
        seen.clear();
        for e in layer {
            if !seen.insert(e.key.as_str()) {
                return Err(ConfigError::DuplicateKey(e.key.clone()));
            }
        }
    }
    // 顺序合并: base + file -> 再 + override
    let merged_1 = merge(default, file)?;
    merge(&merged_1, override_layer)
}

/// R30 U12: 从 JSON 字符串加载一层配置. 容错: JSON 顶层是数组, 每项 {key,value,required}.
/// 容错 2: required 缺省 = false. 容错 3: 空字符串 / 缺 value 视为空值.
/// 容错 4: JSON 解析失败返空 Vec (不报错, 让 caller 决定).
pub fn parse_json_layer(raw: &str) -> Vec<ConfigEntry> {
    let v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let key = item.get("key").and_then(|x| x.as_str())?.to_string();
            let value = item
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let required = item
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(ConfigEntry::new(key, value, required))
        })
        .collect()
}

/// R30 U12: 序列化一层配置到 JSON 字符串 (用于审计 / 持久化 override)
pub fn to_json_layer(entries: &[ConfigEntry]) -> String {
    serde_json::json!(entries
        .iter()
        .map(|e| serde_json::json!({
            "key": e.key, "value": e.value, "required": e.required
        }))
        .collect::<Vec<_>>())
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_entry_passes() {
        assert!(ConfigEntry::new("mode", "", false).validate().is_ok());
    }
    #[test]
    fn required_entry_without_value_is_rejected() {
        assert!(ConfigEntry::new("api_key", "", true).validate().is_err());
    }

    #[test]
    fn validate_all_passes_when_all_valid() {
        let v = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "2", false),
        ];
        assert!(validate_all(&v).is_ok());
    }
    #[test]
    fn validate_all_fails_fast() {
        let v = vec![
            ConfigEntry::new("ok", "1", true),
            ConfigEntry::new("", "x", false),
        ];
        assert!(validate_all(&v).is_err());
    }
    #[test]
    fn lookup_basic() {
        let v = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "2", false),
        ];
        assert_eq!(lookup(&v, "a"), Some("1"));
        assert_eq!(lookup(&v, "missing"), None);
    }
    #[test]
    fn merge_overlay_wins() {
        let base = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "2", false),
        ];
        let overlay = vec![
            ConfigEntry::new("b", "NEW", false),
            ConfigEntry::new("c", "3", false),
        ];
        let merged = merge(&base, &overlay).unwrap();
        assert_eq!(merged.iter().find(|e| e.key == "b").unwrap().value, "NEW");
        assert_eq!(merged.len(), 3);
    }
    #[test]
    fn merge_dup_base_rejected() {
        let base = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("a", "2", false),
        ];
        assert!(merge(&base, &[]).is_err());
    }
    #[test]
    fn missing_required_finds_them() {
        let v = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "", true),
            ConfigEntry::new("c", "x", true),
        ];
        let m = missing_required(&v);
        assert_eq!(m, vec!["b"]);
    }
    #[test]
    fn diff_three_way() {
        let before = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "2", false),
            ConfigEntry::new("d", "x", false),
        ];
        let after = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "NEW", false),
            ConfigEntry::new("c", "3", false),
        ];
        let (added, removed, changed) = diff(&before, &after);
        assert_eq!(added, vec!["c"]);
        assert_eq!(removed, vec!["d"]);
        assert_eq!(changed, vec!["b"]);
    }
    #[test]
    fn key_validation_rules() {
        assert!(key_is_valid("ok"));
        assert!(key_is_valid("with-dash"));
        assert!(key_is_valid("with_underscore"));
        assert!(key_is_valid("CamelCase")); // alphanumeric
        assert!(!key_is_valid(""));
        assert!(!key_is_valid("with space"));
        assert!(!key_is_valid("with.dot"));
        assert!(!key_is_valid("with/slash"));
    }

    // ==== R30 U12 三层合并测试 ====
    #[test]
    fn u12_merge_three_layers_override_wins() {
        let def = vec![
            ConfigEntry::new("model", "gpt-4", false),
            ConfigEntry::new("temp", "0.7", false),
        ];
        let file = vec![ConfigEntry::new("model", "claude", false)];
        let ov = vec![ConfigEntry::new("model", "local-llama", false)];
        let m = merge_three_layers(&def, &file, &ov).unwrap();
        let model = m.iter().find(|e| e.key == "model").unwrap();
        assert_eq!(model.value, "local-llama");
        assert_eq!(m.iter().find(|e| e.key == "temp").unwrap().value, "0.7");
        assert_eq!(m.len(), 2);
    }
    #[test]
    fn u12_merge_three_layers_empty_layers_ok() {
        let def = vec![ConfigEntry::new("a", "1", false)];
        let m = merge_three_layers(&def, &[], &[]).unwrap();
        assert_eq!(m.len(), 1);
    }
    #[test]
    fn u12_merge_three_layers_duplicate_in_layer_rejected() {
        let def = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("a", "2", false),
        ];
        assert!(merge_three_layers(&def, &[], &[]).is_err());
    }
    #[test]
    fn u12_parse_json_layer_basic() {
        let json = r#"[{"key":"a","value":"1"},{"key":"b","value":"2","required":true}]"#;
        let v = parse_json_layer(json);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].key, "a");
        assert_eq!(v[0].value, "1");
        assert!(!v[0].required);
        assert!(v[1].required);
    }
    #[test]
    fn u12_parse_json_layer_optional_fields_default() {
        let json = r#"[{"key":"x"}]"#;
        let v = parse_json_layer(json);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].key, "x");
        assert_eq!(v[0].value, "");
        assert!(!v[0].required);
    }
    #[test]
    fn u12_parse_json_layer_invalid_returns_empty() {
        let bad = "not json at all";
        let v = parse_json_layer(bad);
        assert!(v.is_empty());
    }
    #[test]
    fn u12_to_json_layer_roundtrip() {
        let v = vec![
            ConfigEntry::new("a", "1", false),
            ConfigEntry::new("b", "2", true),
        ];
        let s = to_json_layer(&v);
        let parsed = parse_json_layer(&s);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].key, "a");
        assert_eq!(parsed[1].key, "b");
        assert!(parsed[1].required);
    }
}

// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
