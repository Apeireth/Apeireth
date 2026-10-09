//! IM 快捷接入: 回复分段 / 长度截断策略 (显式口径)。
//!
//! 回复流回 IM 时的硬规则 (顺序执行, 0 隐式丢弃):
//!
//! 1. **段边界**: 先按空行 (`\n\n`) 切段, 段内超预算再按行 (`\n`) 切,
//!    单行仍超预算则按字符硬切 (UTF-8 / 字符边界安全);
//! 2. **单段预算**: 每段字节数 ≤ `max_segment_bytes` (默认取该 kind 的
//!    [`crate::im::channel::ImChannelKind::max_text_bytes`]);
//! 3. **段数预算**: 最多 `max_segments` 段; 超出的尾部整体丢弃并计
//!    `dropped_chars`, 末段追加 `truncation_marker` (追加后仍满足单段预算;
//!    放不下就从末段尾部让位, 让位字符计入丢弃);
//! 4. 空输入 = 0 段 (调用方直接跳过发送)。
//!
//! `truncated` / `dropped_chars` 使截断**可见**: 任何被丢内容都有账。

use serde::{Deserialize, Serialize};

use crate::im::channel::ImChannelKind;

/// 默认最大段数 (超长回复的段数预算)。
pub const DEFAULT_MAX_SEGMENTS: usize = 8;

/// 默认截断标记 (末段尾部追加, 显式宣告丢弃)。
pub const DEFAULT_TRUNCATION_MARKER: &str = "…[已截断]";

/// 分段 / 截断策略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImSegmentPolicy {
    /// 单段字节上限。
    pub max_segment_bytes: usize,
    /// 段数上限。
    pub max_segments: usize,
    /// 截断标记 (追加到末段尾部)。
    pub truncation_marker: String,
}

impl ImSegmentPolicy {
    /// 按 kind 的消息预算取策略 (段数/标记用默认值)。
    pub fn for_kind(kind: ImChannelKind) -> Self {
        Self {
            max_segment_bytes: kind.max_text_bytes(),
            max_segments: DEFAULT_MAX_SEGMENTS,
            truncation_marker: DEFAULT_TRUNCATION_MARKER.to_string(),
        }
    }

    /// 覆盖段数预算。
    pub fn with_max_segments(mut self, max_segments: usize) -> Self {
        self.max_segments = max_segments.max(1);
        self
    }

    /// 覆盖单段字节预算。
    pub fn with_max_segment_bytes(mut self, max_segment_bytes: usize) -> Self {
        self.max_segment_bytes = max_segment_bytes.max(1);
        self
    }

    /// 覆盖截断标记。
    pub fn with_truncation_marker(mut self, marker: impl Into<String>) -> Self {
        self.truncation_marker = marker.into();
        self
    }
}

impl Default for ImSegmentPolicy {
    fn default() -> Self {
        Self::for_kind(ImChannelKind::ImFeishu)
    }
}

/// 一次分段的结果 (截断可见)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImSegmentation {
    /// 分出的段 (每段 ≤ 单段预算, 最多段数预算)。
    pub segments: Vec<String>,
    /// 是否发生截断。
    pub truncated: bool,
    /// 被丢弃的字符数 (含为截断标记让位的字符)。
    pub dropped_chars: usize,
}

/// 按显式策略把回复文本分成可发段落。
pub fn segment_reply(text: &str, policy: &ImSegmentPolicy) -> ImSegmentation {
    if text.is_empty() {
        return ImSegmentation {
            segments: Vec::new(),
            truncated: false,
            dropped_chars: 0,
        };
    }

    let mut pieces: Vec<String> = Vec::new();
    for paragraph in text.split("\n\n") {
        if paragraph.len() <= policy.max_segment_bytes {
            push_piece(&mut pieces, paragraph, policy);
            continue;
        }
        for line in paragraph.split('\n') {
            if line.len() <= policy.max_segment_bytes {
                push_piece(&mut pieces, line, policy);
                continue;
            }
            for chunk in hard_split(line, policy.max_segment_bytes) {
                pieces.push(chunk);
            }
        }
    }

    // 合并相邻小段, 但绝不越过单段预算。
    let mut merged: Vec<String> = Vec::new();
    for piece in pieces {
        match merged.last_mut() {
            Some(last)
                if last.len() + 1 + piece.len() <= policy.max_segment_bytes
                    && !last.contains('\n')
                    && !piece.contains('\n') =>
            {
                last.push('\n');
                last.push_str(&piece);
            }
            _ => merged.push(piece),
        }
    }

    let mut truncated = false;
    let mut dropped_chars = 0usize;
    if merged.len() > policy.max_segments {
        let kept = merged.split_off(policy.max_segments);
        dropped_chars += kept
            .iter()
            .map(|segment| segment.chars().count())
            .sum::<usize>();
        truncated = true;
    }

    if truncated {
        append_marker(&mut merged, policy, &mut dropped_chars);
    }

    ImSegmentation {
        segments: merged,
        truncated,
        dropped_chars,
    }
}

