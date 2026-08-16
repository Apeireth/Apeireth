//! `apeireth-provider::reasoning_adapter` — 推理字段归一化适配件 (N12 ②, VCP reasoningContentAdapter 吸收)
//!
//! **机制** (调研依据: research/source/vcptoolbox/modules/reasoningContentAdapter.js +
//! config.env.example; 任务锚点: team-work-doc §8.3/§8.4, backlog N12):
//!
//! 多家 LLM API 的推理内容字段名各不相同 (DeepSeek `reasoning_content` /
//! Claude `thinking` / Kimi `reasoning` ...). 本模块:
//!
//! 1. **入向归一化**: 从响应对象按 12 个别名 (REASONING_ALIASES, 与 VCP REASONING_KEYS
//!    1:1) 提取推理文本 (支持嵌套对象/数组递归), 去重合并为内部 think 块
//!    (`<think>...</think>`), 前置于可见内容。
//! 2. **按目标模型能力下发**: `should_convert_for_model` 白名单过滤 —
//!    仅当模型命中过滤器才做 think 块转换/下发; 未命中 → 原文不动 (0 改写)。
//! 3. **出向剥离**: `remove_reasoning_fields` 把全部别名字段从 JSON 对象删除
//!    (转发给不支持推理字段的下游模型前使用)。
//!
//! **默认行为对齐 VCP**: enabled=false + 过滤器空 → 任何模型都不转换
//! (保守默认, 显式配置后才生效)。
//!
//! **0 假装**:
//! - 别名清单以 VCP 源码 REASONING_KEYS 实数为准 (12 个); 台账写 "13 别名"
//!   为笔误 (源码核实), 未虚构第 13 个别名。
//! - 流式增量 (SSE chunk 级) 归一化未做: 本模块面向完整 JSON 对象;
//!   流式拼接后调用即可, 不需要独立机制。
//! - JS 版用 seen set 防对象环; serde_json::Value 是树结构无环, 该防护无对应物 (非缺失)。

use serde_json::Value;

/// 推理字段别名表 (与 VCP REASONING_KEYS 1:1, 提取按此顺序)。
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

/// 嵌套对象里视为文本内容的键 (VCP TEXT_VALUE_KEYS)。
const TEXT_VALUE_KEYS: [&str; 6] = ["text", "content", "summary", "value", "reasoning", "thinking"];

/// 标签归一化: "thinking" (不区分大小写) → `thinking`, 其余一律 `think`。
pub fn normalize_tag(tag: &str) -> &'static str {
    if tag.trim().eq_ignore_ascii_case("thinking") {
        "thinking"
    } else {
        "think"
    }
}

/// 把推理文本包成 think 块: `<tag>\n{text}\n</tag>\n`
/// (文本自身末尾已有换行则不重复加; 空文本 → 空串)。
pub fn wrap_reasoning_text(text: &str, tag: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let tag = normalize_tag(tag);
    let closing_prefix = if text.ends_with('\n') { "" } else { "\n" };
    format!("<{tag}>\n{text}{closing_prefix}</{tag}>\n")
}

/// 递归提取单个值里的推理文本 (VCP valueToReasoningText):
/// - 字符串原样; 数字转字符串; null/false → 空
/// - 数组 → 逐项提取, 空项丢弃, `\n` 连接
/// - 对象 → 优先取 TEXT_VALUE_KEYS ∪ REASONING_ALIASES 命中的键, `\n` 连接;
///   无命中键 → 空 (不序列化未知结构, 防噪音注入)
pub fn value_to_reasoning_text(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(false) => String::new(),
        Value::Bool(true) => "true".into(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(value_to_reasoning_text)
                .filter(|s| !s.is_empty())
                .collect();
            parts.join("\n")
        }
        Value::Object(map) => {
            let mut preferred = Vec::new();
            for (key, nested) in map {
                if TEXT_VALUE_KEYS.contains(&key.as_str()) || REASONING_ALIASES.contains(&key.as_str()) {
                    let text = value_to_reasoning_text(nested);
                    if !text.is_empty() {
                        preferred.push(text);
                    }
                }
            }
            preferred.join("\n")
        }
    }
}

/// 收集单个对象里的推理文本片段: 按别名表顺序扫描, 空白片段丢弃。
fn collect_reasoning_parts(source: &Value, out: &mut Vec<String>) {
    let Some(map) = source.as_object() else {
        return;
    };
    for alias in REASONING_ALIASES {
        let Some(value) = map.get(alias) else {
            continue;
        };
        let text = value_to_reasoning_text(value);
        if !text.trim().is_empty() {
            out.push(text);
        }
    }
}

