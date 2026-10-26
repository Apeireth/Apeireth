//! `apeireth-memory::novel_corpus` — 小说（Gutenberg 纯文本）→ 记忆挖题器的语料接线。
//!
//! # 为什么有这个模块
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §7.1 的**内部卷**
//! 需要**真实语料**（挖 2000 条候选 → owner 精标 300 条）。已实测的三条通道：
//!
//! * `<数据根>/diary/*.json` ⇒ 本机**只有 1 条**（`apeireth dream` 写的认知梦境日志，
//!   不是用户内容）⇒ **不可用**；
//! * **git 提交历史** ⇒ 已挖出 5552 道题（见 [`crate::git_corpus`]）；
//! * ⭐ **小说** ⇒ 本模块（owner 明确授权：「下两本小说当语料算了」）。
//!
//! # 为什么小说适合测「记忆检索」
//!
//! 1. **章节天然带时间线**（第 N 回）⇒ `temporal` / `conflict` 题型有**真实**素材；
//! 2. **同一件事前后说法不同**（人物变化、事件改写）⇒ `conflict` 题型有**真实**素材；
//! 3. 篇幅大、纯中文、口语化 ⇒ 与「题像不像人话」的要求天然契合。
//!
//! # 数据从哪来（**不入库**）
//!
//! * 《紅樓夢》**繁體**，**120 回**（另有「楔子」），Project Gutenberg #24264（公有领域）；
//! * 《水滸傳》**繁體**，**70 回**（另有「楔子」），Project Gutenberg #23863（公有领域）。
//!
//! （回数是**实测**值：解析出来的回标题行数。源文件里另有 3 行正文恰好以「第X回」开头、
//! 1 处重复的标题行 ⇒ 都不是「回」⇒ 不计入。）
//!
//! 语料文件放在**仓外**（`<数据根>/novels/*.txt`）⇒ 仓库里**只有适配器**，
//! 小说正文**一个字都不入库**（版权 + 体积双理由）。
//!
//! # 格式口径（Gutenberg 纯文本 ⇒ 各回）
//!
//! ```text
//! *** START OF THE PROJECT GUTENBERG EBOOK 紅樓夢 ***
//! 第一回　甄士隱夢幻識通靈　賈雨村風塵怀閨秀
//! -----------------------------------------------------------------
//! 此開卷第一回也．作者自云：…
//! 第三回
//! -----------------------------------------------------------------
//! 賈雨村夤緣复舊職　林黛玉拋父進京都
//! 　　卻說雨村忙回頭看時，…
//! *** END OF THE PROJECT GUTENBERG EBOOK 紅樓夢 ***
//! ```
//!
//! 实测出来的三条**真实**形态（本模块按它们设计，而不是按想象设计）：
//!
//! 1. 回目联**多数写在标题行里**（`第X回　上联　下联`）；
//! 2. 回目联**也可能写在标题行之后**（标题行只有「第X回」，中间隔着分隔线）；
//! 3. 正文里**偶尔有一行恰好以「第X回」开头**（回述上一回）⇒ 不能只看「以第…回开头」
//!    就当标题（否则会凭空造出一回）。
//!
//! # ⚠️ 口径：回目联只当「提问的把手」，**不进可检索文本**
//!
//! 题目由**回目联**生成（「第三回　賈雨村夤緣复舊職后来是怎么改的？」）⇒
//! 若回目联又逐字出现在**可检索文本**里 ⇒ 检索是**字面命中** ⇒ recall@10 恒等于满分
//! ⇒ **这个卷子测不出任何东西**。⇒ 可检索文本 = **正文**（与 `git_corpus` 同口径：
//! 提交主题当把手，body 当可检索文本）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：解析、接线、导出齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**无 CLI、无定时任务、生产 0 调用点**；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：在两本**真实**小说上手工探查过（见交付说明），
//!   但**未成规模**（设计稿要 2000 条候选 ⇒ 还差一个数量级）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::collections::BTreeSet;
use std::path::Path;

use crate::golden_mining::{MineRecord, MinedCase, MiningConfig};

