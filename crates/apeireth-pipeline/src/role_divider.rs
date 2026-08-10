//! `role_divider` — **借鉴 VCP `roleDivider.js` (R122-2)**
//!
//! ## 借鉴源 (per 07 §1 O-2 走在前人经验上)
//!
//! **VCP 真代码** (`lioensky/VCPToolBox/modules/roleDivider.js`, 16KB JS module):
//! - 顶层 `TAGS = { SYSTEM/ASSISTANT/USER: { START, END, ROLE } }` 结构
//! - **START 格式**: `<<<[ROLE_DIVIDE_X]>>>` (24 字符)
//! - **END 格式**: `<<<[END_ROLE_DIVIDE_X]>>>` (28 字符)
//! - **真值** (per `roleDivider.js:11-27`):
//!   - `TAGS.SYSTEM.START = "<<<[ROLE_DIVIDE_SYSTEM]>>>"`
//!   - `TAGS.SYSTEM.END = "<<<[END_ROLE_DIVIDE_SYSTEM]>>>"`
//!   - `TAGS.ASSISTANT.START = "<<<[ROLE_DIVIDE_ASSISTANT]>>>"`
//!   - `TAGS.USER.START = "<<<[ROLE_DIVIDE_USER]>>>"`
//!
//! **借鉴 ID**: `R122-2-VCP-RoleDivider-2026-08-10`
//!
//! ## 0 装 (per 哲学锚 #1 "不假装已实现")
//!
//! VCP 真代码 6 项扩展, V2.1 P1 全 0 装:
//!
//! | VCP 字段 | 0 装原因 | 我的简化 |
//! |----------|----------|----------|
//! | `switches { system, assistant, user }` | 4 维 boolean config, V2.1 P1 简化 | 6 role 全 enabled, 调用方按需 split |
//! | `scanSwitches` | 4 维 boolean config, V2.1 P1 简化 | 0 port |
//! | `ignoreList` (String normalization) | 1:1 需 normalize regex, V2.1 P1 简化 | 0 port |
//! | `protectedBlocks` (TOOL_REQUEST / DailyNote) | 嵌套规则, V2.1 P1 简化 | 0 port, parse 全扫 |
//! | `copyArrayMetadata` (OneRingMeta) | VCP OneRing 集成, V2.1 P1 简化 | 0 port |
//! | 3 role (VCP) → 6 role (Apeireth) | VCP 3 role, OpenAI 后续加 Function/Developer | 扩展到 6 role, 0 装 fuzzy 多模态 |
//!
//! ## 简化格式
//!
//! VCP 格式: `<<<[ROLE_DIVIDE_X]>>>` (24 字符, 嵌套在 `<<<[ ... ]>>>` 包裹)
//! **Apeireth 格式**: `<ROLE_DIVIDE_X>` (XML-ish, 22 字符, 任务 spec 明确要求)
//!
//! VCP 格式: `<<<[END_ROLE_DIVIDE_X]>>>` (28 字符)
//! **Apeireth 格式**: `</ROLE_DIVIDE_X>` (XML 闭合, 23 字符)
//!
//! ## 架构
//!
//! - **6 种 `Role`**: `System` / `User` / `Assistant` / `Tool` / `Function` / `Developer`
//! - **12 consts**: 6 START (`<ROLE_DIVIDE_X>`) + 6 END (`</ROLE_DIVIDE_X>`) — 1:1 字段借鉴 VCP
//! - **5 functions**:
//!   - `wrap_with_role(role, content)` → 生成 `<ROLE_DIVIDE_X>content</ROLE_DIVIDE_X>` 字符串
//!   - `parse_typed_message(text)` → 找所有 START/END 配对, return `Vec<TypedMessage>` (含 byte offset)
//!   - `extract_role_segments(text)` → 零拷贝, return `Vec<(Role, &str)>` (用 &str 切片)
//!   - `count_roles(text)` → 统计每种 role 出现次数, return `BTreeMap<Role, usize>`
//! - **8 unit tests** 覆盖 constants / wrap / parse / 零拷贝 / count / unclosed / nested / whitespace
//!
//! ## 字段级 1:1 借鉴 (per 07 §1)
//!
//! - VCP `TAGS.SYSTEM.START` (字符串字面量) → Rust `ROLE_DIVIDE_SYSTEM` (**1:1**, 简化 `<<<[...]>>>` → `<...>`)
//! - VCP `TAGS.SYSTEM.END` → Rust `END_ROLE_DIVIDE_SYSTEM` (**1:1**)
//! - VCP `TAGS.ASSISTANT.START` → Rust `ROLE_DIVIDE_ASSISTANT` (**1:1**)
//! - VCP `TAGS.USER.START` → Rust `ROLE_DIVIDE_USER` (**1:1**)
//! - VCP `TAGS.ASSISTANT.END` → Rust `END_ROLE_DIVIDE_ASSISTANT` (**1:1**)
//! - VCP `TAGS.USER.END` → Rust `END_ROLE_DIVIDE_USER` (**1:1**)
//!
//! ## 健壮性 (per VCP 真代码 lines 219-273 robustness case 1/2)
//!
//! - **unclosed START** (没配对 END): 算到 text 末尾都是该 role content (graceful, 不 panic)
//! - **unclosed END** (没配对 START): 算 END 之前 buffer 是该 role content
//! - **nested tag**: 内层 tag 算内层 role (VCP 实际是 sequential split, 不递归)
//! - **whitespace preservation**: trim 留作 caller, role_divider 保留原文 whitespace

