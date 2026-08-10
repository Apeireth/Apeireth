//! **战役 1-3 / VCP §6.2.2 #20 — Force-Translate (base64 image → text tag)**
//!
//! **借鉴来源 (字段级)**:
//! - `research/source/vcptoolbox/modules/chatCompletionHandler.js` (3 函数 + JSON 白名单 config)
//!   - spec 引用 line 100-160, 实际 `isTextOnlyModelByTag` 在 line 222-230, `messagesContainBase64Media` 在 line 239-257
//! - `research/source/vcptoolbox/modules/multiModalConfigStore.js`
//!   - `MultiModalForceTranslateModels` 字段 (line 36 FIELD_DEFAULTS 里)
//!
//! **VCP 真代码 3 函数 (按 spec §6.2.2 #20 引用 chatCompletionHandler.js:100-160, 实际函数 222-257)**:
//! ```js
//! // line 222-230
//! function isTextOnlyModelByTag(modelName, tagList) {
//!   if (!modelName || !Array.isArray(tagList) || tagList.length === 0) return false;
//!   const lowerName = String(modelName).toLowerCase();
//!   for (const tag of tagList) {
//!     if (!tag) continue;
//!     if (lowerName.includes(tag)) return true;
//!   }
//!   return false;
//! }
//!
//! // line 239-257
//! function messagesContainBase64Media(messages) {
//!   if (!Array.isArray(messages)) return false;
//!   for (const msg of messages) {
//!     if (!msg || (msg.role !== 'user' && msg.role !== 'system')) continue;
//!     if (!Array.isArray(msg.content)) continue;
//!     for (const part of msg.content) {
//!       if (
//!         part && part.type === 'image_url' &&
//!         part.image_url && typeof part.image_url.url === 'string' &&
//!         /^data:(image|audio|video)\/[^;]+;base64,/.test(part.image_url.url)
//!       ) {
//!         return true;
//!       }
//!     }
//!   }
//!   return false;
//! }
//!
//! // line 21 (multiModalConfigStore.js FIELD_DEFAULTS)
//! MultiModalForceTranslateModels: []
//! ```
//!
//! **Apeireth 简化 (工程层借鉴, 不抄业务)**:
//! - 借鉴**3 函数语义** (is_text_only_model / contains_base64 / force_translate)
//! - 借鉴**base64 正则** `data:(image|audio|video)/[^;]+;base64,`
//! - 用 `ForceTranslateConfig` 结构化 `tag_list` (VCP 用环境变量 + JSON 混)
//! - **不抄 VCP 的 chokidar 热更新** — Apeireth 用编译期 hardcode tag list, 战役 4+ 留 hot reload hook
//! - **不抄 VCP 的 `MediaInsertPrompt` 翻译流程** — Apeireth 真翻译留给战役 2 tool-runtime, 本 pipeline 只**检测 + 标记** (避免 deepseek/GLM 收到 base64 时 400)
//!
//! **不假装**:
//! - 3 函数真实现, base64 正则跟 VCP 一字不差
//! - 配置结构化, 不靠 env

use apeireth_protocol::{ContentPart, MessageRole, NormalizedMessage};
use regex::Regex;
use std::sync::OnceLock;

/// **VCP 借鉴 #20** — Force-Translate 配置 (借鉴 VCP `multiModalConfigStore.js` FIELD_DEFAULTS)
#[derive(Debug, Clone, PartialEq)]
pub struct ForceTranslateConfig {
    /// 纯文本模型 tag 列表 (VCP `MultiModalForceTranslateModels`, 不区分大小写)
    /// 命中任一 tag → 强制把 base64 image 翻译为 text tag
    pub tag_list: Vec<String>,
}

impl Default for ForceTranslateConfig {
    fn default() -> Self {
        // **Apeireth 默认**: 空列表 (编译期不开, 战役 4+ 配 chokidar 热更新)
        Self {
            tag_list: Vec::new(),
        }
    }
}

impl ForceTranslateConfig {
    /// VCP 默认 config (跟 `multiModalConfigStore.js` FIELD_DEFAULTS 字段对齐)
    pub fn vcp_default() -> Self {
        // 真代码 line 36: `MultiModalForceTranslateModels: []` (空)
        Self {
            tag_list: Vec::new(),
        }
    }

