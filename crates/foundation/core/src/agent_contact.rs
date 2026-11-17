//! 联系人 / Agent 身份 + 人设（后端合同 B1 批的领域层）。
//!
//! # 本模块负责什么
//!
//! - 一个联系人拥有**后端铸造**的 [`ContactId`] 与一个可绑定的 agent 身份
//!   [`ContactAgentId`]；展示名只是标签，**不唯一**（同名不同 id 是两条记录）。
//! - 人设文本 [`PersonaText`] **构造即校验**：字节上限、禁止控制字符、拒绝空文本。
//! - 每次写入都带 `expected_revision` 乐观并发；过期 ⇒ `RevisionConflict` 且状态**零变化**。
//! - 归档是**软删除**（墓碑）：`status ⇒ Archived`、`revision + 1`、**不可复活**。
//!
//! # 本模块刻意不做什么（诚实边界）
//!
//! - **不映射** [`ContractMemoryScope`] 到 `engine/memory` 的 `MemoryScope`
//!   —— 那是 B2 批的事；本批只放受管枚举的占位类型。
//! - **不做会话绑定**（B2）、**不提供 HTTP 面**（B3）、**不做审计**（B2）。
//! - [`ContactCapabilities`] **只是展示投影**：它**不是**授权依据，
//!   **不参与**任何 `Permission` / `tool_scope` 判定（设计稿 §未决第 5 条）。
//!
//! # 换行裁决（本批口径）
//!
//! 人设文本**允许**普通换行 `\n`（`U+000A`）：人设是散文型长文本，
//! 注入槽是文本块，硬拒换行会让正常的人设写不出来。
//! 其余**全部**控制字符（`U+0000..U+001F` 除 `\n`、`U+007F`）与
//! `U+2028` / `U+2029` 一律拒绝：它们能编码终端转义、能把一段文本
//! 拆成多段（日志/渲染/审计行数都被操纵），也正是不变量「不泄露」的破口。

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::kernel::ids::{ContactAgentId, ContactId};
use crate::storage_atomic;
use crate::stored_doc::{self, DocCompat};

/// 人设文本的**字节**上限（不是字符数上限）。
///
/// ⚠️ **字节不是字符**：`MAX_PERSONA_BYTES` 计的是 UTF-8 编码后的字节数，
/// 所以多字节语言的实际可用字符数**更少**（例如一行 4096 字节的日文
/// 大约只有 1365 个字符）。这是有意的：上限要卡住的是**载荷体积**
/// （进 system 文本、进日志、进数据库），不是「多少个汉字」。
pub const MAX_PERSONA_BYTES: usize = 4096;

/// 展示名的**字节**上限。
pub const MAX_DISPLAY_NAME_BYTES: usize = 128;

/// 副标题的**字节**上限。
pub const MAX_SUBTITLE_BYTES: usize = 256;

/// 头像引用的**字节**上限（`avatar_ref` 是不透明引用串，不是文件路径）。
pub const MAX_AVATAR_REF_BYTES: usize = 512;

/// 单次列表返回的条数上限（列表有界）。
pub const MAX_LISTED_CONTACTS: usize = 512;

/// 联系人文档名（落盘信封的 identity）。
const CONTACT_DOC_NAME: &str = "agent-contact";

/// 联系人文档版本（落盘信封的 version）。
const CONTACT_DOC_VERSION: u32 = 1;

/// 单个联系人目录下的文档文件名。
const CONTACT_FILE_NAME: &str = "contact.json";

/// 联系人集合在磁盘上的相对目录（`<root>/agents/contacts/`）。
const CONTACTS_DIR: [&str; 2] = ["agents", "contacts"];

// ---------------------------------------------------------------------------
// 受管枚举
// ---------------------------------------------------------------------------

/// 联系人的**合同态**。
///
/// ⚠️ 与运行时 `AgentStatus`（`Ready|Running|Paused|…`）是**两套枚举**，
/// 语义不同（合同态 vs 运行态），投影时显式映射、**不合并**（设计稿 §裁决 4）。
/// 合并会让「归档」变成一种运行态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactStatus {
    /// 已就绪。
    Ready,
    /// 草稿（新建联系人的初态；客户端**不能**直接置为就绪）。
    Draft,
    /// 已归档（软删除墓碑，不可复活）。
    Archived,
    /// 暂不可用。
    Unavailable,
}

