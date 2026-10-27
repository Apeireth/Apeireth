//! `apeireth-memory::anchor_injection` — **结构化锚点** + **主体段定位**。
//!
//! # 这个模块解决什么
//!
//! 2026-10-27 的实测（`docs/status/chapter-anchor-value-2026-10-27.md`）给出了
//! 一条**有量化支撑**的正面结论：**「信号从哪来」的答案（第一版）= 用户问句里
//! **已有的两个结构化信息**」**：
//!
//! | 步骤 | 做什么 | 效果 |
//! |---|---|---|
//! | ① **结构化锚点** | 用户说「第12回」⇒ 定位到那一回 | `recall@10` **0.2983 → 1.0000** |
//! | ② **主体段定位** | 题面里的主体 ⇒ 取那一回里**含它的段** | `groundedness` **0.0924 → 0.4202**（**4.55×**） |
//!
//! ⇒ ⇒ **两个信号都来自用户问句** ⇒ ⇒ **全程非 oracle** ⇒ ⇒ **可上线**。
//!
//! # 为什么「结构化锚点」必须**独立成字段**
//!
//! ⚠️ **实测**：语料的**可检索文本 = 正文**（**刻意不含回目联**）⇒
//! ⇒ **正文里根本没有「第12回」这种字符串**
//! ⇒ ⇒ BM25 只能拿到「第」「回」两个**超高频词** ⇒ ⇒ **判别力为零**
//! ⇒ ⇒ 实测：带锚点的题在生产检索下 **recall = 0.0833 / 0.1429**（几乎全错）
//! ⇒ ⇒ **推论**：锚点**必须作为结构化字段参与检索**（塞进正文文本**无效**）。
//!
//! # 三条纪律
//!
//! 1. **不猜**：解析不出来 ⇒ 返回 `None` / 空 `Vec`
//!    ⇒ ⇒ **不许**把「没解析出来」当成「解析成 0」；
//! 2. **不越界**：本模块**只做「从问句里读结构化信息」+「在正文里定位一段」**
//!    ⇒ ⇒ **不碰检索排序**（那是检索器的事）⇒ ⇒ **不碰注入预算**（那是产品口径）；
//! 3. **口径自述**：本模块 = **IMPLEMENTED**（库级）；**生产 0 调用点**
//!    ⇒ ⇒ 对外一律 **NOT IMPLEMENTED（新增未接线）**。

use std::collections::BTreeSet;

/// 从问句里解析出来的**结构化时间锚点**。
///
/// # 为什么要有这个类型
///
/// 「第12回」这种信息**在正文里不存在**（见模块文档）⇒ ⇒ 它只能作为
/// **结构化字段**被检索器使用 ⇒ ⇒ ⇒ 首先要能**可靠地**把它从问句里读出来。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeAnchor {
    /// 「第N回」/「第N章」⇒ **回数/章数**（**1 基**）。
    Chapter(usize),
    /// 「N年前」/「N天後」⇒ **相对量 + 单位**。
    Relative {
        /// 数量。
        amount: u64,
        /// 单位。
        unit: TimeUnit,
    },
    /// 「次日」/「當日」/「昨夜」… ⇒ **固定时间词**。
    Word(String),
}

/// 时间单位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeUnit {
    /// 年。
    Year,
    /// 月。
    Month,
    /// 天。
    Day,
}

impl TimeUnit {
    /// 中文名（**用于报告**）。
    pub fn label(self) -> &'static str {
        match self {
            TimeUnit::Year => "年",
            TimeUnit::Month => "月",
            TimeUnit::Day => "天",
        }
    }
}

/// 固定时间词表（**从问句里识别「时间锚」用**）。
///
/// # 口径
///
/// * 只收**在小说/日记里真的会出现的**相对时间词；
/// * ⚠️ **刻意不含**「早上/晚上」这类**时刻**词
///   ⇒ 它们**不缩小检索范围**（一整天的记录都可能含它）⇒ ⇒ **不算锚点**。
pub const FIXED_TIME_WORDS: &[&str] = &[
    "次日", "翌日", "當日", "同日", "同年", "前日", "後日", "昨夜", "前夜", "上回", "下回", "上月",
    "下月", "開年", "年初", "年底", "前月", "後月",
];

/// 「N年前 / N天後 …」里认的**后缀**。
const RELATIVE_SUFFIXES: &[(&str, TimeUnit)] = &[
    ("年前", TimeUnit::Year),
    ("年後", TimeUnit::Year),
    ("月前", TimeUnit::Month),
    ("月後", TimeUnit::Month),
    ("天前", TimeUnit::Day),
    ("天後", TimeUnit::Day),
    ("日前", TimeUnit::Day),
    ("日後", TimeUnit::Day),
];