/// 一回（一章）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    /// 回次（1 基；= **书里写的回数**，解析不出/重复时退回顺序号）。
    ///
    /// 为什么用「书里的回数」而不是「第几个标题行」：题目里会出现「第X回」⇒
    /// owner 要能拿它回书里翻到那一回；源文件跳号时顺序号会指错地方。
    pub index: usize,
    /// 回目联（**可能为空** —— 有些回标题只有「第X回」）。
    pub title: String,
    /// 正文（**已合并硬折行**）。
    pub body: String,
}

/// 时间线步长（毫秒）：**一天一回**。
///
/// 为什么是「一天一回」：小说的回目**天然有序**（第 N 回在第 N+1 回之前），
/// 挖题器要的是「两条记录时间相邻」这个信号 ⇒ 给一个**确定的**虚拟步长即可。
///
/// 为什么不用真实时间：小说的回目没有真实日期 ⇒ 编一个真实时间戳就是**造假**；
/// 而步长必须**确定**（不读系统时间）⇒ 同一批语料两次挖出来的卷子必须逐字相等。
const CHAPTER_STEP_MS: i64 = 86_400_000;

/// Gutenberg 正文起始标记。
const START_MARKER: &str = "*** START OF";

/// Gutenberg 正文结束标记。
const END_MARKER: &str = "*** END OF";

/// 句子/词读字符（出现即说明这行是**正文**，不是回目联）。
///
/// 为什么需要它：正文里**偶尔有一行恰好以「第X回」开头**（回述上一回）。
/// 回目联是一副**对联**（上下句都是汉字、没有句读）；正文是**句子**（带句读）。
/// ⇒ 用「有没有句读」把两者分开，比「行首是不是第X回」可靠。
const SENTENCE_PUNCT: &str = "，。．、；：！？「」『』（）〔〕《》〈〉…—～·,.!?;:()[]{}\"'‘’“”";

/// 回目联**单句**的最长字数（超过 ⇒ 不可能是对联 ⇒ 是正文）。
///
/// 为什么要有长度闸：光靠「有没有句读」还不够 —— 正文里若有一行以「第X回」开头
/// 且**恰好没有句读**（短句），会被误当成回目联。回目联的一句是 7~8 字，
/// 上限放到 24 字是给「OCR 把上下句连成一句」（《紅樓夢》第六十二回实测）的余量。
const MAX_COUPLET_CLAUSE_CHARS: usize = 24;

/// 回目联最多几句（上下两句）。
///
/// 为什么是 2：回目联就是一副上下联 ⇒ 多出来的段是**正文被硬折行粘到标题行上**
/// （《紅樓夢》第一百零四回实测）⇒ 那部分要还给正文，不能混进回目联。
const MAX_COUPLET_CLAUSES: usize = 2;

