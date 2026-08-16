//! **N10 — 宽松文本工具协议层 (VCP `vcpLoop/toolCallParser.js` 字段级移植)**
//!
//! **背景**: VCP vcpLoop 让模型用文本标记发起工具调用, 比严格 JSON 宽容.
//! 本模块是 tool-runtime 的协议增强层, 与既有严格 parser (`parser.rs`, ASCII
//! `<<<[TOOL_REQUEST]>>>` + `key:<<<value>>>`) **并存**, 不改动既有行为 (向后兼容).
//!
//! **五大机制** (字段级对照 `research/source/vcptoolbox/modules/vcpLoop/`):
//! 1. **始末语法解析** — 块: `<<<[TOOL_REQUEST]>>>` … `<<<[END_TOOL_REQUEST]>>>`;
//!    字段: `key:「始」value「末」` (对照 `toolCallParser.js:5-20` MARKERS + `_scanFields`)
//! 2. **ESCAPE 转义防注入** — `「始ESCAPE」…「末ESCAPE」` 字段内可安全携带字面量
//!    `<<<[END_TOOL_REQUEST]>>>` (块结束扫描跳过 escape 区, 对照 `_findBlockEnd`);
//!    `<<<[*_ESCAPE]>>>` 字面量映射还原 (对照 `ESCAPED_LITERAL_MAP`)
//! 3. **模糊标记匹配** — 块标记大小写/空白/尖括号数量容错
//!    (`<{2,4}\s*\[\s*TOOL_REQUEST\s*\]\s*>{2,4}`, 对照 `toolMarkerFuzzyMatcher.js:48`);
//!    字段标记 4 种括号变体 `「始」/{始}/{始」/「始}` (对照 `matchFieldStartMarker` 候选表)
//! 4. **archery 式解析与执行分离** — `separate()` 拆 normal / archery (fire-and-forget),
//!    异步分发由 `ToolExecutor::execute_separated` 承接 (对照 `toolExecutor.js` archery no-reply)
//! 5. **思考块剥离** — `<think>/<thinking>` (大小写/属性/嵌套容错), **未闭合开始标签
//!    丢弃其后全部内容** —— 防思考文本里潜藏的工具调用被误执行 (对照 `stripReasoningBlocks`)
//!
//! **不假装**:
//! - ✅ 真移植 5 机制, 每机制有正常 + 失败路径单测
//! - ✅ VCP `river/vref/valet` 语义保留: river/vref 透传 args, valet→maid 镜像
//! - ❌ 不实现 VCP `ink`(mark_history) 之外的 VCP 专属编排 (vref 语义检索属 toolExecutor 范畴)
//! - ❌ 不做运行期配置热加载 (fuzzy 常开; 严格路径走 `parser.rs::ToolCallParser`)

use std::sync::OnceLock;

use regex::Regex;

use crate::parser::{parse_field_value, ParsedToolCall};

// ============================================================
// 协议常量 (VCP toolCallParser.js:5-20 字段级)
// ============================================================

/// 块起始标记 canonical 形式 (VCP `MARKERS.START`)
pub const BLOCK_START_CANONICAL: &str = "<<<[TOOL_REQUEST]>>>";
/// 块结束标记 canonical 形式 (VCP `MARKERS.END`)
pub const BLOCK_END_CANONICAL: &str = "<<<[END_TOOL_REQUEST]>>>";

/// 字段值起始标记候选 (VCP `matchFieldStartMarker` strictCandidates, 模糊常开)
pub const FIELD_START_CANDIDATES: &[&str] = &["「始」", "{始}", "{始」", "「始}"];
/// 字段值结束标记候选 (VCP `findFieldEndMarker` strictCandidates, 模糊常开)
pub const FIELD_END_CANDIDATES: &[&str] = &["「末」", "{末}", "{末」", "「末}"];

// ============================================================
// 正则 (惰性构建一次, 全模块共享)
// ============================================================

/// 块起始模糊正则: 2-4 个尖括号 + 括号/空白容错 + 大小写不敏感
fn block_start_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)<{2,4}\s*\[\s*TOOL_REQUEST\s*\]\s*>{2,4}").expect("block start regex")
    })
}

/// 块结束模糊正则
fn block_end_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)<{2,4}\s*\[\s*END_TOOL_REQUEST\s*\]\s*>{2,4}").expect("block end regex")
    })
}

