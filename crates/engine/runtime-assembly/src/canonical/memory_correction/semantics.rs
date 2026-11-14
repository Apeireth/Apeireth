//! Finite source assertions for automatic memory correction.
//!
//! The grammar is closed on purpose. A correction may only rewrite a value
//! that one clause of the current user states for one property. This module
//! owns the property lexicon, the value grammar, the scope rules and the
//! whole-source consistency check that the correction module relies on.

use std::ops::Range;

use super::{absolute_date_prefix, assertion_body};

/// Clause and sentence separators.
const BOUNDARY: [char; 8] = ['，', ',', '。', '！', '!', '；', ';', '\n'];
/// Separators that end a sentence.
const SENTENCE_BOUNDARY: [char; 6] = ['。', '！', '!', '；', ';', '\n'];
/// Adverbs that carry no subject of their own.
const ADVERBS: [&str; 20] = [
    "其实", "已经", "刚刚", "刚才", "现在", "目前", "之前", "此前", "原来", "本来", "曾经", "实际",
    "今天", "昨天", "前天", "常年", "一直", "确实", "已", "刚",
];
/// Adverbs that place the fact before the current state.
const HISTORICAL_ADVERBS: [&str; 5] = ["之前", "此前", "原来", "本来", "曾经"];
/// Adverbs that place the fact in the current state.
const CURRENT_ADVERBS: [&str; 6] = ["现在", "目前", "刚刚", "刚才", "已经", "已"];
/// Relative date expressions.
const RELATIVE_DATES: [&str; 3] = ["今天", "昨天", "前天"];
/// Markers of reported, quoted, hypothetical or illustrative text.
const FRAME_MARKERS: [&str; 12] = [
    "表示", "声称", "提到", "写道", "转述", "引用", "假设", "假如", "如果", "比如", "例如", "设想",
];
/// The user admits that an earlier statement was wrong.
const SELF_SPEECH: [&str; 4] = ["我说错", "我之前说错", "我刚才说错", "我记错"];
/// Prefixes that deny a statement.
const NEGATIONS: [&str; 3] = ["不再", "没有", "不"];
/// Verbs that introduce the literal value of a named property.
const VALUE_CONNECTORS: [&str; 10] = [
    "更正为",
    "纠正为",
    "修正为",
    "改为",
    "改成",
    "换成",
    "变成",
    "是",
    "为",
    "在",
];
/// Verbs that introduce a transition from a previous value.
const TRANSITION_CONNECTORS: [&str; 5] = ["改为", "改成", "换成", "变成", "变更为"];
/// Characters that may follow a named property.
const PROPERTY_TAILS: [char; 22] = [
    '是', '为', '在', '改', '更', '换', '变', '调', '需', '已', '现', '目', '之', '原', '从', '不',
    '其', '有', '记', '写', '：', ':',
];
/// Named properties of the user. The order decides which label wins.
const ATTRIBUTES: [(&str, &str); 27] = [
    ("居住城市", "residence_city"),
    ("常住城市", "residence_city"),
    ("城市", "residence_city"),
    ("工作地点", "work_location"),
    ("工作单位", "employer"),
    ("电子邮箱", "email"),
    ("邮件地址", "email"),
    ("电话号码", "phone"),
    ("手机号码", "phone"),
    ("手机号", "phone"),
    ("联系方式", "contact"),
    ("语言偏好", "language_preference"),
    ("常住地", "address"),
    ("居住地", "address"),
    ("现居地", "address"),
    ("地址", "address"),
    ("住址", "address"),
    ("名字", "name"),
    ("姓名", "name"),
    ("昵称", "nickname"),
    ("邮箱", "email"),
    ("时区", "timezone"),
    ("职业", "occupation"),
    ("学校", "school"),
    ("专业", "major"),
    ("生日", "birthday"),
    ("偏好", "preference"),
];
/// Predicate verbs with the property they state, whether they carry a
/// literal value, and whether they mark a move.
const PREDICATES: [(&str, &str, bool, bool); 24] = [
    ("名叫", "name", true, false),
    ("叫", "name", true, false),
    ("就职于", "employer", true, false),
    ("任职于", "employer", true, false),
    ("供职于", "employer", true, false),
    ("就读于", "school", true, false),
    ("上学于", "school", true, false),
    ("居住在", "residence", true, false),
    ("常住在", "residence", true, false),
    ("住在", "residence", true, false),
    ("现居于", "residence", true, false),
    ("居住", "residence", true, false),
    ("常住", "residence", true, false),
    ("现居", "residence", true, false),
    ("搬到了", "residence", true, true),
    ("搬到", "residence", true, true),
    ("搬去了", "residence", true, true),
    ("搬去", "residence", true, true),
    ("迁居到", "residence", true, true),
    ("迁居", "residence", true, true),
    ("移居到", "residence", true, true),
    ("移居", "residence", true, true),
    ("搬进新家", "residence", false, false),
    ("搬家", "residence", false, false),
];