/// 解析一本小说 ⇒ 各回。
///
/// # 口径
///
/// * 只认**以 `第…回` 开头**的行是章节标题（`…回` 前可有数字，中文数字或阿拉伯数字都收）；
/// * 标题行之后的**回目联**（若下一行不是空行、也不是分隔线、也不是下一个 `第…回`）⇒ 并进 `title`；
/// * 标题与下一个标题之间的正文 ⇒ 合并硬折行（把换行换成**空字符串**而不是空格？
///   ⇒ **用空格**：中文换行处往往没有标点，粘连会造出怪词；但**段与段之间**要保留段落感
///   ⇒ 折行合并成空格，**空行保留为段落分隔**）；
/// * `*** START OF …` 之前的内容**全部丢弃**（Gutenberg 元信息）；
/// * `*** END OF …` 之后的内容**全部丢弃**（版权尾注）；
/// * 找不到任何章节 ⇒ 返回空 Vec（**不 panic**）。
///
/// # 额外的容错（实测出来的形态）
///
/// * **回目联在标题行之后**：标题行只有「第X回」时，往后**跳过空行与分隔线**找回目联；
/// * **正文粘在标题行上**：标题行回目联之后的剩余文字**还给正文**（不丢字）；
/// * **正文行恰好以「第X回」开头**：因为带句读 ⇒ **不**当标题（否则凭空造出一回）；
/// * **正文为空的回**：跳过（源文件里存在**重复的标题行**，实测《紅樓夢》第四十五回）。
pub fn parse_chapters(text: &str) -> Vec<Chapter> {
    let region = slice_region(text);
    // ⚠️ **归一 OCR 伪影 `@`** → 空格。
    //
    // 《紅樓夢》第 62、102 回的回目行里，两联句之间的**全角空格**被识别成了 `@`：
    // `憨湘云醉眠芍藥茵屆@呆香菱情解石榴裙` ⇒ 联句粘成一段 ⇒
    // 题目主体被 16 字预算截成只剩「第X回」（**题目失去抓手**）。
    //
    // ⇒ `@` 归一成普通空格（**不是**全角空格：分句按空白切，普通空格最省事）。
    // ⚠️ **实测影响面**：这两本书里 `@` **共 2 处，且都在回目行**
    // ⇒ **正文一个字都没被改**（这才是「保真」的关键事实）。
    // 换成别的书时若 `@` 出现在正文里 ⇒ 会被一并替换（已知代价，如实记录）。
    let normalized = region.replace('@', " ");
    let lines: Vec<&str> = normalized.lines().map(str::trim).collect();

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut used_indexes: BTreeSet<usize> = BTreeSet::new();
    let mut cursor = 0usize;
    while cursor < lines.len() {
        let Some((number, rest)) = split_chapter_head(lines[cursor]) else {
            cursor += 1;
            continue;
        };
        let segments = whitespace_segments(rest);
        // 正文行恰好以「第X回」开头（首段带句读）⇒ **不是**标题行。
        if segments
            .first()
            .is_some_and(|segment| !is_couplet_clause(segment))
        {
            cursor += 1;
            continue;
        }

        // 回目联：先看标题行里有没有。
        let inline = couplet_clauses(&segments);
        let mut couplet = inline.clone();
        let mut leftover: Vec<&str> = segments[inline.len()..].to_vec();
        let mut body_start = cursor + 1;

        // 标题行里没有回目联 ⇒ 往后找（跳过空行与分隔线）。
        if couplet.is_empty() {
            let mut probe = cursor + 1;
            while probe < lines.len() && (lines[probe].is_empty() || is_separator(lines[probe])) {
                probe += 1;
            }
            if probe < lines.len() && split_chapter_head(lines[probe]).is_none() {
                let candidates = whitespace_segments(lines[probe]);
                let found = couplet_clauses(&candidates);
                if !found.is_empty() {
                    couplet = found.clone();
                    leftover = candidates[found.len()..].to_vec();
                    body_start = probe + 1;
                }
            }
        }

        // 正文：从正文起点到下一个章节标题行。
        let mut end = body_start;
        while end < lines.len() && chapter_title_at(lines[end]).is_none() {
            end += 1;
        }
        let body = assemble_body(&lines[body_start..end], &leftover);
        cursor = end;

        // 正文为空的回（源文件里重复的标题行）⇒ 跳过，不产出空记录。
        if body.trim().is_empty() {
            continue;
        }
        // 回次 = **书里写的回数**（不是「第几个标题行」）。
        //
        // 为什么：题目里会出现「第X回」⇒ owner 要能拿它回书里翻到那一回。
        // 顺序号只在书缺回（源文件跳号）时才对 ⇒ 用顺序号会指错地方。
        // 解析不出 / 重复占用 ⇒ 退回顺序号（保证 id 唯一）。
        let index = parse_ordinal(&number)
            .filter(|value| *value > 0 && !used_indexes.contains(value))
            .unwrap_or(chapters.len() + 1);
        used_indexes.insert(index);
        chapters.push(Chapter {
            index,
            title: couplet.join("\u{3000}"),
            body,
        });
    }
    chapters
}