impl ContactStatus {
    /// 由状态**派生**的展示投影初值（能力字段派生而非客户端接收）。
    ///
    /// 这只是展示层的初值，**不是**授权依据；真正落 `Permission` 的映射粒度
    /// 尚未裁决（设计稿 §未决第 5 条），本批**不做**该映射。
    pub fn default_capabilities(self) -> ContactCapabilities {
        match self {
            Self::Ready => ContactCapabilities {
                read: true,
                write: true,
                approval_required: true,
            },
            Self::Draft => ContactCapabilities {
                read: true,
                write: false,
                approval_required: true,
            },
            Self::Archived => ContactCapabilities {
                read: true,
                write: false,
                approval_required: true,
            },
            Self::Unavailable => ContactCapabilities {
                read: false,
                write: false,
                approval_required: true,
            },
        }
    }
}

/// 合同层的记忆作用域（**受管枚举占位**）。
///
/// ⚠️ **本批不与 `engine/memory` 的 `MemoryScope` 做映射**（那是 B2）。
/// 尤其注意：**`ContractMemoryScope::None` ≠ `MemoryScope::Global`** ——
/// `Global` 的 `is_visible_in` 语义与「不写记忆」**不等价**
/// （`Global` 是「对所有作用域可见」，`none` 是「根本不写」），
/// 拿 `Global` 当 `none` 会把一个不写记忆的联系人变成全局可见。
/// 真正的映射规则（`subject→User`、`contact→Persona`、`session→Session`、
/// `none→不写记忆`）留给 B2 批，写在 DTO 层、不改 `MemoryScope` 本体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractMemoryScope {
    /// 主体域记忆。
    Subject,
    /// 联系人自己的记忆（隔离单元：联系人 A 读不到联系人 B）。
    Contact,
    /// 会话域记忆。
    Session,
    /// 不写记忆（**不是** `Global`）。
    None,
}

impl Default for ContractMemoryScope {
    /// 缺省**不写记忆**（fail-closed）。
    fn default() -> Self {
        Self::None
    }
}

/// 联系人的能力**展示投影**。
///
/// ⚠️⚠️ **它不是授权依据。**
/// 这三个布尔只供 UI 展示「这个联系人看起来能做什么」；
/// 它们**不参与**任何 `Permission` / `tool_scope` 判定，
/// 也**不**从客户端请求接收（线格式的入面 [`ContactIn`] 里根本没有这个字段）。
/// 真正的授权判定走治理层，与联系人能力投影无关（设计稿 §未决第 5 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCapabilities {
    /// 展示用：可读。
    pub read: bool,
    /// 展示用：可写。
    pub write: bool,
    /// 展示用：写入需要人工审批。
    pub approval_required: bool,
}

impl Default for ContactCapabilities {
    /// 最保守的展示投影：不可读、不可写、写入需审批。
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            approval_required: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 人设文本
// ---------------------------------------------------------------------------

/// 已校验的人设文本。
///
/// 只能通过 [`PersonaText::new`]（或本模块内的反序列化）构造 ⇒ **构造即校验**：
/// 字节上限、禁止控制字符、拒绝空文本，任何一条不满足都拿不到这个类型。
#[derive(Clone, PartialEq, Eq)]
pub struct PersonaText {
    text: String,
}

impl PersonaText {
    /// 校验并封装一段人设文本。
    ///
    /// 校验项：
    /// 1. 非空（空串或纯空白 ⇒ 拒）；
    /// 2. 字节长度 ≤ [`MAX_PERSONA_BYTES`]（**字节**不是字符）；
    /// 3. 不含被禁字符（见模块顶部「换行裁决」）。
    pub fn new(raw: impl Into<String>) -> Result<Self, ContactError> {
        let text = raw.into();
        validate_persona(&text)?;
        Ok(Self { text })
    }

    /// 人设正文。
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// 人设正文的**字节**长度。
    pub fn byte_len(&self) -> usize {
        self.text.len()
    }
}

impl fmt::Debug for PersonaText {
    /// 人设正文**不直接进日志**：只报字节长度。
    ///
    /// 人设是自由文本（可能含用户私密内容），把它原样打进日志/错误帧
    /// 等于把内容复制到日志系统里；A1 的 `Workspace` 也有同样的脱敏先例。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersonaText")
            .field("bytes", &self.text.len())
            .finish()
    }
}

impl Serialize for PersonaText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

impl<'de> Deserialize<'de> for PersonaText {
    /// 反序列化同样走校验：超限 / 含控制字符 / 空文本 ⇒ 反序列化**失败**
    /// （落盘读回与线格式读入都是 fail-closed，不会悄悄放行脏文本）。
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        PersonaText::new(raw).map_err(serde::de::Error::custom)
    }
}

