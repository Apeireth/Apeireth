//! `apeireth-memory::signal_sources` — **非答案侧**的题面信号来源（端到端的那一半）。
//!
//! # 为什么
//!
//! W-30 实测：题面加一个判别信号 ⇒ `recall@10` **0.2983 → 0.6113**（≈ 翻倍）。
//! ⚠️ **但那是 oracle 上界** —— 加进题面的信号**来自答案片段**（答案侧信息）
//! ⇒ ⇒ 证明的是「**信号到位之后**能涨多少」，**不是**「这套题面就能上线」。
//!
//! # 本批要回答的问题
//!
//! **信号从哪来？** 且 **不能泄漏答案** ⇒
//! 信号必须来自「**提问时就已知**」的信息（历史、共现、时间、上下文…）
//! ⇒ **不是**来自「答案本身」。
//!
//! # 三条纪律
//!
//! 1. **不许泄漏答案**：信号来源**不得**读期望片段/答案字段
//!    ⇒ 本模块的输入**只有**「语料 + 题面」（**没有**期望片段）
//!    ⇒ ⇒ 这从**接口**上杜绝了泄漏；
//! 2. **对照实验**：每个来源都报 `recall@k` 与基线的差（**不许**只报判别力）；
//! 3. **如实**：没用的来源**照样列出来**并标明「**实测无效**」（不许藏）。
//!
//! # 本模块的**实现口径**（自创部分钉死在这里，别在别处自创）
//!
//! * **主体** = 题面里**最长**的实词。
//!   ⚠️ **口径与 `query_signals::parse_signals` 同源**，但**显式**把期望片段置空
//!   （见 [`subject_of`]）⇒ 主体判定**只**看题面 ⇒ **答案侧信息进不来**。
//! * **语料侧实词** = `query_signals::tokenize`（CJK bigram + ASCII 词，**同口径**）
//!   再过一道**轻量实词过滤**（≥2 字 / 非纯数字 / 不含高频虚词字 / 不以数量字开头 /
//!   不在泛称表里）⇒ 只用于**统计候选池**（共现、开头句），**不用于主体判定**。
//! * **信号词进题面的方式** = 在原问句末尾追加 `（词1、词2）`（**原问句逐字保留**）
//!   ⇒ 与 W-30 的改写格式**逐字一致** ⇒ 两批的数字可比。
//! * **每个来源最多加 3 个词**（[`MAX_TERMS_PER_SOURCE`]）⇒ **所有来源统一**
//!   ⇒ **不许**为某个来源调权重凑分数。
//!
//! # 「切词切出来的碎片」要**单独剔掉**（[`subject_fragments`]）
//!
//! 切词是「**相邻两字**」⇒ 「寶玉敘話」会切出「玉敘」这种**根本不是词**的东西。
//! 它在语料里**只出现在主体旁边** ⇒ 共现统计里会**冒充**「强共现词」
//! ⇒ **必须**按**边界**剔掉。
//! ⚠️ **不能**按「含主体的字」剔（那会误伤「寶釵」这类**真**的同字词）
//! ⇒ 只剔**与主体的出现位置重叠**的那些 bigram。
//!
//! # 「定位到哪条记录」的两条路（**都要说清**）
//!
//! * [`SourceKind::RecordHead`] / [`SourceKind::ParagraphHead`] 用的是
//!   **伪相关反馈（PRF）**：先拿**原题面**检索一次 ⇒ 取 **top-1** 记录
//!   ⇒ 用它的开头句扩充题面 ⇒ 再检索。
//!   ⚠️ 这是**两遍检索**的经典做法，**不含**任何答案侧信息。
//! * ⚠️ **数据集构造红利**：本批的题是**从记录里挖出来的**（主体 = 该记录的最高频实体）
//!   ⇒ 「主体最突出的那条记录」天然偏向**答案记录**
//!   ⇒ 这两个来源的数字含**构造红利**，**不等于**真实用户提问的收益。
//!   （报告里必须带这一条提醒。）
//!
//! # 状态：NOT IMPLEMENTED（新增未接线）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、计算、报告齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（生产 0 调用点）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✔（部分）：已在《紅樓夢》《水滸傳》**真实语料**上跑过
//!   （2026-10-27，见 `examples/signal_source_report.rs`），**未在第二台机器复现**。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::golden_run::{IndexRecord, InjectionMode};
use crate::golden_set::{GoldenCase, GoldenSet};
use crate::hybrid_search::{Bm25Config, Bm25Index};

/// 每个来源最多往题面里追加几个词。
///
/// # 为什么所有来源**统一**上限
///
/// 「加几个词」本身就是个**可以调**的旋钮 ⇒ 若每个来源各配一个上限，
/// 分数就变成了「上限调出来的」而不是「信号本身的价值」。
/// ⇒ **统一 3 个** ⇒ 唯一的差别只剩「**取什么词**」。
pub const MAX_TERMS_PER_SOURCE: usize = 3;

/// 同主题历史：取记录开头多少字做词频统计。
///
/// # 为什么取「开头若干字」而不是整条
///
/// 整条记录 ⇒ 词频会被**中段**的剧情词主导 ⇒ 那已经接近「答案片段」了（答案片段
/// 就是从记录里裁的）⇒ 有**泄漏味道**。⇒ 只看**开头**（题面之外、答案之外的固定窗口）。
const HISTORY_HEAD_CHARS: usize = 60;

/// 共现：词至少出现在这么多条记录里（⇒ 不是偶然的一次性搭配）。
const COOC_MIN_DF: usize = 3;

/// 共现：强度下限（= 与主体共现的记录数 ÷ 该词自身的记录数）。
const COOC_MIN_STRENGTH: f64 = 0.5;

/// 共现：词自身的记录数上限（占语料的比例）⇒ 不许取「泛词」。
const COOC_MAX_DF_RATIO: f64 = 0.25;