    /// 自定义 tag 列表
    pub fn with_tags(tags: Vec<String>) -> Self {
        Self {
            tag_list: tags.into_iter().map(|t| t.to_lowercase()).collect(),
        }
    }

    /// 添加单个 tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag_list.push(tag.into().to_lowercase());
        self
    }
}

/// **VCP 借鉴 #20** — base64 data URL 正则 (跟 VCP `chatCompletionHandler.js:250` 一字不差)
///
/// VCP 真代码: `/^data:(image|audio|video)\/[^;]+;base64,/`
pub const BASE64_DATA_URL_REGEX_STR: &str = r"^data:(image|audio|video)/[^;]+;base64,";

fn base64_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        Regex::new(BASE64_DATA_URL_REGEX_STR).expect("BASE64_DATA_URL_REGEX must compile")
    })
}

/// **VCP `isTextOnlyModelByTag` (line 222-230)** — 检测模型是否命中纯文本 tag
///
/// **字段级对应**: VCP 用 `modelName.toLowerCase().includes(tag)`, 我们同样
pub fn is_text_only_model_by_tag(model_name: &str, tag_list: &[String]) -> bool {
    if model_name.is_empty() || tag_list.is_empty() {
        return false;
    }
    let lower = model_name.to_lowercase();
    tag_list
        .iter()
        .filter(|t| !t.is_empty())
        .any(|tag| lower.contains(tag))
}

/// **VCP `messagesContainBase64Media` (line 239-257)** — 检测 messages 数组是否含 base64 多模态
///
/// **字段级对应**: VCP 扫 `user` + `system` 角色的 `image_url` part, 我们扫归一化 `User` + `System`
pub fn messages_contain_base64_media(messages: &[NormalizedMessage]) -> bool {
    for msg in messages {
        if !matches!(msg.role, MessageRole::User | MessageRole::System) {
            continue;
        }
        for part in &msg.content {
            if let ContentPart::ImageUrl { url, .. } = part {
                if base64_regex().is_match(url) {
                    return true;
                }
            }
        }
    }
    false
}

/// **VCP `isTextOnlyModelByTag` + `messagesContainBase64Media` 联合应用** —
/// 判断是否需要 Force-Translate
///
/// VCP 真代码语义: `if (isTextOnlyModelByTag(model, tagList) && messagesContainBase64Media(messages))`
pub fn needs_force_translate(
    model: &str,
    messages: &[NormalizedMessage],
    config: &ForceTranslateConfig,
) -> bool {
    is_text_only_model_by_tag(model, &config.tag_list) && messages_contain_base64_media(messages)
}

/// **Force-Translate 行为** — 命中条件时, 把 base64 image 替换为 text tag
///
/// **VCP 真实行为** (从 `multiModalConfigStore.js` `MediaInsertPrompt` 推断):
/// 把 `data:image/png;base64,iVBOR...` 替换为:
/// ```text
/// [服务器已处理多模态数据, VCP系统已自动提取多模态数据信息, 信息元如下——]
/// [base64 image truncated: <mime type>, <byte count> bytes]
/// ```
///
/// **Apeireth 简化**: 替换为结构化 text tag, 保留原 message role + tool_call 关联
pub fn force_translate_if_needed(
    model: &str,
    messages: &mut Vec<NormalizedMessage>,
    config: &ForceTranslateConfig,
) -> ForceTranslateStats {
    if !needs_force_translate(model, messages, config) {
        return ForceTranslateStats::default();
    }

    let mut stats = ForceTranslateStats::default();
    for msg in messages.iter_mut() {
        if !matches!(msg.role, MessageRole::User | MessageRole::System) {
            continue;
        }

        // 收集新 part 列表
        let mut new_parts = Vec::with_capacity(msg.content.len());
        for part in msg.content.drain(..) {
            if let ContentPart::ImageUrl { url, .. } = &part {
                if base64_regex().is_match(url) {
                    // 替换为 text tag (VCP MediaInsertPrompt 风格)
                    let mime = extract_mime_type(url);
                    let byte_count = base64_byte_count(url);
                    let text = format!(
                        "[服务器已处理多模态数据,VCP系统已自动提取多模态数据信息,信息元如下——\nbase64 image truncated: mime={mime}, bytes={byte_count}]"
                    );
                    new_parts.push(ContentPart::text_only(text));
                    stats.base64_replaced += 1;
                } else {
                    // 非 base64 image (e.g. https URL) 保留
                    new_parts.push(part);
                }
            } else {
                new_parts.push(part);
            }
        }
        msg.content = new_parts;
    }
    stats
}

