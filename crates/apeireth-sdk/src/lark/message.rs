//! # Lark 消息 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书 IM `im/v1/messages` API 支持 6 消息类型 (per v0.9.21 商业版 `MessageType` enum):
//! 1. **text** — 纯文本
//! 2. **post** — 富文本 (per v0.9.21 商业版 `post` 消息, 支持 inline @user / link / image)
//! 3. **image** — 图片 (image_key, 走 upload_image 上传后获得)
//! 4. **file** — 文件 (file_key, 走 upload_file 上传后获得)
//! 5. **card** — 消息卡片 (per v0.9.21 商业版 `interactive` 老版)
//! 6. **interactive** — 消息卡片新版 (per v0.9.21 商业版 `card` JSON 模板, 含 button / form / select)
//!
//! **当前 STUB**: 所有 6 类型字段保留 1:1 翻译, 不真发飞书 API, 走 `send_message` 返 `NotImplemented`.
//!
//! ## 6 消息类型守门常量
//!
//! - `SUPPORTED_MESSAGE_TYPES` (6 entries, 编译期 hardcode)
//! - `MESSAGE_TYPE_COUNT == 6`

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::LarkError;

// ============================================================================
// §1 6 消息类型 enum (K-1 强校验守门, 编译期 hardcode 6 variant)
// ============================================================================

/// 消息类型 (6 variant, 1:1 翻译 @larksuiteoapi/lark-sdk v0.9.21 商业版 `MessageType` enum).
///
/// 6 类型 snake_case 字符串严格匹配飞书 Open API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// **纯文本** (per v0.9.21 商业版 `msg_type: "text"`).
    #[default]
    Text,
    /// **富文本** (per v0.9.21 商业版 `msg_type: "post"`, 支持 inline 元素).
    Post,
    /// **图片** (per v0.9.21 商业版 `msg_type: "image"`, 需先 `im/v1/images` 上传).
    Image,
    /// **文件** (per v0.9.21 商业版 `msg_type: "file"`, 需先 `im/v1/files` 上传).
    File,
    /// **消息卡片 (老版)** (per v0.9.21 商业版 `msg_type: "card"`, 1.0 之前的 card JSON 模板).
    Card,
    /// **消息卡片 (新版)** (per v0.9.21 商业版 `msg_type: "interactive"`, 含 button / form / select).
    Interactive,
}

impl MessageType {
    /// 6 类型 hardcode 常量.
    pub const COUNT: usize = 6;

    /// 字符串 (1:1 翻译 v0.9.21 商业版 `msg_type` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "text",
            MessageType::Post => "post",
            MessageType::Image => "image",
            MessageType::File => "file",
            MessageType::Card => "card",
            MessageType::Interactive => "interactive",
        }
    }

    /// 从字符串解析 (per 飞书 Open API 响应 `msg_type` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(MessageType::Text),
            "post" => Some(MessageType::Post),
            "image" => Some(MessageType::Image),
            "file" => Some(MessageType::File),
            "card" => Some(MessageType::Card),
            "interactive" => Some(MessageType::Interactive),
            _ => None,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_MESSAGE_TYPES 长度 == 6 (K-1 强校验守门, 6 消息类型 hardcode).
pub const SUPPORTED_MESSAGE_TYPES: &[MessageType] = &[
    MessageType::Text,
    MessageType::Post,
    MessageType::Image,
    MessageType::File,
    MessageType::Card,
    MessageType::Interactive,
];
const _: () = assert!(SUPPORTED_MESSAGE_TYPES.len() == 6);

// ============================================================================
// §2 各类型消息内容结构 (per v0.9.21 商业版 `content` JSON 字段 1:1 翻译)
// ============================================================================

/// 文本消息内容 (per v0.9.21 商业版 `content: { "text": "..." }` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    /// 文本 (e.g. "Hello, 飞书!")
    pub text: String,
    /// @用户列表 (可选, e.g. `["ou_user1", "ou_user2"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at_open_ids: Vec<String>,
}

impl TextContent {
    /// 创建文本内容.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            at_open_ids: Vec::new(),
        }
    }

    /// 校验 @user 列表 (per K-1 #4 open_id 强校验).
    pub fn validate(&self) -> Result<(), LarkError> {
        for oid in &self.at_open_ids {
            LarkError::validate_open_id(oid)?;
        }
        Ok(())
    }
}

/// 富文本段落 (per v0.9.21 商业版 `post` 消息 paragraph 元素).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostParagraph {
    /// 段落内 inline 元素.
    pub elements: Vec<PostElement>,
}

/// 富文本 inline 元素 (per v0.9.21 商业版 `post` 消息 element 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum PostElement {
    /// 文本 (per `tag: "text"`).
    Text {
        /// 文本内容.
        text: String,
    },
    /// @用户 (per `tag: "at"`).
    At {
        /// 用户 open_id.
        user_id: String,
    },
    /// 链接 (per `tag: "a"`).
    Link {
        /// 链接文本.
        text: String,
        /// 链接 URL.
        href: String,
    },
    /// 图片 (per `tag: "img"`).
    Img {
        /// image_key.
        image_key: String,
    },
}