/// ESCAPE 字段开始标记 (`「始ESCAPE」` 及括号变体, 大小写不敏感)
fn escape_start_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)[「{]始ESCAPE[」}]").expect("escape start regex"))
}

/// ESCAPE 字段结束标记
fn escape_end_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)[「{]末ESCAPE[」}]").expect("escape end regex"))
}

/// 思考标签 (VCP stripReasoningBlocks: `<\s*(\/?)\s*(think(?:ing)?)\b[^>]*>`, ci)
fn think_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)<\s*(/?)\s*(think(?:ing)?)\b[^>]*>").expect("think tag regex")
    })
}

// ============================================================
// 思考块剥离 (机制 ⑤)
// ============================================================

/// 从可见正文中剥离模型思考块 (VCP `stripReasoningBlocks` 字段级移植)
///
/// 支持: `<think>…</think>` / `<thinking>…</thinking>` / 大小写 / 标签内空白与属性 /
/// 同类或混合标签嵌套 / **未闭合开始标签 → 丢弃其后全部内容** (防潜藏工具调用被误执行).
/// 未配对的结束标签只移除标签本身, 不吞掉其后可见正文.
pub fn strip_reasoning_blocks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize;
    let mut depth = 0usize;

    for caps in think_tag_re().captures_iter(content) {
        let m = caps.get(0).expect("whole match");
        if depth == 0 {
            out.push_str(&content[cursor..m.start()]);
        }
        let is_closing = caps.get(1).map(|g| g.as_str() == "/").unwrap_or(false);
        if is_closing {
            depth = depth.saturating_sub(1);
        } else {
            depth += 1;
        }
        cursor = m.end();
    }

    // depth > 0: 思考标签未闭合 — 保守丢弃最后一个开始标签之后的全部内容,
    // 避免思考文本里潜藏的工具调用被执行 (VCP toolCallParser.js:60-64)
    if depth == 0 {
        out.push_str(&content[cursor..]);
    }
    out
}

// ============================================================
// ESCAPE 字面量还原 (机制 ②)
// ============================================================

/// 还原 escape 字段内的转义字面量 (VCP `ESCAPED_LITERAL_MAP` + `_restoreEscapedLiterals`)
///
/// - `<<<[TOOL_REQUEST_ESCAPE]>>>` → `<<<[TOOL_REQUEST]>>>`
/// - `<<<[END_TOOL_REQUEST_ESCAPE]>>>` → `<<<[END_TOOL_REQUEST]>>>`
/// - `[「{]始ESCAPE[」}]` → `「始」`; `[「{]末ESCAPE[」}]` → `「末」`
pub fn restore_escaped_literals(content: &str) -> String {
    let mut out = content
        .replace("<<<[TOOL_REQUEST_ESCAPE]>>>", BLOCK_START_CANONICAL)
        .replace("<<<[END_TOOL_REQUEST_ESCAPE]>>>", BLOCK_END_CANONICAL);
    out = escape_start_re().replace_all(&out, "「始」").into_owned();
    out = escape_end_re().replace_all(&out, "「末」").into_owned();
    out
}

// ============================================================
// 块提取 (机制 ① + ② + ③)
// ============================================================

struct BlockInfo {
    content: String,
    next_offset: usize,
    /// 实际匹配到的块起始标记原文 (可能是模糊变体, 存 raw_marker 供追溯)
    start_marker: String,
}

/// 从 `from` 起找下一个块结束标记, 跳过 ESCAPE 区域 (VCP `_findBlockEnd` 字段级)
///
/// 返回结束标记的绝对 (start, end). ESCAPE 开始无对应结束 → None (畸形块).
fn find_block_end(content: &str, from: usize) -> Option<(usize, usize)> {
    let mut cursor = from;
    loop {
        let end_m = block_end_re().find(&content[cursor..])?;
        let end_abs = cursor + end_m.start();

        let esc_m = escape_start_re().find(&content[cursor..]);
        match esc_m {
            // 无 escape, 或结束标记在 escape 之前 → 该结束标记有效
            None => return Some((end_abs, cursor + end_m.end())),
            Some(e) if cursor + e.start() >= end_abs => {
                return Some((end_abs, cursor + end_m.end()));
            }
            Some(e) => {
                // 跳过 「始ESCAPE」…「末ESCAPE」 区域 (内含字面量结束标记不终止块)
                let after = cursor + e.end();
                let esc_end = escape_end_re().find(&content[after..])?;
                cursor = after + esc_end.end();
            }
        }
    }
}