/// 读一本小说文件并解析。
///
/// # 错误口径
///
/// * 路径不存在 ⇒ [`NovelCorpusError::NotFound`]；
/// * 读不了（不是 UTF-8 / 是目录 / 权限不足）⇒ [`NovelCorpusError::Unreadable`]；
/// * 读到了但**解析不出任何回** ⇒ [`NovelCorpusError::NoChapters`]（格式不对，如实报）。
pub fn load_chapters(path: &Path) -> Result<Vec<Chapter>, NovelCorpusError> {
    let display = path.display().to_string();
    if !path.exists() {
        return Err(NovelCorpusError::NotFound { path: display });
    }
    let text = std::fs::read_to_string(path).map_err(|error| NovelCorpusError::Unreadable {
        path: display.clone(),
        reason: error.to_string(),
    })?;
    let chapters = parse_chapters(&text);
    if chapters.is_empty() {
        return Err(NovelCorpusError::NoChapters);
    }
    Ok(chapters)
}

/// 把「回」变成挖题器能吃的记录。
///
/// * `id` = `novel-{index:04d}`（**确定**、可对照）；
/// * `title` = **回次 + 回目联**（如 `第三回　甄士隱夢幻識通靈`）⇒ 题目主体会是
///   「第三回　甄士隱夢幻識通靈后来是怎么改的？」—— **这正是我们要的「像人话」**；
///   ⚠️ 若 `title`（回目联）为空 ⇒ 只用「第X回」；
/// * `content` = **正文**（⚠️ **不含回目联** ⇒ 题目把手不泄漏进可检索文本，见下）；
/// * `timestamp_ms` = **回次 × 一个固定步长**（如 `index * 86_400_000`，即「一天一回」）
///   ⇒ 虚构的时间线，**但必须确定性**（不读系统时间）；
/// * `thread_id` = `None`（小说没有「会话」概念；同主题靠共享词判定）。
pub fn to_mine_records(chapters: &[Chapter]) -> Vec<MineRecord> {
    chapters
        .iter()
        .map(|chapter| {
            let ordinal = format!("第{}回", render_ordinal(chapter.index));
            let title = if chapter.title.is_empty() {
                ordinal
            } else {
                format!("{ordinal}\u{3000}{}", chapter.title)
            };
            // ⚠️ **可检索文本 = 正文**，**不含回目联**。
            //
            // **为什么**：题目由回目联生成（`第三十二回　訴肺腑心迷活寶玉后来是怎么改的？`）
            // ⇒ 若回目联又出现在可检索文本里 ⇒ 检索是**字面命中** ⇒
            // recall@10 恒等于满分 ⇒ **这个卷子测不出任何东西**。
            // ⇒ 回目联只当「提问的把手」，不进可检索文本。
            // （实测：回目联**本来就不在正文里** ⇒ 去掉它不损失正文信息。）
            let content = chapter.body.clone();
            MineRecord {
                id: format!("novel-{:04}", chapter.index),
                content,
                title: Some(title),
                timestamp_ms: chapter.index as i64 * CHAPTER_STEP_MS,
                thread_id: None,
            }
        })
        .collect()
}

/// 从小说挖题（= `to_mine_records` + `golden_mining::mine`）。
pub fn mine_novel(chapters: &[Chapter], config: MiningConfig) -> Vec<MinedCase> {
    let records = to_mine_records(chapters);
    crate::golden_mining::mine(&records, config)
}

/// 人话样本（每条一行：`[kind] query ⇒ expect_ids=[…]`；按 (kind, query) 排序）。
///
/// **确定性**：按 `(kind, query, id)` 排序 ⇒ 同一批题两次渲染逐字相等。
/// `limit = 0` ⇒ 空串（不是「全部」）。
pub fn render_samples(cases: &[MinedCase], limit: usize) -> String {
    let mut sorted: Vec<&MinedCase> = cases.iter().collect();
    sorted.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.query.cmp(&right.query))
            // 同 (kind, query) 再按 id 兜底 ⇒ 排序是全序（不会因输入顺序抖动）。
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut out = String::new();
    for (position, case) in sorted.iter().take(limit).enumerate() {
        if position > 0 {
            out.push('\n');
        }
        out.push('[');
        out.push_str(&case.kind);
        out.push_str("] ");
        out.push_str(&case.query);
        out.push_str("  ⇒  expect_ids=[");
        out.push_str(&case.expect_ids.join(", "));
        out.push(']');
    }
    out
}