/// 校验人设文本（构造与反序列化共用同一条路径）。
fn validate_persona(text: &str) -> Result<(), ContactError> {
    if text.trim().is_empty() {
        return Err(ContactError::InvalidPersona {
            reason: "人设文本不得为空（空串或纯空白都不行）".to_string(),
        });
    }
    let bytes = text.len();
    if bytes > MAX_PERSONA_BYTES {
        return Err(ContactError::InvalidPersona {
            reason: format!("人设文本 {bytes} 字节，超过上限 {MAX_PERSONA_BYTES} 字节"),
        });
    }
    if let Some(found) = text.chars().find_map(persona_forbidden_char) {
        return Err(ContactError::InvalidPersona {
            reason: format!(
                "人设文本含被禁字符 {found:?}：{}",
                forbidden_char_reason(found)
            ),
        });
    }
    Ok(())
}

/// 判定一个字符是否被人设校验拒绝（返回被拒原因）。
///
/// 普通换行 `U+000A` 放行（模块顶部「换行裁决」），其余控制字符全拒。
fn persona_forbidden_char(c: char) -> Option<char> {
    let code = u32::from(c);
    if code == 0x000A {
        return None;
    }
    if code < 0x0020 {
        return Some(c);
    }
    if code == 0x007F {
        return Some(c);
    }
    if code == 0x2028 || code == 0x2029 {
        return Some(c);
    }
    None
}

/// 被禁字符的原因说明（给人看的）。
fn forbidden_char_reason(c: char) -> &'static str {
    let code = u32::from(c);
    if code == 0x000A {
        "普通换行（本批放行）"
    } else if code < 0x0020 {
        "ASCII 控制字符 (U+0000..U+001F)"
    } else if code == 0x007F {
        "ASCII 删除字符 (U+007F)"
    } else if code == 0x2028 {
        "行分隔符 (U+2028)"
    } else if code == 0x2029 {
        "段分隔符 (U+2029)"
    } else {
        "未知控制字符"
    }
}

// ---------------------------------------------------------------------------
// 领域类型
// ---------------------------------------------------------------------------

/// 一个联系人。
///
/// 字段**私有**、不提供任意 getter：外部只能通过具名访问器或
/// [`Contact::to_out`]（快照）读。`Debug` **脱敏**（人设正文只报字节长度）。
#[derive(Clone, PartialEq, Eq)]
pub struct Contact {
    contact_id: ContactId,
    agent_id: ContactAgentId,
    display_name: String,
    subtitle: Option<String>,
    avatar_ref: Option<String>,
    status: ContactStatus,
    persona_revision: u64,
    persona: Option<PersonaText>,
    memory_scope: ContractMemoryScope,
    capabilities: ContactCapabilities,
    revision: u64,
    updated_at_ms: u64,
}

impl Contact {
    /// 构造一条**草稿**联系人（新建联系人的初态）。
    ///
    /// `contact_id` 与 `agent_id` 在这里**由后端铸造**：入参里根本没有这两个
    /// 字段，客户端**无法**指定自己的身份。
    fn new_draft(request: &ContactCreateRequest) -> Result<Self, ContactError> {
        let display_name = validate_display_name(&request.display_name)?;
        let subtitle =
            validate_optional_text("subtitle", request.subtitle.as_deref(), MAX_SUBTITLE_BYTES)?;
        let avatar_ref = validate_optional_text(
            "avatar_ref",
            request.avatar_ref.as_deref(),
            MAX_AVATAR_REF_BYTES,
        )?;
        let status = ContactStatus::Draft;
        Ok(Self {
            contact_id: ContactId::new(),
            agent_id: ContactAgentId::new(),
            display_name,
            subtitle,
            avatar_ref,
            status,
            // 新建联系人的初始人设版本号是 0（第 0 版），此后每次
            // `update_persona` 恰好 +1。
            persona_revision: 0,
            persona: request.persona.clone(),
            memory_scope: request.memory_scope,
            capabilities: status.default_capabilities(),
            revision: 0,
            updated_at_ms: request.now_ms,
        })
    }

    /// 联系人身份。
    pub fn contact_id(&self) -> ContactId {
        self.contact_id
    }

    /// 该联系人的 agent 身份（合同里的 `agent_id`）。
    pub fn agent_id(&self) -> ContactAgentId {
        self.agent_id
    }

    /// 展示名（标签，不唯一）。
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// 副标题（可选）。
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// 头像引用（可选；不透明引用串，不是文件路径）。
    pub fn avatar_ref(&self) -> Option<&str> {
        self.avatar_ref.as_deref()
    }

