//! **LoCoMo 真实对话记忆基准**的加载与考卷构造。
//!
//! # 为什么有这个东西（**D-13 的方法论缺口**）
//!
//! 此前所有记忆质量数字都来自**两部古典小说**，而且「正确答案」是
//! **我们自己**按「含主体的首句」挖出来的 ⇒ ⇒
//! * 字面重合红利**不可控**；
//! * 送达率只能给**区间**（4.6× ~ 9.4×），因为乐观端含「基准构造红利」。
//!
//! ⇒ **LoCoMo**（Snap Research）是**公开的真实对话记忆基准**：
//! * 10 个**长期**对话（每个 19–35 个**跨日期**会话）；
//! * **1,986 组人工标注 QA**，其中 **99%** 带 `evidence`
//!   （指向**具体对话轮次** `D<会话>:<轮次>`）；
//! ⇒ ⇒ **ground truth 是人标的** ⇒ ⇒ 没有「我们自己挖题」的红利
//! ⇒ ⇒ ⇒ 送达率可以从**区间**收成**一个点**。
//!
//! # 口径（**逐条钉死**）
//!
//! * **一条记忆 = 一个对话轮次**：`id = "{sample_id}::{dia_id}"`，
//!   `text = "{speaker}: {turn_text}"`；
//! * **证据片段**取该轮**原文**（过长则取前 [`FRAGMENT_MAX_CHARS`] 字）
//!   ⇒ ⇒ 必须是 memory 正文的**真子串**（**有 assert 钉着**）；
//! * ⚠️ **对抗类（`category = 5`）不计入考卷**：那类题答案是 `None`
//!   ⇒ ⇒「召回」对它**不是正确指标** ⇒ ⇒ 计入等于**把陷阱题当普通题算**
//!   ⇒ ⇒ 如实**单独统计**（见 [`LocomoSample::adversarial_cases`]）；
//! * **只用有 `evidence` 且轮次真的存在的题** ⇒ ⇒ 其余**如实计数**，不静默丢。

use std::collections::BTreeMap;
use std::path::Path;

use crate::golden_run::IndexRecord;
use crate::golden_set::GoldenCase;

/// 证据片段最多取多少字（**过长只取前缀** ⇒ 保证它仍是正文子串）。
///
/// ⚠️ 生产注入口径每条证据只给 **120 字**（`memory_injection::EVIDENCE_MAX_CHARS`）
/// ⇒ ⇒ 片段若超过 120 字，**即使检索完全正确也永远 ground 不上**
/// ⇒ ⇒ 所以这里取 **100 字**，留出 `"speaker: "` 前缀的余量。
pub const FRAGMENT_MAX_CHARS: usize = 100;

/// 对抗类题目的 `category` 值（LoCoMo 定义：答案是 `None` ⇒ 陷阱题）。
pub const ADVERSARIAL_CATEGORY: u32 = 5;

/// 一个 LoCoMo 样本（= 一段长期对话）。
#[derive(Debug, Clone)]
pub struct LocomoSample {
    /// 样本 id（LoCoMo 里的 `sample_id`）。
    pub sample_id: String,
    /// 两位说话人。
    pub speakers: (String, String),
    /// 会话日期（`session_N` → 日期时间字符串），按会话序号升序。
    pub session_dates: Vec<(usize, String)>,
    /// 记忆库（一条 = 一个对话轮次）。
    pub records: Vec<IndexRecord>,
    /// 考卷（**已剔除对抗类**）。
    pub cases: Vec<GoldenCase>,
    /// 对抗类题数（**不计入考卷** ⇒ 如实单独统计）。
    pub adversarial_cases: usize,
    /// 因 `evidence` 指向不存在的轮次 / 缺 `evidence` 而**没进考卷**的题数。
    pub skipped_cases: usize,
    /// 每个类别进了考卷多少题（`category` → 题数）。
    pub cases_per_category: BTreeMap<u32, usize>,
}

impl LocomoSample {
    /// 轮次总数。
    pub fn turn_count(&self) -> usize {
        self.records.len()
    }
}

/// 解析原始 JSON 文本（**纯函数** ⇒ 好测）。
///
/// # 返回
///
/// `Err(message)` ⇒ JSON 不合法 / 形状不对 ⇒ **明确报错**（不静默返回空）。
pub fn parse(json_text: &str) -> Result<Vec<LocomoSample>, String> {
    let root: serde_json::Value =
        serde_json::from_str(json_text).map_err(|error| format!("JSON 解析失败：{error}"))?;
    let samples = root
        .as_array()
        .ok_or_else(|| "顶层应是数组（LoCoMo 的样本列表）".to_string())?;
    let mut out = Vec::with_capacity(samples.len());
    for sample in samples {
        out.push(parse_sample(sample)?);
    }
    if out.is_empty() {
        return Err("一个样本都没解析出来".to_string());
    }
    Ok(out)
}

/// 从磁盘加载。
pub fn load(path: &Path) -> Result<Vec<LocomoSample>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读不到 {}：{error}", path.display()))?;
    parse(&text)
}

