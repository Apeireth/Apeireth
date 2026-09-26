//! # lark 消息面 (6 消息类型 + 发送 wire 契约)
//!
//! 平台 IM 发送端点: `POST /im/v1/messages?receive_id_type=...`。
//!
//! 支持 6 消息类型 (闭合枚举 [`MessageType`]):
//! `text` / `post` / `image` / `file` / `card` / `interactive`。
//!
//! ## wire 契约 (严格形状 + 未知字段容错)
//!
//! - 请求体: `{"receive_id": <必填>, "msg_type": <必填>, "content": <JSON 字符串 必填>,
//!   "uuid": <可选 幂等去重>}`; `receive_id_type` 走 query 参数。
//! - 响应 `data`: `{"message_id": <非空字符串 必填>, "create_time": <可选>,
//!   "chat_id": <可选>}` —— 未知字段忽略, 缺 `message_id` = 永久错误。
//!
//! ## 字段校验 (发送前置, 全部本地完成)
//!
//! - `receive_id` 按 `receive_id_type` 走 K-1 强校验 (chat/open/user/email/union);
//! - `content` 必须是合法 JSON 对象, 且与 `msg_type` 形状匹配;
//! - 文本消息正文非空且 ≤ [`crate::lark::MAX_MESSAGE_TEXT_BYTES`] 字节;
//! - `uuid` 若给出必须非空。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;
use crate::lark::http::ApiRequest;

// ============================================================================
// §1 6 消息类型 enum (闭合枚举)
// ============================================================================

/// 消息类型 (6 variant, snake_case 严格匹配 wire `msg_type`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 纯文本 (`msg_type: "text"`)。
    #[default]
    Text,
    /// 富文本 (`msg_type: "post"`, 支持 inline 元素)。
    Post,
    /// 图片 (`msg_type: "image"`, 需先上传拿 image_key)。
    Image,
    /// 文件 (`msg_type: "file"`, 需先上传拿 file_key)。
    File,
    /// 消息卡片 (`msg_type: "card"`)。
    Card,
    /// 交互卡片 (`msg_type: "interactive"`, 含 button / form / select)。
    Interactive,
}

impl MessageType {
    /// 6 类型 hardcode 常量。
    pub const COUNT: usize = 6;

    /// wire 字符串 (snake_case 严格匹配)。
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

    /// 从 wire 字符串解析 (未知值 → `None`, 调用方按永久错误处理)。
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

/// 编译期守门: SUPPORTED_MESSAGE_TYPES 长度 == 6。
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
// §2 各类型消息内容结构 (content JSON 的形状)
// ============================================================================

/// 文本消息内容 (`content: {"text": "...", "at_open_ids": [...]}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    /// 文本正文。
    pub text: String,
    /// @用户 open_id 列表 (可选)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at_open_ids: Vec<String>,
}

impl TextContent {
    /// 创建文本内容。
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            at_open_ids: Vec::new(),
        }
    }

    /// 校验 @user 列表 (K-1 #4 open_id 强校验)。
    pub fn validate(&self) -> Result<(), LarkError> {
        for oid in &self.at_open_ids {
            LarkError::validate_open_id(oid)?;
        }
        Ok(())
    }
}

/// 富文本段落。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostParagraph {
    /// 段落内 inline 元素。
    pub elements: Vec<PostElement>,
}

/// 富文本 inline 元素 (tag 判别)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum PostElement {
    /// 文本 (`tag: "text"`)。
    Text {
        /// 文本内容。
        text: String,
    },
    /// @用户 (`tag: "at"`)。
    At {
        /// 用户 open_id。
        user_id: String,
    },
    /// 链接 (`tag: "a"`)。
    Link {
        /// 链接文本。
        text: String,
        /// 链接 URL。
        href: String,
    },
    /// 图片 (`tag: "img"`)。
    Img {
        /// image_key。
        image_key: String,
    },
}

/// 富文本消息内容 (`content: {"post": {"zh_cn": {...}}}` 的展开形态)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostContent {
    /// 国际化 locale 字典 (e.g. `{"zh_cn": ..., "en_us": ...}`)。
    pub locale: HashMap<String, PostLocale>,
}

/// 富文本 locale 描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostLocale {
    /// 标题。
    pub title: String,
    /// 段落列表。
    pub content: Vec<PostParagraph>,
}