    /// 合同态。
    pub fn status(&self) -> ContactStatus {
        self.status
    }

    /// 人设版本号（单调；每次人设写入恰好 +1）。
    pub fn persona_revision(&self) -> u64 {
        self.persona_revision
    }

    /// 当前人设（可选）。
    pub fn persona(&self) -> Option<&PersonaText> {
        self.persona.as_ref()
    }

    /// 合同层记忆作用域（占位枚举；本批**不**映射到 `MemoryScope`）。
    pub fn memory_scope(&self) -> ContractMemoryScope {
        self.memory_scope
    }

    /// 能力**展示投影**（不是授权依据）。
    pub fn capabilities(&self) -> ContactCapabilities {
        self.capabilities
    }

    /// 记录版本号（单调；任何写入 +1）。
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 最后写入时间（epoch 毫秒）。
    pub fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    /// 线格式快照（读面）。
    ///
    /// ⚠️ 只含**展示**字段，**不含任何内部路径类字段**（本类型里根本没有路径字段）。
    pub fn to_out(&self) -> ContactOut {
        ContactOut {
            contact_id: self.contact_id,
            agent_id: self.agent_id,
            display_name: self.display_name.clone(),
            subtitle: self.subtitle.clone(),
            avatar_ref: self.avatar_ref.clone(),
            status: self.status,
            persona_revision: self.persona_revision,
            persona: self.persona.as_ref().map(|p| p.as_str().to_string()),
            memory_scope: self.memory_scope,
            capabilities: self.capabilities,
            revision: self.revision,
            updated_at_ms: self.updated_at_ms,
        }
    }

    /// 落盘用的后端记录（`StoredDoc` 的 body）。
    fn to_record(&self) -> ContactRecord {
        ContactRecord {
            contact_id: self.contact_id,
            agent_id: self.agent_id,
            display_name: self.display_name.clone(),
            subtitle: self.subtitle.clone(),
            avatar_ref: self.avatar_ref.clone(),
            status: self.status,
            persona_revision: self.persona_revision,
            persona: self.persona.clone(),
            memory_scope: self.memory_scope,
            capabilities: self.capabilities,
            revision: self.revision,
            updated_at_ms: self.updated_at_ms,
        }
    }

    /// 从落盘记录还原（fail-closed：字段不合法就拒，不回退默认值）。
    fn from_record(record: ContactRecord) -> Result<Self, ContactError> {
        let display_name = validate_display_name(&record.display_name)?;
        let subtitle =
            validate_optional_text("subtitle", record.subtitle.as_deref(), MAX_SUBTITLE_BYTES)?;
        let avatar_ref = validate_optional_text(
            "avatar_ref",
            record.avatar_ref.as_deref(),
            MAX_AVATAR_REF_BYTES,
        )?;
        Ok(Self {
            contact_id: record.contact_id,
            agent_id: record.agent_id,
            display_name,
            subtitle,
            avatar_ref,
            status: record.status,
            persona_revision: record.persona_revision,
            persona: record.persona,
            memory_scope: record.memory_scope,
            capabilities: record.capabilities,
            revision: record.revision,
            updated_at_ms: record.updated_at_ms,
        })
    }

    /// 写入人设：人设版本 +1、记录版本 +1。
    fn apply_persona(&mut self, persona: PersonaText, now_ms: u64) {
        self.persona = Some(persona);
        self.persona_revision += 1;
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }

    /// 写入能力展示投影：记录版本 +1（**不动**人设版本号）。
    fn apply_capabilities(&mut self, capabilities: ContactCapabilities, now_ms: u64) {
        self.capabilities = capabilities;
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }

    /// 归档（软删除）：状态 ⇒ `Archived`、记录版本 +1。
    fn apply_archive(&mut self, now_ms: u64) {
        self.status = ContactStatus::Archived;
        self.revision += 1;
        self.updated_at_ms = now_ms;
    }
}

impl fmt::Debug for Contact {
    /// 人设正文脱敏（只报字节长度），其余字段照实输出。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Contact")
            .field("contact_id", &self.contact_id)
            .field("agent_id", &self.agent_id)
            .field("display_name", &self.display_name)
            .field("subtitle", &self.subtitle)
            .field("avatar_ref", &self.avatar_ref)
            .field("status", &self.status)
            .field("persona_revision", &self.persona_revision)
            .field(
                "persona_bytes",
                &self.persona.as_ref().map(|p| p.byte_len()),
            )
            .field("memory_scope", &self.memory_scope)
            .field("capabilities", &self.capabilities)
            .field("revision", &self.revision)
            .field("updated_at_ms", &self.updated_at_ms)
            .finish()
    }
}

