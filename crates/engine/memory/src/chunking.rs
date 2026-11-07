//! `apeireth-memory::chunking` — 语料的**段落级切分**（召回缺口「切分粒度」假设的**验证器**）。
//!
//! # 为什么有这个模块
//!
//! 诊断（`golden_diag`，W-16 实测）证明：目标记录**每次都进了候选池**（NotInPool = 0），
//! 但 62.4% 的题**目标排在 top-10 之外**（名次 p50 = 16）。
//! 当时的假设是：**根因 = 切分粒度**（一回 = **7401 字**一条记录 ⇒ 主体词平均出现在
//! **88/120** 条里 ⇒ **top-10 里 9/10 条都是同主体记录**）⇒ 本模块就是来**验证**这条假设的。
//!
//! # ⚠️ 实测结论（2026-10-27，真实语料 + 生产 BM25，k=10）
//!
//! **「切分粒度 = 召回根因」这条假设被否证了** —— 段落化不是把 recall 拉高，而是**拉低**：
//!
//! | 对照口径 | 《紅樓夢》 | 《水滸傳》 |
//! |---|---|---|
//! | 整回（现状） | 0.2983 | 0.6875 |
//! | 段落 300（**同一批题**，只改粒度） | **0.0693** | **0.0625** |
//! | 段落 300（在段落上**重挖题**） | 0.3282 | 0.5418 |
//! | 整回 · 单记录题（与上一行**同题型**） | **0.3820** | **0.8831** |
//!
//! ⇒ **提不上去**：同题型对照（0.3820 → 0.3282 / 0.8831 → 0.5418）**两头都掉**。
//!
//! **为什么**（机制，不是猜测）：题面**只有主体词**这一个判别信号。
//!
//! * **整回粒度**：目标回把主体词说很多遍 ⇒ **词频（TF）本身就是判别信号**
//!   ⇒ 在 88 个同主体候选里还能挤进 top-10；
//! * **段落粒度**：目标段落里主体词只出现一两次 ⇒ **TF 不再判别**
//!   ⇒ 同主体候选从 88 条涨到 **360** 条（紅樓夢）⇒ 目标段落被埋在中间
//!   （实测：仍**在候选池**里，但名次远在 k 之外；k=1000 时 recall 才回到 0.89）。
//!
//! ⇒ 结论：**瓶颈不是粒度，是查询的判别力**（题面只有主体词 ⇒ 没有第二信号）。
//! ⇒ 本模块的定位因此变成：**可复现的切分器 + 可复现的对照实验**
//! （要提 recall，得先改**题面/查询**或上**语义/向量**路，**不是**改切分粒度）。
//!
//! # 纪律
//!
//! * **不丢字**：切分后的所有段落拼起来 ⇒ 与原正文**逐字相同**（[`verify_lossless`] 可验证）；
//! * **不造边界**：段落**不跨**章节（一次 [`chunk_record`] 只处理一条记录的正文）；
//!   句子**不跨**段落（除非该句本身超长 ⇒ 那才允许跨，且**如实**标 [`Chunk::truncated`]）；
//! * **确定性**：同输入 ⇒ 同输出（无随机数、无哈希序依赖、无读时钟）。
//!
//! # 口径细节（钉死，别自创）
//!
//! * **断句符**：`。！？；!?;` 与换行（`\n` / `\r`）—— **与 `golden_mining::split_sentences` 同口径**
//!   （同一组断句符）。但**刻意不同**两处，否则会丢字：
//!   1. 本模块**把断句符留在句段里**（那边是「消费掉」的）；
//!   2. 本模块**不 trim**（那边会 trim）⇒ 空白也逐字保留。
//!   ⇒ 段落拼起来 = 原文**逐字相同**。
//! * **长度口径**：一律按**字符**（不是字节）计 ⇒ 中文不被算成 3 倍。
//! * **硬上限优先于硬下限**：短尾段并入前一段时，若合并后会**超过 `max_chars`**
//!   ⇒ **不并**（宁可留一个短段，也不越硬上限）—— 这是**如实**的取舍，不是 bug。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：切分 + 校验 + 统计齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**未接入**任何生产路径（无 CLI、无定时任务、生产 0 调用点）；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✔（部分）：已在《紅樓夢》《水滸傳》**真实语料**上跑过
//!   （2026-10-27，见 `examples/chunking_report.rs`），**未在第二台机器复现**
//!   ⇒ 不宣称跨机器可复现。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