/// 本模块要**逐个量化**的来源清单（**顺序固定** ⇒ 报告可比）。
///
/// ⚠️ **不含** [`SourceKind::Baseline`]（那是对照组，不是来源）。
pub const SOURCE_KINDS: &[SourceKind] = &[
    SourceKind::SameSubjectHistory,
    SourceKind::Cooccurrence,
    SourceKind::TimeAnchor,
    SourceKind::ParagraphHead,
    SourceKind::RecordHead,
];

/// 一个**信号来源**（一种非答案侧的信息来源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    /// 同主题历史：从**同一主体**的其它记录里取词。
    SameSubjectHistory,
    /// 实体共现：语料里与主体**强共现**的词（纯统计，**不含答案**）。
    Cooccurrence,
    /// 时间锚：从题面/记录里取时间信号。
    TimeAnchor,
    /// 段落首句：目标段落的**开头**（需切分）。
    ParagraphHead,
    /// 原文首句：整条记录的**开头**。
    RecordHead,
    /// **基线**（原题面，逐字不改）—— **不是**一种来源，是**对照组**那一行。
    ///
    /// ⚠️ **偏离契约的一处（待裁决）**：契约给的 `SourceKind` 只有五个变体，
    /// 而测试要求「**基线在结果里**」⇒ 基线必须**占一行** ⇒ 只能给它一个变体。
    /// ⇒ 契约里的**五个变体一字未改**，这里是**新增**一个。
    /// ⇒ 若裁决要求枚举**逐字**照契约：改法是把 [`compare_sources`] 里的基线行去掉，
    ///   基线改由 `delta` 反推（`baseline = recall_at_k - delta`），
    ///   [`render_source_report`] 的 `baseline_recall` 参数**本来就是必填** ⇒ 报告不受影响。
    Baseline,
}

impl SourceKind {
    /// 来源的**中文名**（报告里显示；**固定** ⇒ 报告可比）。
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::SameSubjectHistory => "同主题历史",
            SourceKind::Cooccurrence => "实体共现",
            SourceKind::TimeAnchor => "时间锚",
            SourceKind::ParagraphHead => "段落首句",
            SourceKind::RecordHead => "原文首句",
            SourceKind::Baseline => "基线",
        }
    }
}

/// 一种来源的**产出**（用于**加进题面**的词）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSignal {
    /// 来源种类。
    pub kind: SourceKind,
    /// 取到的词（**有序** ⇒ 确定性；**最多 3 个**）。
    pub terms: Vec<String>,
}

/// 一种来源的**实测效果**。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceOutcome {
    /// 来源种类。
    pub kind: SourceKind,
    /// 该来源下的 `recall@k`（**真的跑检索**出来的）。
    pub recall_at_k: f64,
    /// 相对基线的差（**正数 = 有用**）。
    pub delta: f64,
    /// **用了这个来源的题数**（**分母** ⇒ 越低说明来源越挑剔）。
    ///
    /// # 口径
    ///
    /// = **该来源的前置条件满足**的题数（= 这个来源**能出手**的题数）。
    /// ⚠️ 它**不等于**「真的改了题面」的题数 ⇒ 后者是 [`SourceOutcome::questions_with_terms`]。
    pub questions_covered: usize,
    /// **该来源在多少题上真的给出了词**（`questions_covered` 的子集）。
    ///
    /// ⚠️ 这个数才是「**真的改写了题面**」的题数 ⇒ 「取不到词」的题，
    /// 题面**逐字不改**（按基线计）⇒ 分数**不因为「没词」而虚高/虚低**。
    pub questions_with_terms: usize,
}

/// 从**语料**里算「与某主体强共现的词」（**纯统计，答案侧无关**）。
///
/// # 口径
///
/// * 主体 = 题面里最长的实词（与 `query_signals::parse_signals` 同口径）；
/// * 共现 = 语料里与主体**出现在同一条记录**里的其它实词；
/// * 强度 = 与主体共现的记录数 ÷ 该词自身的记录数（**条件概率式**）
///   ⇒ **只取强度高且自身不算泛的词**；
/// * ⚠️ **不许**用期望片段/答案字段（本函数的输入里**根本没有**）。
///
/// # 筛选与排序（**钉死** ⇒ 分数可复核）
///
/// * `df(term)` = 该词出现在多少条记录里（token 口径 ⇒ 与 W-30 的「记录正文含有该词」等价）；
/// * `co(term)` = 其中**同时**提到主体的记录数；
/// * **入选条件**：`df ≥ 3`（不是偶然）、`strength ≥ 0.5`（与主体强共现）、
///   `df ≤ 语料条数 × 0.25`（**自身不算泛**）；
/// * **排序**：`df` 降序 → `strength` 降序 → 词长降序 → 字典序升序。
///
/// # 为什么用 `df` 降序而不是 `strength` 降序（**排序规则的理由**）
///
/// 「强度高」的词往往是**窄词**（只出现在两三条记录里）⇒ 把它加进题面，
/// 只会把那两三条记录顶上来 ⇒ 而那两三条**多半不是目标记录**
/// （主体出现在几十条记录里，随机两条命中目标的概率极低）
/// ⇒ **候选集被缩到与答案无关的地方** ⇒ 目标被挤出 `top-k` ⇒ **反而掉分**。
/// ⇒ `strength` 当**入选条件**（要求与主体强共现），`df` 当**排序依据**
/// （覆盖广 ⇒ 大概率也覆盖目标记录）⇒ 加进去的词给目标**抬分而不误伤**。
///
/// ⚠️ 这个排序规则是**先验定下的**（按上面的机制推理），**不是**照着分数调出来的。
pub fn cooccurrence_terms(
    subject: &str,
    records: &[(String, String)],
    max_terms: usize,
) -> Vec<String> {
    let index = CorpusIndex::build(records);
    cooccurrence_terms_with(&index, records, subject, records.len(), max_terms)
}