impl Serialize for Contact {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_out().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Contact {
    /// 从**后端记录**（落盘 / 后端自产）还原；不是客户端请求面。
    ///
    /// 客户端请求面是 [`ContactIn`]，它不采信任何状态/能力/版本字段。
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let record = ContactRecord::deserialize(deserializer)?;
        Contact::from_record(record).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// 线格式
// ---------------------------------------------------------------------------

/// 读面快照（`ContactOut`）。
///
/// 只含展示字段；**不含任何内部路径类字段**。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactOut {
    /// 联系人身份。
    pub contact_id: ContactId,
    /// 可绑定的 agent 身份。
    pub agent_id: ContactAgentId,
    /// 展示名（标签，不唯一）。
    pub display_name: String,
    /// 副标题（可选）。
    pub subtitle: Option<String>,
    /// 头像引用（可选）。
    pub avatar_ref: Option<String>,
    /// 合同态。
    pub status: ContactStatus,
    /// 人设版本号。
    pub persona_revision: u64,
    /// 人设正文（可选）。
    pub persona: Option<String>,
    /// 合同层记忆作用域（占位枚举）。
    pub memory_scope: ContractMemoryScope,
    /// 能力**展示投影**（不是授权依据）。
    pub capabilities: ContactCapabilities,
    /// 记录版本号。
    pub revision: u64,
    /// 最后写入时间（epoch 毫秒）。
    pub updated_at_ms: u64,
}

/// 入面请求（`ContactIn`）。
///
/// ⚠️ **不接收** `status` / `capabilities` / `revision` / `persona_revision`
/// / `contact_id` / `agent_id`：
/// - 身份由**后端铸造**（`contact_id` / `agent_id`）；
/// - 状态与能力由**后端派生**（`status` / `capabilities`）；
/// - 版本号由**后端维护**（`revision` / `persona_revision`）。
///
/// 因此客户端在请求体里塞这些字段**不会被采信**：serde 对未知字段默认忽略，
/// 这些值一律丢弃，随后按派生值走。
#[derive(Debug, Clone, Deserialize)]
pub struct ContactIn {
    /// 展示名（必填，有界）。
    pub display_name: String,
    /// 副标题（可选，有界）。
    #[serde(default)]
    pub subtitle: Option<String>,
    /// 头像引用（可选，有界）。
    #[serde(default)]
    pub avatar_ref: Option<String>,
    /// 人设正文（可选；校验在 [`ContactIn::into_request`] 里做）。
    #[serde(default)]
    pub persona: Option<String>,
    /// 合同层记忆作用域（可选；缺省**不写记忆**）。
    #[serde(default)]
    pub memory_scope: Option<ContractMemoryScope>,
}

impl ContactIn {
    /// 校验并转成创建请求。
    ///
    /// 这一步会：校验展示名、校验副标题/头像引用、校验人设
    /// （超限 / 控制字符 / 空文本 ⇒ [`ContactError::InvalidPersona`]）。
    pub fn into_request(self, now_ms: u64) -> Result<ContactCreateRequest, ContactError> {
        let display_name = validate_display_name(&self.display_name)?;
        let subtitle =
            validate_optional_text("subtitle", self.subtitle.as_deref(), MAX_SUBTITLE_BYTES)?;
        let avatar_ref = validate_optional_text(
            "avatar_ref",
            self.avatar_ref.as_deref(),
            MAX_AVATAR_REF_BYTES,
        )?;
        let persona = match self.persona {
            Some(raw) => Some(PersonaText::new(raw)?),
            None => None,
        };
        Ok(ContactCreateRequest {
            display_name,
            subtitle,
            avatar_ref,
            persona,
            memory_scope: self.memory_scope.unwrap_or_default(),
            now_ms,
        })
    }
}

/// 落盘记录（`StoredDoc` 的 body；后端自产，字段不合法就拒绝打开）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactRecord {
    /// 联系人身份。
    pub contact_id: ContactId,
    /// 可绑定的 agent 身份。
    pub agent_id: ContactAgentId,
    /// 展示名。
    pub display_name: String,
    /// 副标题（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// 头像引用（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_ref: Option<String>,
    /// 合同态。
    pub status: ContactStatus,
    /// 人设版本号。
    pub persona_revision: u64,
    /// 人设正文（可选；反序列化时同样校验）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona: Option<PersonaText>,
    /// 合同层记忆作用域。
    pub memory_scope: ContractMemoryScope,
    /// 能力展示投影。
    pub capabilities: ContactCapabilities,
    /// 记录版本号。
    pub revision: u64,
    /// 最后写入时间（epoch 毫秒）。
    pub updated_at_ms: u64,
}