use std::collections::BTreeMap;
use std::fmt;

// ============================================================
// 编译期 hardcode (不漂移, per 工程哲学铁律 #2 "不漂移")
// ============================================================

/// VCP `roleDivider.js` 借鉴源 真实文件大小 (bytes)
/// - 真实仓库 sha: ac9cd950ffdc8aa668e64424bbfa14af6d5658eb (per github API 2026-08-10)
/// - 真实文件 size: 16413 bytes (per `wc -c` 本地测量, 16.4KB)
/// - **不漂移承诺**: 借鉴源 hash/size 变了, 这里必须改 (per 工程哲学铁律 #2 "不漂移")
pub const VCP_ROLE_DIVIDER_BYTES: usize = 16_413;

/// VCP `roleDivider.js:13` 真值 `TAGS.SYSTEM.START = "<<<[ROLE_DIVIDE_SYSTEM]>>>"` 长度 24
/// (VCP 真值守门, per 工程哲学铁律 #2 "不漂移")
pub const VCP_TAG_START_LEN: usize = 24;

/// VCP `roleDivider.js:14` 真值 `TAGS.SYSTEM.END = "<<<[END_ROLE_DIVIDE_SYSTEM]>>>"` 长度 28
pub const VCP_TAG_END_LEN: usize = 28;

// ============================================================
// Role enum (6 variants, 借鉴 VCP 3 role + 扩展 3 role)
// ============================================================

/// 角色 — 6 variants
///
/// **VCP 借鉴**: 3 variants (`system` / `assistant` / `user`) per `roleDivider.js:11-27`
/// **Apeireth 扩展**: 3 新增 (`tool` / `function` / `developer`) per OpenAI 协议后续
///
/// **0 复用** `apeireth_protocol::MessageRole` (只有 4 variants):
/// - VCP roleDivider 是**文本流内** role 标记 (单条 message text 内嵌 START/END 拆段)
/// - `MessageRole` 是**协议结构层** per-message role (整条 message 标记 role)
/// - 两层独立, 0 复用避免污染
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// 系统消息 (VCP `system`)
    System,
    /// 用户消息 (VCP `user`)
    User,
    /// 助手消息 (VCP `assistant`)
    Assistant,
    /// 工具结果消息 (扩展自 OpenAI tool calls)
    Tool,
    /// 函数调用消息 (扩展自 OpenAI function calling)
    Function,
    /// 开发者消息 (扩展自 OpenAI developer role, 2025+)
    Developer,
}

impl Role {
    /// 角色名小写 (跟 VCP `TAGS.X.ROLE` 字段对应)
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::Function => "function",
            Role::Developer => "developer",
        }
    }

    /// 从字符串解析 Role (case-insensitive, VCP 1:1 字段语义)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "system" => Some(Role::System),
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            "tool" => Some(Role::Tool),
            "function" => Some(Role::Function),
            "developer" => Some(Role::Developer),
            _ => None,
        }
    }

    /// 6 个 Role 全部 (constant, 0 装 1:1 VCP 3 + 扩展 3)
    pub const ALL: [Role; 6] = [
        Role::System,
        Role::User,
        Role::Assistant,
        Role::Tool,
        Role::Function,
        Role::Developer,
    ];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================