/// 富文本消息内容 (per v0.9.21 商业版 `content: { "post": { "zh_cn": { "title": "...", "content": [...] } } }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostContent {
    /// 国际化 locale 字典 (e.g. `{"zh_cn": ..., "en_us": ...}`).
    pub locale: HashMap<String, PostLocale>,
}

/// 富文本 locale 描述 (per v0.9.21 商业版 `post.zh_cn` / `post.en_us` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostLocale {
    /// 标题.
    pub title: String,
    /// 段落列表.
    pub content: Vec<PostParagraph>,
}

impl PostContent {
    /// 创建中文富文本.
    pub fn new_zh_cn(title: impl Into<String>, paragraphs: Vec<PostParagraph>) -> Self {
        let mut locale = HashMap::new();
        locale.insert(
            "zh_cn".to_string(),
            PostLocale {
                title: title.into(),
                content: paragraphs,
            },
        );
        Self { locale }
    }
}

/// 图片消息内容 (per v0.9.21 商业版 `content: { "image_key": "img_xxx" }` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageContent {
    /// image_key (per `im/v1/images` upload 响应, R21 续真接)
    pub image_key: String,
}

/// 文件消息内容 (per v0.9.21 商业版 `content: { "file_key": "file_xxx" }` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContent {
    /// file_key (per `im/v1/files` upload 响应, R21 续真接)
    pub file_key: String,
}

/// 卡片消息内容 (per v0.9.21 商业版 `content: { "config": {...}, "header": {...}, "elements": [...] }` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardContent {
    /// 卡片配置 (wide_screen_mode / enable_forward 等).
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// 卡片 header (title / template).
    #[serde(default)]
    pub header: HashMap<String, serde_json::Value>,
    /// 卡片 elements (文本 / button / divider / image 等).
    #[serde(default)]
    pub elements: Vec<HashMap<String, serde_json::Value>>,
}

impl CardContent {
    /// 创建简单文本卡片.
    pub fn plain(title: impl Into<String>, body: impl Into<String>) -> Self {
        let mut header = HashMap::new();
        header.insert(
            "title".to_string(),
            serde_json::json!({"tag": "plain_text", "content": title.into()}),
        );
        let mut body_elem = HashMap::new();
        body_elem.insert(
            "tag".to_string(),
            serde_json::Value::String("div".to_string()),
        );
        body_elem.insert(
            "text".to_string(),
            serde_json::json!({"tag": "plain_text", "content": body.into()}),
        );
        Self {
            config: HashMap::new(),
            header,
            elements: vec![body_elem],
        }
    }
}

/// Interactive 卡片内容 (跟 Card 相同, 1:1 翻译 `interactive` 类型, 走新版 card 模板).
pub type InteractiveContent = CardContent;

// ============================================================================
// §3 Message 顶层结构 (per v0.9.21 商业版 1:1)
// ============================================================================

/// 消息接收者 ID 类型 (per v0.9.21 商业版 `receive_id_type` 字段).
///
/// 飞书 `im/v1/messages` API 要求 `receive_id_type` 指定 `chat_id` / `open_id` / `user_id` / `email` / `union_id`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveIdType {
    /// 群 ID (oc_xxx / on_xxx).
    #[default]
    ChatId,
    /// 用户 Open ID (ou_xxx).
    OpenId,
    /// 用户 User ID (per 飞书 user_id 字段, R21 续真接时支持).
    UserId,
    /// 邮箱.
    Email,
    /// Union ID (跨租户用户 ID).
    UnionId,
}

impl ReceiveIdType {
    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReceiveIdType::ChatId => "chat_id",
            ReceiveIdType::OpenId => "open_id",
            ReceiveIdType::UserId => "user_id",
            ReceiveIdType::Email => "email",
            ReceiveIdType::UnionId => "union_id",
        }
    }
}