/// 从**同主题历史**取词（同主体记录的**开头**片段里出现最多的实词）。
///
/// # 口径
///
/// * 找出所有**提到该主体**的记录（**不含**期望片段 ⇒ 无泄漏）；
/// * 取这些记录**开头**若干字的实词；
/// * 按**词频**排序 ⇒ 取前 `max_terms` 个。
///
/// # 「词频」的口径
///
/// = 该词在**多少条**同主体记录的**开头**里出现过（每条记一次）
/// ⇒ 不是「出现多少次」⇒ 一个开头里重复十遍的词不会压过十条不同记录都提到的词。
///
/// # 排序
///
/// 词频降序 → 词长降序 → 字典序升序（**确定性**）。
///
/// # 为什么要排除主体本身
///
/// 主体就在题面里 ⇒ 再加一遍**不增加信息** ⇒ 还可能让 BM25 的词频偏置失真
/// ⇒ 排除（连同**跨过主体边界**的碎片一起排除，见 [`subject_fragments`]）。
pub fn same_subject_history_terms(
    subject: &str,
    records: &[(String, String)],
    max_terms: usize,
) -> Vec<String> {
    let indices = subject_record_indices(records, subject);
    same_subject_history_terms_with(records, &indices, subject, max_terms)
}

/// 取**时间锚**（从题面/记录里找「第 N 回」「某年某月」这类时间词）。
///
/// # 口径
///
/// * 题面里的「第 N 回」「N 年前」「次日」等；
/// * ⚠️ **不许**编造时间词 ⇒ 找不到 ⇒ 返回**空**（`Ok` 但 `terms` 空）。
///
/// # 认得的时间锚
///
/// * `第N回`（N = 中文数字或阿拉伯数字）；
/// * `N年前 / N年後 / N天前 / N天後 / N月後 / N日前`（N 同上）；
/// * 固定时间词：`次日`、`翌日`、`當日`、`同日`、`同年`、`前日`、`後日`、
///   `昨夜`、`前夜`、`上月`、`下月`、`開年`、`年初`、`年底`。
///
/// # 数量上限
///
/// 最多 [`MAX_TERMS_PER_SOURCE`] 个（**出现顺序** ⇒ 确定性）。
pub fn time_anchor_terms(query: &str) -> Vec<String> {
    let chars: Vec<char> = query.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut index = 0usize;

    while index < chars.len() && out.len() < MAX_TERMS_PER_SOURCE {
        // ① 「第N回」
        if chars[index] == '第' {
            let mut cursor = index + 1;
            while cursor < chars.len() && is_ordinal_char(chars[cursor]) {
                cursor += 1;
            }
            if cursor > index + 1 && cursor < chars.len() && chars[cursor] == '回' {
                push_unique(&mut out, chars[index..=cursor].iter().collect());
                index = cursor + 1;
                continue;
            }
        }
        // ② 「N年前 / N天後 …」
        if is_ordinal_char(chars[index]) {
            let mut cursor = index;
            while cursor < chars.len() && is_ordinal_char(chars[cursor]) {
                cursor += 1;
            }
            if cursor > index {
                let mut matched = false;
                for suffix in ["年前", "年後", "天前", "天後", "月後", "日前"] {
                    let end = cursor + suffix.chars().count();
                    if end <= chars.len() {
                        let tail: String = chars[cursor..end].iter().collect();
                        if tail == suffix {
                            push_unique(&mut out, chars[index..end].iter().collect());
                            index = end;
                            matched = true;
                            break;
                        }
                    }
                }
                if matched {
                    continue;
                }
                // ⚠️⚠️ **死循环修复（2026-10-27）**：
                // 原写法是 `if index != cursor { continue; }` ⇒ 但**没匹配上后缀时
                // `index` 根本没被改** ⇒ `index != cursor` 恒真 ⇒ `continue`
                // ⇒ ⇒ **游标不前进** ⇒ **无限循环** ⇒
                // ⇒ **实测后果**：真实语料里出现「3個 / 5年」这类**数字后面不接后缀**的题面
                // ⇒ ⇒ 报告**永远跑不完**（交付物不可用）。
                // ⇒ 现在：**没匹配上 ⇒ 把游标推进到数字串末尾**（`index = cursor`）。
                index = cursor;
                continue;
            }
        }
        // ③ 固定时间词
        for word in FIXED_TIME_WORDS {
            let word_chars: Vec<char> = word.chars().collect();
            let end = index + word_chars.len();
            if end <= chars.len() && chars[index..end] == word_chars[..] {
                push_unique(&mut out, word.to_string());
                index = end;
                break;
            }
        }
        index += 1;
    }
    out
}

/// 取**原文首句**（整条记录的第一句）。
///
/// # 口径
///
/// * 第一句 = 到第一个句读符（`。！？`）为止（**含**句读符）
/// * ⇒ 去掉句读符后的实词。
///
/// # 补充口径（**实测必需**）
///
/// 古籍/Project Gutenberg 繁体本里主要的句读符是**全角句点「．」（U+FF0E）**
/// ⇒ 只认 `。！？` 会把**整条记录**当成一句话 ⇒ 实词全来自中段 ⇒ 有**泄漏味道**。
/// ⇒ 本实现**额外**认 `．`（与 `。！？` 同级）。
///
/// # 取词顺序
///
/// 按**出现顺序**取前 `max_terms` 个实词（首句很短 ⇒ 顺序即信息密度）。
pub fn record_head_terms(content: &str, max_terms: usize) -> Vec<String> {
    if content.trim().is_empty() || max_terms == 0 {
        return Vec::new();
    }
    let first = first_sentence(content);
    let mut out: Vec<String> = Vec::new();
    for token in crate::query_signals::tokenize(&first) {
        if is_corpus_content_word(&token) && !out.contains(&token) {
            out.push(token);
        }
        if out.len() >= max_terms {
            break;
        }
    }
    out
}