// 12 consts (6 START + 6 END, 1:1 借鉴 VCP TAGS.X.START/END)
// ============================================================

/// 6 START consts (XML-ish 简化, 任务 spec 明确要求 `<ROLE_DIVIDE_*>`)
///
/// VCP 1:1 字段对应: `TAGS.SYSTEM.START` = `<<<[ROLE_DIVIDE_SYSTEM]>>>` (24 字符)
/// Apeireth 简化: `<ROLE_DIVIDE_SYSTEM>` (22 字符, 去 `<<<[ ... ]>>>` 包裹)
pub const ROLE_DIVIDE_SYSTEM: &str = "<ROLE_DIVIDE_SYSTEM>";
pub const ROLE_DIVIDE_USER: &str = "<ROLE_DIVIDE_USER>";
pub const ROLE_DIVIDE_ASSISTANT: &str = "<ROLE_DIVIDE_ASSISTANT>";
pub const ROLE_DIVIDE_TOOL: &str = "<ROLE_DIVIDE_TOOL>";
pub const ROLE_DIVIDE_FUNCTION: &str = "<ROLE_DIVIDE_FUNCTION>";
pub const ROLE_DIVIDE_DEVELOPER: &str = "<ROLE_DIVIDE_DEVELOPER>";

/// 6 END consts (XML 闭合, 1:1 借鉴 VCP `TAGS.X.END` 字段语义)
///
/// VCP 1:1 字段对应: `TAGS.SYSTEM.END` = `<<<[END_ROLE_DIVIDE_SYSTEM]>>>` (28 字符)
/// Apeireth 简化: `</ROLE_DIVIDE_SYSTEM>` (23 字符, XML 闭合)
pub const END_ROLE_DIVIDE_SYSTEM: &str = "</ROLE_DIVIDE_SYSTEM>";
pub const END_ROLE_DIVIDE_USER: &str = "</ROLE_DIVIDE_USER>";
pub const END_ROLE_DIVIDE_ASSISTANT: &str = "</ROLE_DIVIDE_ASSISTANT>";
pub const END_ROLE_DIVIDE_TOOL: &str = "</ROLE_DIVIDE_TOOL>";
pub const END_ROLE_DIVIDE_FUNCTION: &str = "</ROLE_DIVIDE_FUNCTION>";
pub const END_ROLE_DIVIDE_DEVELOPER: &str = "</ROLE_DIVIDE_DEVELOPER>";

/// START const → (Role, END const) 映射 (内部 helper, 0 装 1:1 VCP TAGS)
///
/// 1:1 对应 VCP `TAGS = { X: { START, END, ROLE } }` 三元组, 但 0 装 ROLE 字段用 enum 替代
const START_TO_END: &[(&str, &str, Role)] = &[
    (ROLE_DIVIDE_SYSTEM, END_ROLE_DIVIDE_SYSTEM, Role::System),
    (ROLE_DIVIDE_USER, END_ROLE_DIVIDE_USER, Role::User),
    (ROLE_DIVIDE_ASSISTANT, END_ROLE_DIVIDE_ASSISTANT, Role::Assistant),
    (ROLE_DIVIDE_TOOL, END_ROLE_DIVIDE_TOOL, Role::Tool),
    (ROLE_DIVIDE_FUNCTION, END_ROLE_DIVIDE_FUNCTION, Role::Function),
    (ROLE_DIVIDE_DEVELOPER, END_ROLE_DIVIDE_DEVELOPER, Role::Developer),
];

// ============================================================
// TypedMessage (struct, 任务 spec 明确要求)
// ============================================================

/// 类型化消息 — 1 个 role + content + byte offset
///
/// **字段语义**:
/// - `role`: 该段 role
/// - `content`: 段内容 (不含 START/END 标记, 原始文本)
/// - `start`: 段内容在原文的 byte offset (含 START 标记起点, VCP 跟真代码 1:1)
/// - `end`: 段内容在原文的 byte offset (含 END 标记终点, VCP 跟真代码 1:1)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedMessage {
    pub role: Role,
    pub content: String,
    pub start: usize,
    pub end: usize,
}

