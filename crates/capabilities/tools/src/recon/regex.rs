//! 字节级正则子集：YARA `$a = /pattern/` 的零依赖实现。
//!
//! 支持：字面量、`.`、字符类 `[a-z]`/`[^...]`、`\d \w \s` 及大写反向、
//! `* + ? {n} {n,} {n,m}`、分组 `( )`、分支 `|`、锚点 `^` `$`、`\xNN`。
//! 不支持：反向引用、lookaround、Unicode 属性、惰性量词（`*?` 视为贪婪）。
//! 匹配是字节级的；非法或过深的模式返回错误，匹配有步数预算，不会指数爆炸。

use std::collections::BTreeSet;

use thiserror::Error;

/// 正则解析/匹配错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegexError {
    /// 模式为空。
    #[error("empty regex")]
    Empty,
    /// 语法错误。
    #[error("malformed regex: {0}")]
    Malformed(&'static str),
    /// 不支持的构造。
    #[error("unsupported regex construct: {0}")]
    Unsupported(&'static str),
}

/// 编译后的正则。
#[derive(Debug, Clone)]
pub struct Regex {
    ast: Node,
}

#[derive(Debug, Clone)]
enum Node {
    Empty,
    Byte(u8),
    Any,
    Class {
        negated: bool,
        ranges: Vec<(u8, u8)>,
    },
    Start,
    End,
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: usize,
        max: Option<usize>,
    },
}

impl Regex {
    /// 编译模式（ASCII 字节语义）。
    pub fn parse(pattern: &str) -> Result<Self, RegexError> {
        if pattern.is_empty() {
            return Err(RegexError::Empty);
        }
        let mut parser = Parser {
            chars: pattern.chars().collect(),
            position: 0,
        };
        let ast = parser.parse_alt()?;
        if parser.position != parser.chars.len() {
            return Err(RegexError::Malformed("unexpected trailing characters"));
        }
        Ok(Self { ast })
    }

    /// 在 `text` 中查找全部命中起点（上限 `max_hits`）。
    pub fn find(&self, text: &[u8], nocase: bool, fullword: bool, max_hits: usize) -> Vec<u64> {
        let context = MatchContext { text, nocase };
        let mut hits = Vec::new();
        let mut budget = 2_000_000usize;
        for start in 0..=text.len() {
            if budget == 0 {
                break;
            }
            let ends = match_ends(&self.ast, &context, start, &mut budget);
            if let Some(&end) = ends.iter().next() {
                if !fullword || is_fullword(text, start, end) {
                    hits.push(start as u64);
                    if hits.len() >= max_hits {
                        break;
                    }
                }
            }
        }
        hits
    }
}

struct MatchContext<'a> {
    text: &'a [u8],
    nocase: bool,
}

fn match_ends(
    node: &Node,
    context: &MatchContext<'_>,
    position: usize,
    budget: &mut usize,
) -> BTreeSet<usize> {
    if *budget == 0 {
        return BTreeSet::new();
    }
    *budget -= 1;
    let mut out = BTreeSet::new();
    match node {
        Node::Empty => {
            out.insert(position);
        }
        Node::Byte(byte) => {
            if let Some(&actual) = context.text.get(position) {
                let equal = if context.nocase {
                    actual.eq_ignore_ascii_case(byte)
                } else {
                    actual == *byte
                };
                if equal {
                    out.insert(position + 1);
                }
            }
        }
        Node::Any => {
            if let Some(&byte) = context.text.get(position) {
                if byte != b'\n' {
                    out.insert(position + 1);
                }
            }
        }
        Node::Class { negated, ranges } => {
            if let Some(&byte) = context.text.get(position) {
                let target = if context.nocase {
                    byte.to_ascii_lowercase()
                } else {
                    byte
                };
                let inside = ranges.iter().any(|(low, high)| {
                    let (low, high) = if context.nocase {
                        (low.to_ascii_lowercase(), high.to_ascii_lowercase())
                    } else {
                        (*low, *high)
                    };
                    target >= low && target <= high
                });
                if inside != *negated {
                    out.insert(position + 1);
                }
            }
        }
        Node::Start => {
            if position == 0 {
                out.insert(0);
            }
        }
        Node::End => {
            if position == context.text.len() {
                out.insert(position);
            }
        }
        Node::Concat(parts) => {
            let mut current = BTreeSet::from([position]);
            for part in parts {
                let mut next = BTreeSet::new();
                for offset in current {
                    next.extend(match_ends(part, context, offset, budget));
                }
                if next.is_empty() {
                    return BTreeSet::new();
                }
                current = next;
            }
            out = current;
        }
        Node::Alt(branches) => {
            for branch in branches {
                out.extend(match_ends(branch, context, position, budget));
            }
        }
        Node::Repeat { node, min, max } => {
            let limit = max
                .unwrap_or(context.text.len() + 1)
                .min(context.text.len() + 1);
            let mut current = BTreeSet::from([position]);
            let mut count = 0usize;
            loop {
                if count >= *min {
                    out.extend(current.iter().copied());
                }
                if count >= limit {
                    break;
                }
                let mut next = BTreeSet::new();
                for offset in &current {
                    next.extend(match_ends(node, context, *offset, budget));
                }
                if next.is_empty() {
                    break;
                }
                current = next;
                count += 1;
            }
        }
    }
    out
}