/// 创建请求（后端内部结构）。
///
/// ⚠️ **没有** `contact_id` / `agent_id` 字段 ⇒ 身份只能由后端铸造。
#[derive(Debug, Clone)]
pub struct ContactCreateRequest {
    /// 展示名。
    pub display_name: String,
    /// 副标题（可选）。
    pub subtitle: Option<String>,
    /// 头像引用（可选）。
    pub avatar_ref: Option<String>,
    /// 人设（可选；已校验）。
    pub persona: Option<PersonaText>,
    /// 合同层记忆作用域。
    pub memory_scope: ContractMemoryScope,
    /// 写入时间（epoch 毫秒）。
    pub now_ms: u64,
}

// ---------------------------------------------------------------------------
// 错误
// ---------------------------------------------------------------------------

/// 联系人领域的错误集。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContactError {
    /// 联系人不存在。
    #[error("联系人不存在: {0}")]
    NotFound(ContactId),
    /// 乐观并发冲突：`expected_revision` 与实际不符。
    ///
    /// 冲突时状态**零变化**（含 `updated_at_ms`）。
    #[error("版本冲突: 期望 {expected}, 实际 {actual}")]
    RevisionConflict {
        /// 调用方声称的版本号。
        expected: u64,
        /// 注册表里的真实版本号。
        actual: u64,
    },
    /// 联系人已归档：**不可**再修改，也**不可**复活。
    #[error("联系人已归档，不可修改且不可复活")]
    Archived,
    /// 展示名非法。
    #[error("展示名非法: {reason}")]
    InvalidDisplayName {
        /// 拒的原因。
        reason: String,
    },
    /// 人设非法（超限 / 控制字符 / 空文本）。
    #[error("人设非法: {reason}")]
    InvalidPersona {
        /// 拒的原因。
        reason: String,
    },
    /// 请求非法（含副标题/头像引用不合法，以及**持久化失败**）。
    ///
    /// 持久化故障也归到这里：错误集刻意只有六个变体，
    /// 让「只 match 已知变体」的调用方不会漏掉存储故障分支；
    /// 存储故障的 `reason` 以 `"storage: "` 开头。
    #[error("请求非法: {reason}")]
    InvalidRequest {
        /// 拒的原因。
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// 注册表
// ---------------------------------------------------------------------------

/// 联系人注册表。
///
/// 语义约定：
/// - `list()` **默认排除已归档**（墓碑不进活跃列表）；`list_all()` 含墓碑。
/// - 所有写入都带 `expected_revision`；不匹配 ⇒ [`ContactError::RevisionConflict`]，
///   且状态**零变化**。
/// - 归档是软删除：**没有**任何复活入口。
pub trait ContactRegistry: Send + Sync {
    /// 列出联系人（**有界**，**默认排除已归档**）。
    fn list(&self) -> Vec<Contact>;

    /// 列出全部联系人（含已归档的墓碑；有界）。
    fn list_all(&self) -> Vec<Contact>;

    /// 按身份取一个联系人（含已归档的墓碑）。
    fn get(&self, contact_id: &ContactId) -> Option<Contact>;

    /// 新建联系人（身份由后端铸造）。
    fn create(&self, request: ContactCreateRequest) -> Result<Contact, ContactError>;

    /// 写入人设（人设版本 +1、记录版本 +1）。
    fn update_persona(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        persona: PersonaText,
        now_ms: u64,
    ) -> Result<Contact, ContactError>;

    /// 写入能力**展示投影**（记录版本 +1；人设版本不动）。
    fn update_capabilities(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        capabilities: ContactCapabilities,
        now_ms: u64,
    ) -> Result<Contact, ContactError>;

    /// 归档（软删除；不可复活）。
    fn archive(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Contact, ContactError>;
}

/// 注册表内部状态。
#[derive(Debug, Default)]
struct RegistryInner {
    contacts: BTreeMap<ContactId, Contact>,
}

/// 进程内联系人注册表。
#[derive(Debug, Default)]
pub struct InMemoryContactRegistry {
    inner: Mutex<RegistryInner>,
}

impl InMemoryContactRegistry {
    /// 一个空注册表。
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContactRegistry for InMemoryContactRegistry {
    fn list(&self) -> Vec<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        list_contacts(&guard, false)
    }

    fn list_all(&self) -> Vec<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        list_contacts(&guard, true)
    }

    fn get(&self, contact_id: &ContactId) -> Option<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.contacts.get(contact_id).cloned()
    }

    fn create(&self, request: ContactCreateRequest) -> Result<Contact, ContactError> {
        let contact = Contact::new_draft(&request)?;
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.contacts.insert(contact.contact_id(), contact.clone());
        Ok(contact)
    }

    fn update_persona(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        persona: PersonaText,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            None,
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_persona(persona, now_ms);
            },
        )
    }

    fn update_capabilities(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        capabilities: ContactCapabilities,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            None,
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_capabilities(capabilities, now_ms);
            },
        )
    }

    fn archive(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            None,
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_archive(now_ms);
            },
        )
    }
}