/// 切分配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkConfig {
    /// 目标段落长度（字符）。
    ///
    /// 段落**累加**到 `target_chars` 附近就收段（可以略微超出 ⇒ 句子是最小单位，不切碎）。
    pub target_chars: usize,
    /// 硬上限（超过就切在半句处 ⇒ **如实**标明 [`Chunk::truncated`]）。
    ///
    /// 保证：**每个段落的字符数 ≤ `max_chars`**（含硬切出来的段）。
    pub max_chars: usize,
    /// 硬下限（太短的段落**并入前一段**，除非它是最后一段）。
    ///
    /// ⚠️ **口径澄清**：本字段的注释说「除非它是最后一段」，
    /// 而 [`chunk_record`] 的口径是「**尾段**并入前一段（除非它是唯一一段）」。
    /// ⇒ 两者在「**最后一段**」上不一致 ⇒ 本模块按**函数级口径**实现
    /// （**尾段并入前一段**），因为它是更具体的那一条；
    /// 唯一段（整条记录只切出一段）⇒ 保持原样（不并，因为没有前一段可并）。
    pub min_chars: usize,
}

impl ChunkConfig {
    /// 构造（`target = 300` / `max = 400` / `min = 120`）。
    pub fn new(target_chars: usize, max_chars: usize, min_chars: usize) -> Self {
        Self {
            target_chars,
            max_chars,
            min_chars,
        }
    }
}

/// 一段（**切分产物**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// 段落的**全局序号**（从 0 起，跨章连续）。
    pub index: usize,
    /// 所属记录的 id（**如 `novel-0042`** ⇒ 可回溯到原记录）。
    pub record_id: String,
    /// 段落正文。
    ///
    /// ⚠️ **逐字切片**：本段是原记录正文的**连续切片**（含空白与断句符）
    /// ⇒ 所有段落的 `text` 拼起来 == 原正文（[`verify_lossless`] 可验证）。
    pub text: String,
    /// 该段在原记录正文里的**字符起点**（⇒ 可回溯、可验证「不丢字」）。
    ///
    /// ⚠️ 按**字符**计（不是字节）⇒ `原文.chars().skip(start_char)` 即本段开头。
    pub start_char: usize,
    /// 该段是否**被硬切过**（超长句被切断 ⇒ **如实**标明）。
    pub truncated: bool,
}

/// 把一条长记录切成段落。
///
/// # 口径
///
/// * 以**句**为最小单位（用 `。！？；` 等断句，**逐字照实现自己的断句**：
///   与 `golden_mining::split_sentences` **同口径**（同一组断句符），
///   但本模块**保留断句符、不 trim** ⇒ 不丢字）；
/// * 段落长度累加到 `target_chars` 附近就收段；
/// * 超过 `max_chars` ⇒ **硬切**并标 `truncated = true`（**如实**，不假装完整）；
/// * 短于 `min_chars` 的尾段 ⇒ **并入前一段**（除非它是唯一一段）；
/// * **不丢字**：所有段落的 `text` 拼起来 == 原正文（`start_char` 可逐段验证）。
///
/// # 边界
///
/// * 空正文 ⇒ 返回**空 Vec**（不 panic）；
/// * `max_chars == 0` ⇒ 按 **1** 处理（避免死循环；每段至少 1 字）。
pub fn chunk_record(record_id: &str, body: &str, config: ChunkConfig) -> Vec<Chunk> {
    split_body(body, config)
        .into_iter()
        .enumerate()
        .map(|(index, raw)| Chunk {
            index,
            record_id: record_id.to_string(),
            text: raw.text,
            start_char: raw.start_char,
            truncated: raw.truncated,
        })
        .collect()
}

/// 切一批记录（**保持记录顺序** ⇒ 段落序号跨记录连续）。
///
/// # 口径
///
/// * `records` 的每一项是 `(记录 id, 正文)`；
/// * 段落序号**跨记录连续**（第 2 条记录的第一段接着第 1 条记录的最后一段编号）；
/// * 每条记录**独立切分** ⇒ 段落**不跨记录**（= 不跨章节）。
pub fn chunk_records(records: &[(String, String)], config: ChunkConfig) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(records.len());
    for (id, body) in records {
        for raw in split_body(body, config) {
            out.push(Chunk {
                index: out.len(),
                record_id: id.clone(),
                text: raw.text,
                start_char: raw.start_char,
                truncated: raw.truncated,
            });
        }
    }
    out
}

