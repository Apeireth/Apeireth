//! `re.yara`：YARA 子集规则引擎（零新依赖）。
//!
//! 支持：
//! * `rule Name { strings: $a = "text"  $b = { 4D 5A ?? } condition: $a and not $b }`
//! * 字符串（引号）、十六进制（含 `??` 与半字节通配、`[n]` / `[n-m]` / `[n-]`
//!   跳转、`( ... | ... )` 单层分支）、布尔条件
//!   （`and` / `or` / `not` / 括号 / `any of them` / `all of them` / `true`）。
//!
//! 不支持：正则、模块（pe/elf/hash…）、整数表达式、外部变量、嵌套十六进制分支。
//! 这些差距写入 [`YaraReport::limitations`]，不冒充完整 YARA。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// 一条命中的规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleMatch {
    /// 规则名。
    pub rule: String,
    /// 命中的字符串标识（`$a` 等）。
    pub strings: Vec<String>,
    /// 命中偏移（去重、升序、上限保护）。
    pub offsets: Vec<u64>,
}

/// 一次扫描结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YaraReport {
    /// 规则数量。
    pub rules: usize,
    /// 命中列表。
    pub matches: Vec<RuleMatch>,
    /// 扫描字节数。
    pub scanned_bytes: usize,
    /// 显式局限（子集能力边界）。
    pub limitations: Vec<String>,
}

/// 解析/匹配错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum YaraError {
    /// 没有规则。
    #[error("no rule found")]
    NoRule,
    /// 规则体缺失。
    #[error("rule {0:?} has no body")]
    MissingBody(String),
    /// 条件缺失。
    #[error("rule {0:?} has no condition")]
    MissingCondition(String),
    /// 字符串定义畸形。
    #[error("malformed string definition {0:?}")]
    BadString(String),
    /// 十六进制模式畸形。
    #[error("malformed hex pattern {0:?}")]
    BadHex(String),
    /// 条件表达式畸形。
    #[error("malformed condition: {0}")]
    BadCondition(String),
    /// 引用了未定义的字符串。
    #[error("unknown string reference {0:?}")]
    UnknownReference(String),
}

const MAX_MATCHES_PER_STRING: usize = 256;
/// 十六进制跳转的最大跨度；`[n-]` 的无上限按此封顶，避免最坏情况爆炸。
const MAX_HEX_JUMP: usize = 4096;

/// 编译后的规则集。
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    /// 解析规则源码。
    pub fn parse(source: &str) -> Result<Self, YaraError> {
        let cleaned = strip_comments(source);
        let mut rules = Vec::new();
        let mut cursor = 0usize;
        while let Some(position) = find_keyword(&cleaned, "rule", cursor) {
            cursor = position + 4;
            let rest_raw = &cleaned[cursor..];
            let skip = rest_raw.len() - rest_raw.trim_start().len();
            let rest = &rest_raw[skip..];
            let Some(name) = read_ident(rest) else {
                return Err(YaraError::MissingBody("<anonymous>".to_string()));
            };
            let after_name = cursor + skip + name.len();
            let Some(body_start) = cleaned[after_name..].find('{') else {
                return Err(YaraError::MissingBody(name));
            };
            let body_start = after_name + body_start;
            let body_end = find_matching_brace(&cleaned, body_start)
                .ok_or_else(|| YaraError::MissingBody(name.clone()))?;
            let body = &cleaned[body_start + 1..body_end];
            let strings = parse_strings(body)?;
            let condition = parse_condition(body, &strings)?;
            rules.push(Rule {
                name,
                strings,
                condition,
            });
            cursor = body_end + 1;
        }
        if rules.is_empty() {
            return Err(YaraError::NoRule);
        }
        Ok(Self { rules })
    }

    /// 规则数量。
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 扫描一段字节。
    pub fn scan(&self, bytes: &[u8]) -> YaraReport {
        let mut matches = Vec::new();
        for rule in &self.rules {
            let mut hit_ids = Vec::new();
            let mut offsets = BTreeSet::new();
            let mut hits: BTreeMap<String, Vec<u64>> = BTreeMap::new();
            for string in &rule.strings {
                let found = match string {
                    RuleString::Text {
                        bytes: needle,
                        modifiers,
                        ..
                    } => find_text(bytes, needle, *modifiers),
                    RuleString::Hex { pattern, .. } => find_hex(bytes, pattern),
                    RuleString::Regex {
                        regex, modifiers, ..
                    } => regex.find(
                        bytes,
                        modifiers.nocase,
                        modifiers.fullword,
                        MAX_MATCHES_PER_STRING,
                    ),
                };
                if !found.is_empty() {
                    hit_ids.push(string.id().to_string());
                    offsets.extend(found.iter().copied());
                }
                hits.insert(string.id().to_string(), found);
            }
            let state = ScanState { data: bytes, hits };
            if eval_bool(&rule.condition, &state) {
                matches.push(RuleMatch {
                    rule: rule.name.clone(),
                    strings: hit_ids,
                    offsets: offsets.into_iter().collect(),
                });
            }
        }
        YaraReport {
            rules: self.rules.len(),
            matches,
            scanned_bytes: bytes.len(),
            limitations: vec![
                "YARA subset: byte-level regex + pe/elf/hash module subset; no external variables"
                    .to_string(),
                "hex patterns support byte/nibble wildcards, [n]/[n-m]/[n-] jumps and one-level ( a | b ) alternations; nested alternations are rejected"
                    .to_string(),
                "hex jumps are capped at 4096 bytes".to_string(),
                "private/global rules are not supported".to_string(),
            ],
        }
    }
}

struct Rule {
    name: String,
    strings: Vec<RuleString>,
    condition: Expr,
}

enum RuleString {
    Text {
        id: String,
        bytes: Vec<u8>,
        modifiers: TextModifiers,
    },
    Hex {
        id: String,
        pattern: HexPattern,
    },
    Regex {
        id: String,
        regex: crate::recon::regex::Regex,
        modifiers: TextModifiers,
    },
}

/// 编译后的十六进制模式：字节/通配、跳转与单层分支组成的 NFA。
struct HexPattern {
    /// 节点表；索引 0 固定为 `Match`。
    nodes: Vec<HexNode>,
    /// 模式入口节点。
    entry: usize,
}

/// 十六进制模式的 NFA 节点。
enum HexNode {
    /// 一个字节（可带半字节掩码）。
    Byte { mask: u8, value: u8, next: usize },
    /// 跳过 `min..=max` 个字节。
    Jump { min: usize, max: usize, next: usize },
    /// 分支入口：每个备选子序列一个入口。
    Split { alternatives: Vec<usize> },
    /// 模式结束。
    Match,
}

/// 解析阶段的十六进制原子；分支内部只允许字节与跳转。
enum HexAtom {
    Byte { mask: u8, value: u8 },
    Jump { min: usize, max: usize },
    Alt(Vec<Vec<HexAtom>>),
}

impl HexPattern {
    /// 把原子序列编译为后向拼接的 NFA；`Match` 固定在 0 号节点。
    fn compile(atoms: &[HexAtom]) -> Self {
        let mut nodes = vec![HexNode::Match];
        let entry = compile_hex_sequence(atoms, 0, &mut nodes);
        Self { nodes, entry }
    }
}