/// 取**段落首句**（含主体的那个段落的开头句）。
///
/// # 为什么需要它（[`SourceKind::ParagraphHead`] 在契约里没有配套函数）
///
/// 契约给了这个来源**种类**但没给**产出函数** ⇒ 本模块补一个（**唯一新增**的公开函数）。
///
/// # 口径
///
/// * 段落 = 按**换行**切（真实语料里空行分隔段落 ⇒ 切出来等价于「段」）；
/// * 目标段落 = **第一个提到主体**的段落（没有 ⇒ 第一段）；
/// * 首句 = 该段的第一个句子（同 [`record_head_terms`] 的句读符口径）；
/// * 实词 = 轻量实词口径（按出现顺序取前 `max_terms` 个）。
///
/// # 为什么「含主体的段落」而不是「第一段」
///
/// 记录的第一句（[`record_head_terms`]）说的是「这一回从哪儿讲起」，
/// 而**主体所在段落**的开头说的是「**这一段**在讲什么」⇒ 后者才是「这件事的主题句」。
pub fn paragraph_head_terms(content: &str, subject: &str, max_terms: usize) -> Vec<String> {
    if content.trim().is_empty() || max_terms == 0 {
        return Vec::new();
    }
    let paragraph = subject_paragraph(content, subject);
    let first = first_sentence(paragraph);
    let mut out: Vec<String> = Vec::new();
    for token in crate::query_signals::tokenize(&first) {
        if is_corpus_content_word(&token) && !out.contains(&token) {
            out.push(token);
        }
        if out.len() >= max_terms {
            break;
        }
    }
    out
}

/// 某道题在某来源下的**信号**（**非答案侧**）。
///
/// # 口径
///
/// * 只用**题面 + 语料** ⇒ **不读** `expect_ids` / `expect_fragments`；
/// * 主体 = [`subject_of`]（**显式**把期望片段置空 ⇒ 答案侧进不来）。
pub fn signal_for_case(
    case: &GoldenCase,
    records: &[(String, String)],
    kind: SourceKind,
) -> SourceSignal {
    let index = CorpusIndex::build(records);
    let bm25 = build_bm25(records);
    let mut cache: BTreeMap<(SourceKind, String), Vec<String>> = BTreeMap::new();
    let (_, terms) = case_signal(&index, &bm25, records, case, kind, &mut cache);
    SourceSignal { kind, terms }
}

/// **逐个来源**跑一遍对照（**真的检索**）。
///
/// # 口径
///
/// * 对每个来源：用它产出的词**改写题面** ⇒ 跑 `golden_run::run` ⇒ 取 `recall@k`；
/// * **基线**（原题面）也跑一遍（**同口径**）⇒ 给出 `delta`；
/// * ⚠️ **不许**为某个来源调权重凑分数（**统一**：每个来源加**同样数量**的词）。
///
/// # 返回顺序
///
/// * **第 0 项是基线**（[`SourceKind::Baseline`]，题面**逐字未改**）；
/// * 之后是 [`SOURCE_KINDS`] 里的来源，**顺序固定**；
/// * **空卷 ⇒ 空 `Vec`**（不 panic、不返回占位行）。
///
/// # 分母口径（**每行都一样**）
///
/// `recall@k` 一律在**整卷**上算：某个来源在某些题上**取不到词** ⇒
/// 那些题**题面逐字不改** ⇒ 该题按基线计 ⇒ 分数**不因为「没词」而虚高/虚低**。
pub fn compare_sources(
    golden: &crate::golden_set::GoldenSet,
    records: &[(String, String)],
    k: usize,
) -> Vec<SourceOutcome> {
    if golden.cases.is_empty() {
        return Vec::new();
    }
    let index = CorpusIndex::build(records);
    let bm25 = build_bm25(records);
    let index_records: Vec<IndexRecord> = records
        .iter()
        .map(|(id, text)| IndexRecord {
            id: id.clone(),
            text: text.clone(),
        })
        .collect();

    // --- 基线（原题面，逐字不改）---
    let __t0 = std::time::Instant::now();
    let baseline_recall = recall_of(&index_records, golden, k);
    eprintln!("[profile] baseline recall took {:?}", __t0.elapsed());
    let __t1 = std::time::Instant::now();
    eprintln!("[profile] index build done {:?}", __t1.elapsed());
    let mut outcomes: Vec<SourceOutcome> = Vec::with_capacity(SOURCE_KINDS.len() + 1);
    outcomes.push(SourceOutcome {
        kind: SourceKind::Baseline,
        recall_at_k: baseline_recall,
        delta: 0.0,
        questions_covered: golden.cases.len(),
        questions_with_terms: 0,
    });

    // --- 逐个来源（题面改写 ⇒ 真的跑检索）---
    //
    // ⚠️ **缓存必须提到来源循环外**（2026-10-27 性能修复）：
    // 原来每换一个来源就新建一个缓存 ⇒ **同一主体在不同来源下被重算多次**
    // ⇒ 而 `cooccurrence_terms` **每个主体要 214 ms**（实测）
    // ⇒ 主体数上千 ⇒ **报告跑 10 分钟出不来** ⇒ **交付物不可用**。
    // ⇒ 现在按 **(来源, 主体)** 记 ⇒ 每个 (来源, 主体) **只算一次**。
    let mut cache: BTreeMap<(SourceKind, String), Vec<String>> = BTreeMap::new();
    for kind in SOURCE_KINDS {
        let mut covered = 0usize;
        let mut with_terms = 0usize;
        let __t_sig = std::time::Instant::now();
        let rewritten: Vec<GoldenCase> = golden
            .cases
            .iter()
            .map(|case| {
                let (is_covered, terms) =
                    case_signal(&index, &bm25, records, case, *kind, &mut cache);
                if is_covered {
                    covered += 1;
                }
                if terms.is_empty() {
                    // 取不到词 ⇒ 题面**逐字不改**（该题按基线计）。
                    return case.clone();
                }
                with_terms += 1;
                let extra: Vec<String> = terms
                    .iter()
                    .filter(|term| !case.query.contains(term.as_str()))
                    .cloned()
                    .collect();
                let mut rewritten = case.clone();
                rewritten.query = rewrite_query(&case.query, &extra);
                rewritten
            })
            .collect();

        eprintln!(
            "[profile] {} signal took {:?}",
            kind.label(),
            __t_sig.elapsed()
        );
        let set = GoldenSet::new(format!("{}-{}", golden.version, kind.label()), rewritten);
        let __t = std::time::Instant::now();
        let recall = recall_of(&index_records, &set, k);
        eprintln!("[profile] {} recall took {:?}", kind.label(), __t.elapsed());
        outcomes.push(SourceOutcome {
            kind: *kind,
            recall_at_k: recall,
            delta: recall - baseline_recall,
            questions_covered: covered,
            questions_with_terms: with_terms,
        });
    }
    outcomes
}