/// Whether a clause states or denies a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Polarity {
    Positive,
    Negative,
}

/// One self statement of the source text.
#[derive(Debug, Clone)]
pub(super) struct ParsedAssertion {
    /// Byte range of the clause inside the source text.
    pub span: Range<usize>,
    /// Property the clause states, when the clause is a self statement.
    pub property: Option<&'static str>,
    /// Whether the clause denies the value.
    pub polarity: Polarity,
    /// Byte range of the literal value inside the source text.
    pub value: Option<Range<usize>>,
    /// Byte range of the literal value the clause replaces.
    pub previous_value: Option<Range<usize>>,
    /// Byte range of the time expression bound to the clause.
    pub effective_expression: Option<Range<usize>>,
    /// The clause places the fact before the current state.
    pub historical: bool,
    /// The clause states the fact for the current state.
    pub explicit_current: bool,
    /// The clause carries a relative date expression.
    pub relative_expression: bool,
    /// The clause states the correction the user is making now.
    pub in_correction_region: bool,
}

/// Byte range of a part inside the text it was taken from.
fn offset_in(text: &str, part: &str) -> Range<usize> {
    let start = part.as_ptr() as usize - text.as_ptr() as usize;
    start..start + part.len()
}

/// Byte range of the first occurrence of a needle inside a slice of the text.
fn find_in(text: &str, haystack: &str, needle: &str) -> Option<Range<usize>> {
    let at = haystack.find(needle)?;
    Some(offset_in(text, &haystack[at..at + needle.len()]))
}

/// Clauses of the text with their byte ranges.
fn clauses(text: &str) -> Vec<(&str, Range<usize>)> {
    let mut found = Vec::new();
    let mut start = 0;
    for part in text.split_inclusive(BOUNDARY) {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let at = start + offset_in(part, trimmed).start;
            found.push((trimmed, at..at + trimmed.len()));
        }
        start += part.len();
    }
    found
}

/// Remove leading adverbs. The flags record the time frame they state.
pub(super) fn strip_adverbs(rest: &str) -> (&str, bool, bool) {
    let mut rest = rest;
    let mut historical = false;
    let mut explicit_current = false;
    for _ in 0..16 {
        let Some(adverb) = ADVERBS.iter().find(|prefix| rest.starts_with(**prefix)) else {
            break;
        };
        if HISTORICAL_ADVERBS.contains(adverb) {
            historical = true;
        }
        if CURRENT_ADVERBS.contains(adverb) {
            explicit_current = true;
        }
        rest = rest[adverb.len()..].trim_start();
    }
    (rest, historical, explicit_current)
}

/// Named property of the user and the text that follows the label.
pub(super) fn attribute_tail(rest: &str) -> Option<(&'static str, &str)> {
    ATTRIBUTES.iter().find_map(|(label, key)| {
        let after = rest.strip_prefix(label)?.trim_start();
        (after.is_empty() || after.starts_with(PROPERTY_TAILS)).then_some((*key, after))
    })
}

