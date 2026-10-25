//! `apeireth-memory::perltqa_adapter` — 外部公开卷（PerLTQA）→ 本仓考卷格式的转换。
//!
//! **状态：NOT IMPLEMENTED（新增未接线，2026-10-25）**
//! 四级口径：IMPLEMENTED ✔（代码 + 测试） / PRODUCTION WIRED ✘（生产 0 调用点）
//! / DEFAULT ENABLED ✘（无开关位） / HARDWARE VALIDATED ✘（真实数据未落地）。
//!
//! # 为什么需要这个适配器
//!
//! 设计稿（`docs/01-architecture/memory-knowledge-depth-system-spec.md` §7）定的是**双轨考卷**：
//! 外部公开卷证明「不落后于公开水平」，内部卷证明「我们自己的场景好用」。
//! 外部卷里 **PerLTQA**（[arXiv 2402.16288](https://arxiv.org/abs/2402.16288)，SIGHAN-2024，中文）
//! 的每条 QA 是四元组：
//!
//! | PerLTQA 字段 | 含义 | 本仓对应 |
//! |---|---|---|
//! | `question` | 问题 | [`GoldenCase::query`] |
//! | `reference memory` | 该命中的记忆（标准答案） | [`GoldenCase::expect_ids`] |
//! | `memory anchor` | 答案里与记忆对齐的**关键片段** | [`GoldenCase::expect_fragments`] |
//! | `answer` | 标准答案 | （本仓只测检索/注入，**不吃 LLM 判分**） |
//!
//! ⇒ 转换是**纯字段映射**，不需要跑任何模型 ⇒ 便宜、可复现。
//!
//! # 关键口径（不猜）
//!
//! * **`expect_ids` 用什么 id**：PerLTQA 的记忆库里**没有**稳定 id 字符串
//!   （它是按角色组织的若干条记忆文本）⇒ 本仓用**内容指纹**（`sha256` 前 12 位十六进制，
//!   与 [`GoldenSet::load_jsonl`] 的版本号口径一致）作为记录 id。
//!   ⇒ **同一份数据转换两次必得同一批 id**（可复现），且能跨卷对照。
//! * **`expect_fragments` 取什么**：优先取 `memory anchor`（它就是"答案里对齐记忆的关键片段"）；
//!   没有 anchor 时退化为该条 reference memory 的**首句**（截断到 ≤120 字）。
//! * **不做的事**：**不下载数据集**（数据不入库）、**不跑 LLM 判分**、**不重排**。
//!   数据到位前本模块只保证「转换正确」。

use crate::golden_set::{GoldenCase, GoldenSet};

/// PerLTQA 的一条原始 QA（**只取我们要用的字段**；其余字段忽略）。
///
/// 用 `serde_json::Value` 接收而不是强类型 struct ⇒ PerLTQA 的字段名在不同版本里
/// 可能有出入（`memory` / `reference_memory` 等），由本模块做**多名字兼容**。
#[derive(Debug, Clone, PartialEq)]
pub struct PerLtqaItem {
    /// 原始 JSON 记录。
    pub raw: serde_json::Value,
}

impl PerLtqaItem {
    /// 构造。
    pub fn new(raw: serde_json::Value) -> Self {
        Self { raw }
    }

    /// 取问题文本。
    pub fn question(&self) -> Option<&str> {
        pick_str(&self.raw, &["question", "query", "q"])
    }

    /// 取「该命中的记忆」列表（可能是一条，也可能是多条）。
    pub fn reference_memories(&self) -> Vec<&str> {
        pick_str_list(
            &self.raw,
            &[
                "reference memory",
                "reference_memory",
                "reference",
                "memory",
            ],
        )
    }

    /// 取「记忆锚点」（答案里对齐记忆的关键片段）。
    pub fn memory_anchors(&self) -> Vec<&str> {
        pick_str_list(
            &self.raw,
            &["memory anchor", "memory_anchor", "anchor", "anchors"],
        )
    }

    /// 取角色 id（PerLTQA 按角色组织记忆 ⇒ 同一题的 reference memory 可能跨角色）。
    pub fn character(&self) -> Option<&str> {
        pick_str(
            &self.raw,
            &["character", "persona", "role_id", "profile_id"],
        )
    }
}

/// 一份 PerLTQA 数据文件的解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct PerLtqaDataset {
    /// 解析出的题目（已转成考卷格式）。
    pub cases: Vec<GoldenCase>,
    /// 被跳过（字段缺失）的原始条目数。
    pub skipped: usize,
}

/// 转换配置。
#[derive(Debug, Clone, Copy)]
pub struct AdapterConfig {
    /// 单个 `expect_fragment` 的最大字符数（截断到字符边界）。
    pub max_fragment_chars: usize,
    /// 没有 anchor 时，从 reference memory 取的句子最大字符数。
    pub max_sentence_chars: usize,
}
impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            max_fragment_chars: 120,
            max_sentence_chars: 120,
        }
    }
}