/// 人话报告（**逐来源对照表** + **口径提醒**）。
///
/// # 必须包含
///
/// * 每个来源的 `recall@k` / `delta` / 覆盖题数；
/// * ⚠️ **实测无效的来源也要列**（**不许藏**）；
/// * ⚠️ **口径提醒**：「这些来源**不含答案信息** ⇒ 与 W-30 的 oracle 上界**不可直接比**」
///   （W-30 用的是答案侧信号 ⇒ 它的 0.6113 是**上界**）。
pub fn render_source_report(outcomes: &[SourceOutcome], baseline_recall: f64) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("══ 非答案侧信号来源 · 逐来源对照（recall@k）══".to_string());
    lines.push(format!("基线 recall = {baseline_recall:.4}"));
    lines.push(String::new());
    lines.push(format!(
        "  {:<12} {:>10} {:>12} {:>10} {:>10}  {}",
        "来源", "recall@k", "相对基线", "覆盖题数", "出词题数", "判定"
    ));
    for outcome in outcomes {
        lines.push(format!(
            "  {:<12} {:>10.4} {:>+12.4} {:>10} {:>10}  {}",
            outcome.kind.label(),
            outcome.recall_at_k,
            outcome.delta,
            outcome.questions_covered,
            outcome.questions_with_terms,
            verdict(*outcome)
        ));
    }
    lines.push(String::new());
    lines.push("⚠️ 口径提醒（读数前必看）：".to_string());
    lines.push(
        "  ① **不含答案信息** ⇒ 与 W-30 的 oracle 上界**不可直接比**：\n     W-30 往题面里加的信号**来自答案片段**（答案侧信息）⇒ 它的 0.6113 是\n     「信号到位之后能涨多少」的**上界**；本表的信号**只**来自「语料 + 题面」\n     ⇒ 这里的数字是**非答案侧**的实测值 ⇒ 若这里反而**高于** W-30，\n     ⇒ 说明「答案侧信号」**不是**唯一/最优路径（那才是真正值得深挖的结论）。"
            .to_string(),
    );
    lines.push(
        "  ② **题面被改写** ⇒ 这是**对照实验**（原问句逐字保留，只在末尾追加词）\n     ⇒ 与基线**同卷同 k** ⇒ `相对基线` 这一列**可比**；\n     但**题目内容变了** ⇒ 数字是「这条信号链路的收益」，不是「上线后必然的收益」。"
            .to_string(),
    );
    lines.push(
        "  ③ **实测无效的来源照样列出来**（不藏）：`出词题数 = 0` ⇒ 该来源在本批语料上\n     **根本没取到词** ⇒ 题面逐字未改 ⇒ 分数与基线**相同**（不是「涨了 0」，是「没出手」）。"
            .to_string(),
    );
    lines.push(
        "  ④ **数据集构造红利**：本批题是**从记录里挖出来的**（主体 = 该记录的最高频实体）\n     ⇒ 「主体最突出的那条记录」天然偏向**答案记录** ⇒ 「原文首句 / 段落首句」两行\n     的数字含**构造红利**，**不等于**真实用户提问的收益（读表时按这个折）。"
            .to_string(),
    );
    lines.join("\n")
}

// ============================================================
// 内部：语料统计
// ============================================================

/// 语料的**一次性统计**（分词 + 倒排）⇒ 批量跑不重复分词。
struct CorpusIndex {
    /// 每条记录的实词 token 集合（去重、只留内容词）。
    record_tokens: Vec<BTreeSet<String>>,
    /// token → 出现在哪些记录里（记录下标，**升序** ⇒ 可直接求交集）。
    postings: BTreeMap<String, Vec<usize>>,
    /// **(主体, 记录下标) → 该记录里与主体「位置重叠」的碎片**（**性能缓存**）。
    ///
    /// # 为什么需要这个缓存
    ///
    /// `subject_fragments` 要扫**整篇正文**（《紅樓夢》一回 **7401 字**）
    /// ⇒ 「每道题 × 每条提到主体的记录」⇒ **上亿次**字符比较
    /// ⇒ ⇒ 实测：**逐来源对照跑 10 分钟出不来**（报告不可用）。
    ///
    /// ⇒ 主体数 ≈ 记录数 ≪ 题数 ⇒ ⇒ **按主体缓存** ⇒ 同一主体只算一次。
    /// ⇒ 用 `RefCell`（`cooccurrence_counts` 是 `&self`）⇒ **单线程内**使用。
    fragment_cache: RefCell<BTreeMap<(String, usize), BTreeSet<String>>>,
}