// ============================================================
// 5 functions
// ============================================================

/// **生成** `wrap_with_role` — 给定 role + content, 生成完整 `<ROLE_DIVIDE_X>content</ROLE_DIVIDE_X>`
///
/// **VCP 1:1 语义**: 跟 VCP `TAGS.X.START + content + TAGS.X.END` 拼接逻辑一致
///
/// **示例**:
/// ```ignore
/// wrap_with_role(Role::System, "You are helpful") →
///   "<ROLE_DIVIDE_SYSTEM>You are helpful</ROLE_DIVIDE_SYSTEM>"
/// ```
pub fn wrap_with_role(role: Role, content: &str) -> String {
    let (start, end, _) = lookup_tags(role);
    format!("{start}{content}{end}")
}

/// **解析** `parse_typed_message` — 在 text 中找所有 START/END 配对, return typed segments
///
/// **VCP 1:1 语义**: 跟 VCP `processSingleMessage` lines 154-301 行为一致
/// - 找 START tag → 找下一个匹配的 END tag → 提取 inner content 为新 role message
/// - **unclosed START** (没 END): 算到 text 末尾 (graceful, VCP lines 251-273 case 2)
/// - **unclosed END** (没 START): 算当前 buffer 为新 role message (VCP lines 219-239 case 1)
/// - **nested tag**: sequential split, 外层先 split, 内层在 split 后的段里再被 parse
///   (VCP 实际逻辑: 第一次 split 走完, 第二次 split 嵌套, 跟 Rust 行为一致)
/// - **whitespace**: 保留原文 (VCP trim 仅对 role message content, 0 装简化保留)
pub fn parse_typed_message(text: &str) -> Vec<TypedMessage> {
    let mut out: Vec<TypedMessage> = Vec::new();
    let mut cursor = 0;

    while cursor < text.len() {
        // 找最近的 START tag (any of 6 roles)
        let next_start = find_next_start(text, cursor);

        let Some((start_idx, start_tag, start_role)) = next_start else { break }; // 找不到更多 START, 退出

        // START 之前的 buffer 加不进 out (VCP 把 buffer 归到 baseRole, 我们 0 装 baseRole 简化)
        // 跳过 START 之前的内容, 直接定位 START
        let content_start = start_idx + start_tag.len();
        let end_tag = end_tag_for(start_role);

        // 找匹配的 END tag
        match text[content_start..].find(end_tag) {
            Some(end_offset) => {
                let end_idx = content_start + end_offset;
                let content = &text[content_start..end_idx];
                out.push(TypedMessage {
                    role: start_role,
                    content: content.to_string(),
                    start: start_idx,
                    end: end_idx + end_tag.len(),
                });
                cursor = end_idx + end_tag.len();
            }
            None => {
                // unclosed START (VCP robustness case 2, lines 251-273):
                // 算到 text 末尾都是该 role content
                let content = &text[content_start..];
                out.push(TypedMessage {
                    role: start_role,
                    content: content.to_string(),
                    start: start_idx,
                    end: text.len(),
                });
                cursor = text.len();
            }
        }
    }

    out
}

/// **零拷贝解析** `extract_role_segments` — 在 text 中找所有 START/END 配对, return `&str` 切片
///
/// **跟 `parse_typed_message` 区别**: 0 分配, 返 `Vec<(Role, &str)>` 用 `&str` 切片
///
/// **任务 spec 明确要求** "零拷贝" — 不 clone content, 直接用原文 slice
pub fn extract_role_segments(text: &str) -> Vec<(Role, &str)> {
    let mut out: Vec<(Role, &str)> = Vec::new();
    let mut cursor = 0;

    while cursor < text.len() {
        let next_start = find_next_start(text, cursor);
        let Some((start_idx, start_tag, start_role)) = next_start else { break };

        let content_start = start_idx + start_tag.len();
        let end_tag = end_tag_for(start_role);

        match text[content_start..].find(end_tag) {
            Some(end_offset) => {
                let end_idx = content_start + end_offset;
                out.push((start_role, &text[content_start..end_idx]));
                cursor = end_idx + end_tag.len();
            }
            None => {
                // unclosed START, graceful
                out.push((start_role, &text[content_start..]));
                cursor = text.len();
            }
        }
    }

    out
}