/// **校验「不丢字」**：段落拼起来是否与原文逐字相同。
///
/// # 返回
///
/// * `Ok(())` = 完全一致；
/// * `Err(reason)` = 不一致（**如实报告**，并说明差在哪）。
///
/// # 校验两件事
///
/// 1. **拼接一致**：所有 `text` 拼起来 == `original`（逐字）；
/// 2. **位置一致**：每段的 `start_char` 落在原文的**那个位置**（逐字切片相等）
///    —— 只查拼接会漏掉「顺序错了但总长相同」的情况。
pub fn verify_lossless(chunks: &[Chunk], original: &str) -> Result<(), String> {
    // --- 位置校验（先查它，因为它能定位问题）---
    for (position, chunk) in chunks.iter().enumerate() {
        let actual: String = original
            .chars()
            .skip(chunk.start_char)
            .take(chunk.text.chars().count())
            .collect();
        if actual != chunk.text {
            return Err(format!(
                "第 {} 段的位置对不上：start_char={} 处取到的文本与段落正文不等（原文该处 = {:?}，段落 = {:?}）",
                position,
                chunk.start_char,
                truncate_for_message(&actual),
                truncate_for_message(&chunk.text)
            ));
        }
    }

    // --- 拼接校验 ---
    let mut rebuilt = String::new();
    for chunk in chunks {
        rebuilt.push_str(&chunk.text);
    }
    if rebuilt == original {
        return Ok(());
    }
    let rebuilt_chars: Vec<char> = rebuilt.chars().collect();
    let original_chars: Vec<char> = original.chars().collect();
    let first_diff = rebuilt_chars
        .iter()
        .zip(original_chars.iter())
        .position(|(left, right)| left != right);
    Err(match first_diff {
        Some(position) => format!(
            "拼接后的正文与原文不等：长度 {} vs {} 字；第一处差异在字符 {}（拼接 = {:?}，原文 = {:?}）",
            rebuilt_chars.len(),
            original_chars.len(),
            position,
            truncate_for_message(&rebuilt_chars[position..].iter().collect::<String>()),
            truncate_for_message(&original_chars[position..].iter().collect::<String>())
        ),
        None => format!(
            "拼接后的正文与原文不等：长度 {} vs {} 字（拼接是原文的**前缀** ⇒ 少字）",
            rebuilt_chars.len(),
            original_chars.len()
        ),
    })
}

/// **同主体竞争者密度**（诊断用）：有多少条段落也提到了这个词。
///
/// # 口径
///
/// 统计 `chunks` 里**正文包含该词**的段落数 ⇒ 这是「主体越常见越排不出去」的量化。
///
/// * **空探针**（`subject` 为空串）⇒ 返回 **0**（空串被任何文本包含 ⇒ 若照字面统计会
///   得出「到处都出现」的假结论 ⇒ 这里**明确排除**）。
pub fn competitor_density(chunks: &[Chunk], subject: &str) -> usize {
    if subject.is_empty() {
        return 0;
    }
    chunks
        .iter()
        .filter(|chunk| chunk.text.contains(subject))
        .count()
}

/// 切分质量的汇总（用于报告）。
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkStats {
    /// 段落数。
    pub chunks: usize,
    /// 平均长度（字符；空输入 ⇒ `0.0`）。
    pub avg_chars: f64,
    /// 最短 / 最长（字符；空输入 ⇒ `0`）。
    pub min_chars: usize,
    /// 最长（字符；空输入 ⇒ `0`）。
    pub max_chars: usize,
    /// **被硬切过**的段落数（**该是 0** ⇒ 非 0 说明句子太长）。
    pub truncated: usize,
}

/// 汇总。
pub fn stats(chunks: &[Chunk]) -> ChunkStats {
    if chunks.is_empty() {
        return ChunkStats {
            chunks: 0,
            avg_chars: 0.0,
            min_chars: 0,
            max_chars: 0,
            truncated: 0,
        };
    }
    let mut total = 0usize;
    let mut min_chars = usize::MAX;
    let mut max_chars = 0usize;
    let mut truncated = 0usize;
    for chunk in chunks {
        let length = chunk.text.chars().count();
        total += length;
        min_chars = min_chars.min(length);
        max_chars = max_chars.max(length);
        if chunk.truncated {
            truncated += 1;
        }
    }
    ChunkStats {
        chunks: chunks.len(),
        avg_chars: total as f64 / chunks.len() as f64,
        min_chars,
        max_chars,
        truncated,
    }
}

// ============================================================
// 内部
// ============================================================

/// 切分的中间产物（还没挂上 id / 全局序号）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawChunk {
    /// 段落正文（逐字切片）。
    text: String,
    /// 在原正文里的**字符**起点。
    start_char: usize,
    /// 是否被硬切过。
    truncated: bool,
}