/// 落盘式联系人注册表。
///
/// 落盘布局：`<root>/agents/contacts/<contact_id>/contact.json`
/// （`StoredDoc` 信封 + 持久档原子写 + 文件锁，全部走仓内既有工具）。
///
/// 重启后 [`FileContactRegistry::open`] 会把墓碑也读回来 ⇒ **归档不可复活**
/// 这条不变量跨重启仍然成立（复活入口本来就不存在）。
#[derive(Debug)]
pub struct FileContactRegistry {
    root: PathBuf,
    inner: Mutex<RegistryInner>,
}

impl FileContactRegistry {
    /// 打开（或建出）一个落盘注册表。
    ///
    /// 逐个读回 `agents/contacts/*/contact.json`；**读不动就拒开**
    /// （fail-closed，不静默丢记录、不回退默认值）。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ContactError> {
        let root = root.into();
        let dir = contacts_dir(&root);
        std::fs::create_dir_all(&dir)
            .map_err(|error| storage_error("创建联系人目录", &dir, error))?;
        let mut contacts = BTreeMap::new();
        let entries =
            std::fs::read_dir(&dir).map_err(|error| storage_error("读联系人目录", &dir, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| storage_error("读联系人目录项", &dir, error))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file = path.join(CONTACT_FILE_NAME);
            if !file.is_file() {
                continue;
            }
            let doc = stored_doc::open_single::<ContactRecord>(&file, &contact_doc_compat())
                .map_err(|error| storage_error("读联系人文档", &file, error))?;
            let contact =
                Contact::from_record(doc.body).map_err(|error| ContactError::InvalidRequest {
                    reason: format!("storage: 联系人文档非法 ({}): {error}", file.display()),
                })?;
            contacts.insert(contact.contact_id(), contact);
        }
        Ok(Self {
            root,
            inner: Mutex::new(RegistryInner { contacts }),
        })
    }

    /// 注册表根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ContactRegistry for FileContactRegistry {
    fn list(&self) -> Vec<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        list_contacts(&guard, false)
    }

    fn list_all(&self) -> Vec<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        list_contacts(&guard, true)
    }

    fn get(&self, contact_id: &ContactId) -> Option<Contact> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.contacts.get(contact_id).cloned()
    }

    fn create(&self, request: ContactCreateRequest) -> Result<Contact, ContactError> {
        let contact = Contact::new_draft(&request)?;
        persist_contact(&self.root, &contact)?;
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.contacts.insert(contact.contact_id(), contact.clone());
        Ok(contact)
    }

    fn update_persona(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        persona: PersonaText,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            Some(&self.root),
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_persona(persona, now_ms);
            },
        )
    }

    fn update_capabilities(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        capabilities: ContactCapabilities,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            Some(&self.root),
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_capabilities(capabilities, now_ms);
            },
        )
    }

    fn archive(
        &self,
        contact_id: &ContactId,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<Contact, ContactError> {
        mutate_contact(
            &self.inner,
            Some(&self.root),
            contact_id,
            expected_revision,
            |contact| {
                contact.apply_archive(now_ms);
            },
        )
    }
}

// ---------------------------------------------------------------------------
// 内部实现
// ---------------------------------------------------------------------------

/// 列出联系人（有界；`include_archived` 决定是否含墓碑）。
fn list_contacts(inner: &RegistryInner, include_archived: bool) -> Vec<Contact> {
    inner
        .contacts
        .values()
        .filter(|contact| include_archived || contact.status != ContactStatus::Archived)
        .take(MAX_LISTED_CONTACTS)
        .cloned()
        .collect()
}

/// 乐观并发前置检查：取出当前记录并核对版本与归档态。
///
/// 固定次序：**NotFound → Archived → RevisionConflict**。
/// 过期版本一律**不改状态**。
fn checked_contact(
    inner: &RegistryInner,
    contact_id: &ContactId,
    expected_revision: u64,
) -> Result<Contact, ContactError> {
    let contact = inner
        .contacts
        .get(contact_id)
        .ok_or(ContactError::NotFound(*contact_id))?;
    if contact.status == ContactStatus::Archived {
        return Err(ContactError::Archived);
    }
    if contact.revision != expected_revision {
        return Err(ContactError::RevisionConflict {
            expected: expected_revision,
            actual: contact.revision,
        });
    }
    Ok(contact.clone())
}