impl CorpusIndex {
    /// 建索引（分词一次 ⇒ 后续所有统计都是查表）。
    fn build(records: &[(String, String)]) -> Self {
        let mut record_tokens: Vec<BTreeSet<String>> = Vec::with_capacity(records.len());
        let mut postings: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, (_, text)) in records.iter().enumerate() {
            let mut tokens: BTreeSet<String> = BTreeSet::new();
            for token in crate::query_signals::tokenize(text) {
                if is_corpus_content_word(&token) {
                    tokens.insert(token);
                }
            }
            for token in &tokens {
                postings.entry(token.clone()).or_default().push(index);
            }
            record_tokens.push(tokens);
        }
        Self {
            record_tokens,
            postings,
            fragment_cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// token 出现在多少条记录里（df）。
    fn doc_freq(&self, term: &str) -> usize {
        self.postings.get(term).map_or(0, Vec::len)
    }

    /// 与主体**同现**的词及其共现记录数。
    ///
    /// ⚠️ **跨过主体边界的碎片**被剔掉（见 [`subject_fragments`]）
    /// ⇒ 否则「玉敘」这种**根本不是词**的东西会冒充「强共现词」。
    fn cooccurrence_counts(
        &self,
        records: &[(String, String)],
        subject: &str,
    ) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for record_index in subject_record_indices(records, subject) {
            let Some((_, text)) = records.get(record_index) else {
                continue;
            };
            // ⚠️ **性能**：`subject_fragments` 要扫**整篇正文**（7401 字）
            // ⇒ 「每题 × 每条提到主体的记录」⇒ **上亿次**字符比较 ⇒ 报告跑不完。
            // ⇒ ⇒ 这里**按主体缓存**（主体数 ≈ 记录数 ≪ 题数）⇒ 同一主体只算一次。
            // ⇒ 缓存键 = (主体, 记录下标) ⇒ 不同记录不串。
            let fragments = self
                .fragment_cache
                .borrow_mut()
                .entry((subject.to_string(), record_index))
                .or_insert_with(|| subject_fragments(text, subject))
                .clone();
            // ⚠️ 用 `difference`（一次差集）⇒ 不必**每个 token** 都查一遍碎片表。
            for token in self.record_tokens[record_index].difference(&fragments) {
                *counts.entry(token.clone()).or_insert(0) += 1;
            }
        }
        counts.remove(subject);
        counts
    }
}

/// 共现词的内部实现（复用已建好的索引 ⇒ 批量跑不重复分词）。
fn cooccurrence_terms_with(
    index: &CorpusIndex,
    records: &[(String, String)],
    subject: &str,
    corpus_size: usize,
    max_terms: usize,
) -> Vec<String> {
    if subject.is_empty() || max_terms == 0 || corpus_size == 0 {
        return Vec::new();
    }
    let counts = index.cooccurrence_counts(records, subject);
    let max_df = ((corpus_size as f64) * COOC_MAX_DF_RATIO).floor() as usize;
    let mut candidates: Vec<(String, usize, f64)> = counts
        .into_iter()
        .filter_map(|(term, co)| {
            let df = index.doc_freq(&term);
            if df < COOC_MIN_DF || df > max_df.max(COOC_MIN_DF) {
                return None;
            }
            let strength = co as f64 / df as f64;
            if strength < COOC_MIN_STRENGTH {
                return None;
            }
            Some((term, df, strength))
        })
        .collect();
    // 排序：`df` 降序 → 强度降序 → 词长降序 → 字典序升序（**确定性**）。
    candidates.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.total_cmp(&left.2))
            .then_with(|| right.0.chars().count().cmp(&left.0.chars().count()))
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates
        .into_iter()
        .take(max_terms)
        .map(|(term, _, _)| term)
        .collect()
}

/// 同主题历史的内部实现（复用已建好的索引）。
fn same_subject_history_terms_with(
    records: &[(String, String)],
    indices: &[usize],
    subject: &str,
    max_terms: usize,
) -> Vec<String> {
    if subject.is_empty() || max_terms == 0 {
        return Vec::new();
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for record_index in indices {
        let Some((_, text)) = records.get(*record_index) else {
            continue;
        };
        let head: String = text.chars().take(HISTORY_HEAD_CHARS).collect();
        let fragments = subject_fragments(&head, subject);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for token in crate::query_signals::tokenize(&head) {
            if is_corpus_content_word(&token)
                && !fragments.contains(&token)
                && seen.insert(token.clone())
            {
                *counts.entry(token).or_insert(0) += 1;
            }
        }
    }
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();
    // 排序：词频降序 → 词长降序 → 字典序升序（**确定性**）。
    items.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.0.chars().count().cmp(&left.0.chars().count()))
            .then_with(|| left.0.cmp(&right.0))
    });
    items
        .into_iter()
        .take(max_terms)
        .map(|(term, _)| term)
        .collect()
}

/// 提到 `subject` 的记录下标（**子串口径** ⇒ 与 W-30 的「记录正文含有该词」同口径）。
///
/// # 为什么用子串而不是 token 倒排
///
/// 主体**可能是三个字**（「林黛玉」）⇒ 切词只出 bigram ⇒ 倒排表里**没有**它
/// ⇒ 用子串才找得到 ⇒ 主体是几个字都能用。
fn subject_record_indices(records: &[(String, String)], subject: &str) -> Vec<usize> {
    if subject.is_empty() {
        return Vec::new();
    }
    records
        .iter()
        .enumerate()
        .filter(|(_, (_, text))| text.contains(subject))
        .map(|(index, _)| index)
        .collect()
}

/// 与主体的出现位置**重叠**的 bigram（=「跨词边界的碎片」）。
///
/// # 为什么需要它
///
/// 切词是「**相邻两字**」⇒ 「寶玉敘話」会切出「玉敘」这种**根本不是词**的东西。
/// 它在语料里**只出现在主体旁边** ⇒ 共现统计里会**冒充**「强共现词」。
///
/// # 为什么**不**按「含主体的字」剔
///
/// 「寶釵」「寶玉」共享「寶」字，但「寶釵」是**真的**词 ⇒ 按字剔会误伤。
/// ⇒ 只剔**位置重叠**的那些 bigram（主体自身的 bigram + 跨过它左右边界的那个）。
fn subject_fragments(text: &str, subject: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    if subject.is_empty() {
        return out;
    }
    let chars: Vec<char> = text.chars().collect();
    let subject_chars: Vec<char> = subject.chars().collect();
    let width = subject_chars.len();
    if width == 0 || chars.len() < width {
        return out;
    }
    for start in 0..=(chars.len() - width) {
        if chars[start..start + width] != subject_chars[..] {
            continue;
        }
        let end = start + width;
        // 与主体**重叠**的相邻字对：左边界那对、主体内部的、以及右边界那对。
        let first = start.saturating_sub(1);
        let last = end.min(chars.len().saturating_sub(1));
        for pair in first..last {
            if pair + 1 >= chars.len() {
                break;
            }
            if is_cjk_char(chars[pair]) && is_cjk_char(chars[pair + 1]) {
                out.insert(format!("{}{}", chars[pair], chars[pair + 1]));
            }
        }
    }
    out
}