/// 切分主逻辑（`chunk_record` 与 `chunk_records` 共用）。
///
/// # 算法（贪心，确定性）
///
/// 1. 先把正文切成**句段**（[`split_verbatim`]：保留断句符与空白 ⇒ 不丢字）；
/// 2. 逐句装段：
///    * 单句 > `max_chars` ⇒ **硬切**成若干 ≤ `max_chars` 的片（标 `truncated`）；
///    * 当前段已达 `target_chars` ⇒ 收段，另起一段；
///    * 当前段 + 本句 ≤ `max_chars` ⇒ 继续装；
///    * 否则收段，另起一段；
/// 3. 收段时若本段 < `min_chars` 且**有前一段**且合并后仍 ≤ `max_chars`
///    ⇒ **并入前一段**（硬上限优先 ⇒ 装不下就不并）。
fn split_body(body: &str, config: ChunkConfig) -> Vec<RawChunk> {
    if body.is_empty() {
        return Vec::new();
    }
    // `max_chars == 0` 会让「装不下任何东西」⇒ 死循环 ⇒ 兜底成 1。
    let max = config.max_chars.max(1);
    let target = config.target_chars.max(1);
    let min = config.min_chars;

    let mut out: Vec<RawChunk> = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;

    for (start, sentence) in split_verbatim(body) {
        let sentence_len = sentence.chars().count();
        if sentence_len > max {
            // 本句本身就超长 ⇒ 先收当前段，再硬切本句。
            flush(&mut out, &mut current, &current_start, min, max);
            let chars: Vec<char> = sentence.chars().collect();
            let mut offset = 0usize;
            while offset < chars.len() {
                let end = (offset + max).min(chars.len());
                out.push(RawChunk {
                    text: chars[offset..end].iter().collect(),
                    start_char: start + offset,
                    truncated: true,
                });
                offset = end;
            }
            continue;
        }

        if current.is_empty() {
            current = sentence.to_string();
            current_start = start;
            continue;
        }
        let current_len = current.chars().count();
        if current_len >= target {
            flush(&mut out, &mut current, &current_start, min, max);
            current = sentence.to_string();
            current_start = start;
            continue;
        }
        if current_len + sentence_len <= max {
            current.push_str(sentence);
            continue;
        }
        flush(&mut out, &mut current, &current_start, min, max);
        current = sentence.to_string();
        current_start = start;
    }
    flush(&mut out, &mut current, &current_start, min, max);
    out
}

/// 收段：太短 ⇒ 并入前一段（装得下才并；**硬上限优先**）。
fn flush(
    out: &mut Vec<RawChunk>,
    current: &mut String,
    current_start: &usize,
    min: usize,
    max: usize,
) {
    if current.is_empty() {
        return;
    }
    let chunk = RawChunk {
        text: std::mem::take(current),
        start_char: *current_start,
        truncated: false,
    };
    let length = chunk.text.chars().count();
    if length < min {
        if let Some(previous) = out.last_mut() {
            let merged = previous.text.chars().count() + length;
            if merged <= max {
                previous.text.push_str(&chunk.text);
                previous.truncated = previous.truncated || chunk.truncated;
                return;
            }
            // ⚠️ **硬上限优先于硬下限**：合并会越上限 ⇒ 不并（如实留一个短段）。
        }
        // 没有前一段可并（这是唯一一段）⇒ 保持原样。
    }
    out.push(chunk);
}

/// 把正文切成**句段**（**保留断句符与空白** ⇒ 拼回去逐字等于原文）。
///
/// # 与 `golden_mining::split_sentences` 的关系
///
/// **同口径**：同一组断句符（`。！？；!?;` + 换行）⇒ 切出来的「句」边界一致。
///
/// **刻意不同**（否则必然丢字）：
///
/// 1. 本函数**把断句符留在句段里**（那边是「消费掉」的）；
/// 2. 本函数**不 trim**（那边会 trim）⇒ 空白也逐字保留。
///
/// # 返回
///
/// `(字符起点, 句段文本)` 的列表；每段都是 `body` 的**连续切片**。
fn split_verbatim(body: &str) -> Vec<(usize, &str)> {
    let mut out: Vec<(usize, &str)> = Vec::new();
    let mut byte_start = 0usize;
    let mut segment_start_char = 0usize;
    let mut char_index = 0usize;
    for (offset, ch) in body.char_indices() {
        if is_sentence_break(ch) {
            let end = offset + ch.len_utf8();
            out.push((segment_start_char, &body[byte_start..end]));
            byte_start = end;
            segment_start_char = char_index + 1;
        }
        char_index += 1;
    }
    if byte_start < body.len() {
        out.push((segment_start_char, &body[byte_start..]));
    }
    out
}

/// 句子边界字符（**与 `golden_mining::is_sentence_break` 同口径**）。
fn is_sentence_break(ch: char) -> bool {
    matches!(
        ch,
        '。' | '！' | '？' | '!' | '?' | '；' | ';' | '\n' | '\r'
    )
}

/// 错误消息里截断过长文本（只留前 20 字）—— 免得一条错误信息刷屏。
fn truncate_for_message(text: &str) -> String {
    let mut out: String = text.chars().take(20).collect();
    if text.chars().count() > 20 {
        out.push('…');
    }
    out
}