/// 把一份 PerLTQA JSON（顶层是数组，或 `{"data": [...]}`）转成考卷**条目**。
///
/// * **不收版本号**：版本号是**整卷的属性**，由 [`to_golden_set`] 统一盖上
///   （单一职责：转换只管条目，版本在装卷时给）。
/// * **确定性**：同一输入必得同一输出（id 是内容指纹，无随机、无哈希序依赖）。
pub fn convert(json: &serde_json::Value, config: AdapterConfig) -> PerLtqaDataset {
    let mut cases = Vec::new();
    let mut skipped = 0usize;
    for item in items_of(json) {
        let parsed = PerLtqaItem::new(item);
        let Some(query) = parsed.question().map(str::trim).filter(|q| !q.is_empty()) else {
            skipped += 1;
            continue;
        };
        let memories = parsed.reference_memories();
        if memories.is_empty() {
            skipped += 1;
            continue;
        }
        // 记录 id = 内容指纹（可复现；跨卷可对照）。
        let mut expect_ids = Vec::new();
        for memory in &memories {
            let id = content_id(memory);
            if !expect_ids.contains(&id) {
                expect_ids.push(id);
            }
        }
        // 片段：优先 anchor；否则取 reference memory 的首句。
        let anchors = parsed.memory_anchors();
        let mut expect_fragments = Vec::new();
        if anchors.is_empty() {
            for memory in &memories {
                let sentence = first_sentence(memory, config.max_sentence_chars);
                if !sentence.is_empty() && !expect_fragments.contains(&sentence) {
                    expect_fragments.push(sentence);
                }
            }
        } else {
            for anchor in &anchors {
                let trimmed = truncate_chars(anchor.trim(), config.max_fragment_chars);
                if !trimmed.is_empty() && !expect_fragments.contains(&trimmed) {
                    expect_fragments.push(trimmed);
                }
            }
        }
        // 角色作为 tag（便于按角色分片评测）。
        let mut tags = vec!["external".to_string(), "perltqa".to_string()];
        if let Some(character) = parsed.character() {
            if !character.trim().is_empty() {
                tags.push(format!("character:{}", character.trim()));
            }
        }
        cases.push(GoldenCase {
            id: content_id(query),
            query: query.to_string(),
            expect_ids,
            expect_fragments,
            tags,
        });
    }
    // 确定性排序：按 id。
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    PerLtqaDataset { cases, skipped }
}

/// 把转换结果装成考卷（**版本号在这里盖上**，人可读，如 `perltqa-test-v1`）。
///
/// 与 [`GoldenSet::load_jsonl_with_version`] 的口径一致（人可读版本号），
/// 但不走 JSONL 往返 ⇒ 不依赖外部数据文件的存在。
pub fn to_golden_set(dataset: PerLtqaDataset, version: &str) -> GoldenSet {
    GoldenSet::new(version.to_string(), dataset.cases)
}

/// 记录内容的指纹（`sha256:<12 位十六进制>`）。
///
/// **为什么用内容指纹当 id**：PerLTQA 的记忆条目**没有**稳定 id 字符串
/// ⇒ 用内容指纹才能保证「同一份数据转换两次得到同一批 id」（可复现、可对照）。
pub fn content_id(text: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{}", &hex[..12])
}

/// 取文本的第一个句子（按中英文句号/换行切），并按字符边界截断。
///
/// ⚠️ **必须按字符切，不能按字节下标切**：`str::find` 返回的是**字节偏移**，
/// 而中文句号「。」是多字节字符 ⇒ 直接 `&text[..end]` 会落在字符中间而**panic**。
/// ⇒ 这里用 `char_indices` 逐字符找切点，天然落在字符边界上。
fn first_sentence(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let end = trimmed
        .char_indices()
        .find(|(_, ch)| matches!(ch, '。' | '.' | '\n' | '！' | '!' | '？' | '?'))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(trimmed.len());
    truncate_chars(&trimmed[..end], max_chars)
}

/// 按**字符边界**截断（不切坏 UTF-8）。
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

/// 取第一个命中的字符串字段。
fn pick_str<'a>(value: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(found) = value.get(*key).and_then(|item| item.as_str()) {
            return Some(found);
        }
    }
    None
}

/// 取第一个命中的**列表**字段（也接受单个字符串）。
fn pick_str_list<'a>(value: &'a serde_json::Value, keys: &[&str]) -> Vec<&'a str> {
    for key in keys {
        match value.get(*key) {
            Some(serde_json::Value::String(text)) => return vec![text.as_str()],
            Some(serde_json::Value::Array(items)) => {
                let list: Vec<&str> = items.iter().filter_map(|item| item.as_str()).collect();
                if !list.is_empty() {
                    return list;
                }
            }
            _ => {}
        }
    }
    Vec::new()
}

/// 取顶层条目数组（兼容 `[{...}]` 与 `{"data": [{...}]}` 两种形状）。
fn items_of(value: &serde_json::Value) -> Vec<serde_json::Value> {
    match value {
        serde_json::Value::Array(items) => items.clone(),
        serde_json::Value::Object(map) => map
            .get("data")
            .and_then(|item| item.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