/// Property of a named attribute, before normalization.
pub(super) fn attribute_property(rest: &str) -> Option<&'static str> {
    attribute_tail(rest).map(|(property, _)| property)
}

/// Properties that state the same fact share one key.
pub(super) fn normalized_property(property: &'static str) -> &'static str {
    if ["address", "residence_city"].contains(&property) {
        "residence"
    } else {
        property
    }
}

/// Possessive syntax is accepted only for an explicit property of this user.
pub(super) fn self_attribute(text: &str) -> Option<&'static str> {
    let rest = text
        .trim()
        .strip_prefix("我的")
        .or_else(|| text.trim().strip_prefix("本人的"))?;
    attribute_property(rest)
}

/// Remove a negation prefix and report whether the clause denies its value.
fn strip_negation(rest: &str) -> (&str, bool) {
    match NEGATIONS
        .iter()
        .find_map(|negation| rest.strip_prefix(negation))
    {
        Some(after) => (after.trim_start(), true),
        None => (rest, false),
    }
}

/// Remove a time modifier and report the date it binds.
fn strip_date_modifier(rest: &str) -> (&str, Option<&str>) {
    for prefix in ["于", "从", "自"] {
        let Some(dated) = rest.strip_prefix(prefix) else {
            continue;
        };
        let Some((_, after)) = absolute_date_prefix(dated) else {
            continue;
        };
        let date = &dated[..dated.len() - after.len()];
        let after = after.trim_start();
        let after = if prefix == "于" {
            after
        } else {
            // 从 and 自 mark the start of a state and are followed by 起.
            match after.strip_prefix('起') {
                Some(rest) => rest.trim_start(),
                None => continue,
            }
        };
        return (after, Some(date));
    }
    (rest, None)
}

/// True when the text carries 从 or 自 followed by an absolute date and 起.
fn absolute_start_modifier(text: &str) -> bool {
    for prefix in ["从", "自"] {
        let Some(dated) = text.strip_prefix(prefix) else {
            continue;
        };
        let Some((_, after)) = absolute_date_prefix(dated) else {
            continue;
        };
        if after.trim_start().starts_with('起') {
            return true;
        }
    }
    false
}

/// True when the text carries an absolute start modifier anywhere.
pub(super) fn has_absolute_date_modifier(text: &str) -> bool {
    text.match_indices(['从', '自'])
        .any(|(at, _)| absolute_start_modifier(&text[at..]))
}

/// True when a prefix marks the start of a state and 起 follows it.
pub(super) fn is_start_modifier(prefix: &str, after: &str) -> bool {
    ["从", "自"].contains(&prefix) && after.trim_start().starts_with('起')
}

/// Split a transition of the form 从 previous 改为 value.
fn split_transition(rest: &str) -> Option<(&str, &str)> {
    let after_from = rest.strip_prefix('从')?;
    for connector in TRANSITION_CONNECTORS {
        if let Some((previous, value)) = after_from.split_once(connector) {
            let previous = previous.trim();
            if previous.is_empty() {
                return None;
            }
            return Some((previous, value.trim()));
        }
    }
    None
}