// ============================================================
// 内部：主体 / 逐题信号
// ============================================================

/// 题面里的**主体**（= 最长的实词）。
///
/// # 为什么「显式置空期望片段」
///
/// 主体判定是**信号链路的第一个环节** ⇒ 它必须**不含**答案侧信息。
/// W-30 的 `parse_signals` 在**同长度**候选里会优先选「**出现在期望片段里**」的那个
/// ⇒ 直接用它 ⇒ 主体就带上了答案侧信息（哪怕只是「选谁当主体」这一层）。
/// ⇒ 这里**复制一份题面**并把期望片段**置空**再调 ⇒ 主体判定**只**看题面
/// ⇒ 口径与 W-30 **同源**（同一份实词表、同一条「最长实词」规则），
/// 只在「同长度怎么选」这一处不同（本实现按**题面里最靠前**）。
pub fn subject_of(case: &GoldenCase) -> Option<String> {
    let probe = GoldenCase {
        id: case.id.clone(),
        query: case.query.clone(),
        expect_ids: Vec::new(),
        expect_fragments: Vec::new(),
        tags: Vec::new(),
    };
    crate::query_signals::parse_signals(&probe).subject
}

/// 一道题在某来源下的信号（内部实现：`covered` = 前置条件是否满足）。
fn case_signal(
    index: &CorpusIndex,
    bm25: &Bm25Index,
    records: &[(String, String)],
    case: &GoldenCase,
    kind: SourceKind,
    cache: &mut BTreeMap<(SourceKind, String), Vec<String>>,
) -> (bool, Vec<String>) {
    match kind {
        SourceKind::Baseline => (true, Vec::new()),
        // 时间锚：**题面**里扫（与语料无关）⇒ 永远「可施加」。
        SourceKind::TimeAnchor => (true, time_anchor_terms(&case.query)),
        SourceKind::SameSubjectHistory => {
            let Some(subject) = subject_of(case) else {
                return (false, Vec::new());
            };
            if let Some(cached) = cache.get(&(kind, subject.clone())) {
                return (true, cached.clone());
            }
            let indices = subject_record_indices(records, &subject);
            if indices.is_empty() {
                return (false, Vec::new());
            }
            let terms =
                same_subject_history_terms_with(records, &indices, &subject, MAX_TERMS_PER_SOURCE);
            cache.insert((kind, subject.clone()), terms.clone());
            (true, terms)
        }
        SourceKind::Cooccurrence => {
            let Some(subject) = subject_of(case) else {
                return (false, Vec::new());
            };
            if let Some(cached) = cache.get(&(kind, subject.clone())) {
                return (true, cached.clone());
            }
            let terms = cooccurrence_terms_with(
                index,
                records,
                &subject,
                records.len(),
                MAX_TERMS_PER_SOURCE,
            );
            cache.insert((kind, subject.clone()), terms.clone());
            (true, terms)
        }
        // 下面两个：**伪相关反馈（PRF）** —— 先用原题面检索一次，取 top-1 记录。
        // 缓存键 = **题面**（PRF 结果依赖题面，不只是主体）。
        SourceKind::RecordHead => {
            if let Some(cached) = cache.get(&(kind, case.query.clone())) {
                return (true, cached.clone());
            }
            let Some(content) = prf_top_content(bm25, records, &case.query) else {
                return (false, Vec::new());
            };
            let terms = record_head_terms(&content, MAX_TERMS_PER_SOURCE);
            cache.insert((kind, case.query.clone()), terms.clone());
            (true, terms)
        }
        SourceKind::ParagraphHead => {
            if let Some(cached) = cache.get(&(kind, case.query.clone())) {
                return (true, cached.clone());
            }
            let Some(content) = prf_top_content(bm25, records, &case.query) else {
                return (false, Vec::new());
            };
            let subject = subject_of(case).unwrap_or_default();
            let terms = paragraph_head_terms(&content, &subject, MAX_TERMS_PER_SOURCE);
            cache.insert((kind, case.query.clone()), terms.clone());
            (true, terms)
        }
    }
}

/// 伪相关反馈：用**原题面**检索一次 ⇒ 取 **top-1** 记录的正文。
///
/// # 为什么这是「非答案侧」
///
/// 检索用的是**题面 + 语料** ⇒ 没有任何期望片段/答案字段参与。
fn prf_top_content(bm25: &Bm25Index, records: &[(String, String)], query: &str) -> Option<String> {
    let hit = bm25.search(query, 1).into_iter().next()?;
    records
        .iter()
        .find(|(id, _)| *id == hit.id)
        .map(|(_, text)| text.clone())
}

/// 建 BM25 索引（**生产同口径**：`Bm25Config::default()`，与 `golden_run` 一致）。
fn build_bm25(records: &[(String, String)]) -> Bm25Index {
    let mut index = Bm25Index::new(Bm25Config::default());
    for (id, text) in records {
        index.insert(id.clone(), text);
    }
    index
}

/// 跑一次批 ⇒ `recall@k`（跑失败 ⇒ 记 0.0，**不 panic**）。
fn recall_of(index: &[IndexRecord], golden: &GoldenSet, k: usize) -> f64 {
    crate::golden_run::run(index, golden, k, InjectionMode::Production)
        .map(|report| report.metrics.recall_at_k)
        .unwrap_or(0.0)
}

/// 题面追加信号词（**原问句逐字保留**，与 W-30 的改写格式逐字一致）。
fn rewrite_query(query: &str, extra: &[String]) -> String {
    if extra.is_empty() {
        return query.to_string();
    }
    let joined = extra.join("、");
    match query.strip_suffix('？') {
        Some(stem) => format!("{stem}（{joined}）？"),
        None => format!("{query}（{joined}）"),
    }
}