/// 统一的写入路径：核对 → 改副本 → （落盘）→ 提交内存。
///
/// 落盘失败时**内存状态零变化**（先落盘、后提交）。
fn mutate_contact(
    inner: &Mutex<RegistryInner>,
    root: Option<&Path>,
    contact_id: &ContactId,
    expected_revision: u64,
    apply: impl FnOnce(&mut Contact),
) -> Result<Contact, ContactError> {
    let mut guard = inner.lock().unwrap_or_else(|e| e.into_inner());
    let inner = &mut *guard;
    let mut updated = checked_contact(inner, contact_id, expected_revision)?;
    apply(&mut updated);
    if let Some(root) = root {
        persist_contact(root, &updated)?;
    }
    inner.contacts.insert(*contact_id, updated.clone());
    Ok(updated)
}

/// 联系人集合目录：`<root>/agents/contacts`。
fn contacts_dir(root: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in CONTACTS_DIR {
        path.push(segment);
    }
    path
}

/// 单个联系人的文档路径：`<root>/agents/contacts/<contact_id>/contact.json`。
fn contact_file(root: &Path, contact_id: &ContactId) -> PathBuf {
    contacts_dir(root)
        .join(contact_id.to_string())
        .join(CONTACT_FILE_NAME)
}

/// 联系人文档的版本契约。
fn contact_doc_compat() -> DocCompat {
    DocCompat::exact(CONTACT_DOC_NAME, CONTACT_DOC_VERSION)
}

/// 把一条联系人写盘（持久档原子写 + 文件锁，全部走仓内既有工具）。
fn persist_contact(root: &Path, contact: &Contact) -> Result<(), ContactError> {
    let file = contact_file(root, &contact.contact_id());
    let lock = storage_atomic::lock_path_for(&file);
    let record = contact.to_record();
    storage_atomic::with_file_lock(&lock, || {
        stored_doc::save_single(
            &file,
            &contact_doc_compat(),
            record,
            stored_doc::DEFAULT_DOC_MODE,
        )
    })
    .map_err(|error| storage_error("加锁", &lock, error))?
    .map_err(|error| storage_error("写联系人文档", &file, error))?;
    Ok(())
}

/// 存储故障 ⇒ [`ContactError::InvalidRequest`]（`reason` 以 `"storage: "` 开头）。
fn storage_error(action: &str, path: &Path, error: impl fmt::Display) -> ContactError {
    ContactError::InvalidRequest {
        reason: format!("storage: {action} 失败 ({}): {error}", path.display()),
    }
}

/// 校验展示名（非空、字节上限、无控制字符）。
fn validate_display_name(raw: &str) -> Result<String, ContactError> {
    if raw.trim().is_empty() {
        return Err(ContactError::InvalidDisplayName {
            reason: "展示名不得为空（空串或纯空白都不行）".to_string(),
        });
    }
    let bytes = raw.len();
    if bytes > MAX_DISPLAY_NAME_BYTES {
        return Err(ContactError::InvalidDisplayName {
            reason: format!("展示名 {bytes} 字节，超过上限 {MAX_DISPLAY_NAME_BYTES} 字节"),
        });
    }
    if let Some(found) = raw.chars().find(|c| c.is_control()) {
        return Err(ContactError::InvalidDisplayName {
            reason: format!("展示名含控制字符 {found:?}"),
        });
    }
    Ok(raw.to_string())
}

/// 校验可选的短文本字段（副标题 / 头像引用）。
fn validate_optional_text(
    field: &str,
    raw: Option<&str>,
    max_bytes: usize,
) -> Result<Option<String>, ContactError> {
    let Some(text) = raw else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Err(ContactError::InvalidRequest {
            reason: format!("{field} 不得为空串"),
        });
    }
    let bytes = text.len();
    if bytes > max_bytes {
        return Err(ContactError::InvalidRequest {
            reason: format!("{field} {bytes} 字节，超过上限 {max_bytes} 字节"),
        });
    }
    if let Some(found) = text.chars().find(|c| c.is_control()) {
        return Err(ContactError::InvalidRequest {
            reason: format!("{field} 含控制字符 {found:?}"),
        });
    }
    Ok(Some(text.to_string()))
}