/// 逆序编译一段原子序列，返回其入口节点；`next` 是序列结束后的续接节点。
fn compile_hex_sequence(atoms: &[HexAtom], next: usize, nodes: &mut Vec<HexNode>) -> usize {
    let mut next = next;
    for atom in atoms.iter().rev() {
        let node = match atom {
            HexAtom::Byte { mask, value } => HexNode::Byte {
                mask: *mask,
                value: *value,
                next,
            },
            HexAtom::Jump { min, max } => HexNode::Jump {
                min: *min,
                max: *max,
                next,
            },
            HexAtom::Alt(branches) => HexNode::Split {
                alternatives: branches
                    .iter()
                    .map(|branch| compile_hex_sequence(branch, next, nodes))
                    .collect(),
            },
        };
        let index = nodes.len();
        nodes.push(node);
        next = index;
    }
    next
}

/// 文本字符串修饰符（YARA 子集）。
#[derive(Debug, Clone, Copy, Default)]
struct TextModifiers {
    /// 忽略 ASCII 大小写。
    nocase: bool,
    /// 同时匹配 UTF-16LE 形态。
    wide: bool,
    /// 显式要求 ASCII 形态（与 `wide` 同用时两者都匹配）。
    ascii: bool,
    /// 要求前后是非单词字符。
    fullword: bool,
    /// XOR 键范围（`xor` = 0..=255，`xor(n)` / `xor(n-m)`）。
    xor: Option<(u8, u8)>,
}

impl RuleString {
    fn id(&self) -> &str {
        match self {
            Self::Text { id, .. } | Self::Hex { id, .. } | Self::Regex { id, .. } => id,
        }
    }
}

enum Expr {
    Reference(String),
    AnyOfThem,
    AllOfThem,
    True,
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Count(String),
    Offset(String, usize),
    Int(u64),
    Filesize,
    Uint {
        width: u8,
        big_endian: bool,
        offset: Box<Expr>,
    },
    Compare {
        left: Box<Expr>,
        op: Cmp,
        right: Box<Expr>,
    },
    At {
        id: String,
        offset: Box<Expr>,
    },
    In {
        id: String,
        low: Box<Expr>,
        high: Box<Expr>,
    },
    Literal(String),
    Module {
        module: String,
        field: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

struct ScanState<'a> {
    data: &'a [u8],
    hits: BTreeMap<String, Vec<u64>>,
}

fn eval_bool(expr: &Expr, state: &ScanState<'_>) -> bool {
    match expr {
        Expr::Reference(id) => state.hits.get(id).is_some_and(|hits| !hits.is_empty()),
        Expr::AnyOfThem => state.hits.values().any(|hits| !hits.is_empty()),
        Expr::AllOfThem => {
            !state.hits.is_empty() && state.hits.values().all(|hits| !hits.is_empty())
        }
        Expr::True => true,
        Expr::And(left, right) => eval_bool(left, state) && eval_bool(right, state),
        Expr::Or(left, right) => eval_bool(left, state) || eval_bool(right, state),
        Expr::Not(inner) => !eval_bool(inner, state),
        Expr::At { id, offset } => {
            let Some(value) = eval_int(offset, state) else {
                return false;
            };
            state.hits.get(id).is_some_and(|hits| hits.contains(&value))
        }
        Expr::In { id, low, high } => {
            let (Some(low), Some(high)) = (eval_int(low, state), eval_int(high, state)) else {
                return false;
            };
            state
                .hits
                .get(id)
                .is_some_and(|hits| hits.iter().any(|offset| *offset >= low && *offset < high))
        }
        other => eval_int(other, state).is_some_and(|value| value != 0),
    }
}

fn eval_int(expr: &Expr, state: &ScanState<'_>) -> Option<u64> {
    match expr {
        Expr::Count(id) => Some(state.hits.get(id).map_or(0, |hits| hits.len() as u64)),
        Expr::Offset(id, index) => state
            .hits
            .get(id)
            .and_then(|hits| hits.get(index.saturating_sub(1)).copied()),
        Expr::Int(value) => Some(*value),
        Expr::Filesize => Some(state.data.len() as u64),
        Expr::Uint {
            width,
            big_endian,
            offset,
        } => {
            let at = usize::try_from(eval_int(offset, state)?).ok()?;
            let width = usize::from(*width);
            let bytes = state.data.get(at..at + width)?;
            let mut value = 0u64;
            if *big_endian {
                for byte in bytes {
                    value = (value << 8) | u64::from(*byte);
                }
            } else {
                for (index, byte) in bytes.iter().enumerate() {
                    value |= u64::from(*byte) << (8 * index);
                }
            }
            Some(value)
        }
        Expr::Compare { left, op, right } => {
            if let (Some(left), Some(right)) = (eval_str(left, state), eval_str(right, state)) {
                let result = match op {
                    Cmp::Eq => left == right,
                    Cmp::Ne => left != right,
                    _ => false,
                };
                return Some(u64::from(result));
            }
            let left = eval_int(left, state)?;
            let right = eval_int(right, state)?;
            let result = match op {
                Cmp::Lt => left < right,
                Cmp::Le => left <= right,
                Cmp::Gt => left > right,
                Cmp::Ge => left >= right,
                Cmp::Eq => left == right,
                Cmp::Ne => left != right,
            };
            Some(u64::from(result))
        }
        Expr::Module {
            module,
            field,
            args,
        } => eval_module_int(module, field, args, state),
        Expr::Literal(_) => None,
        Expr::Reference(_)
        | Expr::AnyOfThem
        | Expr::AllOfThem
        | Expr::True
        | Expr::And(_, _)
        | Expr::Or(_, _)
        | Expr::Not(_)
        | Expr::At { .. }
        | Expr::In { .. } => Some(u64::from(eval_bool(expr, state))),
    }
}

fn eval_module_int(
    module: &str,
    field: &str,
    _args: &[Expr],
    state: &ScanState<'_>,
) -> Option<u64> {
    match module {
        "elf" => elf_field(state.data, field),
        "pe" => pe_field(state.data, field),
        _ => None,
    }
}

fn eval_str(expr: &Expr, state: &ScanState<'_>) -> Option<String> {
    match expr {
        Expr::Literal(text) => Some(text.clone()),
        Expr::Module {
            module,
            field,
            args,
        } if module == "hash" && field == "sha256" => {
            let offset = usize::try_from(eval_int(args.first()?, state)?).ok()?;
            let size = usize::try_from(eval_int(args.get(1)?, state)?).ok()?;
            let slice = state.data.get(offset..offset.checked_add(size)?)?;
            let digest = Sha256::digest(slice);
            Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
        }
        _ => None,
    }
}

fn read_u16_at(data: &[u8], offset: u64, little: bool) -> Option<u16> {
    let offset = usize::try_from(offset).ok()?;
    let raw = data.get(offset..offset + 2)?;
    Some(if little {
        u16::from_le_bytes([raw[0], raw[1]])
    } else {
        u16::from_be_bytes([raw[0], raw[1]])
    })
}

fn read_u32_at(data: &[u8], offset: u64, little: bool) -> Option<u32> {
    let offset = usize::try_from(offset).ok()?;
    let raw = data.get(offset..offset + 4)?;
    let arr = [raw[0], raw[1], raw[2], raw[3]];
    Some(if little {
        u32::from_le_bytes(arr)
    } else {
        u32::from_be_bytes(arr)
    })
}

fn read_u64_at(data: &[u8], offset: u64, little: bool) -> Option<u64> {
    let offset = usize::try_from(offset).ok()?;
    let raw = data.get(offset..offset + 8)?;
    let mut arr = [0u8; 8];
    arr.copy_from_slice(raw);
    Some(if little {
        u64::from_le_bytes(arr)
    } else {
        u64::from_be_bytes(arr)
    })
}

fn elf_field(data: &[u8], field: &str) -> Option<u64> {
    if !data.starts_with(b"\x7fELF") || data.len() < 52 {
        return None;
    }
    let class = data[4];
    let little = match data[5] {
        1 => true,
        2 => false,
        _ => return None,
    };
    match field {
        "machine" => Some(u64::from(read_u16_at(data, 18, little)?)),
        "type" => Some(u64::from(read_u16_at(data, 16, little)?)),
        "entry_point" => {
            if class == 2 {
                read_u64_at(data, 24, little)
            } else {
                Some(u64::from(read_u32_at(data, 24, little)?))
            }
        }
        "number_of_sections" => {
            let offset = if class == 2 { 60 } else { 48 };
            Some(u64::from(read_u16_at(data, offset, little)?))
        }
        _ => None,
    }
}

fn pe_field(data: &[u8], field: &str) -> Option<u64> {
    if !data.starts_with(b"MZ") || data.len() < 0x40 {
        return None;
    }
    let pe = usize::try_from(read_u32_at(data, 0x3c, true)?).ok()?;
    if data.get(pe..pe + 4) != Some(b"PE\0\0") {
        return None;
    }
    let coff = pe + 4;
    let optional = coff + 20;
    match field {
        "machine" => Some(u64::from(read_u16_at(data, coff as u64, true)?)),
        "number_of_sections" => Some(u64::from(read_u16_at(data, coff as u64 + 2, true)?)),
        "entry_point" => Some(u64::from(read_u32_at(data, optional as u64 + 16, true)?)),
        "subsystem" => Some(u64::from(read_u16_at(data, optional as u64 + 68, true)?)),
        _ => None,
    }
}

fn strip_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut index = 0usize;
    let mut quote: Option<char> = None;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = quote {
            out.push(ch);
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }
        match ch {
            '"' => {
                quote = Some('"');
                out.push(ch);
                index += 1;
            }
            '/' if chars.get(index + 1) == Some(&'/') => {
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            '/' if chars.get(index + 1) == Some(&'*') => {
                index += 2;
                while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                    out.push(' ');
                    index += 1;
                }
                index = (index + 2).min(chars.len());
            }
            _ => {
                out.push(ch);
                index += 1;
            }
        }
    }
    out
}