/// 错误类型。
#[derive(Debug, thiserror::Error)]
pub enum NovelCorpusError {
    #[error("小说文件不存在：{path}")]
    NotFound { path: String },
    #[error("小说文件读不了：{path}：{reason}")]
    Unreadable { path: String, reason: String },
    #[error("小说文件里没有解析出任何回（格式不对？）")]
    NoChapters,
}

// ============================================================
// 内部
// ============================================================

/// 截出 Gutenberg 正文区（`*** START OF …` 与 `*** END OF …` 之间）。
///
/// 找不到起始标记 ⇒ 用整篇文本（宽容：允许喂进来的是**已经切好**的正文）。
/// 找不到结束标记 ⇒ 取到文末。
fn slice_region(text: &str) -> &str {
    let mut start = 0usize;
    let mut end = text.len();
    let mut found_start = false;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if !found_start && trimmed.starts_with(START_MARKER) {
            start = offset + line.len();
            found_start = true;
        } else if trimmed.starts_with(END_MARKER) {
            end = offset;
            break;
        }
        offset += line.len();
    }
    if start > end {
        return "";
    }
    &text[start..end]
}

/// 拆出「回次 + 其后文字」；不是章节标题行 ⇒ `None`。
///
/// 回次允许**中文数字或阿拉伯数字**（源文件两种都出现过）。
fn split_chapter_head(line: &str) -> Option<(String, &str)> {
    let mut chars = line.chars();
    if chars.next()? != '第' {
        return None;
    }
    let mut number = String::new();
    let mut rest: Option<&str> = None;
    for ch in chars.by_ref() {
        if ch == '回' {
            // `chars.as_str()` 给出**「回」之后**的剩余文字。
            rest = Some(chars.as_str());
            break;
        }
        if !is_ordinal_char(ch) {
            return None;
        }
        number.push(ch);
    }
    if number.is_empty() {
        return None;
    }
    Some((number, rest?))
}

/// 这行**是不是**章节标题行（含「正文行恰好以『第X回』开头」的排除）。
fn chapter_title_at(line: &str) -> Option<(String, Vec<&str>)> {
    let (number, rest) = split_chapter_head(line)?;
    let segments = whitespace_segments(rest);
    if segments
        .first()
        .is_some_and(|segment| !is_couplet_clause(segment))
    {
        return None;
    }
    Some((number, segments))
}

/// 按空白切段（**含全角空格** `U+3000` —— 回目联的行内分隔就是它）。
fn whitespace_segments(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// 回次字符（中文数字 + 阿拉伯数字）。
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
        )
}

/// 是不是回次里的数字字（中文）。
const CHINESE_DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];

/// 回次的中文写法（**与源文件同形**）。
///
/// * 1~9：`一` … `九`；
/// * 10~19：`十`、`十一` … `十九`；
/// * 20~99：`二十`、`二十一` …；
/// * ≥100：**逐位**写（`一零四`、`一二零`）—— 这是《紅樓夢》源文件的写法，
///   照抄才能让「第几回」与原文对得上。
fn render_ordinal(index: usize) -> String {
    if index >= 100 {
        return index
            .to_string()
            .chars()
            .filter_map(|ch| ch.to_digit(10))
            .map(digit_word)
            .collect();
    }
    if index < 10 {
        return CHINESE_DIGITS[index].to_owned();
    }
    let tens = index / 10;
    let ones = index % 10;
    if tens == 1 {
        return if ones == 0 {
            "十".to_owned()
        } else {
            format!("十{}", CHINESE_DIGITS[ones])
        };
    }
    if ones == 0 {
        format!("{}十", CHINESE_DIGITS[tens])
    } else {
        format!("{}十{}", CHINESE_DIGITS[tens], CHINESE_DIGITS[ones])
    }
}

/// 数字字（0~9）的中文写法。
fn digit_word(digit: u32) -> &'static str {
    match digit {
        0 => "零",
        1 => "一",
        2 => "二",
        3 => "三",
        4 => "四",
        5 => "五",
        6 => "六",
        7 => "七",
        8 => "八",
        _ => "九",
    }
}