/// 提取从 `from` 起的下一个工具块 (VCP `extractNextToolBlock` 字段级)
fn extract_next_tool_block(content: &str, from: usize) -> Option<BlockInfo> {
    let start_m = block_start_re().find(&content[from..])?;
    let abs_start = from + start_m.start();
    let block_start = from + start_m.end();

    let (end_start, end_end) = find_block_end(content, block_start)?;

    Some(BlockInfo {
        content: content[block_start..end_start].trim().to_string(),
        next_offset: end_end,
        start_marker: content[abs_start..block_start].to_string(),
    })
}

// ============================================================
// 字段扫描 (机制 ①+②+③)
// ============================================================

fn skip_ws_commas(s: &str, mut idx: usize) -> usize {
    while idx < s.len() {
        match s[idx..].chars().next() {
            Some(c) if c.is_whitespace() || c == ',' => idx += c.len_utf8(),
            _ => break,
        }
    }
    idx
}

fn skip_ws(s: &str, mut idx: usize) -> usize {
    while idx < s.len() {
        match s[idx..].chars().next() {
            Some(c) if c.is_whitespace() => idx += c.len_utf8(),
            _ => break,
        }
    }
    idx
}

/// 在 `[from, end)` 内找最先出现的字段结束标记, 返回 (绝对 idx, 标记长度)
fn find_field_end(block: &str, from: usize, end: usize) -> Option<(usize, usize)> {
    let slice = &block[from..end];
    let mut best: Option<(usize, usize)> = None;
    for cand in FIELD_END_CANDIDATES {
        if let Some(i) = slice.find(cand) {
            let abs = from + i;
            if best.map(|(bi, _)| abs < bi).unwrap_or(true) {
                best = Some((abs, cand.len()));
            }
        }
    }
    best
}