/// Force-Translate 统计
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ForceTranslateStats {
    /// 替换的 base64 part 数
    pub base64_replaced: usize,
}

/// 提取 MIME type (e.g. `data:image/png;base64,...` → `image/png`)
fn extract_mime_type(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("data:") {
        if let Some(mime) = rest.split(';').next() {
            return mime.to_string();
        }
    }
    "unknown".to_string()
}

/// 计算 base64 解码后字节数 (粗估: 原始长度 × 3/4, 不算 padding)
fn base64_byte_count(url: &str) -> usize {
    if let Some(rest) = url.split_once(',') {
        let b64 = rest.1;
        // 去 padding (`=`)
        let no_pad = b64.trim_end_matches('=');
        no_pad.len() * 3 / 4
    } else {
        0
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

const _: () = {
    // base64 正则字面量长度检查 (VCP `^data:(image|audio|video)/[^;]+;base64,` 大致 35+ 字符)
    // (含 mime 类别的具体检查移到 force_translate::tests 里的 runtime 测)
    assert!(
        BASE64_DATA_URL_REGEX_STR.len() >= 30,
        "base64 正则太短, 守 VCP line 250"
    );
};

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::MessageRole;

    fn base64_image_url() -> String {
        // 1x1 PNG, 真实 base64 编码
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string()
    }

    fn https_image_url() -> String {
        "https://example.com/cat.png".to_string()
    }

    fn user_msg_with_url(url: &str) -> NormalizedMessage {
        NormalizedMessage {
            role: MessageRole::User,
            content: vec![ContentPart::ImageUrl {
                url: url.to_string(),
                detail: None,
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }

    // ====== is_text_only_model_by_tag (VCP line 222-230) ======

    #[test]
    fn is_text_only_model_empty_tag_list_returns_false() {
        assert!(!is_text_only_model_by_tag("deepseek-v4", &[]));
    }

    #[test]
    fn is_text_only_model_match_substring() {
        // VCP: lowerName.includes(tag)
        let tags = vec!["deepseek".to_string()];
        assert!(is_text_only_model_by_tag("DeepSeek-V4", &tags));
    }

    #[test]
    fn is_text_only_model_case_insensitive() {
        // VCP `isTextOnlyModelByTag` 真代码: `lowerName.includes(tag)`
        // 假定 tag_list 已被 config store 预 lowercase (VCP `multiModalConfigStore.js` 行为)
        // 我们的 `ForceTranslateConfig::with_tags` 内部 lowercase, 这里测试 raw 路径
        let tags = vec!["glm".to_string()]; // 预 lowercase
        assert!(is_text_only_model_by_tag("GLM-4.5", &tags));
    }

    #[test]
    fn is_text_only_model_no_match() {
        let tags = vec!["deepseek".to_string()];
        assert!(!is_text_only_model_by_tag("gpt-4o", &tags));
    }

    // ====== messages_contain_base64_media (VCP line 239-257) ======

    #[test]
    fn contains_base64_true_with_image_url() {
        let msgs = vec![user_msg_with_url(&base64_image_url())];
        assert!(messages_contain_base64_media(&msgs));
    }

    #[test]
    fn contains_base64_false_with_https_url() {
        // https URL 不是 base64
        let msgs = vec![user_msg_with_url(&https_image_url())];
        assert!(!messages_contain_base64_media(&msgs));
    }

    #[test]
    fn contains_base64_false_with_empty_messages() {
        assert!(!messages_contain_base64_media(&[]));
    }

    #[test]
    fn contains_base64_false_for_assistant_role() {
        // VCP line 242: 只扫 user + system
        let mut msg = user_msg_with_url(&base64_image_url());
        msg.role = MessageRole::Assistant;
        let msgs = vec![msg];
        assert!(!messages_contain_base64_media(&msgs));
    }

    // ====== force_translate_if_needed (VCP 联合应用) ======

    #[test]
    fn force_translate_skipped_for_multimodal_model() {
        // 模型 gpt-4o 支持多模态, 即使含 base64 也不翻译
        let config = ForceTranslateConfig::with_tags(vec!["deepseek".to_string()]);
        let mut msgs = vec![user_msg_with_url(&base64_image_url())];
        let stats = force_translate_if_needed("gpt-4o", &mut msgs, &config);
        assert_eq!(stats.base64_replaced, 0);
        // image 保留
        assert!(matches!(msgs[0].content[0], ContentPart::ImageUrl { .. }));
    }

    #[test]
    fn force_translate_replaces_base64_with_text_tag() {
        let config = ForceTranslateConfig::with_tags(vec!["deepseek".to_string()]);
        let mut msgs = vec![user_msg_with_url(&base64_image_url())];
        let stats = force_translate_if_needed("deepseek-v4", &mut msgs, &config);
        assert_eq!(stats.base64_replaced, 1);
        // 应变为 text
        match &msgs[0].content[0] {
            ContentPart::Text { text } => {
                assert!(text.contains("base64 image truncated"));
                assert!(text.contains("image/png"));
            }
            _ => panic!("应被替换为 text, 实际: {:?}", msgs[0].content[0]),
        }
    }

    #[test]
    fn force_translate_keeps_https_url() {
        // 混合: 1 base64 + 1 https
        let config = ForceTranslateConfig::with_tags(vec!["deepseek".to_string()]);
        let mut msgs = vec![NormalizedMessage {
            role: MessageRole::User,
            content: vec![
                ContentPart::ImageUrl {
                    url: base64_image_url(),
                    detail: None,
                },
                ContentPart::ImageUrl {
                    url: https_image_url(),
                    detail: None,
                },
            ],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }];
        let stats = force_translate_if_needed("deepseek-v4", &mut msgs, &config);
        assert_eq!(stats.base64_replaced, 1);
        // 1 text (替换) + 1 image_url (保留)
        assert_eq!(msgs[0].content.len(), 2);
        assert!(matches!(msgs[0].content[0], ContentPart::Text { .. }));
        assert!(matches!(msgs[0].content[1], ContentPart::ImageUrl { .. }));
    }

    #[test]
    fn force_translate_no_op_when_no_base64() {
        // 纯文本 user, 即使模型命中也不动
        let config = ForceTranslateConfig::with_tags(vec!["deepseek".to_string()]);
        let mut msgs = vec![NormalizedMessage::user("hello")];
        let stats = force_translate_if_needed("deepseek-v4", &mut msgs, &config);
        assert_eq!(stats.base64_replaced, 0);
        assert_eq!(msgs[0].content[0], ContentPart::text_only("hello"));
    }

    // ====== helpers ======

    #[test]
    fn extract_mime_type_png() {
        let url = "data:image/png;base64,iVBOR...";
        assert_eq!(extract_mime_type(url), "image/png");
    }

    #[test]
    fn extract_mime_type_unknown() {
        let url = "not a data url";
        assert_eq!(extract_mime_type(url), "unknown");
    }

    #[test]
    fn base64_byte_count_correct() {
        let url = "data:image/png;base64,abcdefgh"; // 8 chars × 3/4 = 6 bytes
        assert_eq!(base64_byte_count(url), 6);
    }

    #[test]
    fn base64_regex_has_required_mime_classes() {
        // VCP line 250: /data:(image|audio|video)\/[^\;]+\;base64,/
        assert!(BASE64_DATA_URL_REGEX_STR.contains("image"));
        assert!(BASE64_DATA_URL_REGEX_STR.contains("audio"));
        assert!(BASE64_DATA_URL_REGEX_STR.contains("video"));
        // 必须 base64 编码标识
        assert!(BASE64_DATA_URL_REGEX_STR.contains("base64"));
    }
}