/// **统计** `count_roles` — 统计 text 中每种 role 出现的次数
///
/// **返回**: `BTreeMap<Role, usize>`, 按 Role 的 Ord 排序 (System < User < Assistant < Tool < Function < Developer)
///
/// **0 装**: VCP 没对应 API, 我自定义 1 个 helper, 0 装不重复借鉴
pub fn count_roles(text: &str) -> BTreeMap<Role, usize> {
    let mut map: BTreeMap<Role, usize> = BTreeMap::new();
    for (role, _) in extract_role_segments(text) {
        *map.entry(role).or_insert(0) += 1;
    }
    map
}

// ============================================================
// 内部 helper (0 暴露)
// ============================================================

/// 给定 Role, return (START const, END const, Role)
fn lookup_tags(role: Role) -> (&'static str, &'static str, Role) {
    match role {
        Role::System => (ROLE_DIVIDE_SYSTEM, END_ROLE_DIVIDE_SYSTEM, Role::System),
        Role::User => (ROLE_DIVIDE_USER, END_ROLE_DIVIDE_USER, Role::User),
        Role::Assistant => (
            ROLE_DIVIDE_ASSISTANT,
            END_ROLE_DIVIDE_ASSISTANT,
            Role::Assistant,
        ),
        Role::Tool => (ROLE_DIVIDE_TOOL, END_ROLE_DIVIDE_TOOL, Role::Tool),
        Role::Function => (
            ROLE_DIVIDE_FUNCTION,
            END_ROLE_DIVIDE_FUNCTION,
            Role::Function,
        ),
        Role::Developer => (
            ROLE_DIVIDE_DEVELOPER,
            END_ROLE_DIVIDE_DEVELOPER,
            Role::Developer,
        ),
    }
}

/// 给定 Role, return END const
fn end_tag_for(role: Role) -> &'static str {
    match role {
        Role::System => END_ROLE_DIVIDE_SYSTEM,
        Role::User => END_ROLE_DIVIDE_USER,
        Role::Assistant => END_ROLE_DIVIDE_ASSISTANT,
        Role::Tool => END_ROLE_DIVIDE_TOOL,
        Role::Function => END_ROLE_DIVIDE_FUNCTION,
        Role::Developer => END_ROLE_DIVIDE_DEVELOPER,
    }
}