impl PostContent {
    /// 创建单 locale 富文本。
    pub fn new(
        locale_key: impl Into<String>,
        title: impl Into<String>,
        paragraphs: Vec<PostParagraph>,
    ) -> Self {
        let mut locale = HashMap::new();
        locale.insert(
            locale_key.into(),
            PostLocale {
                title: title.into(),
                content: paragraphs,
            },
        );
        Self { locale }
    }

    /// 创建中文富文本 (便捷构造)。
    pub fn new_zh_cn(title: impl Into<String>, paragraphs: Vec<PostParagraph>) -> Self {
        Self::new("zh_cn", title, paragraphs)
    }
}

/// 图片消息内容 (`content: {"image_key": "img_xxx"}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageContent {
    /// image_key (上传接口颁发)。
    pub image_key: String,
}

/// 文件消息内容 (`content: {"file_key": "file_xxx"}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContent {
    /// file_key (上传接口颁发)。
    pub file_key: String,
}

/// 卡片消息内容 (`content: {"config": {...}, "header": {...}, "elements": [...]}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardContent {
    /// 卡片配置 (wide_screen_mode / enable_forward 等)。
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// 卡片 header (title / template)。
    #[serde(default)]
    pub header: HashMap<String, serde_json::Value>,
    /// 卡片 elements (文本 / button / divider / image 等)。
    #[serde(default)]
    pub elements: Vec<HashMap<String, serde_json::Value>>,
}

impl CardContent {
    /// 创建简单文本卡片。
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

/// Interactive 卡片内容 (与 [`CardContent`] 同形状, 不同 `msg_type`)。
pub type InteractiveContent = CardContent;

// ============================================================================
// §3 Message 顶层结构
// ============================================================================

/// 消息接收者 ID 类型 (wire `receive_id_type`)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveIdType {
    /// 群 ID (`oc_` / `on_` 前缀)。
    #[default]
    ChatId,
    /// 用户 Open ID (`ou_` 前缀)。
    OpenId,
    /// 用户 User ID (租户内 user_id)。
    UserId,
    /// 邮箱。
    Email,
    /// Union ID (跨租户用户 ID)。
    UnionId,
}