/// 解析问句里的**时间锚点**（**全部**都返回；**按出现顺序**、**去重**）。
///
/// # 口径
///
/// * 「第N回」/「第N章」⇒ [`TimeAnchor::Chapter`]（**1 基**）
///   ⇒ ⚠️ **数字只认阿拉伯数字**（`12`）⇒ **中文数字（十二）不认**
///   ⇒ ⇒ **理由**：中文数字要处理「十/二十/廿」等写法，**歧义大**
///   ⇒ ⇒ 且**用户/模型通常写阿拉伯数字**；
/// * 「N年前 / N天後 …」⇒ [`TimeAnchor::Relative`];
/// * 固定时间词（[`FIXED_TIME_WORDS`]）⇒ [`TimeAnchor::Word`];
/// * **解析不出来 ⇒ 返回空 `Vec`**（**不是** `None` ⇒ ⇒ 调用方自己决定怎么用）。
///
/// # ⚠️ 与 `signal_sources::time_anchor_terms` 的**区别**
///
/// 那个函数是**取「可以追加进题面的词」**（面向**词面检索**）；
/// 本函数是**取「结构化锚点」**（面向**字段检索**）
/// ⇒ ⇒ 两者**目标不同** ⇒ ⇒ **不要**互相替代。
pub fn parse_time_anchors(query: &str) -> Vec<TimeAnchor> {
    let chars: Vec<char> = query.chars().collect();
    let mut out: Vec<TimeAnchor> = Vec::new();
    let mut seen: BTreeSet<TimeAnchor> = BTreeSet::new();
    let mut index = 0usize;

    while index < chars.len() {
        // ① 「第N回 / 第N章」
        if chars[index] == '第' {
            let mut cursor = index + 1;
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor > index + 1 && cursor < chars.len() {
                let unit = chars[cursor];
                if unit == '回' || unit == '章' || unit == '節' {
                    let digits: String = chars[index + 1..cursor].iter().collect();
                    if let Ok(number) = digits.parse::<usize>() {
                        let anchor = TimeAnchor::Chapter(number);
                        if seen.insert(anchor.clone()) {
                            out.push(anchor);
                        }
                    }
                    index = cursor + 1;
                    continue;
                }
            }
        }
        // ② 「N年前 / N天後 …」
        if chars[index].is_ascii_digit() {
            let mut cursor = index;
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                cursor += 1;
            }
            let digits: String = chars[index..cursor].iter().collect();
            let amount = digits.parse::<u64>().unwrap_or(0);
            let mut matched = false;
            for (suffix, unit) in RELATIVE_SUFFIXES {
                let suffix_chars: Vec<char> = suffix.chars().collect();
                let end = cursor + suffix_chars.len();
                if end <= chars.len() && chars[cursor..end] == suffix_chars[..] {
                    let anchor = TimeAnchor::Relative {
                        amount,
                        unit: *unit,
                    };
                    if seen.insert(anchor.clone()) {
                        out.push(anchor);
                    }
                    index = end;
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
            // ⚠️ **没匹配上后缀 ⇒ 必须推进游标**（否则 `index` 不变 ⇒ **死循环**）
            // ⇒ （这个坑今天踩过一次：真实语料里有「3個 / 5年」这类数字后不接后缀。）
            index = cursor;
            continue;
        }
        // ③ 固定时间词
        for word in FIXED_TIME_WORDS {
            let word_chars: Vec<char> = word.chars().collect();
            let end = index + word_chars.len();
            if end <= chars.len() && chars[index..end] == word_chars[..] {
                let anchor = TimeAnchor::Word((*word).to_string());
                if seen.insert(anchor.clone()) {
                    out.push(anchor);
                }
                index = end;
                break;
            }
        }
        index += 1;
    }
    out
}

/// 一段**在正文里的位置**（`start` 是**字符偏移**，**不是**字节偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// 起始**字符**偏移。
    pub start: usize,
    /// 取到的文本。
    pub text: String,
}

