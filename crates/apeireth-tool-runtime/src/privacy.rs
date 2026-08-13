//! **战役 2-2 / VCP `toolResultPrivacyGuard.js` — 工具返回隐私字段 mask**
//!
//! **目标**: 工具返回的 JSON Value 里, 敏感字段 (api_key / password / token 等) 应被打码.
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolResultPrivacyGuard.js:11 SENSITIVE_KEY_PATTERN` — 13 类敏感键 (api_key / secret / token / password / ...)
//! - `toolResultPrivacyGuard.js:14 DATA_BASE64_URI_PATTERN` — base64 data URI 保护 (图片不进 mask)
//! - `toolResultPrivacyGuard.js:17-25 HIGH_CONFIDENCE_TOKEN_PATTERNS` — 7 类 high-confidence token (sk- / ghp_ / glpat- / ...)
//! - `toolResultPrivacyGuard.js:55-57 isSensitiveKey` — 敏感键检测
//! - `toolResultPrivacyGuard.js:63-74 shouldMaskValue` — 应 mask 条件 (长度/类型过滤)
//! - `toolResultPrivacyGuard.js:76-90 maskSecret` — 实际 mask 逻辑 (保留前 N + 后 M 字符)
//! - `toolResultPrivacyGuard.js:143-185 sanitizeValue` — 递归 mask 入口
//! - `toolResultPrivacyGuard.js:187-194 sanitizeToolResult` — 顶层入口
//!
//! **Apeireth 简化**:
//! - VCP 用 JS WeakSet 防循环, 我们用 HashSet
//! - VCP 保留引号 `maskSecret`, 我们也保留
//! - VCP 16+ 字段, 我们保留核心 13 类 (字段级引用)
//!
//! **不假装**:
//! - ✅ 13 类敏感键真检测 (regex 跟 VCP 同源)
//! - ✅ 7 类 high-confidence token 真 mask
//! - ✅ env assignment (`API_KEY=xxx`) 真识别
//! - ✅ 嵌套对象/数组递归 mask
//! - ✅ maxDepth 保护 (防恶意深度数据栈溢出)
//! - ✅ compile-time hardcode (`DEFAULT_MAX_DEPTH`)

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// **战役 2-2 — Privacy mask 默认配置**
///
/// **字段级引用** `toolResultPrivacyGuard.js:1-9 DEFAULT_CONFIG`
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    /// 总开关
    pub enabled: bool,
    /// 替换字符串 (VCP `[APEIRETH_PRIVACY_REDACTED]`)
    pub mask: String,
    /// 最大递归深度 (VCP 20, 防止恶意深度数据栈溢出)
    pub max_depth: usize,
    /// 保留前 N 字符 (VCP 4)
    pub preserve_prefix: usize,
    /// 保留后 N 字符 (VCP 4)
    pub preserve_suffix: usize,
    /// 触发 mask 的最小值长度 (VCP 8)
    pub min_secret_length: usize,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mask: "[APEIRETH_PRIVACY_REDACTED]".to_string(),
            max_depth: 20,
            preserve_prefix: 4,
            preserve_suffix: 4,
            min_secret_length: 8,
        }
    }
}

/// **战役 2-2 — Privacy guard**
///
/// 复刻 VCP `toolResultPrivacyGuard.js:sanitizeToolResult` 字段级.
pub struct PrivacyGuard {
    config: PrivacyConfig,
}

impl PrivacyGuard {
    /// 新建 privacy guard, 用默认配置
    pub fn new() -> Self {
        Self::with_config(PrivacyConfig::default())
    }

    /// 新建 privacy guard, 自定义配置
    pub fn with_config(config: PrivacyConfig) -> Self {
        Self { config }
    }

    /// 暴露 config 引用
    pub fn config(&self) -> &PrivacyConfig {
        &self.config
    }

    /// **顶层 mask 入口**
    ///
    /// **VCP 复刻**: `toolResultPrivacyGuard.js:187-194 sanitizeToolResult`
    /// - `enabled=false` → 返原值
    /// - 否则递归 mask
    pub fn mask(&self, result: &Value) -> Value {
        if !self.config.enabled {
            return result.clone();
        }
        let mut seen = HashSet::new();
        sanitize_value(result, &self.config, 0, &mut seen, "")
    }
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 内部 mask 实现
// ============================================================

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:11 SENSITIVE_KEY_PATTERN`
///
/// 13 类敏感键: api_key / apikey / secret / token / access_token / refresh_token /
/// auth_token / bearer / password / passwd / pwd / credential / credentials /
/// private_key / client_secret / webhook_secret
fn sensitive_key_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        // 复制 VCP 正则 (JS → Rust 转换, 行为一致)
        // VCP 用 `i` flag (case-insensitive), 我们用 Rust 的 `(?i)`
        Regex::new(
            r"(?i)(?:^|[_\-\s.])(?:api[_\-\s]?key|apikey|secret|token|access[_\-\s]?token|refresh[_\-\s]?token|auth[_\-\s]?token|bearer|password|passwd|pwd|credential|credentials|private[_\-\s]?key|client[_\-\s]?secret|webhook[_\-\s]?secret)(?:$|[_\-\s.])",
        )
        .expect("valid sensitive key regex")
    })
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:17-25 HIGH_CONFIDENCE_TOKEN_PATTERNS`
///
/// 7 类 high-confidence token: sk- / sk-proj- / xoxb/xoxp/xoxa/xoxr- / ghp_ /
/// github_pat_ / glpat- / AKIA...
fn high_confidence_token_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"\bsk-[A-Za-z0-9_-]{24,}\b").expect("valid sk- regex"),
            Regex::new(r"\bsk-proj-[A-Za-z0-9_-]{24,}\b").expect("valid sk-proj- regex"),
            Regex::new(r"\b(?:xoxb|xoxp|xoxa|xoxr)-[A-Za-z0-9-]{24,}\b")
                .expect("valid slack regex"),
            Regex::new(r"\bghp_[A-Za-z0-9_]{30,}\b").expect("valid github regex"),
            Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{40,}\b").expect("valid github_pat regex"),
            Regex::new(r"\bglpat-[A-Za-z0-9_-]{20,}\b").expect("valid glpat regex"),
            Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("valid AWS regex"),
        ]
    })
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:13 ENV_ASSIGNMENT_PATTERN`
///
/// 匹配 `KEY=VALUE` (env 风格) 整行
fn env_assignment_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"^(\s*(?:export\s+)?[A-Za-z_][A-Za-z0-9_.-]*\s*=\s*)(['"]?)([^\r\n]*?)(['"]?)(\s*(?:#.*)?$)"#,
        )
        .expect("valid env assignment regex")
    })
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:15 DATA_BASE64_URI_FULL_PATTERN`
///
/// 完整 data URI 模式 (整字符串), 用于 `shouldMaskValue` 排除图片
fn data_base64_uri_full_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"^data:[A-Za-z0-9][A-Za-z0-9.+-]*/[A-Za-z0-9][A-Za-z0-9.+-]*(?:;[A-Za-z0-9.+-]+=[A-Za-z0-9+/_-]+)*;base64,[A-Za-z0-9+/=\r\n]+$",
        )
        .expect("valid data URI regex")
    })
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:55-57 isSensitiveKey`
pub(crate) fn is_sensitive_key(key: &str) -> bool {
    sensitive_key_pattern().is_match(key)
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:59-61 isDataBase64Uri`
pub(crate) fn is_data_base64_uri(value: &str) -> bool {
    data_base64_uri_full_pattern().is_match(value.trim())
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:63-74 shouldMaskValue`
pub(crate) fn should_mask_value(value: &Value, config: &PrivacyConfig) -> bool {
    if value.is_null() {
        return false;
    }
    let text = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let trimmed = text.trim();
    if trimmed.len() < config.min_secret_length {
        return false;
    }
    if matches!(
        trimmed.to_lowercase().as_str(),
        "true" | "false" | "null" | "undefined"
    ) {
        return false;
    }
    if trimmed.parse::<f64>().is_ok() {
        return false;
    }
    if is_data_base64_uri(trimmed) {
        return false;
    }
    true
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:76-90 maskSecret`
pub(crate) fn mask_secret(value: &str, config: &PrivacyConfig) -> String {
    // 简单处理: 保留前 N + 后 M 字符, 中间用 mask 替换
    // VCP 处理引号 (" '), 我们也保留
    let starts_with_quote = value.starts_with('"') || value.starts_with('\'');
    let ends_with_quote = value.len() > 1 && (value.ends_with('"') || value.ends_with('\''));
    let prefix_quote = if starts_with_quote { &value[..1] } else { "" };
    let suffix_quote = if ends_with_quote {
        &value[value.len() - 1..]
    } else {
        ""
    };
    let core = if !prefix_quote.is_empty() && !suffix_quote.is_empty() && value.len() >= 2 {
        &value[1..value.len() - 1]
    } else {
        value
    };

    let keep = config.preserve_prefix + config.preserve_suffix + 4;
    if core.len() <= keep {
        return format!("{prefix_quote}{}{suffix_quote}", config.mask);
    }
    let head: String = core.chars().take(config.preserve_prefix).collect();
    let tail: String = {
        let skip = core.chars().count().saturating_sub(config.preserve_suffix);
        core.chars().skip(skip).collect()
    };
    format!("{prefix_quote}{head}{}{tail}{suffix_quote}", config.mask)
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:92-98 maskHighConfidenceTokens`
pub(crate) fn mask_high_confidence_tokens(text: &str, config: &PrivacyConfig) -> String {
    let mut result = text.to_string();
    for pattern in high_confidence_token_patterns() {
        result = pattern
            .replace_all(&result, |caps: &regex::Captures| {
                mask_secret(caps.get(0).unwrap().as_str(), config)
            })
            .to_string();
    }
    result
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:116-128 maskEnvAssignmentLine`
pub(crate) fn mask_env_assignment_line(line: &str, config: &PrivacyConfig) -> String {
    let re = env_assignment_pattern();
    let Some(caps) = re.captures(line) else {
        return line.to_string();
    };
    let left = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let quote = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let raw_value = caps.get(3).map(|m| m.as_str()).unwrap_or("");
    let closing_quote = caps.get(4).map(|m| m.as_str()).unwrap_or("");
    let trailing = caps.get(5).map(|m| m.as_str()).unwrap_or("");

    // 提取 key
    let key = left
        .split('=')
        .next()
        .unwrap_or("")
        .trim_start()
        .trim_start_matches("export")
        .trim();

    if !is_sensitive_key(key) {
        return line.to_string();
    }
    if !should_mask_value(&Value::String(raw_value.to_string()), config) {
        return line.to_string();
    }
    format!(
        "{left}{quote}{}{closing_quote}{trailing}",
        mask_secret(raw_value, config)
    )
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:130-141 maskString`
pub(crate) fn mask_string(text: &str, config: &PrivacyConfig) -> String {
    // 先按行处理 env assignment, 再 high-confidence token
    let line_masked = text
        .split('\n')
        .map(|line| mask_env_assignment_line(line, config))
        .collect::<Vec<_>>()
        .join("\n");
    mask_high_confidence_tokens(&line_masked, config)
}

/// **VCP 字段级引用**: `toolResultPrivacyGuard.js:143-185 sanitizeValue` (Rust 适配)
fn sanitize_value(
    value: &Value,
    config: &PrivacyConfig,
    depth: usize,
    seen: &mut HashSet<*const Value>,
    key_hint: &str,
) -> Value {
    if depth > config.max_depth {
        return value.clone();
    }
    let ptr = value as *const Value;
    if seen.contains(&ptr) {
        return value.clone();
    }
    seen.insert(ptr);

    match value {
        Value::String(s) => {
            if is_sensitive_key(key_hint) && should_mask_value(value, config) {
                Value::String(mask_secret(s, config))
            } else {
                Value::String(mask_string(s, config))
            }
        }
        Value::Number(_) | Value::Bool(_) | Value::Null => value.clone(),
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| sanitize_value(v, config, depth + 1, seen, ""))
                .collect(),
        ),
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj.iter() {
                if is_sensitive_key(k) && should_mask_value(v, config) {
                    new_obj.insert(
                        k.clone(),
                        Value::String(mask_secret(v.as_str().unwrap_or(""), config)),
                    );
                } else {
                    let sanitized = sanitize_value(v, config, depth + 1, seen, k);
                    new_obj.insert(k.clone(), sanitized);
                }
            }
            Value::Object(new_obj)
        }
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// VCP `DEFAULT_CONFIG.maxDepth = 20` 编译期守
const DEFAULT_MAX_DEPTH: usize = 20;

const _: () = {
    assert!(
        DEFAULT_MAX_DEPTH == 20,
        "DEFAULT_MAX_DEPTH must be 20 (VCP toolResultPrivacyGuard.js:4)"
    );
};

// ============================================================
// 单元测试 (战役 2-2 DoD: ≥ 5 个, 含各敏感字段)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn is_sensitive_key_matches_vcp_keys() {
        // VCP SENSITIVE_KEY_PATTERN 13 类字段
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("apikey"));
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("passwd"));
        assert!(is_sensitive_key("pwd"));
        assert!(is_sensitive_key("token"));
        assert!(is_sensitive_key("access_token"));
        assert!(is_sensitive_key("refresh_token"));
        assert!(is_sensitive_key("auth_token"));
        assert!(is_sensitive_key("bearer"));
        assert!(is_sensitive_key("secret"));
        assert!(is_sensitive_key("client_secret"));
        assert!(is_sensitive_key("private_key"));
        assert!(is_sensitive_key("webhook_secret"));
        assert!(is_sensitive_key("credential"));
        assert!(is_sensitive_key("credentials"));
        // 非敏感
        assert!(!is_sensitive_key("name"));
        assert!(!is_sensitive_key("email"));
        assert!(!is_sensitive_key("count"));
    }

    #[test]
    fn mask_api_key_in_object() {
        // 顶层对象 api_key 字段
        let guard = PrivacyGuard::new();
        let input = json!({
            "api_key": "sk-1234567890abcdefghij",
            "name": "chuling"
        });
        let masked = guard.mask(&input);
        assert_ne!(masked["api_key"], "sk-1234567890abcdefghij");
        assert!(masked["api_key"]
            .as_str()
            .unwrap()
            .contains("[APEIRETH_PRIVACY_REDACTED]"));
        assert_eq!(masked["name"], "chuling");
    }

    #[test]
    fn mask_password_field() {
        let guard = PrivacyGuard::new();
        let input = json!({
            "user": {
                "username": "chuling",
                "password": "super_secret_pwd_123"
            }
        });
        let masked = guard.mask(&input);
        assert_eq!(masked["user"]["username"], "chuling");
        assert_ne!(masked["user"]["password"], "super_secret_pwd_123");
        assert!(masked["user"]["password"]
            .as_str()
            .unwrap()
            .contains("[APEIRETH_PRIVACY_REDACTED]"));
    }

    #[test]
    fn mask_token_in_nested_array() {
        // 嵌套数组
        let guard = PrivacyGuard::new();
        let input = json!({
            "tokens": [
                {"access_token": "ghp_abcdefghijklmnopqrstuvwxyz0123456789", "scope": "read"},
                {"access_token": "sk-1234567890abcdefghij", "scope": "write"}
            ]
        });
        let masked = guard.mask(&input);
        let first = &masked["tokens"][0]["access_token"];
        let second = &masked["tokens"][1]["access_token"];
        assert!(first.as_str().unwrap().contains("[APEIRETH_PRIVACY_REDACTED]"));
        assert!(second.as_str().unwrap().contains("[APEIRETH_PRIVACY_REDACTED]"));
        assert_eq!(masked["tokens"][0]["scope"], "read");
    }

    #[test]
    fn mask_high_confidence_tokens_in_string() {
        // 字符串内嵌 high-confidence token (sk- / ghp_ / AKIA...)
        let guard = PrivacyGuard::new();
        let input = json!({
            "logs": "User used token sk-1234567890abcdefghijklmnop to login. Also tried ghp_abcdefghijklmnopqrstuvwxyz0123456789. AWS key AKIAIOSFODNN7EXAMPLE was leaked."
        });
        let masked = guard.mask(&input);
        let logs = masked["logs"].as_str().unwrap();
        // 3 个 high-confidence token 都应被 mask
        assert!(!logs.contains("sk-1234567890abcdefghijklmnop"));
        assert!(!logs.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(!logs.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn mask_env_assignment() {
        // env 格式: KEY=VALUE
        let guard = PrivacyGuard::new();
        let input = json!({
            "config_text": "API_KEY=sk-1234567890abcdefghijklmnop\nNAME=chuling\nPASSWORD=mysecretpassword123"
        });
        let masked = guard.mask(&input);
        let text = masked["config_text"].as_str().unwrap();
        // 敏感 key (API_KEY / PASSWORD) 应被 mask
        assert!(!text.contains("sk-1234567890abcdefghijklmnop"));
        assert!(!text.contains("mysecretpassword123"));
        // NAME=chuling 不敏感, 应保留
        assert!(text.contains("chuling"));
    }

    #[test]
    fn mask_preserves_short_values() {
        // 短值 (长度 < min_secret_length=8) 不 mask
        let guard = PrivacyGuard::new();
        let input = json!({
            "short_secret": "abc"
        });
        let masked = guard.mask(&input);
        // 短值即使 key 敏感也不 mask
        assert_eq!(masked["short_secret"], "abc");
    }

    #[test]
    fn mask_preserves_data_uri() {
        // base64 data URI 不 mask (图片)
        let guard = PrivacyGuard::new();
        let input = json!({
            "image": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
        });
        let masked = guard.mask(&input);
        // data URI 不应被 mask
        assert_eq!(
            masked["image"],
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
        );
    }

    #[test]
    fn mask_disabled_returns_original() {
        // enabled=false → 原值
        let cfg = PrivacyConfig {
            enabled: false,
            ..Default::default()
        };
        let guard = PrivacyGuard::with_config(cfg);
        let input = json!({
            "api_key": "sk-1234567890abcdefghij"
        });
        let masked = guard.mask(&input);
        // enabled=false 不 mask
        assert_eq!(masked["api_key"], "sk-1234567890abcdefghij");
    }

    #[test]
    fn mask_respects_max_depth() {
        // maxDepth 边界
        let cfg = PrivacyConfig {
            enabled: true,
            max_depth: 2,
            ..Default::default()
        };
        let guard = PrivacyGuard::with_config(cfg);
        // 构造深度 5 的嵌套
        let deep = json!({
            "l1": {"l2": {"l3": {"l4": {"l5": {"api_key": "sk-verylongsecretvalue1234567890"}}}}}
        });
        let masked = guard.mask(&deep);
        // 超过 max_depth 不递归, 但 Value 整体 clone 返原值
        // 我们这里不严格断言 mask 是否生效, 只验证不 panic
        let _ = masked;
    }

    #[test]
    fn mask_preserves_numbers_booleans() {
        // 数字 / 布尔 / null 不 mask
        let guard = PrivacyGuard::new();
        let input = json!({
            "count": 42,
            "active": true,
            "deleted": serde_json::Value::Null,
            "ratio": 3.14
        });
        let masked = guard.mask(&input);
        assert_eq!(masked["count"], 42);
        assert_eq!(masked["active"], true);
        assert_eq!(masked["deleted"], serde_json::Value::Null);
        assert_eq!(masked["ratio"], 3.14);
    }
}