/// 解析单个样本。
fn parse_sample(sample: &serde_json::Value) -> Result<LocomoSample, String> {
    let sample_id = sample
        .get("sample_id")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let conversation = sample
        .get("conversation")
        .ok_or_else(|| format!("{sample_id}：缺 conversation"))?;
    let speakers = (
        conversation
            .get("speaker_a")
            .and_then(|value| value.as_str())
            .unwrap_or("A")
            .to_string(),
        conversation
            .get("speaker_b")
            .and_then(|value| value.as_str())
            .unwrap_or("B")
            .to_string(),
    );

    // ① 会话序号（`session_<n>` / `session_<n>_date_time`）⇒ 按**数值**升序（不是字典序）。
    let mut session_indexes: Vec<usize> = Vec::new();
    let mut session_dates: Vec<(usize, String)> = Vec::new();
    if let Some(object) = conversation.as_object() {
        for key in object.keys() {
            if let Some(rest) = key.strip_prefix("session_") {
                if let Some(number) = rest.strip_suffix("_date_time") {
                    if let Ok(index) = number.parse::<usize>() {
                        if !session_indexes.contains(&index) {
                            session_indexes.push(index);
                        }
                        let date = object
                            .get(key)
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string();
                        session_dates.push((index, date));
                    }
                }
            }
        }
    }
    session_indexes.sort_unstable();
    session_dates.sort_by_key(|(index, _)| *index);

    // ② 一条记忆 = 一个对话轮次。
    let mut records: Vec<IndexRecord> = Vec::new();
    // `dia_id` → 该轮存入的正文（构造证据片段时要用 ⇒ **必须与实际正文一致**）。
    let mut turn_texts: BTreeMap<String, String> = BTreeMap::new();
    for index in &session_indexes {
        let key = format!("session_{index}");
        let Some(turns) = conversation.get(&key).and_then(|value| value.as_array()) else {
            continue;
        };
        for turn in turns {
            let Some(dia_id) = turn.get("dia_id").and_then(|value| value.as_str()) else {
                continue;
            };
            let speaker = turn
                .get("speaker")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let body = turn
                .get("text")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if body.trim().is_empty() {
                continue;
            }
            let text = format!("{speaker}: {body}");
            let id = format!("{sample_id}::{dia_id}");
            turn_texts.insert(dia_id.to_string(), text.clone());
            records.push(IndexRecord { id, text });
        }
    }
    if records.is_empty() {
        return Err(format!("{sample_id}：一个对话轮次都没解析出来"));
    }

    // ③ 考卷：只用有 `evidence` 且轮次存在的题；**剔除对抗类**。
    let mut cases: Vec<GoldenCase> = Vec::new();
    let mut adversarial_cases = 0usize;
    let mut skipped_cases = 0usize;
    let mut cases_per_category: BTreeMap<u32, usize> = BTreeMap::new();
    if let Some(qa_list) = sample.get("qa").and_then(|value| value.as_array()) {
        for (position, qa) in qa_list.iter().enumerate() {
            let category = qa
                .get("category")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as u32;
            if category == ADVERSARIAL_CATEGORY {
                // ⚠️ 对抗类（答案 `None`）⇒ **不进考卷** ⇒ 单独计数。
                adversarial_cases += 1;
                continue;
            }
            let question = qa
                .get("question")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if question.trim().is_empty() {
                skipped_cases += 1;
                continue;
            }
            let evidence: Vec<String> = qa
                .get("evidence")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            if evidence.is_empty() {
                skipped_cases += 1;
                continue;
            }
            let mut expect_ids: Vec<String> = Vec::new();
            let mut expect_fragments: Vec<String> = Vec::new();
            let mut missing = false;
            for dia_id in &evidence {
                let Some(text) = turn_texts.get(dia_id) else {
                    missing = true;
                    break;
                };
                // ⚠️ **断言**：片段必须是正文的真子串（⇒ 否则永远 ground 不上，
                // 而**沉默的偏差**以前害过我 —— 见 README-worktree.md §9）。
                let fragment: String = text.chars().take(FRAGMENT_MAX_CHARS).collect();
                debug_assert!(
                    text.contains(&fragment),
                    "片段必须是正文子串：{fragment:?} ⊄ {text:?}"
                );
                let id = format!("{sample_id}::{dia_id}");
                if !expect_ids.contains(&id) {
                    expect_ids.push(id);
                }
                if !expect_fragments.contains(&fragment) {
                    expect_fragments.push(fragment);
                }
            }
            if missing || expect_ids.is_empty() {
                skipped_cases += 1;
                continue;
            }
            *cases_per_category.entry(category).or_insert(0) += 1;
            cases.push(GoldenCase {
                id: format!("{sample_id}::qa{position}::cat{category}"),
                query: question.to_string(),
                expect_ids,
                expect_fragments,
                tags: vec![format!("cat:{category}")],
            });
        }
    }
    if cases.is_empty() {
        return Err(format!("{sample_id}：一道可评的题都没有"));
    }

    Ok(LocomoSample {
        sample_id,
        speakers,
        session_dates,
        records,
        cases,
        adversarial_cases,
        skipped_cases,
        cases_per_category,
    })
}

/// 找 LoCoMo 数据文件（`$APEIRETH_DATA_DIR/real-corpus/` 或 `~/.apeireth/real-corpus/`）。
pub fn default_path() -> Option<std::path::PathBuf> {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        roots.push(std::path::PathBuf::from(dir).join("real-corpus"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(
            std::path::PathBuf::from(home)
                .join(".apeireth")
                .join("real-corpus"),
        );
    }
    roots
        .into_iter()
        .map(|root| root.join("locomo10.json"))
        .find(|path| path.exists())
}