fn find_keyword(haystack: &str, keyword: &str, from: usize) -> Option<usize> {
    let mut search = from;
    while let Some(relative) = haystack[search..].find(keyword) {
        let position = search + relative;
        let before_ok = position == 0
            || haystack[..position]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_alphanumeric() && c != '_');
        let after = position + keyword.len();
        let after_ok = after >= haystack.len()
            || haystack[after..]
                .chars()
                .next()
                .is_some_and(|c| !c.is_alphanumeric() && c != '_');
        if before_ok && after_ok {
            return Some(position);
        }
        search = position + keyword.len();
    }
    None
}

fn read_ident(source: &str) -> Option<String> {
    let mut ident = String::new();
    for ch in source.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            break;
        }
    }
    (!ident.is_empty()).then_some(ident)
}

fn find_matching_brace(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0i64;
    let mut index = open;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active) = quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == active {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' => quote = Some(b'"'),
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn parse_strings(body: &str) -> Result<Vec<RuleString>, YaraError> {
    let Some(section) = section_of(body, "strings:", "condition:") else {
        return Ok(Vec::new());
    };
    let mut strings = Vec::new();
    let mut rest = section;
    while let Some(dollar) = rest.find('$') {
        rest = &rest[dollar + 1..];
        let Some(equals) = rest.find('=') else {
            break;
        };
        let id = rest[..equals].trim().to_string();
        if id.is_empty() {
            return Err(YaraError::BadString(rest[..equals].to_string()));
        }
        let after = rest[equals + 1..].trim_start();
        if let Some(quoted) = after.strip_prefix('"') {
            let end = quoted
                .find('"')
                .ok_or_else(|| YaraError::BadString(id.clone()))?;
            let trailing = &quoted[end + 1..];
            let line_end = trailing
                .find(|ch: char| ch == '\n' || ch == '$')
                .unwrap_or(trailing.len());
            let modifiers = parse_text_modifiers(&trailing[..line_end])?;
            strings.push(RuleString::Text {
                id,
                bytes: quoted[..end].as_bytes().to_vec(),
                modifiers,
            });
            rest = &trailing[line_end..];
        } else if after.starts_with('{') {
            let end = after
                .find('}')
                .ok_or_else(|| YaraError::BadHex(id.clone()))?;
            let pattern = parse_hex(&after[..=end])?;
            strings.push(RuleString::Hex { id, pattern });
            rest = &after[end + 1..];
        } else if let Some(body) = after.strip_prefix('/') {
            let mut end = None;
            let mut escaped = false;
            for (index, ch) in body.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '/' {
                    end = Some(index);
                    break;
                }
            }
            let end = end.ok_or_else(|| YaraError::BadString(id.clone()))?;
            let pattern = &body[..end];
            let trailing = &body[end + 1..];
            let line_end = trailing
                .find(|ch: char| ch == '\n' || ch == '$')
                .unwrap_or(trailing.len());
            let modifiers = parse_text_modifiers(&trailing[..line_end])?;
            if modifiers.wide || modifiers.xor.is_some() {
                return Err(YaraError::BadString(format!(
                    "regex {id} does not support wide/xor modifiers"
                )));
            }
            let regex = crate::recon::regex::Regex::parse(pattern)
                .map_err(|_| YaraError::BadString(format!("bad regex {id}")))?;
            strings.push(RuleString::Regex {
                id,
                regex,
                modifiers,
            });
            rest = &trailing[line_end..];
        } else {
            return Err(YaraError::BadString(id));
        }
    }
    Ok(strings)
}

fn parse_hex(value: &str) -> Result<HexPattern, YaraError> {
    let start = value
        .find('{')
        .ok_or_else(|| YaraError::BadHex(value.to_string()))?;
    let end = value
        .rfind('}')
        .ok_or_else(|| YaraError::BadHex(value.to_string()))?;
    if end <= start {
        return Err(YaraError::BadHex(value.to_string()));
    }
    let inner = &value[start + 1..end];
    let mut parser = HexParser {
        chars: inner.chars().collect(),
        position: 0,
        source: inner,
    };
    let atoms = parser.parse_top()?;
    parser.skip_ws();
    if parser.position != parser.chars.len() || atoms.is_empty() {
        return Err(YaraError::BadHex(inner.to_string()));
    }
    Ok(HexPattern::compile(&atoms))
}

/// 十六进制模式的字符级解析器；所有畸形输入都返回 [`YaraError::BadHex`]。
struct HexParser<'a> {
    chars: Vec<char>,
    position: usize,
    source: &'a str,
}

impl<'a> HexParser<'a> {
    fn error(&self) -> YaraError {
        YaraError::BadHex(self.source.to_string())
    }