/// 从单个对象提取推理文本: 按别名表顺序扫描, 完全重复的文本去重, `\n` 连接。
/// 非对象输入 → 空串。
pub fn extract_reasoning_text(source: &Value) -> String {
    let mut parts = Vec::new();
    collect_reasoning_parts(source, &mut parts);
    dedup_join(parts)
}

/// 多源提取 (message 本体 + 附加源如 delta/usage)。
/// **片段级全局去重** (对 VCP 的小改进: VCP 按整源文本去重, 流式场景 delta
/// 与 message 累积重复时会双发; 这里按片段去重消除该重复)。
pub fn extract_reasoning_text_from_sources(sources: &[&Value]) -> String {
    let mut parts = Vec::new();
    for source in sources {
        collect_reasoning_parts(source, &mut parts);
    }
    dedup_join(parts)
}

fn dedup_join(parts: Vec<String>) -> String {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    parts
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从对象删除全部推理别名字段 (出向剥离; 非对象 → 原样返回)。返回删除的字段数。
pub fn remove_reasoning_fields(source: &mut Value) -> usize {
    let Some(map) = source.as_object_mut() else {
        return 0;
    };
    let mut removed = 0;
    for alias in REASONING_ALIASES {
        if map.remove(alias).is_some() {
            removed += 1;
        }
    }
    removed
}

/// 适配器配置 (VCP config.env: ReasoningToContentEnabled / ReasoningToContentModel / Tag)。
#[derive(Debug, Clone)]
pub struct ReasoningAdapterConfig {
    /// 总开关 (VCP 默认 false)。
    pub enabled: bool,
    /// 模型白名单过滤器: 模型名 (小写) 包含任一过滤器子串才转换。空 = 不转换任何模型。
    pub model_filters: Vec<String>,
    /// think 块标签 (归一化后仅 think/thinking)。
    pub tag: String,
}

impl Default for ReasoningAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_filters: Vec::new(),
            tag: "think".into(),
        }
    }
}

impl ReasoningAdapterConfig {
    /// 从环境变量构造 (对齐 http_dispatch from_env 风格):
    /// - `APEIRETH_REASONING_ENABLED` = "1"/"true" 开启 (默认关)
    /// - `APEIRETH_REASONING_MODEL_FILTERS` = 逗号分隔子串白名单
    /// - `APEIRETH_REASONING_TAG` = 标签 (默认 think)
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

    /// 目标模型是否需要推理转换 (VCP shouldConvertReasoningForModel 1:1):
    /// enabled && 模型名非空 && 过滤器非空 && 任一过滤器子串命中 (大小写不敏感)。
    pub fn should_convert_for_model(&self, model_name: &str) -> bool {
        if !self.enabled || model_name.trim().is_empty() || self.model_filters.is_empty() {
            return false;
        }
        let model = model_name.to_ascii_lowercase();
        self.model_filters.iter().any(|f| model.contains(f.as_str()))
    }
}

/// 构造客户端可见内容 (VCP buildClientVisibleContent 1:1):
/// - 模型未命中过滤器 → message.content 原样 (字符串 content 之外 → 空串)
/// - 命中 → think 块前置于 content (无推理文本 → 原样)
pub fn build_client_visible_content(
    message: &Value,
    config: &ReasoningAdapterConfig,
    model_name: &str,
    additional_sources: &[&Value],
) -> String {
    let visible = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if !config.should_convert_for_model(model_name) {
        return visible;
    }

    let mut sources: Vec<&Value> = vec![message];
    sources.extend_from_slice(additional_sources);
    let reasoning_text = extract_reasoning_text_from_sources(&sources);
    if reasoning_text.is_empty() {
        return visible;
    }
    format!("{}{}", wrap_reasoning_text(&reasoning_text, &config.tag), visible)
}