fn push_piece(pieces: &mut Vec<String>, piece: &str, policy: &ImSegmentPolicy) {
    if piece.is_empty() {
        return;
    }
    if piece.len() <= policy.max_segment_bytes {
        pieces.push(piece.to_string());
    } else {
        for chunk in hard_split(piece, policy.max_segment_bytes) {
            pieces.push(chunk);
        }
    }
}

/// 按字符边界硬切 (字节数不超过预算; 不劈开多字节字符)。
fn hard_split(line: &str, max_bytes: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        if !current.is_empty() && current.len() + ch.len_utf8() > max_bytes {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// 末段追加截断标记; 标记放不下就从末段尾部让位 (让位字符计入丢弃)。
fn append_marker(segments: &mut [String], policy: &ImSegmentPolicy, dropped_chars: &mut usize) {
    let Some(last) = segments.last_mut() else {
        return;
    };
    let marker = policy.truncation_marker.as_str();
    if last.len() + marker.len() > policy.max_segment_bytes {
        let budget = policy.max_segment_bytes.saturating_sub(marker.len());
        let mut trimmed = String::new();
        for ch in last.chars() {
            if trimmed.len() + ch.len_utf8() > budget {
                *dropped_chars += 1;
                continue;
            }
            trimmed.push(ch);
        }
        *last = trimmed;
    }
    last.push_str(marker);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_reply_is_a_single_untruncated_segment() {
        let out = segment_reply("你好, 世界", &ImSegmentPolicy::default());
        assert_eq!(out.segments, vec!["你好, 世界"]);
        assert!(!out.truncated);
        assert_eq!(out.dropped_chars, 0);
    }

    #[test]
    fn empty_reply_yields_no_segments() {
        let out = segment_reply("", &ImSegmentPolicy::default());
        assert!(out.segments.is_empty());
        assert!(!out.truncated);
    }

    #[test]
    fn paragraph_and_line_boundaries_drive_the_split() {
        let policy = ImSegmentPolicy::for_kind(ImChannelKind::ImWecom).with_max_segment_bytes(20);
        let out = segment_reply("line one\nline two\n\nsecond paragraph", &policy);
        assert!(out.segments.len() >= 2, "{:?}", out.segments);
        for segment in &out.segments {
            assert!(segment.len() <= 20, "{segment:?}");
        }
        assert!(out.segments.iter().any(|s| s.contains("second paragraph")));
    }

    #[test]
    fn long_word_is_hard_split_on_char_boundaries() {
        let policy = ImSegmentPolicy::default().with_max_segment_bytes(10);
        let word = "超长单行无空格内容超长单行无空格内容";
        let out = segment_reply(word, &policy);
        assert!(out.segments.len() > 1, "{:?}", out.segments);
        for segment in &out.segments {
            assert!(segment.len() <= 10, "{segment:?}");
            assert_eq!(
                segment.chars().count() * 3,
                segment.len(),
                "字符边界必须保持"
            );
        }
        assert!(!out.truncated);
    }

    #[test]
    fn segment_budget_truncates_tail_and_marks_it_explicitly() {
        let policy = ImSegmentPolicy::default()
            .with_max_segment_bytes(8)
            .with_max_segments(2)
            .with_truncation_marker("[cut]");
        let text = "aaaa\nbbbb\ncccc\ndddd";
        let out = segment_reply(text, &policy);
        assert!(out.truncated);
        assert_eq!(out.segments.len(), 2);
        assert!(out.segments[1].ends_with("[cut]"), "{:?}", out.segments);
        assert!(out.dropped_chars >= 4, "{}", out.dropped_chars);
        for segment in &out.segments {
            assert!(segment.len() <= 8, "{segment:?}");
        }
    }

    #[test]
    fn policy_for_each_kind_respects_the_kind_budget() {
        for kind in ImChannelKind::ALL {
            let policy = ImSegmentPolicy::for_kind(kind);
            assert_eq!(policy.max_segment_bytes, kind.max_text_bytes());
            assert_eq!(policy.max_segments, DEFAULT_MAX_SEGMENTS);
            assert_eq!(policy.truncation_marker, DEFAULT_TRUNCATION_MARKER);
        }
    }
}