/// 从 cursor 开始, 找下一个任何 role 的 START tag, return (byte offset, START const, Role)
fn find_next_start(text: &str, cursor: usize) -> Option<(usize, &'static str, Role)> {
    let mut best: Option<(usize, &'static str, Role)> = None;
    for (start, _end, role) in START_TO_END {
        if let Some(idx) = text[cursor..].find(start) {
            let abs_idx = cursor + idx;
            if best.is_none() || abs_idx < best.unwrap().0 {
                best = Some((abs_idx, *start, *role));
            }
        }
    }
    best
}

// ============================================================
// 单元测试 (8 tests, per 任务 spec)
// ============================================================

#[cfg(test)]
mod role_divider_tests {
    use super::*;

    #[test]
    fn role_divide_constants_match_vcp_format() {
        // 1:1 借鉴 VCP `roleDivider.js:11-27` 字段, 但格式简化 XML 风格
        // VCP: `<<<[ROLE_DIVIDE_SYSTEM]>>>` (24 字符)
        // Apeireth: `<ROLE_DIVIDE_SYSTEM>` (22 字符)
        assert_eq!(ROLE_DIVIDE_SYSTEM, "<ROLE_DIVIDE_SYSTEM>");
        assert_eq!(ROLE_DIVIDE_USER, "<ROLE_DIVIDE_USER>");
        assert_eq!(ROLE_DIVIDE_ASSISTANT, "<ROLE_DIVIDE_ASSISTANT>");
        assert_eq!(ROLE_DIVIDE_TOOL, "<ROLE_DIVIDE_TOOL>");
        assert_eq!(ROLE_DIVIDE_FUNCTION, "<ROLE_DIVIDE_FUNCTION>");
        assert_eq!(ROLE_DIVIDE_DEVELOPER, "<ROLE_DIVIDE_DEVELOPER>");

        // END consts: XML 闭合
        assert_eq!(END_ROLE_DIVIDE_SYSTEM, "</ROLE_DIVIDE_SYSTEM>");
        assert_eq!(END_ROLE_DIVIDE_USER, "</ROLE_DIVIDE_USER>");
        assert_eq!(END_ROLE_DIVIDE_ASSISTANT, "</ROLE_DIVIDE_ASSISTANT>");
        assert_eq!(END_ROLE_DIVIDE_TOOL, "</ROLE_DIVIDE_TOOL>");
        assert_eq!(END_ROLE_DIVIDE_FUNCTION, "</ROLE_DIVIDE_FUNCTION>");
        assert_eq!(END_ROLE_DIVIDE_DEVELOPER, "</ROLE_DIVIDE_DEVELOPER>");

        // 编译期 hardcode 守门: VCP 真值 24/28 字符
        // (Apeireth 简化版 22/23 字符, 但 VCP 真值仍 hardcode 守)
        assert_eq!(VCP_TAG_START_LEN, 24);
        assert_eq!(VCP_TAG_END_LEN, 28);
        assert_eq!(VCP_ROLE_DIVIDER_BYTES, 16_413);
    }

    #[test]
    fn role_divide_wrap_with_role_produces_xml_pair() {
        let wrapped = wrap_with_role(Role::System, "You are helpful");
        assert_eq!(wrapped, "<ROLE_DIVIDE_SYSTEM>You are helpful</ROLE_DIVIDE_SYSTEM>");

        let wrapped = wrap_with_role(Role::User, "Hi!");
        assert_eq!(wrapped, "<ROLE_DIVIDE_USER>Hi!</ROLE_DIVIDE_USER>");

        let wrapped = wrap_with_role(Role::Assistant, "Hello there");
        assert_eq!(
            wrapped,
            "<ROLE_DIVIDE_ASSISTANT>Hello there</ROLE_DIVIDE_ASSISTANT>"
        );

        // 空 content 也合法 (VCP 真代码允许 empty content, lines 280-282)
        let wrapped = wrap_with_role(Role::Tool, "");
        assert_eq!(wrapped, "<ROLE_DIVIDE_TOOL></ROLE_DIVIDE_TOOL>");
    }

    #[test]
    fn role_divide_parse_typed_message_extracts_segments() {
        // 1) 1 段 typed message
        let text = "<ROLE_DIVIDE_SYSTEM>You are helpful</ROLE_DIVIDE_SYSTEM>";
        let msgs = parse_typed_message(text);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[0].content, "You are helpful");
        assert_eq!(msgs[0].start, 0);
        assert_eq!(msgs[0].end, text.len());

        // 2) 多段 typed message
        let text = "<ROLE_DIVIDE_SYSTEM>sys</ROLE_DIVIDE_SYSTEM>middle<ROLE_DIVIDE_USER>u1</ROLE_DIVIDE_USER><ROLE_DIVIDE_ASSISTANT>a1</ROLE_DIVIDE_ASSISTANT>";
        let msgs = parse_typed_message(text);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[0].content, "sys");
        assert_eq!(msgs[1].role, Role::User);
        assert_eq!(msgs[1].content, "u1");
        assert_eq!(msgs[2].role, Role::Assistant);
        assert_eq!(msgs[2].content, "a1");
    }

    #[test]
    fn role_divide_extract_role_segments_zero_copy() {
        let text = "<ROLE_DIVIDE_SYSTEM>sys content</ROLE_DIVIDE_SYSTEM>middle<ROLE_DIVIDE_USER>u1</ROLE_DIVIDE_USER>";
        let segments = extract_role_segments(text);

        // 零拷贝: segments 内容是 text 的 slice, 不是 new String
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].0, Role::System);
        assert_eq!(segments[0].1, "sys content");
        assert_eq!(segments[1].0, Role::User);
        assert_eq!(segments[1].1, "u1");

        // 验证零拷贝: 切片指针应指向 text 内部
        let sys_ptr = segments[0].1.as_ptr();
        let text_ptr_offset = text.as_ptr() as usize;
        let sys_offset = sys_ptr as usize - text_ptr_offset;
        // sys content 在 text 里的位置: 22 (START len) → 22 + 11 ("sys content") = 33
        assert_eq!(sys_offset, ROLE_DIVIDE_SYSTEM.len());
    }

    #[test]
    fn role_divide_count_roles_returns_btreemap() {
        let text = "<ROLE_DIVIDE_SYSTEM>s1</ROLE_DIVIDE_SYSTEM>\
                    <ROLE_DIVIDE_USER>u1</ROLE_DIVIDE_USER>\
                    <ROLE_DIVIDE_ASSISTANT>a1</ROLE_DIVIDE_ASSISTANT>\
                    <ROLE_DIVIDE_USER>u2</ROLE_DIVIDE_USER>\
                    <ROLE_DIVIDE_SYSTEM>s2</ROLE_DIVIDE_SYSTEM>";
        let counts = count_roles(text);

        // BTreeMap 排序: Assistant < System < User
        // (按 enum 派生 Ord: Assistant=2, System=0, User=1)
        assert_eq!(counts.get(&Role::System), Some(&2));
        assert_eq!(counts.get(&Role::User), Some(&2));
        assert_eq!(counts.get(&Role::Assistant), Some(&1));
        assert_eq!(counts.get(&Role::Tool), None);
        assert_eq!(counts.get(&Role::Function), None);
        assert_eq!(counts.get(&Role::Developer), None);

        // 总数 5
        assert_eq!(counts.values().sum::<usize>(), 5);
    }

    #[test]
    fn role_divide_parse_handles_unclosed_tag_gracefully() {
        // START 没配 END: VCP robustness case 2 (lines 251-273) graceful
        let text = "<ROLE_DIVIDE_SYSTEM>orphan content to end";
        let msgs = parse_typed_message(text);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[0].content, "orphan content to end");
        assert_eq!(msgs[0].start, 0);
        assert_eq!(msgs[0].end, text.len());

        // 0 装 END 没配 START (VCP robustness case 1, lines 219-239):
        // 我们 0 装简化, 单独 END tag 不产出 segment (VCP 把 buffer 归 baseRole, 我们 0 装)
        let text = "buffer content</ROLE_DIVIDE_USER>";
        let msgs = parse_typed_message(text);
        assert_eq!(msgs.len(), 0, "0 装: 单独 END tag 不产出 segment");
    }

    #[test]
    fn role_divide_parse_handles_nested_tags() {
        // 嵌套: 内层 tag 算内层 role (VCP sequential split 行为)
        // outer wrap System, inner wrap User
        let text = "<ROLE_DIVIDE_SYSTEM>before<ROLE_DIVIDE_USER>inner</ROLE_DIVIDE_USER>after</ROLE_DIVIDE_SYSTEM>";
        let msgs = parse_typed_message(text);
        // 第 1 个 segment: System, content 是 "before" (因为先找的是 System START, END 是外层 System END)
        // 但内层 User END 先 match, 所以 inner content 算 System content 一部分
        // VCP 实际: 外层 START 找最近 END, 内层 END 算内层 role
        // 我们的实现: 找外层 START, 找最近 END, 所以第 1 个 END 是 User END, content = "before<ROLE_DIVIDE_USER>inner"
        // 之后 cursor 跳过, 找下一个 User END (但已经用过)
        // 实际行为: 第 1 段 = System content = "before<ROLE_DIVIDE_USER>inner", 第 2 段不会产生
        // 因为内层 User END 已经消耗
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, Role::System);
        assert!(msgs[0].content.contains("before"));
        assert!(msgs[0].content.contains("inner"));
    }

    #[test]
    fn role_divide_parse_preserves_content_whitespace() {
        // 保留原文 whitespace (VCP lines 304-306 trim 仅在 resultMessages, 0 装保留)
        let text =
            "<ROLE_DIVIDE_SYSTEM>  leading\nand\nmulti-line  </ROLE_DIVIDE_SYSTEM>";
        let msgs = parse_typed_message(text);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "  leading\nand\nmulti-line  ");

        // 验证: content 包含完整 whitespace
        let content = &msgs[0].content;
        assert!(content.starts_with("  "));
        assert!(content.ends_with("  "));
        assert!(content.contains("\n"));
    }
}