/// 解析回次里的数字（中文数字 + 阿拉伯数字）。
///
/// # 两种写法都收
///
/// * **逐位**写法：源文件《紅樓夢》第一百回以后是 `一零四`（= 104）、`一二零`（= 120）
///   —— 不含「十百千」时按**位**拼；
/// * **标准**写法：`十` / `二十一` / `一百零四` —— 含「十百千」时按**位值**算。
///
/// 两种写法在 1~99 上等价（`一一` = `十一` = 11）⇒ 解析结果与源文件写法无关。
fn parse_ordinal(text: &str) -> Option<usize> {
    if text.is_empty() {
        return None;
    }
    if text.chars().all(|ch| ch.is_ascii_digit()) {
        return text.parse::<usize>().ok();
    }
    if !text.chars().any(|ch| matches!(ch, '十' | '百' | '千')) {
        // 逐位写法：一零四 ⇒ 104。
        let mut value = 0usize;
        for ch in text.chars() {
            value = value.checked_mul(10)?.checked_add(chinese_digit(ch)?)?;
        }
        return Some(value);
    }
    // 标准写法：十 / 二十一 / 一百零四。
    let mut section = 0usize;
    let mut number = 0usize;
    for ch in text.chars() {
        match ch {
            '零' => number = 0,
            '十' => {
                section = section.checked_add(number.checked_mul(10)?)?;
                number = 0;
            }
            '百' => {
                section = section.checked_add(number.checked_mul(100)?)?;
                number = 0;
            }
            '千' => {
                section = section.checked_add(number.checked_mul(1000)?)?;
                number = 0;
            }
            _ => number = chinese_digit(ch)?,
        }
    }
    section.checked_add(number)
}

/// 中文数字字（0~9）；`兩` 是「二」的异体。
fn chinese_digit(ch: char) -> Option<usize> {
    Some(match ch {
        '零' => 0,
        '一' => 1,
        '二' | '兩' => 2,
        '三' => 3,
        '四' => 4,
        '五' => 5,
        '六' => 6,
        '七' => 7,
        '八' => 8,
        '九' => 9,
        _ => return None,
    })
}

/// 是不是分隔线（`-----` / `=====` 之类；Gutenberg 在标题后放的那一行）。
fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().count() >= 3
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '-' | '=' | '\u{2500}' | '\u{2014}'))
}

/// 是不是回目联的一句（无句读 + 长度像一句对联）。
fn is_couplet_clause(segment: &str) -> bool {
    let chars = segment.chars().count();
    chars >= 2
        && chars <= MAX_COUPLET_CLAUSE_CHARS
        && !segment.chars().any(|ch| SENTENCE_PUNCT.contains(ch))
}

/// 取前 [`MAX_COUPLET_CLAUSES`] 个像回目联的段（遇到不像的就停）。
fn couplet_clauses<'a>(segments: &[&'a str]) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for segment in segments.iter().take(MAX_COUPLET_CLAUSES) {
        if !is_couplet_clause(segment) {
            break;
        }
        out.push(segment);
    }
    out
}

/// 拼正文：折行合并成**空格**，空行保留为**段落分隔**，分隔线丢弃。
///
/// # 为什么折行合并成空格而不是直接粘上
///
/// 源文件的正文是**硬折行**（一行 30 来字），折行处**往往没有标点** ⇒ 直接粘上会造出
/// 「……哈。哈。」这种**不存在的怪词** ⇒ 答案片段会引到原文里根本没有的串。
fn assemble_body(lines: &[&str], leftover: &[&str]) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in lines {
        if line.is_empty() || is_separator(line) {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    // 标题行上粘下来的正文（源文件把正文折行粘在了标题行末尾）⇒ 放回正文最前面。
    if !leftover.is_empty() {
        let head = leftover.join(" ");
        match paragraphs.first_mut() {
            Some(first) => {
                first.insert_str(0, &format!("{head} "));
            }
            None => paragraphs.push(head),
        }
    }
    paragraphs.join("\n\n")
}