impl std::fmt::Display for ReceiveIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 消息顶层结构 (per v0.9.21 商业版 `im/v1/messages` POST 请求 body 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// 接收者 ID (per `receive_id` 字段).
    pub receive_id: String,
    /// 接收者 ID 类型 (per `receive_id_type` 字段, K-1 强校验决定格式).
    pub receive_id_type: ReceiveIdType,
    /// 消息类型 (per `msg_type` 字段, 6 variant hardcode).
    pub msg_type: MessageType,
    /// 消息内容 (per `content` 字段, JSON 字符串, 1:1 翻译 v0.9.21 商业版).
    pub content: String,
    /// UUID (per `uuid` 字段, 幂等去重用, 可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

impl Message {
    /// 构造文本消息.
    pub fn text(receive_id: String, receive_id_type: ReceiveIdType, text: String) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content = TextContent::new(text);
        content.validate()?;
        let content_json = serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Text,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造富文本消息.
    pub fn post(receive_id: String, receive_id_type: ReceiveIdType, post: PostContent) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json = serde_json::to_string(&post).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Post,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造图片消息.
    pub fn image(receive_id: String, receive_id_type: ReceiveIdType, image_key: String) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        if image_key.is_empty() {
            return Err(LarkError::Other("image_key is empty".to_string()));
        }
        let content = ImageContent { image_key };
        let content_json = serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Image,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造文件消息.
    pub fn file(receive_id: String, receive_id_type: ReceiveIdType, file_key: String) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        if file_key.is_empty() {
            return Err(LarkError::Other("file_key is empty".to_string()));
        }
        let content = FileContent { file_key };
        let content_json = serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::File,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造卡片消息 (老版).
    pub fn card(receive_id: String, receive_id_type: ReceiveIdType, card: CardContent) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json = serde_json::to_string(&card).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Card,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造 Interactive 消息 (新版).
    pub fn interactive(receive_id: String, receive_id_type: ReceiveIdType, card: InteractiveContent) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json = serde_json::to_string(&card).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Interactive,
            content: content_json,
            uuid: None,
        })
    }

    /// 设置幂等 UUID (per `uuid` 字段).
    pub fn with_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

    /// 校验 receive_id (per receive_id_type 走 K-1 强校验).
    fn validate_receive_id(receive_id_type: ReceiveIdType, receive_id: &str) -> Result<(), LarkError> {
        match receive_id_type {
            ReceiveIdType::ChatId => LarkError::validate_chat_id(receive_id),
            ReceiveIdType::OpenId => LarkError::validate_open_id(receive_id),
            ReceiveIdType::UserId => {
                if receive_id.is_empty() {
                    Err(LarkError::Other("user_id is empty".to_string()))
                } else {
                    Ok(())
                }
            }
            ReceiveIdType::Email => LarkError::validate_email(receive_id),
            ReceiveIdType::UnionId => {
                if receive_id.is_empty() || !receive_id.starts_with("on_") {
                    Err(LarkError::Other(format!(
                        "union_id invalid: {receive_id} (expected prefix 'on_')"
                    )))
                } else {
                    Ok(())
                }
            }
        }
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_type_6_kinds() {
        assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 6);
        assert_eq!(MessageType::COUNT, 6);
        for mt in SUPPORTED_MESSAGE_TYPES {
            // 每个 type 应能 round-trip parse
            let s = mt.as_str();
            let parsed = MessageType::parse(s).expect("parse must succeed");
            assert_eq!(parsed, *mt);
        }
    }

    #[test]
    fn text_content_validate_at_open_ids() {
        let content = TextContent {
            text: "Hello".to_string(),
            at_open_ids: vec!["ou_user1234567890abcdef".to_string()],
        };
        assert!(content.validate().is_ok());
    }

    #[test]
    fn text_content_reject_invalid_open_id() {
        let content = TextContent {
            text: "Hello".to_string(),
            at_open_ids: vec!["invalid".to_string()],
        };
        assert!(matches!(content.validate(), Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn message_text_construction() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello, 飞书!".to_string(),
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Text);
        assert!(msg.content.contains("Hello"));
    }

    #[test]
    fn message_text_reject_invalid_chat_id() {
        let result = Message::text(
            "invalid".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        );
        assert!(matches!(result, Err(LarkError::ChatIdInvalid(_))));
    }

    #[test]
    fn message_post_construction() {
        let para = PostParagraph {
            elements: vec![PostElement::Text {
                text: "标题".to_string(),
            }],
        };
        let post = PostContent::new_zh_cn("通知", vec![para]);
        let msg = Message::post(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            post,
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Post);
    }

    #[test]
    fn message_image_construction() {
        let msg = Message::image(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "img_v2_abc123".to_string(),
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Image);
    }

    #[test]
    fn message_image_reject_empty_image_key() {
        let result = Message::image(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            String::new(),
        );
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn message_file_construction() {
        let msg = Message::file(
            "ou_user1234567890abcdef".to_string(),
            ReceiveIdType::OpenId,
            "file_v2_abc".to_string(),
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::File);
    }

    #[test]
    fn message_card_construction() {
        let card = CardContent::plain("标题", "正文");
        let msg = Message::card(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            card,
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Card);
    }

    #[test]
    fn message_interactive_construction() {
        let card = InteractiveContent::plain("标题", "正文");
        let msg = Message::interactive(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            card,
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Interactive);
    }

    #[test]
    fn message_email_receive_id() {
        let result = Message::text(
            "user@example.com".to_string(),
            ReceiveIdType::Email,
            "Hello".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn message_with_uuid() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid")
        .with_uuid("uuid-12345");
        assert_eq!(msg.uuid.as_deref(), Some("uuid-12345"));
    }
}