impl ReceiveIdType {
    /// wire 字符串。
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

/// 消息顶层结构 (发送请求的领域形态)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// 接收者 ID。
    pub receive_id: String,
    /// 接收者 ID 类型 (决定 K-1 校验口径)。
    pub receive_id_type: ReceiveIdType,
    /// 消息类型 (6 variant)。
    pub msg_type: MessageType,
    /// 消息内容 (JSON 字符串, 形状按 `msg_type`)。
    pub content: String,
    /// 幂等 UUID (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

impl Message {
    /// 构造文本消息。
    pub fn text(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        text: String,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content = TextContent::new(text);
        content.validate()?;
        let content_json =
            serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Text,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造富文本消息。
    pub fn post(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        post: PostContent,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json =
            serde_json::to_string(&post).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Post,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造图片消息。
    pub fn image(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        image_key: String,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        if image_key.is_empty() {
            return Err(LarkError::Other("image_key is empty".to_string()));
        }
        let content = ImageContent { image_key };
        let content_json =
            serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Image,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造文件消息。
    pub fn file(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        file_key: String,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        if file_key.is_empty() {
            return Err(LarkError::Other("file_key is empty".to_string()));
        }
        let content = FileContent { file_key };
        let content_json =
            serde_json::to_string(&content).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::File,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造卡片消息。
    pub fn card(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        card: CardContent,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json =
            serde_json::to_string(&card).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Card,
            content: content_json,
            uuid: None,
        })
    }

    /// 构造 Interactive 消息 (新版卡片)。
    pub fn interactive(
        receive_id: String,
        receive_id_type: ReceiveIdType,
        card: InteractiveContent,
    ) -> Result<Self, LarkError> {
        Self::validate_receive_id(receive_id_type, &receive_id)?;
        let content_json =
            serde_json::to_string(&card).map_err(|e| LarkError::Other(e.to_string()))?;
        Ok(Self {
            receive_id,
            receive_id_type,
            msg_type: MessageType::Interactive,
            content: content_json,
            uuid: None,
        })
    }

    /// 设置幂等 UUID。
    pub fn with_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

    /// 发送前字段校验 (本地完成, 不打网络):
    /// receive_id 按类型 K-1 校验 + content JSON 形状与 msg_type 匹配 + 文本长度上限 + uuid 非空。
    pub fn validate_for_send(&self) -> Result<(), LarkError> {
        Self::validate_receive_id(self.receive_id_type, &self.receive_id)?;
        if let Some(uuid) = &self.uuid {
            if uuid.trim().is_empty() {
                return Err(LarkError::Other("uuid is empty".to_string()));
            }
            if uuid.len() > 128 {
                return Err(LarkError::Other(format!(
                    "uuid too long: {} > 128",
                    uuid.len()
                )));
            }
        }
        let content_value: serde_json::Value = serde_json::from_str(&self.content)
            .map_err(|_| LarkError::Other("content is not valid JSON".to_string()))?;
        if !content_value.is_object() {
            return Err(LarkError::Other(
                "content must be a JSON object".to_string(),
            ));
        }
        match self.msg_type {
            MessageType::Text => {
                let text: TextContent = serde_json::from_value(content_value)
                    .map_err(|_| LarkError::Other("text content shape mismatch".to_string()))?;
                text.validate()?;
                if text.text.trim().is_empty() {
                    return Err(LarkError::Other("text content is empty".to_string()));
                }
                if text.text.len() > crate::lark::MAX_MESSAGE_TEXT_BYTES {
                    return Err(LarkError::Other(format!(
                        "text content too large: {} > {} bytes",
                        text.text.len(),
                        crate::lark::MAX_MESSAGE_TEXT_BYTES
                    )));
                }
            }
            MessageType::Post => {
                let post: PostContent = serde_json::from_value(content_value)
                    .map_err(|_| LarkError::Other("post content shape mismatch".to_string()))?;
                if post.locale.is_empty() {
                    return Err(LarkError::Other("post locale is empty".to_string()));
                }
            }
            MessageType::Image => {
                let image: ImageContent = serde_json::from_value(content_value)
                    .map_err(|_| LarkError::Other("image content shape mismatch".to_string()))?;
                if image.image_key.is_empty() {
                    return Err(LarkError::Other("image_key is empty".to_string()));
                }
            }
            MessageType::File => {
                let file: FileContent = serde_json::from_value(content_value)
                    .map_err(|_| LarkError::Other("file content shape mismatch".to_string()))?;
                if file.file_key.is_empty() {
                    return Err(LarkError::Other("file_key is empty".to_string()));
                }
            }
            MessageType::Card | MessageType::Interactive => {
                let card: CardContent = serde_json::from_value(content_value)
                    .map_err(|_| LarkError::Other("card content shape mismatch".to_string()))?;
                if card.elements.is_empty() && card.header.is_empty() {
                    return Err(LarkError::Other(
                        "card content must carry header or elements".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// 校验 receive_id (按 receive_id_type 走 K-1 强校验)。
    fn validate_receive_id(
        receive_id_type: ReceiveIdType,
        receive_id: &str,
    ) -> Result<(), LarkError> {
        match receive_id_type {
            ReceiveIdType::ChatId => LarkError::validate_chat_id(receive_id),
            ReceiveIdType::OpenId => LarkError::validate_open_id(receive_id),
            ReceiveIdType::UserId => {
                if receive_id.trim().is_empty() {
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
// §4 发送 wire 契约 (请求构造 + 响应解析)
// ============================================================================

/// 发送响应 `data` 载荷 (未知字段容错; `message_id` 必填非空)。
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageData {
    /// 颁发的消息 ID。
    pub message_id: String,
    /// 创建时间 (可选, 平台格式)。
    #[serde(default)]
    pub create_time: Option<String>,
    /// 会话 ID (可选)。
    #[serde(default)]
    pub chat_id: Option<String>,
}

/// 构造发送请求 (`POST /im/v1/messages?receive_id_type=...`)。
pub fn build_send_request(message: &Message) -> Result<ApiRequest, LarkError> {
    message.validate_for_send()?;
    let mut body = serde_json::Map::new();
    body.insert(
        "receive_id".to_string(),
        serde_json::Value::String(message.receive_id.clone()),
    );
    body.insert(
        "msg_type".to_string(),
        serde_json::Value::String(message.msg_type.as_str().to_string()),
    );
    body.insert(
        "content".to_string(),
        serde_json::Value::String(message.content.clone()),
    );
    if let Some(uuid) = &message.uuid {
        body.insert("uuid".to_string(), serde_json::Value::String(uuid.clone()));
    }
    Ok(
        ApiRequest::post("/im/v1/messages", serde_json::Value::Object(body))
            .with_query("receive_id_type", message.receive_id_type.as_str()),
    )
}

/// 从发送响应载荷取 message_id (空串 = 协议体畸形, 永久错误)。
pub fn message_id_of(data: &SendMessageData) -> Result<String, LarkError> {
    if data.message_id.trim().is_empty() {
        return Err(LarkError::Other(
            "malformed response: empty message_id".to_string(),
        ));
    }
    Ok(data.message_id.clone())
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_type_6_kinds() {
        assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 6);
        assert_eq!(MessageType::COUNT, 6);
        for mt in SUPPORTED_MESSAGE_TYPES {
            let s = mt.as_str();
            let parsed = MessageType::parse(s).expect("parse must succeed");
            assert_eq!(parsed, *mt);
        }
        assert!(MessageType::parse("bogus").is_none());
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
        assert!(matches!(
            content.validate(),
            Err(LarkError::OpenIdInvalid(_))
        ));
    }

    #[test]
    fn message_text_construction() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello, lark!".to_string(),
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
        let msg = Message::post("oc_a1b2c3d4e5f6".to_string(), ReceiveIdType::ChatId, post)
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
        let msg = Message::card("oc_a1b2c3d4e5f6".to_string(), ReceiveIdType::ChatId, card)
            .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Card);
    }

    #[test]
    fn message_interactive_construction() {
        let card = InteractiveContent::plain("标题", "正文");
        let msg = Message::interactive("oc_a1b2c3d4e5f6".to_string(), ReceiveIdType::ChatId, card)
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

    // ---- validate_for_send (发送前字段校验) ----

    #[test]
    fn validate_for_send_accepts_valid_text() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        assert!(msg.validate_for_send().is_ok());
    }

    #[test]
    fn validate_for_send_rejects_malformed_content() {
        let mut msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        msg.content = "not json".to_string();
        assert!(matches!(msg.validate_for_send(), Err(LarkError::Other(_))));

        msg.content = "[1,2,3]".to_string();
        assert!(matches!(msg.validate_for_send(), Err(LarkError::Other(_))));
    }

    #[test]
    fn validate_for_send_rejects_type_shape_mismatch() {
        // msg_type = image 但 content 是文本形状 → 拒
        let mut msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        msg.msg_type = MessageType::Image;
        assert!(matches!(msg.validate_for_send(), Err(LarkError::Other(_))));
    }

    #[test]
    fn validate_for_send_enforces_text_size_cap() {
        let huge = "x".repeat(crate::lark::MAX_MESSAGE_TEXT_BYTES + 1);
        let mut msg = Message::text("oc_a1b2c3d4e5f6".to_string(), ReceiveIdType::ChatId, huge)
            .expect("construct ok");
        // text 超限在 validate_for_send 拒 (构造器只校验 receive_id/@列表)
        msg.content = serde_json::to_string(&TextContent::new(
            "x".repeat(crate::lark::MAX_MESSAGE_TEXT_BYTES + 1),
        ))
        .expect("json");
        assert!(matches!(msg.validate_for_send(), Err(LarkError::Other(_))));
    }

    #[test]
    fn validate_for_send_rejects_empty_uuid() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid")
        .with_uuid("   ");
        assert!(matches!(msg.validate_for_send(), Err(LarkError::Other(_))));
    }

    // ---- wire 契约 ----

    #[test]
    fn build_send_request_shapes_request() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid")
        .with_uuid("uuid-1");
        let req = build_send_request(&msg).expect("request");
        assert_eq!(req.path, "/im/v1/messages");
        assert_eq!(
            req.query,
            vec![("receive_id_type".to_string(), "chat_id".to_string())]
        );
        let body = req.body.expect("body");
        assert_eq!(body["receive_id"], "oc_a1b2c3d4e5f6");
        assert_eq!(body["msg_type"], "text");
        assert_eq!(body["uuid"], "uuid-1");
        // content 是 JSON 字符串 (不是内嵌对象)
        assert!(body["content"].is_string());
    }

    #[test]
    fn send_response_data_tolerates_unknown_fields_and_requires_message_id() {
        let data: SendMessageData = serde_json::from_value(serde_json::json!({
            "message_id": "om_abc123",
            "create_time": "1700000000",
            "future_field": {"x": 1}
        }))
        .expect("parse");
        assert_eq!(message_id_of(&data).expect("id"), "om_abc123");

        let data: SendMessageData = serde_json::from_value(serde_json::json!({
            "message_id": "   ",
        }))
        .expect("parse");
        assert!(matches!(message_id_of(&data), Err(LarkError::Other(_))));
    }
}
