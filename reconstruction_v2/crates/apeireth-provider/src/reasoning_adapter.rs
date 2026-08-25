//! reasoning_adapter — 推理字段归一化适配件 (VCP reasoningContentAdapter 吸收).
//!
//! 0 装 PASS 严守: 真 REASONING_ALIASES 12 别名 + 启发式提取 + 真 normalize_tag + 真 wrap + 真 should_convert_for_model.
//! 不抄真 HTTP, 写纯 logic (per v1 1:1).
//!
//! v1 对齐 (N12 ②): 12 别名 + tag 归一化 + text 提取 + 模型能力过滤.

/// 推理字段别名表 (与 VCP REASONING_KEYS 1:1, 12 个).
pub const REASONING_ALIASES: [&str; 12] = [
    "reasoning_content",
    "reasoning",
    "reasoning_chunk",
    "reasoningChunk",
    "reasoning_summary",
    "reasoningSummary",
    "reasoning_details",
    "reasoningDetails",
    "reasoning_text",
    "reasoningText",
    "thinking",
    "thoughts",
];

/// 标签归一化: "thinking" (不区分大小写) → `thinking`, 其余一律 `think`.
pub fn normalize_tag(tag: &str) -> &'static str {
    if tag.trim().eq_ignore_ascii_case("thinking") { "thinking" } else { "think" }
}

/// 把推理文本包成 think 块.
pub fn wrap_reasoning_text(text: &str, tag: &str) -> String {
    if text.is_empty() { return String::new(); }
    let tag = normalize_tag(tag);
    let closing_prefix = if text.ends_with('\n') { "" } else { "\n" };
    format!("<{tag}>\n{text}{closing_prefix}</{tag}>\n")
}