/// 在正文里**定位含 `subject` 的那一段**（**段内定位**）。
///
/// # 口径
///
/// * 「段」= 以换行分隔的非空片段（**与 `signal_sources::subject_paragraph` 同口径**）；
/// * 命中**第一段**含 `subject` 的 ⇒ 返回该段起点 + **最多 `max_chars` 字**；
/// * ⚠️ **从段首开始取**（不是从 `subject` 出现处）
///   ⇒ ⇒ **理由**：答案句可能在该段的**后半部分** ⇒ 从段首取更全；
/// * `subject` 为空 / 正文为空 ⇒ **`None`**（**不许**返回空串冒充）。
///
/// # ⚠️ 已知局限（**实测**）
///
/// 主体在同一回里**出现多次**时 ⇒ 只会取**第一段** ⇒ ⇒ **未必是答案段**
/// ⇒ ⇒ 实测送达能力 **0.4202**（不是 1.0）⇒ ⇒ **段内精确定位仍需更强的信号**。
pub fn find_subject_span(text: &str, subject: &str, max_chars: usize) -> Option<Span> {
    if text.is_empty() || subject.is_empty() || max_chars == 0 {
        return None;
    }
    let mut start = 0usize;
    for paragraph in text.split('\n') {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() && trimmed.contains(subject) {
            // ⚠️ **不要**在这里覆盖 `start`（它已经是**正确偏移**）
            // ⇒ 早先版本误写成「段长」⇒ 注入落在错位置 ⇒ 结论完全反过来。
            let chars: Vec<char> = text.chars().collect();
            let start = start.min(chars.len());
            let end = (start + max_chars).min(chars.len());
            return Some(Span {
                start,
                text: chars[start..end].iter().collect(),
            });
        }
        start += paragraph.chars().count() + 1;
    }
    None
}

/// 取出正文里**第 `chapter` 回**对应的文本（**按调用方给的映射**）。
///
/// # 为什么用回调而不是内置编号
///
/// 「第N回」⇒ 哪条记录，**取决于语料**（小说有回目、日记有日期、数据库有 id）
/// ⇒ ⇒ **本模块不猜** ⇒ ⇒ 由调用方给映射 ⇒ ⇒ **口径不漂**。
pub fn chapter_text<F>(texts: &[String], chapter: usize, chapter_of: F) -> Option<String>
where
    F: Fn(&str) -> Option<usize>,
{
    texts
        .iter()
        .find(|text| chapter_of(text) == Some(chapter))
        .cloned()
}

/// 一条**锚定证据**（**定位 + 送达**都做完了）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchoredEvidence {
    /// 定位到的**章/回数**。
    pub chapter: usize,
    /// 注入的文本（**主体段**；没定位到段 ⇒ 退回**开头 `fallback_chars` 字**）。
    pub evidence: String,
    /// 有没有**真的**定位到主体段（`false` ⇒ 用的是开头兜底）。
    pub subject_span_found: bool,
}

/// 为一条**已定位**的记录构造证据。
///
/// # 口径
///
/// * `subject` 能在正文里定位到段 ⇒ 取该段（最多 `max_chars` 字）⇒ `subject_span_found = true`；
/// * 定位不到 ⇒ **退回开头 `max_chars` 字**（**不返回空** ⇒ ⇒ 宁可弱也不空）
///   ⇒ ⇒ `subject_span_found = false`（**调用方要能看见这一点**）。
pub fn build_anchored_evidence(
    text: &str,
    chapter: usize,
    subject: &str,
    max_chars: usize,
) -> AnchoredEvidence {
    match find_subject_span(text, subject, max_chars) {
        Some(span) => AnchoredEvidence {
            chapter,
            evidence: span.text,
            subject_span_found: true,
        },
        None => AnchoredEvidence {
            chapter,
            evidence: text.chars().take(max_chars).collect(),
            subject_span_found: false,
        },
    }
}

/// 人话报告（**锚点 + 段定位**）。
pub fn render_anchor_report(
    query: &str,
    anchors: &[TimeAnchor],
    evidence: &AnchoredEvidence,
    subject: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("结构化锚点 · 证据构造".to_string());
    lines.push(String::new());
    lines.push(format!("  问句：{query}"));
    lines.push(format!("  主体：{subject}"));
    lines.push(format!("  解析到的锚点：{}", anchors.len()));
    for anchor in anchors {
        let label = match anchor {
            TimeAnchor::Chapter(number) => format!("回数/章数 = {number}"),
            TimeAnchor::Relative { amount, unit } => {
                format!("相对时间 = {amount} {}", unit.label())
            }
            TimeAnchor::Word(word) => format!("固定时间词 = {word}"),
        };
        lines.push(format!("    · {label}"));
    }
    if anchors.is_empty() {
        lines.push("    （**没解析出来** ⇒ 不是「解析成 0」）".to_string());
    }
    lines.push(String::new());
    lines.push(format!(
        "  定位到的章/回：{} · 证据字数：{}",
        evidence.chapter,
        evidence.evidence.chars().count()
    ));
    lines.push(format!(
        "  主体段定位：{}",
        if evidence.subject_span_found {
            "**命中**（注入的是含主体的那一段）"
        } else {
            "**没命中** ⇒ 退回开头兜底（**不空**）"
        }
    ));
    lines.push(String::new());
    lines.push("  ⚠️ 口径：".to_string());
    lines.push(
        "    · 结构化锚点**必须作为独立字段**参与检索（正文里没有「第N回」字符串）".to_string(),
    );
    lines.push("    · 主体段定位**取第一段** ⇒ 主体出现多次时**未必是答案段**".to_string());
    lines.join("\n")
}