/// 便捷入口: 归一化一个完整 Chat Completions JSON 响应体 (非流式)。
///
/// 提取 `choices[].message` (+ `choices[].delta`) 的推理字段 → 首条 message 的
/// 可见内容前置 think 块; 同时把 message/delta 里的别名字段剥离 (0 残留)。
/// body 不是对象/无 choices → 原样返回 (0 改写)。
pub fn normalize_chat_completion_body(body: &str, config: &ReasoningAdapterConfig, model_name: &str) -> String {
    let mut value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return body.to_string(),
    };
    let Some(choices) = value.get_mut("choices").and_then(Value::as_array_mut) else {
        return body.to_string();
    };

    let mut first_visible: Option<String> = None;
    for choice in choices.iter_mut() {
        if first_visible.is_none() {
            if let Some(msg) = choice.get("message") {
                let mut sources: Vec<&Value> = Vec::new();
                if let Some(delta) = choice.get("delta") {
                    sources.push(delta);
                }
                first_visible =
                    Some(build_client_visible_content(msg, config, model_name, &sources));
            }
        }
        if let Some(msg) = choice.get_mut("message") {
            remove_reasoning_fields(msg);
        }
        if let Some(delta) = choice.get_mut("delta") {
            remove_reasoning_fields(delta);
        }
    }

    match first_visible {
        Some(visible) => {
            if let Some(first_choice) = choices.first_mut() {
                if let Some(msg) = first_choice.get_mut("message") {
                    if let Some(content) = msg.get_mut("content") {
                        *content = Value::String(visible);
                    }
                }
            }
            serde_json::to_string(&value).unwrap_or_else(|_| body.to_string())
        }
        None => serde_json::to_string(&value).unwrap_or_else(|_| body.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enabled_config(filters: &[&str]) -> ReasoningAdapterConfig {
        ReasoningAdapterConfig {
            enabled: true,
            model_filters: filters.iter().map(|s| s.to_ascii_lowercase()).collect(),
            tag: "think".into(),
        }
    }

    // ---------- 各别名提取 ----------

    #[test]
    fn each_alias_is_extracted() {
        for alias in REASONING_ALIASES {
            let source = json!({ alias: "deep thought" });
            assert_eq!(extract_reasoning_text(&source), "deep thought", "alias {alias}");
        }
        assert_eq!(REASONING_ALIASES.len(), 12);
    }

    #[test]
    fn unknown_field_yields_empty() {
        // 失败路径: 未知字段不归一化 (0 装 PASS — 不虚构内容)
        let source = json!({ "content": "visible", "random_field": "not reasoning" });
        assert_eq!(extract_reasoning_text(&source), "");
    }

    #[test]
    fn non_object_source_yields_empty() {
        assert_eq!(extract_reasoning_text(&json!("plain string")), "");
        assert_eq!(extract_reasoning_text(&json!(42)), "");
        assert_eq!(extract_reasoning_text(&Value::Null), "");
    }

    #[test]
    fn multiple_aliases_joined_and_deduped() {
        let source = json!({
            "reasoning_content": "step 1",
            "thinking": "step 2",
            "thoughts": "step 1"  // 与 reasoning_content 完全重复 → 去重
        });
        assert_eq!(extract_reasoning_text(&source), "step 1\nstep 2");
    }

    // ---------- 嵌套结构 ----------

    #[test]
    fn nested_object_prefers_text_keys() {
        let source = json!({
            "reasoning_details": {"text": "nested text", "index": 3, "meta": {"x": 1}}
        });
        assert_eq!(extract_reasoning_text(&source), "nested text");
    }

    #[test]
    fn array_of_parts_joined() {
        let source = json!({
            "reasoning_chunk": [ {"text": "a"}, {"text": "b"}, null ]
        });
        assert_eq!(extract_reasoning_text(&source), "a\nb");
    }

    #[test]
    fn nested_alias_key_recognized() {
        let source = json!({ "reasoning": {"reasoning": "inner"} });
        assert_eq!(extract_reasoning_text(&source), "inner");
    }

    #[test]
    fn empty_and_false_values_skipped() {
        let source = json!({ "reasoning_content": "", "thinking": false });
        assert_eq!(extract_reasoning_text(&source), "");
    }

    // ---------- think 块包装 ----------

    #[test]
    fn wrap_adds_newline_before_closing_tag() {
        assert_eq!(wrap_reasoning_text("abc", "think"), "<think>\nabc\n</think>\n");
        assert_eq!(wrap_reasoning_text("abc\n", "think"), "<think>\nabc\n</think>\n");
        assert_eq!(wrap_reasoning_text("", "think"), "");
    }

    #[test]
    fn tag_normalized_to_think_unless_thinking() {
        assert_eq!(normalize_tag("think"), "think");
        assert_eq!(normalize_tag("THINKING"), "thinking");
        assert_eq!(normalize_tag("weird"), "think");
        assert_eq!(normalize_tag(""), "think");
        assert_eq!(wrap_reasoning_text("x", "THINKING"), "<thinking>\nx\n</thinking>\n");
    }

    // ---------- 出向剥离 ----------

    #[test]
    fn remove_reasoning_fields_strips_all_aliases_only() {
        let mut v = json!({
            "content": "keep me",
            "reasoning_content": "x",
            "thinking": "y",
            "thoughts": "z"
        });
        let removed = remove_reasoning_fields(&mut v);
        assert_eq!(removed, 3);
        assert_eq!(v, json!({ "content": "keep me" }));
    }

    #[test]
    fn remove_on_non_object_is_noop() {
        let mut v = json!([1, 2]);
        assert_eq!(remove_reasoning_fields(&mut v), 0);
    }

    // ---------- 模型能力过滤 ----------

    #[test]
    fn filter_substring_match_case_insensitive() {
        let cfg = enabled_config(&["kimi", "claude"]);
        assert!(cfg.should_convert_for_model("kimi-k2-0711"));
        assert!(cfg.should_convert_for_model("Claude-Sonnet-4"));
        assert!(!cfg.should_convert_for_model("gpt-5"));
    }

    #[test]
    fn empty_filters_or_disabled_converts_nothing() {
        let cfg = enabled_config(&[]);
        assert!(!cfg.should_convert_for_model("kimi"));
        let off = ReasoningAdapterConfig::default();
        assert!(!off.should_convert_for_model("kimi"));
        assert!(!cfg.should_convert_for_model(""));
    }

    // ---------- 客户端可见内容 ----------

    #[test]
    fn visible_content_prepends_think_block() {
        let cfg = enabled_config(&["deepseek"]);
        let msg = json!({ "content": "42 is the answer", "reasoning_content": "let me think" });
        let out = build_client_visible_content(&msg, &cfg, "deepseek-chat", &[]);
        assert_eq!(out, "<think>\nlet me think\n</think>\n42 is the answer");
    }

    #[test]
    fn visible_content_untouched_for_unmatched_model() {
        let cfg = enabled_config(&["deepseek"]);
        let msg = json!({ "content": "plain", "reasoning_content": "hidden" });
        assert_eq!(build_client_visible_content(&msg, &cfg, "gpt-x", &[]), "plain");
    }

    #[test]
    fn visible_content_no_reasoning_keeps_content() {
        let cfg = enabled_config(&["kimi"]);
        let msg = json!({ "content": "no reasoning here" });
        assert_eq!(build_client_visible_content(&msg, &cfg, "kimi-k2", &[]), "no reasoning here");
    }

    #[test]
    fn additional_sources_merged_and_deduped() {
        let cfg = enabled_config(&["kimi"]);
        let msg = json!({ "content": "ans", "reasoning_content": "r1" });
        let delta = json!({ "reasoning_content": "r1", "thinking": "r2" });
        let out = build_client_visible_content(&msg, &cfg, "kimi-k2", &[&delta]);
        assert_eq!(out, "<think>\nr1\nr2\n</think>\nans");
    }

    // ---------- 完整响应体归一化 ----------

    #[test]
    fn normalize_chat_completion_body_end_to_end() {
        let cfg = enabled_config(&["deepseek"]);
        let body = serde_json::to_string(&json!({
            "id": "cmpl-1",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "done", "reasoning_content": "thought process"}
            }]
        }))
        .unwrap();
        let out = normalize_chat_completion_body(&body, &cfg, "deepseek-chat");
        let v: Value = serde_json::from_str(&out).unwrap();
        let msg = &v["choices"][0]["message"];
        assert_eq!(msg["content"], "<think>\nthought process\n</think>\ndone");
        assert!(msg.get("reasoning_content").is_none()); // 别名已剥离
    }

    #[test]
    fn normalize_body_unmatched_model_only_strips_aliases() {
        let cfg = enabled_config(&["deepseek"]);
        let body = serde_json::to_string(&json!({
            "choices": [{"message": {"content": "hi", "thinking": "t"}}]
        }))
        .unwrap();
        let out = normalize_chat_completion_body(&body, &cfg, "gpt-x");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["choices"][0]["message"]["content"], "hi");
        assert!(v["choices"][0]["message"].get("thinking").is_none());
    }

    #[test]
    fn normalize_body_invalid_json_returns_original() {
        let cfg = enabled_config(&["kimi"]);
        assert_eq!(normalize_chat_completion_body("not json", &cfg, "kimi"), "not json");
    }

    #[test]
    fn normalize_body_without_choices_returns_original() {
        let cfg = enabled_config(&["kimi"]);
        let body = r#"{"object":"list"}"#;
        assert_eq!(normalize_chat_completion_body(body, &cfg, "kimi"), body);
    }
}