/// 启发式提取 text (per v1 valueToReasoningText 简化).
pub fn extract_reasoning_text(value: &serde_json::Value) -> String {
    if let Some(s) = value.as_str() { return s.to_string(); }
    if let Some(arr) = value.as_array() {
        return arr.iter().filter_map(|v| {
            let t = extract_reasoning_text(v);
            if t.is_empty() { None } else { Some(t) }
        }).collect::<Vec<_>>().join("
");
    }
    if let Some(map) = value.as_object() {
        // 优先取 TEXT_VALUE_KEYS ∪ REASONING_ALIASES 命中的键
        let mut out = Vec::new();
        for k in map.keys() {
            if REASONING_ALIASES.contains(&k.as_str()) {
                if let Some(v) = map.get(k) {
                    let t = extract_reasoning_text(v);
                    if !t.is_empty() { out.push(t); }
                }
            }
        }
        return out.join("
");
    }
    String::new()
}

/// 适配器配置 (VCP config.env 对齐).
#[derive(Debug, Clone)]
pub struct ReasoningAdapterConfig {
    pub enabled: bool,
    pub model_filters: Vec<String>,
    pub tag: String,
}

impl Default for ReasoningAdapterConfig {
    fn default() -> Self {
        Self { enabled: false, model_filters: Vec::new(), tag: "think".into() }
    }
}

impl ReasoningAdapterConfig {
    /// 从环境变量构造 (对齐 v1).
    pub fn from_env() -> Self {
        let enabled = std::env::var("APEIRETH_REASONING_ENABLED")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true"))
            .unwrap_or(false);
        let model_filters = std::env::var("APEIRETH_REASONING_MODEL_FILTERS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let tag = std::env::var("APEIRETH_REASONING_TAG").unwrap_or_else(|_| "think".into());
        Self { enabled, model_filters, tag }
    }

    /// 目标模型是否需要推理转换 (VCP shouldConvertReasoningForModel 1:1).
    pub fn should_convert_for_model(&self, model_name: &str) -> bool {
        if !self.enabled || model_name.trim().is_empty() || self.model_filters.is_empty() { return false; }
        let model = model_name.to_ascii_lowercase();
        self.model_filters.iter().any(|f| model.contains(f.as_str()))
    }
}

/// 从对象删除全部推理别名字段. 返回删除的字段数.
pub fn remove_reasoning_fields(source: &mut serde_json::Value) -> usize {
    let Some(map) = source.as_object_mut() else { return 0; };
    let mut removed = 0;
    for alias in REASONING_ALIASES {
        if map.remove(alias).is_some() { removed += 1; }
    }
    removed
}

/// 构造客户端可见内容: think 块前置于 content (per v1 buildClientVisibleContent 简化).
pub fn build_client_visible_content(message: &serde_json::Value, config: &ReasoningAdapterConfig, model_name: &str) -> String {
    let visible = message.get("content").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    if !config.should_convert_for_model(model_name) { return visible; }
    let reasoning_text = extract_reasoning_text(message);
    if reasoning_text.is_empty() { return visible; }
    format!("{}{}", wrap_reasoning_text(&reasoning_text, &config.tag), visible)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enabled_config(filters: &[&str]) -> ReasoningAdapterConfig {
        ReasoningAdapterConfig { enabled: true, model_filters: filters.iter().map(|s| s.to_ascii_lowercase()).collect(), tag: "think".into() }
    }

    #[test]
    fn aliases_count_12() {
        assert_eq!(REASONING_ALIASES.len(), 12);
    }

    #[test]
    fn each_alias_recognized() {
        for a in REASONING_ALIASES {
            let v = json!({ a: "deep thought" });
            assert_eq!(extract_reasoning_text(&v), "deep thought");
        }
    }

    #[test]
    fn non_object_source_yields_empty() {
        assert_eq!(extract_reasoning_text(&json!("plain")), "");
        assert_eq!(extract_reasoning_text(&json!(42)), "");
        assert_eq!(extract_reasoning_text(&serde_json::Value::Null), "");
    }

    #[test]
    fn array_of_parts_joined() {
        let v = json!({ "reasoning_chunk": ["a", "b", null] });
        assert_eq!(extract_reasoning_text(&v), "a
b");
    }

    #[test]
    fn multiple_aliases_joined() {
        let v = json!({ "reasoning_content": "step 1", "thinking": "step 2" });
        assert_eq!(extract_reasoning_text(&v), "step 1
step 2");
    }

    #[test]
    fn tag_normalized() {
        assert_eq!(normalize_tag("think"), "think");
        assert_eq!(normalize_tag("THINKING"), "thinking");
        assert_eq!(normalize_tag("weird"), "think");
    }

    #[test]
    fn wrap_adds_newline_before_closing() {
        assert_eq!(wrap_reasoning_text("abc", "think"), "<think>
abc
</think>
");
        assert_eq!(wrap_reasoning_text("", "think"), "");
    }

    #[test]
    fn remove_reasoning_fields_strips_aliases_only() {
        let mut v = json!({ "content": "keep me", "reasoning_content": "x", "thinking": "y", "thoughts": "z" });
        let removed = remove_reasoning_fields(&mut v);
        assert_eq!(removed, 3);
        assert_eq!(v, json!({ "content": "keep me" }));
    }

    #[test]
    fn remove_on_non_object_is_noop() {
        let mut v = json!([1, 2]);
        assert_eq!(remove_reasoning_fields(&mut v), 0);
    }

    #[test]
    fn filter_substring_match_case_insensitive() {
        let cfg = enabled_config(&["kimi", "claude"]);
        assert!(cfg.should_convert_for_model("kimi-k2-0711"));
        assert!(cfg.should_convert_for_model("Claude-Sonnet-4"));
        assert!(!cfg.should_convert_for_model("gpt-5"));
    }

    #[test]
    fn empty_filters_converts_nothing() {
        let cfg = enabled_config(&[]);
        assert!(!cfg.should_convert_for_model("kimi"));
        let off = ReasoningAdapterConfig::default();
        assert!(!off.should_convert_for_model("kimi"));
    }

    #[test]
    fn visible_content_prepends_think_block() {
        let cfg = enabled_config(&["deepseek"]);
        let msg = json!({ "content": "42 is the answer", "reasoning_content": "let me think" });
        let out = build_client_visible_content(&msg, &cfg, "deepseek-chat");
        assert!(out.contains("<think>"));
        assert!(out.ends_with("42 is the answer"));
    }

    #[test]
    fn visible_content_untouched_for_unmatched_model() {
        let cfg = enabled_config(&["deepseek"]);
        let msg = json!({ "content": "plain", "reasoning_content": "hidden" });
        assert_eq!(build_client_visible_content(&msg, &cfg, "gpt-x"), "plain");
    }

    #[test]
    fn visible_content_no_reasoning_keeps_content() {
        let cfg = enabled_config(&["kimi"]);
        let msg = json!({ "content": "no reasoning here" });
        assert_eq!(build_client_visible_content(&msg, &cfg, "kimi-k2"), "no reasoning here");
    }
}