/// Literal value that follows a value connector.
fn strip_value_connector(rest: &str) -> Option<&str> {
    VALUE_CONNECTORS
        .iter()
        .find_map(|connector| rest.strip_prefix(connector))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Parse a named property clause. The base is the clause the ranges refer to.
fn parse_attribute(base: &str, rest: &str, parsed: &mut ParsedAssertion) -> bool {
    let Some((key, tail)) = attribute_tail(rest) else {
        return false;
    };
    parsed.property = Some(normalized_property(key));
    let (tail, denied) = strip_negation(tail);
    if denied {
        parsed.polarity = Polarity::Negative;
    }
    let dated = tail;
    let (tail, date) = strip_date_modifier(dated);
    if let Some(date) = date {
        parsed.effective_expression = find_in(base, dated, date);
    }
    if let Some((previous, value)) = split_transition(tail) {
        parsed.previous_value = find_in(base, tail, previous);
        parsed.value = find_in(base, tail, value);
        return true;
    }
    let (tail, _, _) = strip_adverbs(tail);
    if let Some(value) = strip_value_connector(tail) {
        parsed.value = find_in(base, tail, value);
    }
    true
}

/// Parse a predicate clause. The base is the clause the ranges refer to.
fn parse_predicate(base: &str, rest: &str, parsed: &mut ParsedAssertion) -> bool {
    let (rest, denied) = strip_negation(rest);
    if denied {
        parsed.polarity = Polarity::Negative;
    }
    let dated = rest;
    let (rest, date) = strip_date_modifier(dated);
    if let Some(date) = date {
        parsed.effective_expression = find_in(base, dated, date);
    }
    for (verb, property, carries_value, moved) in PREDICATES {
        let Some(after) = rest.strip_prefix(verb) else {
            continue;
        };
        parsed.property = Some(property);
        if carries_value {
            let value = after.trim();
            let value = if moved {
                value.strip_suffix('了').unwrap_or(value).trim_end()
            } else {
                value
            };
            if !value.is_empty() {
                parsed.value = find_in(base, rest, value);
            }
        }
        return true;
    }
    false
}

/// Parse one clause of the source into a self assertion.
fn parse_clause(clause: &str) -> Option<ParsedAssertion> {
    let body = assertion_body(clause);
    let body = body.trim().trim_end_matches(BOUNDARY).trim();
    if body.is_empty() || body.contains(['?', '？']) {
        return None;
    }
    let mut parsed = ParsedAssertion {
        span: 0..0,
        property: None,
        polarity: Polarity::Positive,
        value: None,
        previous_value: None,
        effective_expression: None,
        historical: false,
        explicit_current: false,
        relative_expression: false,
        in_correction_region: true,
    };
    let subject = body
        .strip_prefix("我的")
        .or_else(|| body.strip_prefix("本人的"))
        .or_else(|| body.strip_prefix('我'))
        .or_else(|| body.strip_prefix("本人"))?
        .trim_start();
    let relative = RELATIVE_DATES
        .iter()
        .find_map(|date| find_in(clause, subject, date));
    let (rest, historical, explicit_current) = strip_adverbs(subject);
    parsed.historical = historical;
    parsed.explicit_current = explicit_current;
    if let Some(relative) = relative {
        parsed.relative_expression = true;
        parsed.effective_expression = Some(relative);
    }
    // The possessive marker may follow the subject: 我现在的电子邮箱.
    let rest = if let Some(rest) = rest
        .strip_prefix("使用的")
        .or_else(|| rest.strip_prefix("用的"))
    {
        rest.trim_start()
    } else if let Some(rest) = rest.strip_prefix('的') {
        rest.trim_start()
    } else {
        rest
    };
    let (rest, historical, explicit_current) = strip_adverbs(rest);
    parsed.historical = parsed.historical || historical;
    parsed.explicit_current = parsed.explicit_current || explicit_current;
    if parse_attribute(clause, rest, &mut parsed) {
        return Some(parsed);
    }
    parse_predicate(clause, rest, &mut parsed).then_some(parsed)
}

/// Move a clause relative range into the coordinate system of the source.
fn shift(range: &mut Option<Range<usize>>, offset: usize) {
    if let Some(range) = range {
        range.start += offset;
        range.end += offset;
    }
}

/// True when the clause opens with a correction marker.
fn has_correction_marker(clause: &str) -> bool {
    let trimmed = clause.trim_start();
    ["更正：", "纠正：", "更正:", "纠正:"]
        .iter()
        .any(|marker| trimmed.starts_with(marker))
}

/// All self assertions of the source text.
pub(super) fn parse(source: &str) -> Vec<ParsedAssertion> {
    let parts = clauses(source);
    // Statements before the first correction marker state the prior fact.
    // Only the statements of the correction itself may contradict each other.
    let marker_at = parts
        .iter()
        .position(|(clause, _)| has_correction_marker(clause));
    parts
        .into_iter()
        .enumerate()
        .filter_map(|(index, (clause, span))| {
            let mut parsed = parse_clause(clause)?;
            parsed.span = span.clone();
            parsed.in_correction_region = marker_at.is_none_or(|at| index >= at);
            shift(&mut parsed.value, span.start);
            shift(&mut parsed.previous_value, span.start);
            shift(&mut parsed.effective_expression, span.start);
            Some(parsed)
        })
        .collect()
}

/// True when the text states a fact of this user.
pub(super) fn parses_as_self(text: &str) -> bool {
    clauses(text)
        .into_iter()
        .any(|(clause, _)| parse_clause(clause).is_some())
}

/// Property the clause states.
pub(super) fn property_of(text: &str) -> Option<&'static str> {
    parse_clause(text).and_then(|parsed| parsed.property)
}