/// 字段扫描 (VCP `_scanFields` 字段级移植, 三种字段值形态)
///
/// 形态: ① ESCAPE 字段 `「始ESCAPE」value「末ESCAPE」` (值内可含字面量标记, 还原转义)
/// ② 「始」系字段 `key:「始」value「末」` (含 `{始}` 等括号变体)
/// ③ ASCII 兼容 `key:<<<value>>>` (与既有严格 parser 语法兼容, 宽松层做超集)
fn scan_fields(block: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut cursor = 0usize;

    while cursor < block.len() {
        cursor = skip_ws_commas(block, cursor);
        if cursor >= block.len() {
            break;
        }

        // 读 key (ASCII alnum / _ / -), VCP `[\w_]+`
        let key_start = cursor;
        while cursor < block.len() {
            match block[cursor..].chars().next() {
                Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {
                    cursor += c.len_utf8()
                }
                _ => break,
            }
        }
        if cursor == key_start {
            // 非 key 字符 → 前进一个字符, 防死循环
            cursor += block[cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            continue;
        }
        let key = block[key_start..cursor].to_string();

        cursor = skip_ws(block, cursor);
        if !block[cursor..].starts_with(':') {
            continue;
        }
        cursor += 1;
        cursor = skip_ws(block, cursor);

        let rest = &block[cursor..];

        // 形态 ①: ESCAPE 字段 (VCP getEscapeStartRegex anchored — 必须锚定在值起点)
        // 注: find 可能匹配到后续字段的 ESCAPE 标记, 故只有 start()==0 才走 escape 分支
        let anchored_escape = escape_start_re().find(rest).filter(|e| e.start() == 0);
        if let Some(e) = anchored_escape {
            let after = cursor + e.end();
            match escape_end_re().find(&block[after..]) {
                Some(end_m) => {
                    let raw = &block[after..after + end_m.start()];
                    fields.push((key.clone(), restore_escaped_literals(raw)));
                    cursor = after + end_m.end();
                    continue;
                }
                None => break, // ESCAPE 未闭合 → 块畸形 (VCP: endIndex === -1 → break)
            }
        }

        // 形态 ②: 「始」系字段 (VCP matchFieldStartMarker strictCandidates)
        if let Some(start_marker) = FIELD_START_CANDIDATES
            .iter()
            .find(|c| rest.starts_with(**c))
        {
            cursor += start_marker.len();
            match find_field_end(block, cursor, block.len()) {
                Some((end_idx, end_len)) => {
                    let raw = &block[cursor..end_idx];
                    fields.push((key.clone(), raw.to_string()));
                    cursor = end_idx + end_len;
                    continue;
                }
                None => break, // 无结束标记 → break (VCP 行为)
            }
        }

        // 形态 ③: ASCII 兼容 `<<<value>>>` (与既有 parser.rs 语法兼容)
        if rest.starts_with("<<<") {
            cursor += 3;
            match block[cursor..].find(">>>") {
                Some(i) => {
                    let raw = &block[cursor..cursor + i];
                    fields.push((key.clone(), raw.to_string()));
                    cursor += i + 3;
                    continue;
                }
                None => break,
            }
        }

        // 无匹配形态 → 前进一个字符 (VCP: continue; 防死循环)
        cursor += block[cursor..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
    }

    fields
}

// ============================================================
// 块解析 + 入口
// ============================================================

/// 解析单个工具块 (VCP `parseBlock` 字段级, 供人类直调等入口复用)
pub fn parse_block(block_content: &str, start_marker: &str) -> Option<ParsedToolCall> {
    let fields = scan_fields(block_content);
    if fields.is_empty() {
        return None;
    }

    let mut tool_name: Option<String> = None;
    let mut archery = false;
    let mut archery_no_reply = false;
    let mut args = serde_json::Map::new();

    for (key, value) in fields {
        let value = value.trim().to_string();
        if key == "tool_name" {
            tool_name = Some(value);
        } else if key == "archery" {
            archery = value == "true" || value == "no_reply";
            archery_no_reply = value == "no_reply";
        } else {
            // river / vref / 普通参数一律入 args (VCP parseBlock 分支; 类型启发同严格 parser)
            args.insert(key, parse_field_value(&value));
        }
    }

    let tool_name = tool_name?;
    // valet → maid 镜像 (VCP toolCallParser.js:158-162): 显式 maid 优先
    if args.contains_key("valet") && !args.contains_key("maid") {
        if let Some(v) = args.remove("valet") {
            args.insert("maid".into(), v);
        }
    }

    Some(ParsedToolCall {
        tool_name,
        args: serde_json::Value::Object(args),
        raw_marker: start_marker.to_string(),
        archery,
        archery_no_reply,
    })
}

/// **宽松协议解析结果分流** (VCP `separate`, archery 式分离的解析侧)
#[derive(Debug, Clone, Default)]
pub struct SeparatedCalls {
    /// 普通调用: 同步等待, 结果回灌
    pub normal: Vec<ParsedToolCall>,
    /// archery 调用: fire-and-forget 异步分发 (no_reply = 结果永不回灌)
    pub archery: Vec<ParsedToolCall>,
}

/// **N10 宽松文本工具协议层入口**
///
/// 与严格 `parser.rs::ToolCallParser::parse` 并存; 宽松层永不 Err
/// (畸形块静默跳过, 对齐 VCP `parse` 返回数组语义).
pub struct TextToolProtocol;

impl TextToolProtocol {
    /// 块起始 canonical 标记 (常量再导出, 便于调用方拼提示词)
    pub const MARKER_START: &'static str = BLOCK_START_CANONICAL;
    /// 块结束 canonical 标记
    pub const MARKER_END: &'static str = BLOCK_END_CANONICAL;

    /// 解析 LLM 输出中的所有工具调用 (宽松协议)
    ///
    /// 流程 (VCP `parse` 字段级): 剥思考块 → 顺序提取块 → 逐块 parseBlock,
    /// 失败跳过; 空输入/无块 → 空 Vec.
    pub fn parse(content: &str) -> Vec<ParsedToolCall> {
        let visible = strip_reasoning_blocks(content);
        let mut calls = Vec::new();
        let mut offset = 0usize;
        while offset < visible.len() {
            let block = match extract_next_tool_block(&visible, offset) {
                Some(b) => b,
                None => break,
            };
            if let Some(parsed) = parse_block(&block.content, &block.start_marker) {
                calls.push(parsed);
            }
            offset = block.next_offset;
        }
        calls
    }

    /// 分流 normal / archery (VCP `separate` 字段级)
    pub fn separate(calls: &[ParsedToolCall]) -> SeparatedCalls {
        let mut sep = SeparatedCalls::default();
        for c in calls {
            if c.archery {
                sep.archery.push(c.clone());
            } else {
                sep.normal.push(c.clone());
            }
        }
        sep
    }
}

// ============================================================
// 单元测试 (N10 验收: 正常/畸形/ESCAPE/模糊匹配/思考块剥离失败路径)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- ① 始末语法: 正常路径 ----------

    #[test]
    fn canonical_block_with_shi_mo_fields() {
        let input = "好的, 我来查.\n<<<[TOOL_REQUEST]>>>\ntool_name:「始」DailyNoteSearcher「末」\nquery:「始」明天日程「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "DailyNoteSearcher");
        assert_eq!(calls[0].args["query"], "明天日程");
        assert_eq!(calls[0].raw_marker, BLOCK_START_CANONICAL);
    }

    #[test]
    fn multiple_blocks_parsed_in_order() {
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」A「末」\n<<<[END_TOOL_REQUEST]>>>\n中段文本\n<<<[TOOL_REQUEST]>>>\ntool_name:「始」B「末」\nx:「始」1「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_name, "A");
        assert_eq!(calls[1].tool_name, "B");
        assert_eq!(calls[1].args["x"], serde_json::json!(1));
    }

    // ---------- ③ 模糊标记匹配 ----------

    #[test]
    fn fuzzy_block_marker_case_whitespace_brackets() {
        // 小写 + 标签内空白 + 4 尖括号变体
        let input = "<<<[ tool_request ]>>>\ntool_name:「始」T1「末」\n<<<[ end_tool_request ]>>>\n<<<<[TOOL_REQUEST]>>>>\ntool_name:「始」T2「末」\n<<<<[END_TOOL_REQUEST]>>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_name, "T1");
        assert_eq!(calls[1].tool_name, "T2");
        // raw_marker 记录实际模糊变体
        assert_ne!(calls[0].raw_marker, BLOCK_START_CANONICAL);
    }

    #[test]
    fn fuzzy_field_bracket_variants() {
        // {始}/{末} 与混合括号变体
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:{始}T3{末}\na:{始」va「末}\nb:「始}vb{末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "T3");
        assert_eq!(calls[0].args["a"], "va");
        assert_eq!(calls[0].args["b"], "vb");
    }

    #[test]
    fn ascii_field_compat_with_existing_syntax() {
        // 形态③: ASCII <<<value>>> 与既有严格 parser 语法兼容 (宽松层超集)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<EchoAPI>>>\ninput:<<<hello>>>\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "EchoAPI");
        assert_eq!(calls[0].args["input"], "hello");
    }

    // ---------- ② ESCAPE 转义防注入 ----------

    #[test]
    fn escape_field_swallows_literal_end_marker() {
        // 值内携带字面量结束标记 → 不得终止块 (防注入核心用例)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」FileOperator「末」\ncontent:「始ESCAPE」文档里写着 <<<[END_TOOL_REQUEST]>>> 这只是文本「末ESCAPE」\nafter:「始」ok「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        let content = calls[0].args["content"].as_str().expect("content");
        assert!(content.contains("<<<[END_TOOL_REQUEST]>>>"));
        assert_eq!(calls[0].args["after"], "ok");
    }

    #[test]
    fn escape_literal_map_restores_markers() {
        // ESCAPED_LITERAL_MAP: *_ESCAPE 拼写 → 字面量标记
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」\ndoc:「始ESCAPE」示例: <<<[TOOL_REQUEST_ESCAPE]>>> 和 <<<[END_TOOL_REQUEST_ESCAPE]>>>「末ESCAPE」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        let doc = calls[0].args["doc"].as_str().expect("doc");
        assert!(doc.contains(BLOCK_START_CANONICAL));
        assert!(doc.contains(BLOCK_END_CANONICAL));
        assert!(!doc.contains("ESCAPE"));
    }

    #[test]
    fn restore_escaped_literals_unit() {
        assert_eq!(
            restore_escaped_literals("「始ESCAPE」x「末ESCAPE」"),
            "「始」x「末」"
        );
        assert_eq!(
            restore_escaped_literals("{始escape}y{末escape}"),
            "「始」y「末」"
        );
        assert_eq!(restore_escaped_literals("plain"), "plain");
    }

    #[test]
    fn unclosed_escape_field_makes_block_malformed() {
        // ESCAPE 开始无结束 → 整块畸形 → 静默跳过 (失败路径)
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」\ncontent:「始ESCAPE」没有结束标记\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert!(calls.is_empty());
    }

    // ---------- ⑤ 思考块剥离 ----------

    #[test]
    fn strip_think_and_thinking_with_attrs() {
        let input = "<think>内部A</think>可见1<THINKING foo=\"bar\">内部B</thinking >可见2";
        assert_eq!(strip_reasoning_blocks(input), "可见1可见2");
    }

    #[test]
    fn strip_nested_think_blocks() {
        let input = "前<think>外<thinking>内</thinking>外2</think>后";
        assert_eq!(strip_reasoning_blocks(input), "前后");
    }

    #[test]
    fn unclosed_think_discards_tail_hiding_tool_call() {
        // 安全失败路径: 未闭合思考块里潜藏的工具调用不得被执行
        let input = "先想一下<think>推理...<<<[TOOL_REQUEST]>>>\ntool_name:「始」EvilTool「末」\n<<<[END_TOOL_REQUEST]>>>";
        assert!(TextToolProtocol::parse(input).is_empty());
    }

    #[test]
    fn unmatched_close_tag_only_removes_itself() {
        // 孤立结束标签只删标签本身, 其后正文保留
        let input =
            "前</think>后<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(strip_reasoning_blocks("前</think>后"), "前后");
    }

    #[test]
    fn tool_call_inside_closed_think_not_executed() {
        let input = "<thinking>我应该调用 <<<[TOOL_REQUEST]>>>\ntool_name:「始」X「末」\n<<<[END_TOOL_REQUEST]>>></thinking>最终答复";
        let calls = TextToolProtocol::parse(input);
        assert!(calls.is_empty());
    }

    // ---------- 畸形输入失败路径 ----------

    #[test]
    fn empty_or_no_markers_yields_empty() {
        assert!(TextToolProtocol::parse("").is_empty());
        assert!(TextToolProtocol::parse("只有普通文本").is_empty());
    }

    #[test]
    fn start_without_end_yields_empty() {
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」";
        assert!(TextToolProtocol::parse(input).is_empty());
    }

    #[test]
    fn block_missing_tool_name_skipped_but_next_parses() {
        let input = "<<<[TOOL_REQUEST]>>>\nfoo:「始」1「末」\n<<<[END_TOOL_REQUEST]>>>\n<<<[TOOL_REQUEST]>>>\ntool_name:「始」OK「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "OK");
    }

    // ---------- ④ archery 分流 + VCP 字段兼容 ----------

    #[test]
    fn separate_normal_and_archery() {
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」NormalTool「末」\n<<<[END_TOOL_REQUEST]>>>\n<<<[TOOL_REQUEST]>>>\ntool_name:「始」BgTool「末」\narchery:「始」true「末」\n<<<[END_TOOL_REQUEST]>>>\n<<<[TOOL_REQUEST]>>>\ntool_name:「始」SilentTool「末」\narchery:「始」no_reply「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls.len(), 3);
        let sep = TextToolProtocol::separate(&calls);
        assert_eq!(sep.normal.len(), 1);
        assert_eq!(sep.normal[0].tool_name, "NormalTool");
        assert_eq!(sep.archery.len(), 2);
        assert!(!sep.archery[0].archery_no_reply);
        assert!(sep.archery[1].archery_no_reply);
    }

    #[test]
    fn valet_mirrors_to_maid_unless_explicit() {
        let input = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」\nvalet:「始」v1「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls = TextToolProtocol::parse(input);
        assert_eq!(calls[0].args["maid"], "v1");
        assert!(calls[0].args.get("valet").is_none() || calls[0].args["maid"] == "v1");

        let input2 = "<<<[TOOL_REQUEST]>>>\ntool_name:「始」T「末」\nvalet:「始」v1「末」\nmaid:「始」m1「末」\n<<<[END_TOOL_REQUEST]>>>";
        let calls2 = TextToolProtocol::parse(input2);
        assert_eq!(calls2[0].args["maid"], "m1");
    }

    #[test]
    fn parse_block_direct_entry_reusable() {
        // parseBlock 供人类直调等入口复用 (VCP parseBlock 独立导出语义)
        let parsed = parse_block("tool_name:「始」Direct「末」", BLOCK_START_CANONICAL);
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().tool_name, "Direct");
        assert!(parse_block("", BLOCK_START_CANONICAL).is_none());
    }
}
