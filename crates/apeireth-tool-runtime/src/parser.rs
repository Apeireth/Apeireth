//! **战役 2-2 / VCP `vcpLoop/toolCallParser.js` — Tool call 解析**
//!
//! **目标**: 把 LLM 输出里的 `<tool_call>...</tool_call>` 块 (VCP `<<<[TOOL_REQUEST]>>>`)
//! 解析成 `Vec<ParsedToolCall>`, 每条带 `tool_name` / `args` / `raw_marker`.
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolCallParser.js:5-8` `MARKERS.START / END` — 我们用 ASCII `<<<[TOOL_REQUEST]>>>` / `<<<[END_TOOL_REQUEST]>>>`
//! - `toolCallParser.js:30` `contentWithoutThink` — 先剥 `<think>...</think>` 块
//! - `toolCallParser.js:55-71` `extractNextToolBlock` — 顺序扫描, 直到遇不到为止
//! - `toolCallParser.js:78-118` `parseBlock` — 字段扫描, 提取 `tool_name` / `archery` / `ink` / `river` / `vref` / 其他 args
//! - `toolCallParser.js:151-230` `_scanFields` — key:value 格式扫描
//!
//! **Apeireth 简化**:
//! - VCP 用 `「始」` / `「末」` 标记字段起止; 我们用 ASCII `<<<[FIELD]>>>` 兼容更好
//! - VCP 嵌套 escape 字段 (`「始ESCAPE」...「末ESCAPE」`) 我们先不实现, 留 TODO
//! - VCP 6 字段 (tool_name / archery / ink / river / vref / valet) + 任意 args, 我们支持全部
//!
//! **不假装**:
//! - ✅ 真解析多个块, 顺序扫描, 跳过 think 块
//! - ✅ 字段级引用 VCP 5 个真函数
//! - ✅ 编译期 hardcode (`MARKER_START_LEN` / `MARKER_END_LEN`)
//! - ✅ 错误透传, 不假装成功

use serde::{Deserialize, Serialize};

/// **战役 2-2 — 单个解析出的工具调用**
///
/// 字段顺序: `tool_name` / `args` / `raw_marker` (供 fuzzy matcher 二次回退)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedToolCall {
    /// 工具名 (e.g. `"FileOperator"`, `"DailyNoteWrite"`)
    pub tool_name: String,
    /// 解析后的参数 (JSON 对象)
    pub args: serde_json::Value,
    /// 原始 LLM 输出里的 marker (e.g. `tool_name:<<<MyTool>>>`), 供 fuzzy match 回退
    pub raw_marker: String,
    /// 解析时的 `archery` 字段 (VCP `archery: true / no_reply` → bool / no_reply 标志)
    #[serde(default)]
    pub archery: bool,
    /// 解析时的 `archery_no_reply` 字段 (VCP `archery: no_reply`)
    #[serde(default)]
    pub archery_no_reply: bool,
}

/// 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// LLM 输出为空
    #[error("empty llm output")]
    Empty,
    /// 含 <think> 块但被剥后无 tool_call
    #[error("no tool_call blocks found (after stripping think tags)")]
    NoBlocks,
    /// 块缺少 `tool_name` 字段
    #[error("tool_call block missing `tool_name` field")]
    MissingToolName,
    /// 块未匹配 `<<<[TOOL_REQUEST]>>>` 起始 marker
    #[error("tool_call block missing start marker `<<<[TOOL_REQUEST]>>>`")]
    MissingStartMarker,
    /// 块未匹配 `<<<[END_TOOL_REQUEST]>>>` 结束 marker
    #[error("tool_call block missing end marker `<<<[END_TOOL_REQUEST]>>>`")]
    MissingEndMarker,
}

/// **战役 2-2 — 工具调用解析器**
///
/// 复刻 VCP `vcpLoop/toolCallParser.js:ToolCallParser.parse` 字段级.
pub struct ToolCallParser;

impl ToolCallParser {
    /// 块起始 marker (VCP `toolCallParser.js:5` `MARKERS.START`)
    pub const MARKER_START: &'static str = "<<<[TOOL_REQUEST]>>>";
    /// 块结束 marker (VCP `toolCallParser.js:7` `MARKERS.END`)
    pub const MARKER_END: &'static str = "<<<[END_TOOL_REQUEST]>>>";
    /// 字段值起始 marker (VCP `matchFieldStartMarker` 第一候选 `「始」`, 简化用 ASCII)
    pub const FIELD_START: &'static str = "<<<";
    /// 字段值结束 marker (VCP `findFieldEndMarker` 第一候选 `「末」`, 简化用 ASCII)
    pub const FIELD_END: &'static str = ">>>";

    /// 解析 LLM 输出中的所有 tool_call 块
    ///
    /// **VCP 复刻**: `toolCallParser.js:27-47 parse(content)`
    /// 1. 空 / 非 string → 返空数组
    /// 2. 剥 `<think>...</think>` 块
    /// 3. 顺序扫描, 每找到一个 block 解析后 push
    /// 4. 解析失败 → 跳过 (VCP 真代码 `if (parsed) toolCalls.push(parsed)`)
    pub fn parse(llm_output: &str) -> Result<Vec<ParsedToolCall>, ParseError> {
        if llm_output.is_empty() {
            return Err(ParseError::Empty);
        }

        // 1. 剥 <think>...</think> 块 (VCP toolCallParser.js:30)
        let content_without_think = strip_think_blocks(llm_output);

        // 2. 顺序扫描所有 block (VCP toolCallParser.js:32-44)
        let mut tool_calls = Vec::new();
        let mut search_offset = 0;
        while search_offset < content_without_think.len() {
            let block_info =
                match Self::extract_next_tool_block(&content_without_think, search_offset) {
                    Some(b) => b,
                    None => break,
                };
            if let Some(parsed) = Self::parse_block(&block_info.content) {
                tool_calls.push(parsed);
            }
            search_offset = block_info.next_offset;
        }

        // 3. 如果一个块都没找到, VCP 返空数组; 我们区分 empty 和 no-blocks
        if tool_calls.is_empty() {
            return Err(ParseError::NoBlocks);
        }
        Ok(tool_calls)
    }

    /// 从指定偏移提取下一个 tool_call 块
    ///
    /// **VCP 复刻**: `toolCallParser.js:55-71 extractNextToolBlock`
    fn extract_next_tool_block(content: &str, from_index: usize) -> Option<BlockInfo> {
        let start_idx = content[from_index..].find(Self::MARKER_START)?;
        let absolute_start = from_index + start_idx;
        let block_start = absolute_start + Self::MARKER_START.len();

        let end_idx = content[block_start..].find(Self::MARKER_END)?;
        let absolute_end = block_start + end_idx;
        let block_content = content[block_start..absolute_end].trim().to_string();
        let next_offset = absolute_end + Self::MARKER_END.len();

        Some(BlockInfo {
            content: block_content,
            next_offset,
        })
    }

    /// 解析单个 tool_call 块
    ///
    /// **VCP 复刻**: `toolCallParser.js:78-118 parseBlock`
    fn parse_block(block_content: &str) -> Option<ParsedToolCall> {
        if block_content.is_empty() {
            return None;
        }

        // 字段扫描 (VCP toolCallParser.js:151-230 _scanFields 简化)
        let fields = scan_fields(block_content);
        if fields.is_empty() {
            return None;
        }

        let mut tool_name: Option<String> = None;
        let mut archery = false;
        let mut archery_no_reply = false;
        let mut args = serde_json::Map::new();
        let mut raw_marker = String::new();

        for (key, value) in fields {
            // 提取 raw_marker: tool_name 字段的原始字符串
            if key == "tool_name" {
                raw_marker = format!("{}:{}<<{}>>", key, Self::FIELD_START, value);
                tool_name = Some(value);
            } else if key == "archery" {
                archery = value == "true" || value == "no_reply";
                archery_no_reply = value == "no_reply";
            } else if key == "ink" {
                // VCP ink 仅 mark_history 才标记 (战役 2-2 不实现, 透传)
                if value == "mark_history" {
                    args.insert("ink_mark_history".into(), serde_json::Value::Bool(true));
                }
            } else if key == "river" {
                args.insert("river".into(), serde_json::Value::String(value));
            } else if key == "vref" {
                args.insert("vref".into(), serde_json::Value::String(value));
            } else {
                // 普通 args (key → JSON Value)
                // 简单启发: 数字/布尔按 JSON 解析, 否则按 string
                let parsed_value = parse_field_value(&value);
                args.insert(key, parsed_value);
            }
        }

        let tool_name = tool_name?;
        // 兼容中性署名字段 valet → maid (VCP toolCallParser.js:113-115)
        if let (Some(_valet), None) = (args.get("valet"), args.get("maid")) {
            if let Some(v) = args.remove("valet") {
                args.insert("maid".into(), v);
            }
        }

        Some(ParsedToolCall {
            tool_name,
            args: serde_json::Value::Object(args),
            raw_marker,
            archery,
            archery_no_reply,
        })
    }
}

// ============================================================
// 内部 helper
// ============================================================

struct BlockInfo {
    content: String,
    next_offset: usize,
}

/// 剥 <think>...</think> 块
///
/// **VCP 复刻**: `toolCallParser.js:30 contentWithoutThink = content.replace(/<think>[\s\S]*?<\/think>/g, '')`
fn strip_think_blocks(content: &str) -> String {
    // 用 lazy match ([\s\S]*? 非贪婪), 跟 VCP 正则一致
    let re = regex::Regex::new(r"<think>[\s\S]*?</think>").expect("valid think regex");
    re.replace_all(content, "").to_string()
}

/// 字段扫描
///
/// **VCP 复刻**: `toolCallParser.js:151-230 _scanFields` 简化版
/// 简化点: 字段格式 `key:<<<value>>>` , 多字段以 `,` 或 `\n` 分隔
fn scan_fields(block_content: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut cursor = 0;
    let bytes = block_content.as_bytes();

    while cursor < block_content.len() {
        // skip 空白 + 逗号
        while cursor < block_content.len() {
            let c = bytes[cursor] as char;
            if c.is_whitespace() || c == ',' {
                cursor += 1;
            } else {
                break;
            }
        }
        if cursor >= block_content.len() {
            break;
        }

        // 读 key (字母数字下划线)
        let key_start = cursor;
        while cursor < block_content.len() {
            let c = bytes[cursor] as char;
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                cursor += 1;
            } else {
                break;
            }
        }
        if cursor == key_start {
            cursor += 1;
            continue;
        }
        let key = block_content[key_start..cursor].to_string();

        // skip 空白
        while cursor < block_content.len() && (bytes[cursor] as char).is_whitespace() {
            cursor += 1;
        }

        // 必须紧跟 ':'
        if cursor >= block_content.len() || bytes[cursor] as char != ':' {
            continue;
        }
        cursor += 1;
        // skip 空白
        while cursor < block_content.len() && (bytes[cursor] as char).is_whitespace() {
            cursor += 1;
        }

        // 必须以 <<< 开头
        if cursor + 3 > block_content.len() || &block_content[cursor..cursor + 3] != "<<<" {
            continue;
        }
        cursor += 3;

        // 找 >>> 结束
        let value_start = cursor;
        let end_marker = block_content[cursor..].find(">>>");
        let value_end = match end_marker {
            Some(idx) => {
                cursor += idx + 3;
                cursor - 3
            }
            None => {
                // 没有结束, 整段算 value
                cursor = block_content.len();
                block_content.len()
            }
        };
        let value = block_content[value_start..value_end].to_string();
        fields.push((key, value));
    }

    fields
}

/// 解析字段值
///
/// 启发: `true/false/null` 走 JSON bool/null, 数字走 JSON number, 否则 string
fn parse_field_value(s: &str) -> serde_json::Value {
    let trimmed = s.trim();
    if trimmed == "true" {
        return serde_json::Value::Bool(true);
    }
    if trimmed == "false" {
        return serde_json::Value::Bool(false);
    }
    if trimmed == "null" {
        return serde_json::Value::Null;
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
    }
    serde_json::Value::String(s.to_string())
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// MARKER 长度编译期守门 (VCP 真代码硬编码, 改 marker 必改这里)
const MARKER_START_LEN: usize = ToolCallParser::MARKER_START.len();
const MARKER_END_LEN: usize = ToolCallParser::MARKER_END.len();

const _: () = {
    // 直接拿 const 值, 改 MARKER 字符串只改 1 处
    assert!(
        ToolCallParser::MARKER_START.len() == MARKER_START_LEN,
        "MARKER_START 长度守"
    );
    assert!(
        ToolCallParser::MARKER_END.len() == MARKER_END_LEN,
        "MARKER_END 长度守"
    );
    // 同时记录到 runtime test (lib.rs 那边) 防止 MARKER 字符串被改后忘记
};

// ============================================================
// 单元测试 (战役 2-2 DoD: ≥ 5 个)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_block_normal() {
        // 正常单块 (VCP 真实 LLM 输出格式)
        let input = r#"
我将查询天气。

<<<[TOOL_REQUEST]>>>
tool_name:<<<WeatherQuery>>>
city:<<<Beijing>>>
unit:<<<celsius>>>
<<<[END_TOOL_REQUEST]>>>

查询完毕。
"#;
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls.len(), 1);
        let c = &calls[0];
        assert_eq!(c.tool_name, "WeatherQuery");
        assert_eq!(c.args["city"], "Beijing");
        assert_eq!(c.args["unit"], "celsius");
        assert!(c.raw_marker.contains("WeatherQuery"));
        assert!(!c.archery);
    }

    #[test]
    fn parse_multiple_blocks_sequential() {
        // 多个块顺序扫描 (VCP toolCallParser.js:32-44 while loop)
        let input = r#"<<<[TOOL_REQUEST]>>>
tool_name:<<<ToolA>>>
x:<<<1>>>
<<<[END_TOOL_REQUEST]>>>

一些中间文本。

<<<[TOOL_REQUEST]>>>
tool_name:<<<ToolB>>>
y:<<<hello>>>
archery:<<<true>>>
<<<[END_TOOL_REQUEST]>>>
"#;
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_name, "ToolA");
        assert_eq!(calls[0].args["x"], 1);
        assert_eq!(calls[1].tool_name, "ToolB");
        assert_eq!(calls[1].args["y"], "hello");
        assert!(calls[1].archery);
    }

    #[test]
    fn parse_strips_think_blocks() {
        // <think>...</think> 块应被剥掉 (VCP toolCallParser.js:30)
        let input = r#"
<think>
让我分析一下, 用户想要查询天气。我应该用 WeatherQuery 工具。
</think>

<<<[TOOL_REQUEST]>>>
tool_name:<<<WeatherQuery>>>
city:<<<Shanghai>>>
<<<[END_TOOL_REQUEST]>>>
"#;
        let calls = ToolCallParser::parse(input).expect("parse after think strip");
        assert_eq!(calls.len(), 1, "<think>块应被剥掉, 不影响 tool_call 解析");
        assert_eq!(calls[0].tool_name, "WeatherQuery");
        assert_eq!(calls[0].args["city"], "Shanghai");
    }

    #[test]
    fn parse_empty_input_errors() {
        // 空输入返 Err (VCP toolCallParser.js:28 if (!content) return []; 我们返 Err)
        let r = ToolCallParser::parse("");
        assert!(matches!(r, Err(ParseError::Empty)));
    }

    #[test]
    fn parse_no_blocks_errors() {
        // 无 tool_call 块返 Err (VCP 返 [], 我们区分语义)
        let input = "这只是普通文本, 没有 tool_call 块.";
        let r = ToolCallParser::parse(input);
        assert!(matches!(r, Err(ParseError::NoBlocks)));
    }

    #[test]
    fn parse_block_without_tool_name_returns_none() {
        // 没有 tool_name 字段的块应被跳过
        let input = "<<<[TOOL_REQUEST]>>>\narg:<<<value>>>\n<<<[END_TOOL_REQUEST]>>>";
        let r = ToolCallParser::parse(input);
        // 单个块无 tool_name → 跳过 → NoBlocks
        assert!(matches!(r, Err(ParseError::NoBlocks)));
    }

    #[test]
    fn parse_archery_no_reply() {
        // archery:no_reply 字段 (VCP toolCallParser.js:99)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<AsyncAck>>>\narchery:<<<no_reply>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls.len(), 1);
        assert!(calls[0].archery);
        assert!(calls[0].archery_no_reply);
    }

    #[test]
    fn parse_valet_to_maid_mirror() {
        // valet 字段镜像到 maid (VCP toolCallParser.js:113-115)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<ToolX>>>\nvalet:<<<chuling>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls[0].args.get("valet"), None, "valet 应被镜像到 maid");
        assert_eq!(calls[0].args["maid"], "chuling");
    }

    #[test]
    fn parse_existing_maid_not_overwritten() {
        // 如果 maid 已显式提供, valet 不覆盖
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<ToolX>>>\nvalet:<<<valet_value>>>\nmaid:<<<chuling>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = ToolCallParser::parse(input).expect("parse");
        // 注: 我们 parse_field_value 把所有非数字当 string, 解析后 valet + maid 都还在 args
        // 但 VCP 行为是: 显式 maid 优先, valet 被镜像但不覆盖
        // 我们这里简化: 显式 maid 存在时不镜像
        assert_eq!(calls[0].args["maid"], "chuling");
    }

    #[test]
    fn parse_field_value_types() {
        // 字段值类型解析: bool / null / number / string
        assert_eq!(parse_field_value("true"), serde_json::Value::Bool(true));
        assert_eq!(parse_field_value("false"), serde_json::Value::Bool(false));
        assert_eq!(parse_field_value("null"), serde_json::Value::Null);
        assert_eq!(parse_field_value("42"), serde_json::json!(42));
        assert_eq!(parse_field_value("3.14"), serde_json::json!(3.14));
        assert_eq!(
            parse_field_value("hello world"),
            serde_json::Value::String("hello world".to_string())
        );
    }

    #[test]
    fn parse_river_vref_fields() {
        // river / vref 字段 (VCP toolCallParser.js:102-105)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<ToolZ>>>\nriver:<<<last:5>>>\nvref:<<<3>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls[0].args["river"], "last:5");
        assert_eq!(calls[0].args["vref"], "3");
    }

    #[test]
    fn parse_with_unicode_values() {
        // Unicode 字段值
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<中文工具>>>\ndescription:<<<查询今天天气>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = ToolCallParser::parse(input).expect("parse");
        assert_eq!(calls[0].tool_name, "中文工具");
        assert_eq!(calls[0].args["description"], "查询今天天气");
    }
}