/// 报告里每一行的**判定**（**如实**：无效的来源照样显示「无效」）。
fn verdict(outcome: SourceOutcome) -> &'static str {
    if outcome.kind == SourceKind::Baseline {
        return "对照组";
    }
    if outcome.questions_with_terms == 0 {
        return "⚠️ 实测无效（没取到词）";
    }
    if outcome.delta > 0.001 {
        return "✅ 有帮助";
    }
    if outcome.delta < -0.001 {
        return "⚠️ 掉分";
    }
    "— 持平"
}

// ============================================================
// 内部：切句 / 切段 / 轻量实词
// ============================================================

/// 句读符（**含古籍常用的「．」（U+FF0E）** —— 真实语料里它才是主要句读符）。
const SENTENCE_MARKS: &str = "。！？．!?";

/// 取第一句（**含**句读符）。
fn first_sentence(content: &str) -> String {
    let mut out = String::new();
    for ch in content.chars() {
        out.push(ch);
        if SENTENCE_MARKS.contains(ch) {
            break;
        }
    }
    out
}

/// 含 `subject` 的第一个段落（没有 ⇒ 第一段）。
fn subject_paragraph<'a>(content: &'a str, subject: &str) -> &'a str {
    let paragraphs: Vec<&str> = content
        .split('\n')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .collect();
    if paragraphs.is_empty() {
        return content;
    }
    if !subject.is_empty() {
        for paragraph in &paragraphs {
            if paragraph.contains(subject) {
                return paragraph;
            }
        }
    }
    paragraphs[0]
}

/// 固定时间词（时间锚的词表）。
const FIXED_TIME_WORDS: &[&str] = &[
    "次日", "翌日", "當日", "同日", "同年", "前日", "後日", "昨夜", "前夜", "上月", "下月", "開年",
    "年初", "年底",
];

/// 是不是回数/数量里会出现的字（中文数字 + 阿拉伯数字）。
fn is_ordinal_char(ch: char) -> bool {
    ch.is_ascii_digit()
        || matches!(
            ch,
            '零' | '一'
                | '二'
                | '三'
                | '四'
                | '五'
                | '六'
                | '七'
                | '八'
                | '九'
                | '十'
                | '百'
                | '千'
                | '兩'
                | '两'
        )
}

/// 往 `out` 里塞一个词（**去重保序**）。
fn push_unique(out: &mut Vec<String>, term: String) {
    if !term.is_empty() && !out.contains(&term) {
        out.push(term);
    }
}

/// 语料侧的**轻量实词**（只用于统计候选池，**不用于主体判定**）。
///
/// # 过滤链
///
/// 1. **太短**（<2 字）⇒ 单字没有判别力；
/// 2. **纯数字** ⇒ 编号不是话题；
/// 3. **非字母数字** ⇒ 那不是词；
/// 4. CJK token：含**高频虚词字** ⇒ 多半是跨词边界的碎片；
/// 5. CJK token：**数量字开头** ⇒ 数量短语不是话题；
/// 6. **泛称表**命中 ⇒ 高频且不指事。
///
/// # 为什么**不**直接用 W-30 的实词表
///
/// W-30 的实词表（`query_signals` 内部）是**私有的** ⇒ 本模块没法复用；
/// 而**复制**一份 200 多条的表 ⇒ 两处将来必然漂移。
/// ⇒ 本模块只做**语料统计**用的轻量过滤（停用词那部分交给
/// **强度 / df 上限**两道闸门，见 [`cooccurrence_terms`] 的筛选条件），
/// **主体**判定则**直接调** W-30 的 `parse_signals`（同口径，零漂移）。
fn is_corpus_content_word(token: &str) -> bool {
    let length = token.chars().count();
    if length < 2 {
        return false;
    }
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if !token.chars().all(|ch| ch.is_alphanumeric()) {
        return false;
    }
    if is_cjk_token(token) {
        if token.chars().any(|ch| FUNCTION_CHARS.contains(ch)) {
            return false;
        }
        let first = token.chars().next().unwrap_or('\0');
        if CHINESE_NUMERALS.contains(first) {
            return false;
        }
    }
    !CORPUS_GENERIC_TERMS.contains(&token)
}

/// 是不是 CJK token（全部字符都是汉字）。
fn is_cjk_token(token: &str) -> bool {
    !token.is_empty() && token.chars().all(is_cjk_char)
}

/// 是不是 CJK 汉字。
fn is_cjk_char(ch: char) -> bool {
    let code = ch as u32;
    (0x3400..=0x4DBF).contains(&code)
        || (0x4E00..=0x9FFF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2FA1F).contains(&code)
}

/// 高频虚词字（与 `query_signals` **同源**的口径）。
const FUNCTION_CHARS: &str = "的了着过是在和与就都也而及或但却还又再更很太把被让使给对从向于并之其有无不要会能可说我你他她们这那些什么怎样呢吗吧啊呀嘛哪谁因所果如麼麽與";
/// 中文数词（与 `query_signals` **同源**的口径）。
const CHINESE_NUMERALS: &str = "零一二三四五六七八九十百千万亿两兩";

/// 语料统计用的**泛称表**（高频且不指事 ⇒ 当不得信号）。
///
/// ⚠️ 这张表**故意很短** ⇒ 语料统计里的「泛词」主要由**两道闸门**挡掉
/// （强度 ≥ 0.5 ⇒ 泛词在「主体记录里」并不比在别处更密；df ≤ 语料 × 0.25 ⇒ 泛词太广）。
const CORPUS_GENERIC_TERMS: &[&str] = &[
    "起來", "出來", "進來", "回去", "說道", "說了", "只見", "聽得", "聽見", "不知", "一面", "一起",
    "一聲", "一會", "這個", "那個", "這些", "那些", "東西", "丫頭", "奶奶", "太太", "夫人", "老爺",
    "眾人", "這裏", "那裏", "如此", "如今", "只是", "一直", "自己", "別人", "知道", "如何", "怎樣",
    "為何", "甚麼", "這般", "那般", "咱們", "我們", "你們", "他們", "各位", "當下", "當時", "此時",
    "彼時", "早已", "總是", "總共", "一回", "一時", "一番", "一句", "一件",
];