    fn skip_ws(&mut self) {
        while self
            .chars
            .get(self.position)
            .is_some_and(|ch| ch.is_whitespace())
        {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn parse_top(&mut self) -> Result<Vec<HexAtom>, YaraError> {
        let mut atoms = Vec::new();
        loop {
            self.skip_ws();
            let Some(ch) = self.peek() else {
                return Ok(atoms);
            };
            match ch {
                '[' => atoms.push(self.parse_jump()?),
                '(' => atoms.push(self.parse_alternation()?),
                '|' | ')' | ']' => return Err(self.error()),
                _ => atoms.extend(self.parse_byte_run()?),
            }
        }
    }

    /// 解析一个分支直到 `|` 或 `)`；嵌套分支直接报错。
    fn parse_branch(&mut self) -> Result<Vec<HexAtom>, YaraError> {
        let mut atoms = Vec::new();
        loop {
            self.skip_ws();
            let Some(ch) = self.peek() else {
                return Err(self.error());
            };
            match ch {
                '|' | ')' => return Ok(atoms),
                '(' => return Err(self.error()),
                '[' => atoms.push(self.parse_jump()?),
                ']' => return Err(self.error()),
                _ => atoms.extend(self.parse_byte_run()?),
            }
        }
    }

    fn parse_alternation(&mut self) -> Result<HexAtom, YaraError> {
        self.position += 1; // '('
        let mut branches = Vec::new();
        loop {
            let branch = self.parse_branch()?;
            if branch.is_empty() {
                return Err(self.error());
            }
            branches.push(branch);
            self.skip_ws();
            match self.peek() {
                Some('|') => self.position += 1,
                Some(')') => {
                    self.position += 1;
                    break;
                }
                _ => return Err(self.error()),
            }
        }
        if branches.len() < 2 {
            return Err(self.error());
        }
        Ok(HexAtom::Alt(branches))
    }

    fn parse_jump(&mut self) -> Result<HexAtom, YaraError> {
        self.position += 1; // '['
        let start = self.position;
        while self.peek().is_some_and(|ch| ch != ']') {
            self.position += 1;
        }
        if self.peek() != Some(']') {
            return Err(self.error());
        }
        let raw: String = self.chars[start..self.position].iter().collect();
        self.position += 1; // ']'
        let spec = raw.trim();
        if spec.is_empty() {
            return Err(self.error());
        }
        let cap = MAX_HEX_JUMP as u64;
        let (min_raw, max_raw, bounded_max) = match spec.split_once('-') {
            Some((low, high)) => {
                let min = parse_jump_bound(low.trim(), spec)?;
                let high = high.trim();
                if high.is_empty() {
                    (min, cap, false)
                } else {
                    (min, parse_jump_bound(high, spec)?, true)
                }
            }
            None => {
                let exact = parse_jump_bound(spec, spec)?;
                (exact, exact, true)
            }
        };
        if bounded_max && min_raw > max_raw {
            return Err(self.error());
        }
        // 上限封顶；`[n-]` 的 n 超过上限时按上限处理，避免匹配爆炸。
        let min = min_raw.min(cap) as usize;
        let max = max_raw.min(cap) as usize;
        Ok(HexAtom::Jump { min, max })
    }

    fn parse_byte_run(&mut self) -> Result<Vec<HexAtom>, YaraError> {
        let start = self.position;
        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_hexdigit() || ch == '?')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(self.error());
        }
        let run: String = self.chars[start..self.position].iter().collect();
        if run.len() % 2 != 0 {
            return Err(YaraError::BadHex(run));
        }
        let mut atoms = Vec::with_capacity(run.len() / 2);
        for index in (0..run.len()).step_by(2) {
            let (mask, value) = parse_hex_byte(&run[index..index + 2])?;
            atoms.push(HexAtom::Byte { mask, value });
        }
        Ok(atoms)
    }
}

fn parse_jump_bound(text: &str, spec: &str) -> Result<u64, YaraError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(YaraError::BadHex(spec.to_string()));
    }
    text.parse::<u64>()
        .map_err(|_| YaraError::BadHex(spec.to_string()))
}

fn parse_hex_byte(token: &str) -> Result<(u8, u8), YaraError> {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() != 2 {
        return Err(YaraError::BadHex(token.to_string()));
    }
    let mut mask = 0u8;
    let mut value = 0u8;
    for (index, ch) in chars.iter().enumerate() {
        let shift = if index == 0 { 4 } else { 0 };
        if *ch == '?' {
            continue;
        }
        let digit = ch
            .to_digit(16)
            .ok_or_else(|| YaraError::BadHex(token.to_string()))? as u8;
        mask |= 0x0f << shift;
        value |= digit << shift;
    }
    Ok((mask, value))
}

fn parse_condition(body: &str, strings: &[RuleString]) -> Result<Expr, YaraError> {
    let condition = section_of(body, "condition:", "")
        .ok_or_else(|| YaraError::MissingCondition("<rule>".to_string()))?;
    let ids: BTreeSet<&str> = strings.iter().map(RuleString::id).collect();
    let mut parser = CondParser {
        chars: condition.chars().collect(),
        position: 0,
    };
    let expr = parser.parse_or()?;
    parser.skip_ws();
    if parser.position < parser.chars.len() {
        return Err(YaraError::BadCondition(condition.to_string()));
    }
    validate_refs(&expr, &ids)?;
    Ok(expr)
}