/// Literal value the clause states for a property.
pub(super) fn value_of<'a>(text: &'a str, property: &str) -> Option<&'a str> {
    let parsed = parse_clause(text)?;
    if parsed.property != Some(property) {
        return None;
    }
    let value = parsed.value?;
    Some(&text[value])
}

/// True when the clause states its value instead of denying it.
pub(super) fn is_positive_assertion(text: &str) -> bool {
    parse_clause(text)
        .map(|parsed| parsed.polarity == Polarity::Positive)
        .unwrap_or(false)
}

/// True when the text before the clause may carry mutation authority.
pub(super) fn scope_prefix_allows(source: &str, at: usize) -> bool {
    let prefix = &source[..at];
    // Frame markers reach over the whole text before the clause.
    if FRAME_MARKERS.iter().any(|marker| prefix.contains(marker)) {
        return false;
    }
    // Reported speech reaches over the sentence that holds the clause only.
    let scope_start = prefix
        .char_indices()
        .rev()
        .find(|(_, ch)| SENTENCE_BOUNDARY.contains(ch))
        .map_or(0, |(at, ch)| at + ch.len_utf8());
    let scope = &prefix[scope_start..];
    !scope.contains('说') || SELF_SPEECH.iter().any(|cue| scope.contains(cue))
}

/// True when the whole source agrees with the proposed correction.
///
/// A transition must start from the value the proposal replaces, and two
/// current statements of one property may not contradict each other.
pub(super) fn source_is_consistent(
    source: &str,
    property: &str,
    _new_value: &str,
    old_value: &str,
) -> bool {
    let assertions = parse(source);
    for assertion in &assertions {
        if assertion.property != Some(property) {
            continue;
        }
        let Some(previous) = &assertion.previous_value else {
            continue;
        };
        if source[previous.clone()].trim() != old_value.trim() {
            return false;
        }
    }
    let current: Vec<&ParsedAssertion> = assertions
        .iter()
        .filter(|assertion| {
            assertion.property == Some(property)
                && assertion.in_correction_region
                && !assertion.historical
                && (assertion.effective_expression.is_none()
                    || assertion.explicit_current
                    || assertion.relative_expression)
        })
        .collect();
    for (index, first) in current.iter().enumerate() {
        for second in current.iter().skip(index + 1) {
            let first_value = first
                .value
                .as_ref()
                .map(|range| source[range.clone()].trim());
            let second_value = second
                .value
                .as_ref()
                .map(|range| source[range.clone()].trim());
            let (Some(left), Some(right)) = (first_value, second_value) else {
                continue;
            };
            if left != right && first.polarity == second.polarity {
                return false;
            }
            if left == right && first.polarity != second.polarity {
                return false;
            }
        }
    }
    true
}