fn is_fullword(text: &[u8], start: usize, end: usize) -> bool {
    let before = start == 0 || !is_word(text[start - 1]);
    let after = end >= text.len() || !is_word(text[end]);
    before && after
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

struct Parser {
    chars: Vec<char>,
    position: usize,
}

impl Parser {
    fn parse_alt(&mut self) -> Result<Node, RegexError> {
        let mut branches = vec![self.parse_concat()?];
        while self.peek() == Some('|') {
            self.position += 1;
            branches.push(self.parse_concat()?);
        }
        if branches.len() == 1 {
            Ok(branches.remove(0))
        } else {
            Ok(Node::Alt(branches))
        }
    }

    fn parse_concat(&mut self) -> Result<Node, RegexError> {
        let mut parts = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ')' || ch == '|' {
                break;
            }
            parts.push(self.parse_repeat()?);
        }
        match parts.len() {
            0 => Ok(Node::Empty),
            1 => Ok(parts.remove(0)),
            _ => Ok(Node::Concat(parts)),
        }
    }

    fn parse_repeat(&mut self) -> Result<Node, RegexError> {
        let atom = self.parse_atom()?;
        let mut node = atom;
        loop {
            let (min, max) = match self.peek() {
                Some('*') => {
                    self.position += 1;
                    (0, None)
                }
                Some('+') => {
                    self.position += 1;
                    (1, None)
                }
                Some('?') => {
                    self.position += 1;
                    (0, Some(1))
                }
                Some('{') => {
                    self.position += 1;
                    let min = self.parse_number()?;
                    let max = if self.peek() == Some(',') {
                        self.position += 1;
                        if self.peek() == Some('}') {
                            None
                        } else {
                            Some(self.parse_number()?)
                        }
                    } else {
                        Some(min)
                    };
                    if self.peek() != Some('}') {
                        return Err(RegexError::Malformed("missing }"));
                    }
                    self.position += 1;
                    if let Some(max) = max {
                        if max < min {
                            return Err(RegexError::Malformed("repeat bounds reversed"));
                        }
                    }
                    (min, max)
                }
                _ => break,
            };
            node = Node::Repeat {
                node: Box::new(node),
                min,
                max,
            };
        }
        Ok(node)
    }

    fn parse_number(&mut self) -> Result<usize, RegexError> {
        let mut value = 0usize;
        let mut digits = 0usize;
        while let Some(ch) = self.peek() {
            let Some(digit) = ch.to_digit(10) else {
                break;
            };
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit as usize))
                .ok_or(RegexError::Malformed("repeat count overflow"))?;
            self.position += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(RegexError::Malformed("missing repeat count"));
        }
        Ok(value)
    }

    fn parse_atom(&mut self) -> Result<Node, RegexError> {
        let Some(ch) = self.peek() else {
            return Err(RegexError::Malformed("unexpected end"));
        };
        match ch {
            '(' => {
                self.position += 1;
                let inner = self.parse_alt()?;
                if self.peek() != Some(')') {
                    return Err(RegexError::Malformed("missing )"));
                }
                self.position += 1;
                Ok(inner)
            }
            '.' => {
                self.position += 1;
                Ok(Node::Any)
            }
            '^' => {
                self.position += 1;
                Ok(Node::Start)
            }
            '$' => {
                self.position += 1;
                Ok(Node::End)
            }
            '[' => self.parse_class(),
            '\\' => {
                self.position += 1;
                self.parse_escape()
            }
            ')' | '|' => Err(RegexError::Malformed("unexpected metacharacter")),
            other => {
                self.position += 1;
                let byte = u8::try_from(other as u32)
                    .map_err(|_| RegexError::Unsupported("non-ASCII literal"))?;
                Ok(Node::Byte(byte))
            }
        }
    }

    fn parse_escape(&mut self) -> Result<Node, RegexError> {
        let Some(ch) = self.peek() else {
            return Err(RegexError::Malformed("trailing backslash"));
        };
        self.position += 1;
        let class = |ranges: Vec<(u8, u8)>| Node::Class {
            negated: false,
            ranges,
        };
        match ch {
            'd' => Ok(class(vec![(b'0', b'9')])),
            'w' => Ok(class(vec![
                (b'0', b'9'),
                (b'a', b'z'),
                (b'A', b'Z'),
                (b'_', b'_'),
            ])),
            's' => Ok(class(vec![
                (b' ', b' '),
                (b'\t', b'\t'),
                (b'\n', b'\n'),
                (b'\r', b'\r'),
            ])),
            'D' => Ok(Node::Class {
                negated: true,
                ranges: vec![(b'0', b'9')],
            }),
            'W' => Ok(Node::Class {
                negated: true,
                ranges: vec![(b'0', b'9'), (b'a', b'z'), (b'A', b'Z'), (b'_', b'_')],
            }),
            'S' => Ok(Node::Class {
                negated: true,
                ranges: vec![(b' ', b' '), (b'\t', b'\t'), (b'\n', b'\n'), (b'\r', b'\r')],
            }),
            'n' => Ok(Node::Byte(b'\n')),
            'r' => Ok(Node::Byte(b'\r')),
            't' => Ok(Node::Byte(b'\t')),
            'x' => {
                let high = self.peek().and_then(|ch| ch.to_digit(16));
                let Some(high) = high else {
                    return Err(RegexError::Malformed("bad \\x escape"));
                };
                self.position += 1;
                let low = self.peek().and_then(|ch| ch.to_digit(16));
                let Some(low) = low else {
                    return Err(RegexError::Malformed("bad \\x escape"));
                };
                self.position += 1;
                Ok(Node::Byte((high * 16 + low) as u8))
            }
            other => {
                let byte = u8::try_from(other as u32)
                    .map_err(|_| RegexError::Unsupported("non-ASCII escape"))?;
                Ok(Node::Byte(byte))
            }
        }
    }

    fn parse_class(&mut self) -> Result<Node, RegexError> {
        self.position += 1;
        let negated = if self.peek() == Some('^') {
            self.position += 1;
            true
        } else {
            false
        };
        let mut ranges = Vec::new();
        let mut first = true;
        while let Some(ch) = self.peek() {
            if ch == ']' && !first {
                self.position += 1;
                return Ok(Node::Class { negated, ranges });
            }
            first = false;
            let low = self.read_class_item()?;
            if self.peek() == Some('-') && self.chars.get(self.position + 1) != Some(&']') {
                self.position += 1;
                let high = self.read_class_item()?;
                ranges.push((low, high));
            } else {
                ranges.push((low, low));
            }
        }
        Err(RegexError::Malformed("missing ]"))
    }

    fn read_class_item(&mut self) -> Result<u8, RegexError> {
        let Some(ch) = self.peek() else {
            return Err(RegexError::Malformed("unexpected end in class"));
        };
        if ch == '\\' {
            self.position += 1;
            return match self.parse_escape()? {
                Node::Byte(byte) => Ok(byte),
                _ => Err(RegexError::Unsupported("class escape")),
            };
        }
        self.position += 1;
        u8::try_from(ch as u32).map_err(|_| RegexError::Unsupported("non-ASCII class literal"))
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(pattern: &str, text: &[u8]) -> Vec<u64> {
        Regex::parse(pattern)
            .expect("parse")
            .find(text, false, false, 256)
    }

    #[test]
    fn literals_dot_and_classes_match() {
        assert_eq!(hits("m.a", b"xxmZa"), vec![2]);
        assert_eq!(hits("[0-9]{3}", b"ab123cd"), vec![2]);
        assert_eq!(hits("[^a-z]+", b"abc123"), vec![3, 4, 5]);
    }

    #[test]
    fn quantifiers_and_alternation_work() {
        assert_eq!(hits("ab+c", b"abbbc"), vec![0]);
        assert_eq!(hits("foo|bar", b"xbar"), vec![1]);
        assert_eq!(hits("colou?r", b"color colour"), vec![0, 6]);
    }

    #[test]
    fn anchors_and_escapes_work() {
        assert_eq!(hits("^abc", b"abcabc"), vec![0]);
        assert_eq!(hits("abc$", b"xxabc"), vec![2]);
        assert_eq!(hits("\\x41\\d", b"ZA1"), vec![1]);
    }

    #[test]
    fn nocase_and_fullword_are_optional() {
        let regex = Regex::parse("hello").expect("parse");
        assert_eq!(regex.find(b"HELLO", true, false, 256), vec![0]);
        assert!(regex.find(b"HELLO", false, false, 256).is_empty());
        assert!(regex.find(b"helloworld", false, true, 256).is_empty());
        assert_eq!(regex.find(b"a hello.", false, true, 256), vec![2]);
    }

    #[test]
    fn malformed_patterns_are_rejected() {
        assert!(matches!(Regex::parse("("), Err(RegexError::Malformed(_))));
        assert!(matches!(Regex::parse("[a-"), Err(RegexError::Malformed(_))));
        assert!(matches!(
            Regex::parse("a{3,1}"),
            Err(RegexError::Malformed(_))
        ));
        assert!(matches!(Regex::parse(""), Err(RegexError::Empty)));
    }
}