fn validate_refs(expr: &Expr, ids: &BTreeSet<&str>) -> Result<(), YaraError> {
    match expr {
        Expr::Reference(id)
        | Expr::Count(id)
        | Expr::Offset(id, _)
        | Expr::At { id, .. }
        | Expr::In { id, .. } => {
            if ids.contains(id.as_str()) {
                Ok(())
            } else {
                Err(YaraError::UnknownReference(id.clone()))
            }
        }
        Expr::And(left, right) | Expr::Or(left, right) | Expr::Compare { left, right, .. } => {
            validate_refs(left, ids)?;
            validate_refs(right, ids)
        }
        Expr::Not(inner) => validate_refs(inner, ids),
        Expr::Uint { offset, .. } => validate_refs(offset, ids),
        Expr::Module { args, .. } => {
            for arg in args {
                validate_refs(arg, ids)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

struct CondParser {
    chars: Vec<char>,
    position: usize,
}

impl CondParser {
    fn skip_ws(&mut self) {
        while self
            .chars
            .get(self.position)
            .is_some_and(|ch| ch.is_whitespace())
        {
            self.position += 1;
        }
    }

    fn read_word(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.position;
        // 数字字面量不吞 '.'，否则 `2..5` 会被当成模块名。
        let allow_dot = !self
            .chars
            .get(self.position)
            .is_some_and(|ch| ch.is_ascii_digit());
        while let Some(ch) = self.chars.get(self.position) {
            if ch.is_alphanumeric()
                || matches!(ch, '_' | '$' | '#' | '@')
                || (allow_dot && *ch == '.')
            {
                self.position += 1;
            } else {
                break;
            }
        }
        (self.position > start).then(|| self.chars[start..self.position].iter().collect())
    }

    fn peek_word(&mut self) -> Option<String> {
        let save = self.position;
        let word = self.read_word();
        self.position = save;
        word
    }

    fn parse_or(&mut self) -> Result<Expr, YaraError> {
        let mut left = self.parse_and()?;
        loop {
            if self.peek_word().as_deref() == Some("or") {
                self.read_word();
                let right = self.parse_and()?;
                left = Expr::Or(Box::new(left), Box::new(right));
            } else {
                return Ok(left);
            }
        }
    }

    fn parse_and(&mut self) -> Result<Expr, YaraError> {
        let mut left = self.parse_not()?;
        loop {
            if self.peek_word().as_deref() == Some("and") {
                self.read_word();
                let right = self.parse_not()?;
                left = Expr::And(Box::new(left), Box::new(right));
            } else {
                return Ok(left);
            }
        }
    }

    fn parse_not(&mut self) -> Result<Expr, YaraError> {
        if self.peek_word().as_deref() == Some("not") {
            self.read_word();
            let inner = self.parse_not()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, YaraError> {
        let left = self.parse_operand()?;
        self.skip_ws();
        let Some((op, len)) = self.peek_comparison() else {
            return Ok(left);
        };
        self.position += len;
        let right = self.parse_operand()?;
        Ok(Expr::Compare {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    fn peek_comparison(&self) -> Option<(Cmp, usize)> {
        let rest: String = self.chars[self.position..].iter().take(2).collect();
        if rest.starts_with("==") {
            return Some((Cmp::Eq, 2));
        }
        if rest.starts_with("!=") {
            return Some((Cmp::Ne, 2));
        }
        if rest.starts_with("<=") {
            return Some((Cmp::Le, 2));
        }
        if rest.starts_with(">=") {
            return Some((Cmp::Ge, 2));
        }
        match rest.chars().next() {
            Some('<') => Some((Cmp::Lt, 1)),
            Some('>') => Some((Cmp::Gt, 1)),
            _ => None,
        }
    }

    fn parse_operand(&mut self) -> Result<Expr, YaraError> {
        self.skip_ws();
        if self.chars.get(self.position) == Some(&'(') {
            self.position += 1;
            let inner = self.parse_or()?;
            self.skip_ws();
            if self.chars.get(self.position) != Some(&')') {
                return Err(YaraError::BadCondition("missing )".to_string()));
            }
            self.position += 1;
            return Ok(inner);
        }

        if self.chars.get(self.position) == Some(&'"') {
            self.position += 1;
            let start = self.position;
            while self.chars.get(self.position) != Some(&'"') {
                if self.position >= self.chars.len() {
                    return Err(YaraError::BadCondition("unterminated string".to_string()));
                }
                self.position += 1;
            }
            let text: String = self.chars[start..self.position].iter().collect();
            self.position += 1;
            return Ok(Expr::Literal(text));
        }

        let word = self
            .read_word()
            .ok_or_else(|| YaraError::BadCondition("empty condition".to_string()))?;
        if let Some((module, field)) = word.split_once('.') {
            let mut args = Vec::new();
            self.skip_ws();
            if self.chars.get(self.position) == Some(&'(') {
                self.position += 1;
                loop {
                    args.push(self.parse_or()?);
                    self.skip_ws();
                    match self.chars.get(self.position) {
                        Some(',') => self.position += 1,
                        Some(')') => {
                            self.position += 1;
                            break;
                        }
                        _ => {
                            return Err(YaraError::BadCondition(
                                "expected , or ) in module call".to_string(),
                            ))
                        }
                    }
                }
            }
            return Ok(Expr::Module {
                module: module.to_string(),
                field: field.to_string(),
                args,
            });
        }
        match word.as_str() {
            "true" => Ok(Expr::True),
            "false" => Ok(Expr::Int(0)),
            "filesize" => Ok(Expr::Filesize),
            "any" => {
                self.expect_word("of")?;
                self.expect_word("them")?;
                Ok(Expr::AnyOfThem)
            }
            "all" => {
                self.expect_word("of")?;
                self.expect_word("them")?;
                Ok(Expr::AllOfThem)
            }
            "uint8" | "uint16" | "uint32" | "uint8be" | "uint16be" | "uint32be" => {
                let big_endian = word.ends_with("be");
                let width = match word.trim_end_matches("be") {
                    "uint8" => 1,
                    "uint16" => 2,
                    _ => 4,
                };
                self.expect_char('(')?;
                let offset = self.parse_or()?;
                self.skip_ws();
                self.expect_char(')')?;
                Ok(Expr::Uint {
                    width,
                    big_endian,
                    offset: Box::new(offset),
                })
            }
            other if other.starts_with('$') => {
                let id = other.trim_start_matches('$').to_string();
                if self.peek_word().as_deref() == Some("at") {
                    self.read_word();
                    let offset = self.parse_operand()?;
                    return Ok(Expr::At {
                        id,
                        offset: Box::new(offset),
                    });
                }
                if self.peek_word().as_deref() == Some("in") {
                    self.read_word();
                    self.expect_char('(')?;
                    let low = self.parse_or()?;
                    self.skip_ws();
                    if self.chars.get(self.position..self.position + 2) != Some(&['.', '.'][..]) {
                        return Err(YaraError::BadCondition("expected ..".to_string()));
                    }
                    self.position += 2;
                    let high = self.parse_or()?;
                    self.skip_ws();
                    self.expect_char(')')?;
                    return Ok(Expr::In {
                        id,
                        low: Box::new(low),
                        high: Box::new(high),
                    });
                }
                Ok(Expr::Reference(id))
            }
            other if other.starts_with('#') => {
                Ok(Expr::Count(other.trim_start_matches('#').to_string()))
            }
            other if other.starts_with('@') => {
                let id = other.trim_start_matches('@').to_string();
                let mut index = 1usize;
                self.skip_ws();
                if self.chars.get(self.position) == Some(&'[') {
                    self.position += 1;
                    let digits = self
                        .read_word()
                        .ok_or_else(|| YaraError::BadCondition("missing index".to_string()))?;
                    index = digits
                        .parse()
                        .map_err(|_| YaraError::BadCondition("bad index".to_string()))?;
                    self.skip_ws();
                    self.expect_char(']')?;
                }
                Ok(Expr::Offset(id, index))
            }
            other if other.chars().next().is_some_and(|ch| ch.is_ascii_digit()) => {
                let value = if let Some(hex) = other.strip_prefix("0x") {
                    u64::from_str_radix(hex, 16)
                } else {
                    other.parse()
                }
                .map_err(|_| YaraError::BadCondition(other.to_string()))?;
                Ok(Expr::Int(value))
            }
            other => Err(YaraError::BadCondition(other.to_string())),
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), YaraError> {
        self.skip_ws();
        if self.chars.get(self.position) != Some(&expected) {
            return Err(YaraError::BadCondition(format!("expected {expected}")));
        }
        self.position += 1;
        Ok(())
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), YaraError> {
        match self.read_word().as_deref() {
            Some(word) if word == expected => Ok(()),
            _ => Err(YaraError::BadCondition(format!("expected {expected}"))),
        }
    }
}

fn section_of<'a>(body: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_at = body.find(start)? + start.len();
    let rest = &body[start_at..];
    if end.is_empty() {
        return Some(rest);
    }
    let end_at = rest.find(end)?;
    Some(&rest[..end_at])
}

fn parse_text_modifiers(text: &str) -> Result<TextModifiers, YaraError> {
    let mut modifiers = TextModifiers::default();
    for word in text.split_whitespace() {
        match word {
            "nocase" => modifiers.nocase = true,
            "wide" => modifiers.wide = true,
            "ascii" => modifiers.ascii = true,
            "fullword" => modifiers.fullword = true,
            other if other == "xor" => modifiers.xor = Some((0, 255)),
            other if other.starts_with("xor(") && other.ends_with(')') => {
                let inner = &other[4..other.len() - 1];
                let range = if let Some((low, high)) = inner.split_once('-') {
                    let low: u8 = low
                        .parse()
                        .map_err(|_| YaraError::BadString(other.to_string()))?;
                    let high: u8 = high
                        .parse()
                        .map_err(|_| YaraError::BadString(other.to_string()))?;
                    (low, high)
                } else {
                    let key: u8 = inner
                        .parse()
                        .map_err(|_| YaraError::BadString(other.to_string()))?;
                    (key, key)
                };
                if range.0 > range.1 {
                    return Err(YaraError::BadString(other.to_string()));
                }
                modifiers.xor = Some(range);
            }
            other => {
                return Err(YaraError::BadString(format!(
                    "unsupported string modifier {other:?}"
                )));
            }
        }
    }
    Ok(modifiers)
}

/// 按修饰符匹配文本：默认 ASCII；`wide` 匹配 UTF-16LE；两者同用则都匹配。
fn find_text(haystack: &[u8], needle: &[u8], modifiers: TextModifiers) -> Vec<u64> {
    let mut hits = std::collections::BTreeSet::new();
    if let Some((low, high)) = modifiers.xor {
        // XOR 搜索：key 0 等价明文；只要指定了 xor，就只按 xor 语义匹配。
        let plain = TextModifiers {
            xor: None,
            ..modifiers
        };
        for key in low..=high {
            let keyed: Vec<u8> = needle.iter().map(|byte| byte ^ key).collect();
            for offset in find_bytes_with(haystack, &keyed, plain) {
                hits.insert(offset);
            }
        }
        return hits.into_iter().collect();
    }
    let match_ascii = !modifiers.wide || modifiers.ascii;
    if match_ascii {
        for offset in find_bytes_with(haystack, needle, modifiers) {
            hits.insert(offset);
        }
    }
    if modifiers.wide {
        let mut wide = Vec::with_capacity(needle.len() * 2);
        for &byte in needle {
            wide.push(byte);
            wide.push(0);
        }
        for offset in find_bytes_with(haystack, &wide, modifiers) {
            hits.insert(offset);
        }
    }
    hits.into_iter().collect()
}

fn find_bytes_with(haystack: &[u8], needle: &[u8], modifiers: TextModifiers) -> Vec<u64> {
    let mut hits = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return hits;
    }
    for index in 0..=haystack.len() - needle.len() {
        let window = &haystack[index..index + needle.len()];
        let equal = if modifiers.nocase {
            window
                .iter()
                .zip(needle)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        } else {
            window == needle
        };
        if equal && (!modifiers.fullword || is_fullword(haystack, index, needle.len())) {
            hits.push(index as u64);
            if hits.len() >= MAX_MATCHES_PER_STRING {
                break;
            }
        }
    }
    hits
}

/// 前后都不是 ASCII 字母/数字/下划线。
fn is_fullword(haystack: &[u8], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word_byte(haystack[start - 1]);
    let after = start + len;
    let after_ok = after >= haystack.len() || !is_word_byte(haystack[after]);
    before_ok && after_ok
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Vec<u64> {
    let mut hits = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return hits;
    }
    for index in 0..=haystack.len() - needle.len() {
        if &haystack[index..index + needle.len()] == needle {
            hits.push(index as u64);
            if hits.len() >= MAX_MATCHES_PER_STRING {
                break;
            }
        }
    }
    hits
}

/// 找出所有可以完整匹配的起始偏移（每个起始偏移至多记一次）。
///
/// 在编译出的 NFA 上做带回溯的状态搜索；`seen` 以 `(节点, 位置)` 去重，
/// 因此跳转/分支组合不会指数爆炸。所有路径都尝试后仍未到达 `Match` 即不命中。
fn find_hex(haystack: &[u8], pattern: &HexPattern) -> Vec<u64> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    for start in 0..=haystack.len() {
        if hex_matches_at(haystack, pattern, start, &mut seen, &mut stack) {
            hits.push(start as u64);
            if hits.len() >= MAX_MATCHES_PER_STRING {
                break;
            }
        }
    }
    hits
}

fn hex_matches_at(
    haystack: &[u8],
    pattern: &HexPattern,
    start: usize,
    seen: &mut HashSet<(usize, usize)>,
    stack: &mut Vec<(usize, usize)>,
) -> bool {
    seen.clear();
    stack.clear();
    stack.push((pattern.entry, start));
    while let Some((node_index, position)) = stack.pop() {
        if !seen.insert((node_index, position)) {
            continue;
        }
        let Some(node) = pattern.nodes.get(node_index) else {
            continue;
        };
        match node {
            HexNode::Match => return true,
            HexNode::Byte { mask, value, next } => {
                if haystack
                    .get(position)
                    .is_some_and(|byte| (*byte & *mask) == *value)
                {
                    stack.push((*next, position + 1));
                }
            }
            HexNode::Jump { min, max, next } => {
                let remaining = haystack.len().saturating_sub(position);
                if *min > remaining {
                    continue;
                }
                let upper = (*max).min(remaining);
                // 先压大跨度、后压小跨度，出栈时优先尝试最小跳跃。
                for skip in (*min..=upper).rev() {
                    stack.push((*next, position + skip));
                }
            }
            HexNode::Split { alternatives } => {
                // 逆序入栈，保持源码中的备选顺序先被尝试。
                for next in alternatives.iter().rev() {
                    stack.push((*next, position));
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = r#"
        rule demo_text {
            meta:
                author = "test"
            strings:
                $a = "hello world"
                $b = "goodbye"
            condition:
                $a and not $b
        }
    "#;

    #[test]
    fn text_rule_matches_and_reports_offsets() {
        let rules = RuleSet::parse(RULES).expect("parse");
        assert_eq!(rules.len(), 1);
        let report = rules.scan(b"...hello world...");
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].rule, "demo_text");
        assert_eq!(report.matches[0].strings, vec!["a".to_string()]);
        assert_eq!(report.matches[0].offsets, vec![3]);
    }

    #[test]
    fn negative_and_boolean_conditions_work() {
        let rules = RuleSet::parse(RULES).expect("parse");
        let report = rules.scan(b"hello world goodbye");
        assert!(report.matches.is_empty(), "not $b must fail the rule");
    }

    #[test]
    fn hex_wildcards_match() {
        let rules = RuleSet::parse(r#"rule mz { strings: $mz = { 4D 5A ?? ?? } condition: $mz }"#)
            .expect("parse");
        let report = rules.scan(b"xxMZ\x90\x00rest");
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].offsets, vec![2]);
    }

    #[test]
    fn any_of_them_reads_all_strings() {
        let rules =
            RuleSet::parse(r#"rule any { strings: $a = "aaa" $b = "bbb" condition: any of them }"#)
                .expect("parse");
        assert_eq!(rules.scan(b"zzz bbb").matches.len(), 1);
        assert_eq!(rules.scan(b"zzz").matches.len(), 0);
    }

    #[test]
    fn malformed_rules_are_rejected() {
        assert!(RuleSet::parse("rule x { }").is_err());
        assert!(matches!(RuleSet::parse(""), Err(YaraError::NoRule)));
        assert!(matches!(
            RuleSet::parse("rule x { strings: $a = \"a\" condition: $b }"),
            Err(YaraError::UnknownReference(_))
        ));
        assert!(matches!(
            RuleSet::parse("rule x { strings: $a = \"a\" condition: }"),
            Err(YaraError::BadCondition(_))
        ));
    }
}

#[cfg(test)]
mod modifier_tests {
    use super::*;

    #[test]
    fn nocase_matches_case_insensitively() {
        let rules = RuleSet::parse(r#"rule r { strings: $a = "Hello" nocase condition: $a }"#)
            .expect("parse");
        assert_eq!(rules.scan(b"say hello").matches.len(), 1);
        assert_eq!(rules.scan(b"HELLO").matches.len(), 1);
        assert_eq!(rules.scan(b"goodbye").matches.len(), 0);
    }

    #[test]
    fn wide_matches_utf16le_only() {
        let rules =
            RuleSet::parse(r#"rule r { strings: $a = "AB" wide condition: $a }"#).expect("parse");
        assert_eq!(rules.scan(b"\x41\x00\x42\x00").matches.len(), 1);
        assert_eq!(rules.scan(b"AB").matches.len(), 0);
    }

    #[test]
    fn wide_ascii_matches_both_encodings() {
        let rules = RuleSet::parse(r#"rule r { strings: $a = "AB" wide ascii condition: $a }"#)
            .expect("parse");
        assert_eq!(rules.scan(b"AB").matches.len(), 1);
        assert_eq!(rules.scan(b"\x41\x00\x42\x00").matches.len(), 1);
    }

    #[test]
    fn fullword_rejects_embedded_hits() {
        let rules = RuleSet::parse(r#"rule r { strings: $a = "cat" fullword condition: $a }"#)
            .expect("parse");
        assert_eq!(rules.scan(b"concatenate").matches.len(), 0);
        assert_eq!(rules.scan(b"a cat.").matches.len(), 1);
    }

    #[test]
    fn unsupported_modifiers_fail_closed() {
        assert!(matches!(
            RuleSet::parse(r#"rule r { strings: $a = "x" regex condition: $a }"#),
            Err(YaraError::BadString(_))
        ));
    }
}

#[cfg(test)]
mod condition_tests {
    use super::*;

    fn rule(condition: &str) -> RuleSet {
        RuleSet::parse(&format!(
            r#"rule r {{ strings: $a = "ab" condition: {condition} }}"#
        ))
        .expect("parse")
    }

    #[test]
    fn filesize_is_comparable() {
        let rules = rule("filesize > 4");
        assert_eq!(rules.scan(b"12345").matches.len(), 1);
        assert_eq!(rules.scan(b"123").matches.len(), 0);
    }

    #[test]
    fn string_count_is_comparable() {
        let rules = rule("#a >= 2");
        assert_eq!(rules.scan(b"ab ab ab").matches.len(), 1);
        assert_eq!(rules.scan(b"ab").matches.len(), 0);
    }

    #[test]
    fn at_pins_an_offset() {
        let rules = rule("$a at 3");
        assert_eq!(rules.scan(b"xxxab").matches.len(), 1);
        assert_eq!(rules.scan(b"abxxx").matches.len(), 0);
    }

    #[test]
    fn in_restricts_a_range() {
        let rules = rule("$a in (2..5)");
        assert_eq!(rules.scan(b"xxab").matches.len(), 1);
        assert_eq!(rules.scan(b"abxx").matches.len(), 0);
    }

    #[test]
    fn uint_reads_little_endian() {
        let rules = rule("uint16(0) == 0x5a4d");
        assert_eq!(rules.scan(b"MZrest").matches.len(), 1);
        assert_eq!(rules.scan(b"ZZrest").matches.len(), 0);
    }

    #[test]
    fn offset_of_first_match_is_exposed() {
        let rules = rule("@a == 2");
        assert_eq!(rules.scan(b"xxab").matches.len(), 1);
    }
}

#[cfg(test)]
mod hex_extended_tests {
    use super::*;

    fn rule(pattern: &str) -> RuleSet {
        RuleSet::parse(&format!(
            r#"rule r {{ strings: $h = {pattern} condition: $h }}"#
        ))
        .expect("parse")
    }

    fn offsets(rules: &RuleSet, haystack: &[u8]) -> Vec<u64> {
        let report = rules.scan(haystack);
        assert_eq!(report.matches.len(), 1, "expected one matching rule");
        report.matches[0].offsets.clone()
    }

    fn assert_bad_hex(pattern: &str) {
        let source = format!(r#"rule r {{ strings: $h = {pattern} condition: $h }}"#);
        assert!(
            matches!(RuleSet::parse(&source), Err(YaraError::BadHex(_))),
            "expected BadHex for {pattern:?}"
        );
    }

    #[test]
    fn hex_jump_exact_skips_fixed_bytes() {
        let rules = rule("{ 4D 5A [2] 90 }");
        assert_eq!(offsets(&rules, b"xxMZab\x90yy"), vec![2]);
        assert!(rules.scan(b"xxMZab\x91yy").matches.is_empty());
        assert!(rules.scan(b"xxMZabc\x90").matches.is_empty());
    }

    #[test]
    fn hex_jump_range_tries_each_backtracking_length() {
        let rules = rule("{ 41 [1-3] 42 }");
        assert_eq!(offsets(&rules, b"AxB"), vec![0]);
        assert_eq!(offsets(&rules, b"AxxxB"), vec![0]);
        assert_eq!(offsets(&rules, b"AAxB"), vec![0, 1]);
        assert!(rules.scan(b"AB").matches.is_empty());
        assert!(rules.scan(b"AxxxxB").matches.is_empty());
    }

    #[test]
    fn hex_jump_unbounded_matches_and_is_capped() {
        let rules = rule("{ AA [8-] BB }");
        let mut long_gap = vec![0u8; 100];
        long_gap[0] = 0xAA;
        long_gap[99] = 0xBB;
        assert_eq!(offsets(&rules, &long_gap), vec![0]);

        let mut short_gap = vec![0u8; 9];
        short_gap[0] = 0xAA;
        short_gap[8] = 0xBB;
        assert!(rules.scan(&short_gap).matches.is_empty());

        // `[8-]` 的无上限被封顶到 4096：gap=4096 命中，gap=4097 不命中。
        let mut at_cap = vec![0u8; MAX_HEX_JUMP + 2];
        at_cap[0] = 0xAA;
        at_cap[MAX_HEX_JUMP + 1] = 0xBB;
        assert_eq!(offsets(&rules, &at_cap), vec![0]);

        let mut over_cap = vec![0u8; MAX_HEX_JUMP + 3];
        over_cap[0] = 0xAA;
        over_cap[MAX_HEX_JUMP + 2] = 0xBB;
        assert!(rules.scan(&over_cap).matches.is_empty());
    }

    #[test]
    fn hex_alternation_matches_each_branch() {
        let rules = rule("{ 4D ( 5A | 90 | 00 FF ) }");
        assert_eq!(offsets(&rules, b"MZ"), vec![0]);
        assert_eq!(offsets(&rules, b"M\x90"), vec![0]);
        assert_eq!(offsets(&rules, b"M\x00\xFF"), vec![0]);
        assert!(rules.scan(b"M\x00").matches.is_empty());
        assert!(rules.scan(b"Z").matches.is_empty());
    }

    #[test]
    fn hex_alternation_carries_wildcards_and_jumps() {
        let rules = rule("{ 4D ( 5A ?? [1] 90 | 00 FF ) }");
        assert_eq!(offsets(&rules, b"MZ\x00\x00\x90"), vec![0]);
        assert_eq!(offsets(&rules, b"M\x00\xFF"), vec![0]);
        assert!(rules.scan(b"MZ\x00\x90").matches.is_empty());
    }

    #[test]
    fn hex_alternation_continues_with_following_atoms() {
        // 无空格写法也应可解析：M (Z|90) [1] 00。
        let rules = rule("{4D(5A|90)[1]00}");
        assert_eq!(offsets(&rules, b"MZx\x00"), vec![0]);
        assert_eq!(offsets(&rules, b"M\x90x\x00"), vec![0]);
        assert!(rules.scan(b"MZ\x00").matches.is_empty());
        assert!(rules.scan(b"MZxx\x00").matches.is_empty());
    }

    #[test]
    fn hex_matches_are_capped_at_max_matches() {
        let rules = rule("{ 41 ?? }");
        let haystack = vec![b'A'; 600];
        let report = rules.scan(&haystack);
        assert_eq!(report.matches.len(), 1);
        assert_eq!(
            report.matches[0].offsets.len(),
            MAX_MATCHES_PER_STRING,
            "offset list must stay capped"
        );
        assert_eq!(report.matches[0].offsets.last().copied(), Some(255));
    }

    #[test]
    fn malformed_hex_patterns_are_rejected() {
        for pattern in [
            "{ }",
            "{ 4D [2 5A }",
            "{ 4D [6-2] 5A }",
            "{ 4D [1-2-3] 5A }",
            "{ 4D [x] 5A }",
            "{ 4D [999999999999999999999] 5A }",
            "{ 4D 5 }",
            "{ 4D 5A ?? ? }",
            "{ 4D 5A ] }",
            "{ 4D ( 5A | 90 }",
            "{ 4D ( 5A ) }",
            "{ 4D ( 5A | ) }",
            "{ 4D ( | 5A ) }",
            "{ 4D ( 5A ( 90 | 00 ) | FF ) }",
        ] {
            assert_bad_hex(pattern);
        }
    }
}

#[cfg(test)]
mod xor_tests {
    use super::*;

    fn rule(modifier: &str) -> RuleSet {
        RuleSet::parse(&format!(
            r#"rule r {{ strings: $a = "AB" {modifier} condition: $a }}"#
        ))
        .expect("parse")
    }

    #[test]
    fn xor_default_scans_all_keys() {
        let rules = rule("xor");
        // "AB" ^ 0x01 = "@C"
        assert_eq!(rules.scan(b"...@C...").matches.len(), 1);
        assert_eq!(rules.scan(b"...AB...").matches.len(), 1);
        assert_eq!(rules.scan(b"...ZZ...").matches.len(), 0);
    }

    #[test]
    fn xor_single_key_excludes_plaintext() {
        let rules = rule("xor(1)");
        assert_eq!(rules.scan(b"...@C...").matches.len(), 1);
        assert_eq!(rules.scan(b"...AB...").matches.len(), 0);
    }

    #[test]
    fn xor_range_matches_each_key() {
        let rules = rule("xor(1-2)");
        assert_eq!(rules.scan(b"...@C...").matches.len(), 1);
        // "AB" ^ 0x02 = "C@"
        assert_eq!(rules.scan(b"...C@...").matches.len(), 1);
    }

    #[test]
    fn invalid_xor_ranges_fail_closed() {
        assert!(matches!(
            RuleSet::parse(r#"rule r { strings: $a = "AB" xor(3-1) condition: $a }"#),
            Err(YaraError::BadString(_))
        ));
        assert!(matches!(
            RuleSet::parse(r#"rule r { strings: $a = "AB" xor(x) condition: $a }"#),
            Err(YaraError::BadString(_))
        ));
    }
}

#[cfg(test)]
mod module_tests {
    use super::*;

    fn elf64_header() -> Vec<u8> {
        let mut elf = vec![0u8; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&2u16.to_le_bytes());
        elf[18..20].copy_from_slice(&0x3eu16.to_le_bytes());
        elf[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64u16.to_le_bytes());
        elf[58..60].copy_from_slice(&64u16.to_le_bytes());
        elf
    }

    fn pe_one_section() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[0x98 + 16..0x98 + 20].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes
    }

    fn rule(condition: &str) -> RuleSet {
        RuleSet::parse(&format!(
            r#"rule r {{ strings: $a = "zzz" condition: {condition} }}"#
        ))
        .expect("parse")
    }

    #[test]
    fn elf_module_reads_header_fields() {
        let elf = elf64_header();
        assert_eq!(rule("elf.machine == 0x3e").scan(&elf).matches.len(), 1);
        assert_eq!(
            rule("elf.entry_point == 0x401000").scan(&elf).matches.len(),
            1
        );
        assert_eq!(rule("elf.machine == 0x03").scan(&elf).matches.len(), 0);
    }

    #[test]
    fn pe_module_reads_header_fields() {
        let pe = pe_one_section();
        assert_eq!(
            rule("pe.number_of_sections == 1").scan(&pe).matches.len(),
            1
        );
        assert_eq!(rule("pe.machine == 0x8664").scan(&pe).matches.len(), 1);
        assert_eq!(
            rule("pe.number_of_sections == 2").scan(&pe).matches.len(),
            0
        );
    }

    #[test]
    fn hash_module_returns_hex_digest() {
        let rules = rule(
            r#"hash.sha256(0, filesize) == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad""#,
        );
        assert_eq!(rules.scan(b"abc").matches.len(), 1);
        assert_eq!(rules.scan(b"abcd").matches.len(), 0);
    }

    #[test]
    fn unknown_module_fields_are_false_not_panics() {
        let rules = rule("pe.not_a_field == 1");
        assert!(rules.scan(b"MZ").matches.is_empty());
    }
}

#[cfg(test)]
mod regex_string_tests {
    use super::*;

    fn rule(pattern: &str, modifiers: &str) -> RuleSet {
        RuleSet::parse(&format!(
            r#"rule r {{ strings: $a = /{pattern}/ {modifiers} condition: $a }}"#
        ))
        .expect("parse")
    }

    #[test]
    fn regex_strings_match_offsets() {
        let rules = rule("m.a", "");
        let report = rules.scan(b"xxmZa");
        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].offsets, vec![2]);
    }

    #[test]
    fn regex_nocase_and_fullword() {
        let rules = rule("hello", "nocase");
        assert_eq!(rules.scan(b"HELLO").matches.len(), 1);

        let rules = rule("cat", "fullword");
        assert!(rules.scan(b"concatenate").matches.is_empty());
        assert_eq!(rules.scan(b"a cat.").matches.len(), 1);
    }

    #[test]
    fn invalid_regex_and_unsupported_modifiers_fail_closed() {
        assert!(matches!(
            RuleSet::parse(r#"rule r { strings: $a = /(/ condition: $a }"#),
            Err(YaraError::BadString(_))
        ));
        assert!(matches!(
            RuleSet::parse(r#"rule r { strings: $a = /abc/ wide condition: $a }"#),
            Err(YaraError::BadString(_))
        ));
    }
}
