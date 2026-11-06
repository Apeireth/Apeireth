//! 人格记忆层 (Persona Memory Layer) —— 「我是谁」的独立沉淀面.
//!
//! # 为什么独立成层
//!
//! 既有记忆面把「关于用户的事实」(用户偏好画像 / 情节事实) 当作一等沉淀; 「关于
//! 自己」的沉淀 (我的信念 / 我的偏好 / 我的底线 / 我的自我叙事) 没有独立户口,
//! 混在情节流里会被时间冲淡、被压缩抹平。本层把二者分账:
//!
//! - **用户事实层**: `episodes` 流 + 偏好画像表 (关于「你」);
//! - **人格记忆层**: `persona_memory_*` 分表 (关于「我」), 见 [`crate::migrations`] V16。
//!
//! # 整编管线 (夜间整编 / 整编浅接面共用一条人格提取步)
//!
//! ```text
//! 近期情节 (治理可见) ──▶ 人格提炼 (确定性, 0 模型调用)
//!                          │  自指句判定 (「我…」且非「你/您」) → 分类:
//!                          │   边界 > 信念 > 偏好 三类 + 自述句
//!                          ▼
//!                     人格增量 (PersonaExtract)
//!                          │
//!                          ▼
//!                    合并策略 (不静默覆盖)
//!                     ├─ 同核同极 → 同类合并 (原文逐字保留, 来源并集)
//!                     ├─ 同核反极 → 矛盾标注「人格张力」(双侧原文都留, 不覆盖)
//!                     └─ 其余 → 新条目
//!                          │
//!                          ▼
//!                 persona_memory_* 分表 (状态面) ──▶ 自述节计数 / 最后整编真值
//! ```
//!
//! # 三轴取舍 (保真 / 结构前置 / 保鲜)
//!
//! - **保真分级**: `core_beliefs` / `boundaries` = **逐字保留区**, 压缩检查点绕过
//!   它们 (见 [`compress_persona`]); 偏好与自述可压缩。
//! - **结构前置**: 提炼在整编期一次付清 (确定性规则, 不做模型调用), 查询期只读。
//! - **保鲜**: 合并按归一化键幂等 (重复整编零变更, 见 [`PersonaMemory::apply_extract`]);
//!   张力台账带 `resolved` 位, 裁决状态跨整编保留。
//!
//! # 回取面 (进提示词的两种方式)
//!
//! 1. **恒定小注入** ([`build_constant_injection`]): 自我叙事摘要 ≤ 恒预算,
//!    与查询无关 —— 身份不靠检索命中, 它一直在;
//! 2. **检索补充** ([`build_retrieval_supplement`]): 偏好 / 边界按需补充, 无匹配即空。
//!
//! # 保护语义
//!
//! 人格条目与保护记忆同族: [`PersonaProtection::On`] 为默认, 受保护条目**豁免**
//! 遗忘曲线; 未受保护条目走慢速曲线 ([`PERSONA_SLOW_HALF_LIFE_HOURS`])。
//! 「它是谁」不被时间冲淡。
//!
//! # 0 装诚实边界
//!
//! - 提炼是**保守确定性规则** (关键词命中), 无模型调用; 未命中句不入层 (宁缺勿滥);
//! - 恒定注入预算是**字符口径** (token 的保守代理), 本层不做 token 计数;
//! - 本模块自带纯渲染函数, 上层装配面按需接线。

use std::collections::HashSet;
use std::sync::Mutex;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use apeireth_core::kernel::memory::Episode;

use crate::retention::decay_strength;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

// ============================================
// 1. 数据模型
// ============================================

/// 人格条目类别.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaKind {
    /// 核心信念 (逐字保留区).
    CoreBelief,
    /// 自我偏好.
    Preference,
    /// 底线/边界 (逐字保留区).
    Boundary,
}

impl PersonaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonaKind::CoreBelief => "core_belief",
            PersonaKind::Preference => "preference",
            PersonaKind::Boundary => "boundary",
        }
    }

    pub fn from_str_value(value: &str) -> Option<Self> {
        match value {
            "core_belief" => Some(PersonaKind::CoreBelief),
            "preference" => Some(PersonaKind::Preference),
            "boundary" => Some(PersonaKind::Boundary),
            _ => None,
        }
    }
}

/// 语句极性 (张力判定用).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaPolarity {
    /// 肯定极 (坚持 / 喜欢 / 相信…).
    Positive,
    /// 否定极 (绝不 / 不会 / 拒绝…).
    Negative,
}

impl PersonaPolarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonaPolarity::Positive => "positive",
            PersonaPolarity::Negative => "negative",
        }
    }

    pub fn from_str_value(value: &str) -> Option<Self> {
        match value {
            "positive" => Some(PersonaPolarity::Positive),
            "negative" => Some(PersonaPolarity::Negative),
            _ => None,
        }
    }
}

/// 保护语义: 人格条目 = 保护记忆同族, 默认 `On`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonaProtection {
    /// 保护 (默认): 遗忘曲线豁免.
    On,
    /// 未保护: 慢速遗忘曲线.
    Off,
}

impl PersonaProtection {
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonaProtection::On => "on",
            PersonaProtection::Off => "off",
        }
    }

    pub fn from_str_value(value: &str) -> Option<Self> {
        match value {
            "on" => Some(PersonaProtection::On),
            "off" => Some(PersonaProtection::Off),
            _ => None,
        }
    }
}

impl Default for PersonaProtection {
    fn default() -> Self {
        PersonaProtection::On
    }
}

/// 一条人格沉淀 (原文逐字保留).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaEntry {
    /// 确定性 id (类别 + 归一化键哈希, 跨轮幂等).
    pub id: String,
    /// 条目类别.
    pub kind: PersonaKind,
    /// 原文 (逐字保留; 逐字保留区的文本永不改写).
    pub text: String,
    /// 来源情节 id (并集, 可追溯).
    pub sources: Vec<String>,
    /// 极性.
    pub polarity: PersonaPolarity,
    /// 沉淀时刻 (epoch 秒).
    pub created_at: i64,
    /// 最近一次合并变更时刻 (epoch 秒).
    pub updated_at: i64,
    /// 保护语义 (默认 [`PersonaProtection::On`]).
    pub protection: PersonaProtection,
}

/// 人格张力 (矛盾标注): 同核反极的两条自我陈述并存, 不静默覆盖.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaTension {
    /// 确定性 id.
    pub id: String,
    /// 冲突类别 (既有条目侧).
    pub kind: PersonaKind,
    /// 既有条目 id.
    pub existing_entry_id: String,
    /// 既有侧原文 (逐字保留).
    pub existing_text: String,
    /// 矛盾侧原文 (逐字保留 —— 不静默覆盖, 双侧都在).
    pub incoming_text: String,
    /// 张力标注说明.
    pub note: String,
    /// 标注时刻 (epoch 秒).
    pub detected_at: i64,
    /// 是否已人工裁决.
    pub resolved: bool,
}

/// 人格张力标注说明 (中性措辞, 零编造).
pub const PERSONA_TENSION_NOTE: &str =
    "同一自我主题出现相反陈述: 双侧原文均保留, 不静默覆盖, 待人工裁决";

/// 人格记忆聚合.
///
/// 核心身份形为设计五字段: `core_beliefs` / `preferences` / `boundaries` /
/// `self_narrative` / `updated_at`; `tensions` 与 `last_consolidation_at`
/// 是整编面扩展 (张力台账 + 最后整编真值), 与核心形同库同账。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaMemory {
    /// 核心信念 (逐字保留区).
    pub core_beliefs: Vec<PersonaEntry>,
    /// 自我偏好.
    pub preferences: Vec<PersonaEntry>,
    /// 底线/边界 (逐字保留区).
    pub boundaries: Vec<PersonaEntry>,
    /// 自我叙事 (摘要面, 可压缩).
    pub self_narrative: String,
    /// 最近一次产生沉淀变更的时刻 (epoch 秒).
    pub updated_at: i64,
    /// 张力台账 (矛盾标注, 不静默覆盖).
    pub tensions: Vec<PersonaTension>,
    /// 最后整编真值 (epoch 秒); `None` = 尚未整编 (不编时间).
    pub last_consolidation_at: Option<i64>,
}

/// 自述节 (人格层自述: 只计数, 不回内容 —— 与自述面「有多少不说是什么」同口径).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PersonaStatusSection {
    /// 信念数 (core_beliefs 真值计数).
    pub core_beliefs: u64,
    /// 边界数 (boundaries 真值计数).
    pub boundaries: u64,
    /// 偏好数 (preferences 真值计数).
    pub preferences: u64,
    /// 张力数 (已标注人格张力计数).
    pub tensions: u64,
    /// 最后整编真值 (epoch 秒); `None` = 尚未整编 (如实上报, 不编时间).
    pub last_consolidation_at: Option<i64>,
}

/// 一条人格候选陈述 (原文 + 来源情节).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaStatement {
    /// 候选原文 (逐字).
    pub text: String,
    /// 来源情节 id (可追溯).
    pub source_episode_id: String,
}

/// 人格增量 (一次提炼步的产出).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonaExtract {
    /// 信念候选.
    pub core_beliefs: Vec<PersonaStatement>,
    /// 偏好候选.
    pub preferences: Vec<PersonaStatement>,
    /// 边界候选.
    pub boundaries: Vec<PersonaStatement>,
    /// 自述句候选.
    pub narrative_lines: Vec<PersonaStatement>,
}

impl PersonaExtract {
    /// 候选总数.
    pub fn total(&self) -> usize {
        self.core_beliefs.len() + self.preferences.len() + self.boundaries.len()
    }
}

/// 合并结果 (不静默覆盖: 每个动作可观察).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonaMergeOutcome {
    /// 新增条目 id.
    pub added: Vec<String>,
    /// 同类合并 (来源并入既有条目) 的既有条目 id.
    pub merged: Vec<String>,
    /// 本次新标注的人格张力.
    pub tensions: Vec<PersonaTension>,
    /// 自述新收句数.
    pub narrative_appended: usize,
    /// 是否产生任何沉淀变更 (幂等判定: 重复整编 = false).
    pub changed: bool,
}

// ============================================
// 2. 确定性提炼 (0 模型调用)
// ============================================

/// 边界句标记 (命中即入逐字保留区 `boundaries`).
const BOUNDARY_MARKERS: &[&str] = &[
    "我绝不",
    "我绝不能",
    "我不会",
    "我不做",
    "我不接受",
    "我不允许",
    "我拒绝",
    "我的底线",
];

/// 信念句标记 (命中即入逐字保留区 `core_beliefs`).
const BELIEF_MARKERS: &[&str] = &[
    "我相信",
    "我坚信",
    "我信奉",
    "我的原则",
    "我坚持",
    "我认为最重要",
    "我最看重",
];

/// 偏好句标记 (命中即入 `preferences`).
const PREFERENCE_MARKERS: &[&str] = &[
    "我喜欢",
    "我偏好",
    "我偏爱",
    "我更喜欢",
    "我更愿意",
    "我倾向于",
    "我习惯",
];

/// 自述句标记 (命中即入 `self_narrative`, 且未命中上述分类).
const NARRATIVE_MARKERS: &[&str] = &["我是", "我一直在", "我在学习", "对我来说", "现在的我"];

/// 第二人称标记: 命中即判为「关于用户」的句子, **不**入人格层 (分账纪律).
const SECOND_PERSON_MARKERS: &[&str] = &["你", "您"];

/// 否定极标记 (判定 [`PersonaPolarity::Negative`]).
const NEGATIVE_MARKERS: &[&str] = &[
    "绝不",
    "不会",
    "从不",
    "拒绝",
    "不做",
    "不接受",
    "不允许",
    "不能",
    "不可",
    "永不",
    "不说",
    "不向",
    "不改",
    "不变",
    "不怕",
    "不敢",
    "不碰",
    "不下",
];

/// 主题核剥离词 (按长度降序手工排列: 长词先剥).
const TOPIC_STRIP: &[&str] = &[
    "对我来说",
    "倾向于",
    "更愿意",
    "更喜欢",
    "最重要",
    "我绝不",
    "我不会",
    "我不做",
    "我相信",
    "我坚信",
    "我信奉",
    "我坚持",
    "我喜欢",
    "我偏好",
    "我偏爱",
    "我习惯",
    "我拒绝",
    "我的",
    "我是",
    "原则",
    "底线",
    "认为",
    "看重",
    "信奉",
    "坚信",
    "相信",
    "坚持",
    "喜欢",
    "偏好",
    "偏爱",
    "习惯",
    "倾向",
    "绝不",
    "不会",
    "从不",
    "拒绝",
    "不做",
    "不接受",
    "不允许",
    "不能",
    "不可",
    "永不",
    "一直",
    "总是",
    "非常",
    "一定",
    "必须",
    "应该",
    "还是",
    "或者",
    "但是",
    "来说",
    "现在",
    "学习",
    "而且",
    "并且",
    "我",
    "的",
    "是",
    "会",
    "要",
    "了",
    "得",
    "也",
    "都",
    "就",
    "而",
    "在",
    "很",
    "更",
    "最",
    "和",
    "与",
    "但",
    "或",
    "不",
];

/// 句子切分边界.
const SENTENCE_DELIMITERS: &[char] = &['。', '！', '？', '；', '\n', '!', '?', ';'];

/// 把一段情节文本切成句子 (确定性).
pub fn split_sentences(content: &str) -> Vec<&str> {
    content
        .split(SENTENCE_DELIMITERS)
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

/// 归一化: 只留字母数字 (含 CJK), ASCII 小写; 用于幂等键与主题比对.
pub fn normalize_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// 主题核: 归一化后剥去自指/极性/虚词, 剩余内容词.
pub fn topic_core(text: &str) -> String {
    let mut core = normalize_text(text);
    for word in TOPIC_STRIP {
        core = core.replace(word, "");
    }
    core
}

/// 极性判定: 含否定极标记 → [`PersonaPolarity::Negative`].
pub fn polarity(text: &str) -> PersonaPolarity {
    let norm = normalize_text(text);
    if NEGATIVE_MARKERS.iter().any(|marker| norm.contains(marker)) {
        PersonaPolarity::Negative
    } else {
        PersonaPolarity::Positive
    }
}

/// 同核判定: 核相等或互相包含 (双方核均 ≥ 2 字).
fn cores_match(a: &str, b: &str) -> bool {
    if a.chars().count() < 2 || b.chars().count() < 2 {
        return false;
    }
    a == b || a.contains(b) || b.contains(a)
}

/// 句子分类: 返回该句入哪一类; `None` = 非人格句 (含「关于用户」的句子).
///
/// 分类优先级: 边界 > 信念 > 偏好 (一句话至多一类).
pub fn classify_sentence(sentence: &str) -> Option<PersonaKind> {
    if SECOND_PERSON_MARKERS
        .iter()
        .any(|marker| sentence.contains(marker))
    {
        // 分账纪律: 「关于用户」的句子不进人格层 (它属于用户事实层).
        return None;
    }
    if BOUNDARY_MARKERS.iter().any(|m| sentence.contains(m)) {
        return Some(PersonaKind::Boundary);
    }
    if BELIEF_MARKERS.iter().any(|m| sentence.contains(m)) {
        return Some(PersonaKind::CoreBelief);
    }
    if PREFERENCE_MARKERS.iter().any(|m| sentence.contains(m)) {
        return Some(PersonaKind::Preference);
    }
    None
}

/// 句子是否是自述句 (未命中分类, 但带身份标记).
fn is_narrative_sentence(sentence: &str) -> bool {
    classify_sentence(sentence).is_none() && NARRATIVE_MARKERS.iter().any(|m| sentence.contains(m))
}

/// 确定性人格提炼器 (保守规则, 0 模型调用).
#[derive(Debug, Default, Clone, Copy)]
pub struct PersonaExtractor;

impl PersonaExtractor {
    /// 从一批情节提炼人格增量.
    ///
    /// 只读 `role = "assistant"` 的原始情节 (自述主体是「我」); 派生情节
    /// (如整理洞察) 与用户/工具情节不入料 —— 提炼素材口径与整编洞察同源
    /// (原始证据角色), 防派生记忆自我增殖。
    pub fn extract(&self, episodes: &[Episode]) -> PersonaExtract {
        let mut out = PersonaExtract::default();
        for episode in episodes {
            if episode.role != "assistant" {
                continue;
            }
            for sentence in split_sentences(&episode.content) {
                let statement = PersonaStatement {
                    text: sentence.to_string(),
                    source_episode_id: episode.id.clone(),
                };
                match classify_sentence(sentence) {
                    Some(PersonaKind::CoreBelief) => {
                        push_unique(&mut out.core_beliefs, statement);
                    }
                    Some(PersonaKind::Preference) => {
                        push_unique(&mut out.preferences, statement);
                    }
                    Some(PersonaKind::Boundary) => {
                        push_unique(&mut out.boundaries, statement);
                    }
                    None if is_narrative_sentence(sentence) => {
                        push_unique(&mut out.narrative_lines, statement);
                    }
                    None => {}
                }
            }
        }
        out
    }
}

fn push_unique(target: &mut Vec<PersonaStatement>, statement: PersonaStatement) {
    let norm = normalize_text(&statement.text);
    if norm.is_empty() {
        return;
    }
    if !target
        .iter()
        .any(|existing| normalize_text(&existing.text) == norm)
    {
        target.push(statement);
    }
}

// ============================================
// 3. 合并策略 (同类合并 / 矛盾标注, 不静默覆盖)
// ============================================

/// FNV-1a 64 (自含实现, 0 新外部依赖; 跨轮稳定 —— 作为确定性 id 的散列).
fn fnv1a64(input: &str) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// 条目确定性 id (类别 + 归一化键).
fn entry_id(kind: PersonaKind, norm: &str) -> String {
    format!("persona:{}:{:016x}", kind.as_str(), fnv1a64(norm))
}

/// 张力确定性 id (既有条目 + 矛盾侧归一化键).
fn tension_id(existing_entry_id: &str, incoming_norm: &str) -> String {
    format!(
        "persona-tension:{:016x}",
        fnv1a64(&format!("{existing_entry_id}|{incoming_norm}"))
    )
}

impl PersonaMemory {
    /// 全部条目 (信念 → 偏好 → 边界, 稳定顺序).
    pub fn all_entries(&self) -> Vec<&PersonaEntry> {
        self.core_beliefs
            .iter()
            .chain(self.preferences.iter())
            .chain(self.boundaries.iter())
            .collect()
    }

    /// 自述节 (只计数, 不回内容).
    pub fn status(&self) -> PersonaStatusSection {
        PersonaStatusSection {
            core_beliefs: self.core_beliefs.len() as u64,
            boundaries: self.boundaries.len() as u64,
            preferences: self.preferences.len() as u64,
            tensions: self.tensions.len() as u64,
            last_consolidation_at: self.last_consolidation_at,
        }
    }

    fn entries_mut(&mut self, kind: PersonaKind) -> &mut Vec<PersonaEntry> {
        match kind {
            PersonaKind::CoreBelief => &mut self.core_beliefs,
            PersonaKind::Preference => &mut self.preferences,
            PersonaKind::Boundary => &mut self.boundaries,
        }
    }

    /// 同核反极? 找出与本句冲突的既有条目 (任意类别 —— 自我矛盾不分家).
    fn find_tension(&self, core: &str, incoming: PersonaPolarity) -> Option<&PersonaEntry> {
        self.all_entries()
            .into_iter()
            .find(|entry| entry.polarity != incoming && cores_match(&topic_core(&entry.text), core))
    }

    /// 同类合并目标: 同类 + 同极 + 同核 (或归一化键相等).
    fn find_merge_target(
        &mut self,
        kind: PersonaKind,
        norm: &str,
        core: &str,
        incoming: PersonaPolarity,
    ) -> Option<&mut PersonaEntry> {
        self.entries_mut(kind).iter_mut().find(|entry| {
            let entry_norm = normalize_text(&entry.text);
            entry.polarity == incoming
                && (entry_norm == norm || cores_match(&topic_core(&entry.text), core))
        })
    }

    /// 折入一个人格增量 (合并策略: 同类合并 / 矛盾标注张力, 不静默覆盖).
    ///
    /// 幂等: 同一批增量重复折入零变更 (`changed == false`, 真值刻度不推进).
    pub fn apply_extract(&mut self, extract: &PersonaExtract, now: i64) -> PersonaMergeOutcome {
        let mut outcome = PersonaMergeOutcome::default();
        let buckets = [
            (PersonaKind::CoreBelief, &extract.core_beliefs),
            (PersonaKind::Preference, &extract.preferences),
            (PersonaKind::Boundary, &extract.boundaries),
        ];
        for (kind, texts) in buckets {
            for text in texts {
                self.absorb_statement(kind, text, now, &mut outcome);
            }
        }
        for line in &extract.narrative_lines {
            self.absorb_narrative(line, now, &mut outcome);
        }
        if outcome.changed {
            self.updated_at = now;
            self.last_consolidation_at = Some(now);
        }
        outcome
    }

    fn absorb_statement(
        &mut self,
        kind: PersonaKind,
        statement: &PersonaStatement,
        now: i64,
        outcome: &mut PersonaMergeOutcome,
    ) {
        let text = statement.text.trim();
        let norm = normalize_text(text);
        if norm.is_empty() {
            return;
        }
        let incoming_polarity = polarity(text);
        let core = topic_core(text);

        // ① 矛盾标注「人格张力」: 同核反极 → 双侧原文都留, 不静默覆盖.
        if let Some(existing) = self.find_tension(&core, incoming_polarity) {
            let tension = PersonaTension {
                id: tension_id(&existing.id, &norm),
                kind: existing.kind,
                existing_entry_id: existing.id.clone(),
                existing_text: existing.text.clone(),
                incoming_text: text.to_string(),
                note: PERSONA_TENSION_NOTE.to_string(),
                detected_at: now,
                resolved: false,
            };
            let already = self.tensions.iter().any(|item| item.id == tension.id);
            if !already {
                self.tensions.push(tension.clone());
                outcome.tensions.push(tension);
                outcome.changed = true;
            }
            // 矛盾侧不并入条目 (不静默覆盖也不静默吸收), 原文留存在张力台账.
            return;
        }

        // ② 同类合并: 原文逐字保留 (逐字保留区永不改写), 来源并集.
        //    合并只在吸收新来源时计一次; 无新信息的重复句 = 零变更 (幂等).
        if let Some(entry) = self.find_merge_target(kind, &norm, &core, incoming_polarity) {
            if !entry.sources.contains(&statement.source_episode_id) {
                entry.sources.push(statement.source_episode_id.clone());
                entry.updated_at = now;
                outcome.merged.push(entry.id.clone());
                outcome.changed = true;
            }
            return;
        }

        // ③ 新条目 (protection=on 默认).
        let entry = PersonaEntry {
            id: entry_id(kind, &norm),
            kind,
            text: text.to_string(),
            sources: vec![statement.source_episode_id.clone()],
            polarity: incoming_polarity,
            created_at: now,
            updated_at: now,
            protection: PersonaProtection::default(),
        };
        outcome.added.push(entry.id.clone());
        self.entries_mut(kind).push(entry);
        outcome.changed = true;
    }

    fn absorb_narrative(
        &mut self,
        statement: &PersonaStatement,
        now: i64,
        outcome: &mut PersonaMergeOutcome,
    ) {
        let _ = now;
        let line = statement.text.trim();
        let norm = normalize_text(line);
        if norm.is_empty() {
            return;
        }
        let known: HashSet<String> = split_sentences(&self.self_narrative)
            .iter()
            .map(|sentence| normalize_text(sentence))
            .collect();
        if known.contains(&norm) {
            return;
        }
        if self.self_narrative.trim().is_empty() {
            self.self_narrative = line.to_string();
        } else {
            self.self_narrative.push('\n');
            self.self_narrative.push_str(line);
        }
        self.self_narrative = truncate_chars(&self.self_narrative, PERSONA_NARRATIVE_MAX_CHARS);
        outcome.narrative_appended += 1;
        outcome.changed = true;
    }
}

// ============================================
// 4. 回取面 (恒定小注入 + 检索补充)
// ============================================

/// 恒定小注入预算 (字符口径 —— token 的保守代理, 本层不做 token 计数).
pub const CONSTANT_INJECTION_BUDGET_CHARS: usize = 400;

/// 自我叙事落库上限 (字符口径; 超限截断并留显式注记).
pub const PERSONA_NARRATIVE_MAX_CHARS: usize = 2_000;

/// 检索补充行数上限.
pub const SUPPLEMENT_MAX_LINES: usize = 6;

/// 检索补充单行字符上限.
pub const SUPPLEMENT_LINE_MAX_CHARS: usize = 120;

/// 恒定小注入: 自我叙事摘要 ≤ 恒预算.
///
/// **与查询无关** —— 身份不靠检索命中, 它一直在。超长截断策略: 保头部 +
/// 显式省略注记 (不静默裁剪)。空自述 → 空串 (未沉淀就是未沉淀, 不编)。
pub fn build_constant_injection(memory: &PersonaMemory) -> String {
    let narrative = memory.self_narrative.trim();
    if narrative.is_empty() {
        return String::new();
    }
    let body = truncate_with_note(narrative, CONSTANT_INJECTION_BUDGET_CHARS);
    format!("【我是谁 · 恒定自述】\n{body}")
}

/// 检索补充 (按需): 偏好 / 边界 / 信念按需补充, 无匹配即空串.
///
/// 匹配是确定性的: 主题核互含或归一化双字交集 ≥ [`SUPPLEMENT_BIGRAM_MIN`]。
/// 顺序稳定: 边界 → 偏好 → 信念 (底线先出)。
pub fn build_retrieval_supplement(memory: &PersonaMemory, need: &str) -> String {
    let need_norm = normalize_text(need);
    if need_norm.is_empty() {
        return String::new();
    }
    let order = [
        PersonaKind::Boundary,
        PersonaKind::Preference,
        PersonaKind::CoreBelief,
    ];
    let mut lines: Vec<String> = Vec::new();
    for kind in order {
        for entry in memory.all_entries() {
            if entry.kind != kind {
                continue;
            }
            if !entry_matches_need(entry, &need_norm) {
                continue;
            }
            lines.push(format!(
                "• [{}] {}",
                entry.kind.as_str(),
                truncate_with_note(&entry.text, SUPPLEMENT_LINE_MAX_CHARS)
            ));
            if lines.len() >= SUPPLEMENT_MAX_LINES {
                break;
            }
        }
        if lines.len() >= SUPPLEMENT_MAX_LINES {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("【我是谁 · 按需补充】\n");
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 检索补充的最小双字交集.
pub const SUPPLEMENT_BIGRAM_MIN: usize = 2;

fn entry_matches_need(entry: &PersonaEntry, need_norm: &str) -> bool {
    let entry_norm = normalize_text(&entry.text);
    let core = topic_core(&entry.text);
    let need_core = topic_core_from_norm(need_norm);
    if cores_match(&core, &need_core) {
        return true;
    }
    shared_bigrams(&entry_norm, need_norm) >= SUPPLEMENT_BIGRAM_MIN
}

/// 从已归一化文本取主题核 (剥离词命中归一化形态).
fn topic_core_from_norm(norm: &str) -> String {
    let mut core = norm.to_string();
    for word in TOPIC_STRIP {
        core = core.replace(word, "");
    }
    core
}

/// 归一化文本的双字集合交集计数.
fn shared_bigrams(a: &str, b: &str) -> usize {
    let left = bigrams(a);
    let right = bigrams(b);
    left.iter().filter(|gram| right.contains(*gram)).count()
}

fn bigrams(norm: &str) -> Vec<String> {
    let chars: Vec<char> = norm.chars().collect();
    chars
        .windows(2)
        .map(|window| window.iter().collect())
        .collect()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

/// 截断并留显式省略注记 (不静默裁剪).
fn truncate_with_note(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars).collect();
    format!("{head}…[尾部 {} 字符已省略]…", total - max_chars)
}

// ============================================
// 5. 保真分级 + 保护语义
// ============================================

/// 压缩预算 (字符口径).
#[derive(Debug, Clone, Copy)]
pub struct PersonaCompressBudget {
    /// 偏好面预算.
    pub max_preference_chars: usize,
    /// 自述面预算.
    pub max_narrative_chars: usize,
}

impl Default for PersonaCompressBudget {
    fn default() -> Self {
        Self {
            max_preference_chars: 400,
            max_narrative_chars: 400,
        }
    }
}

/// 压缩视图: 逐字保留区**原文逐字拷贝** (不改写), 可压缩面给出预算内文本.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedPersonaView {
    /// 逐字保留区条目 (core_beliefs + boundaries, 原文逐字).
    pub verbatim: Vec<PersonaEntry>,
    /// 偏好面 (预算内文本).
    pub preferences: Vec<String>,
    /// 自述面 (预算内文本).
    pub self_narrative: String,
    /// 被省略字符数 (诚实计量).
    pub omitted_chars: usize,
}

/// 保真分级压缩: `core_beliefs` / `boundaries` = 逐字保留区, **绕过压缩**;
/// 偏好与自述按预算截断 (显式省略注记)。
pub fn compress_persona(
    memory: &PersonaMemory,
    budget: PersonaCompressBudget,
) -> CompressedPersonaView {
    let mut omitted_chars = 0usize;
    let mut preferences = Vec::new();
    for entry in &memory.preferences {
        let total = entry.text.chars().count();
        let text = truncate_with_note(&entry.text, budget.max_preference_chars);
        omitted_chars += total.saturating_sub(text.chars().count());
        preferences.push(text);
    }
    let narrative_total = memory.self_narrative.chars().count();
    let self_narrative = truncate_with_note(&memory.self_narrative, budget.max_narrative_chars);
    omitted_chars += narrative_total.saturating_sub(self_narrative.chars().count());

    CompressedPersonaView {
        verbatim: memory
            .core_beliefs
            .iter()
            .chain(memory.boundaries.iter())
            .cloned()
            .collect(),
        preferences,
        self_narrative,
        omitted_chars,
    }
}

/// 未受保护人格条目的慢速遗忘曲线半衰期 (小时) = 90 天
/// (设计裁定 R1⑩ 口径统一: 与未保护记忆遗忘半衰期同为 90 天).
///
/// 受保护条目 ([`PersonaProtection::On`], 默认) **豁免**遗忘曲线;
/// 未受保护条目也走慢速曲线 —— 「它是谁」不被时间冲淡。
pub const PERSONA_SLOW_HALF_LIFE_HOURS: f64 = 24.0 * 90.0;

/// 人格条目遗忘曲线强度: 受保护 = 恒 1.0 (豁免); 未受保护 = 慢速衰减.
pub fn persona_decay_strength(entry: &PersonaEntry, now_unix: i64) -> f32 {
    if entry.protection == PersonaProtection::On {
        1.0
    } else {
        decay_strength(entry.updated_at, now_unix, PERSONA_SLOW_HALF_LIFE_HOURS)
    }
}

/// 人格层清理报告.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonaSweepReport {
    /// 扫描条目数.
    pub scanned: usize,
    /// 因慢速曲线低于阈值而移除的未保护条目数.
    pub dropped: usize,
    /// 遗忘曲线豁免 (受保护跳过) 的条目数.
    pub skipped_protected: usize,
}

/// 人格层清理: 受保护条目豁免; 未受保护条目按慢速曲线淘汰。
///
/// 只动条目集, 不动张力台账 (张力是裁决历史, 不被时间冲掉)。
pub fn sweep_persona(
    memory: &mut PersonaMemory,
    now_unix: i64,
    min_strength: f32,
) -> PersonaSweepReport {
    let mut report = PersonaSweepReport::default();
    let threshold = min_strength.clamp(0.0, 1.0);
    for kind in [
        PersonaKind::CoreBelief,
        PersonaKind::Preference,
        PersonaKind::Boundary,
    ] {
        let mut kept: Vec<PersonaEntry> = Vec::new();
        for entry in memory.entries_mut(kind).drain(..) {
            report.scanned += 1;
            if entry.protection == PersonaProtection::On {
                report.skipped_protected += 1;
                kept.push(entry);
                continue;
            }
            if persona_decay_strength(&entry, now_unix) < threshold {
                report.dropped += 1;
            } else {
                kept.push(entry);
            }
        }
        *memory.entries_mut(kind) = kept;
    }
    report
}

// ============================================
// 6. 持久化 (cognitive.sqlite3 persona 分表)
// ============================================

/// 人格记忆存储面 (与用户事实层分表).
pub trait PersonaMemoryStore: Send + Sync {
    /// 读出人格记忆聚合.
    fn load(&self) -> MemoryResult<PersonaMemory>;
    /// 落盘人格记忆聚合 (状态面: 行集 = 当前聚合).
    fn save(&self, memory: &PersonaMemory) -> MemoryResult<()>;
    /// 自述节 (只计数, 不回内容).
    fn status(&self) -> MemoryResult<PersonaStatusSection>;
}

/// 进程内人格存储 (确定性测试 / 无库装配用).
#[derive(Debug, Default)]
pub struct InMemoryPersonaMemoryStore {
    state: Mutex<PersonaMemory>,
}

impl InMemoryPersonaMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PersonaMemoryStore for InMemoryPersonaMemoryStore {
    fn load(&self) -> MemoryResult<PersonaMemory> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone())
    }

    fn save(&self, memory: &PersonaMemory) -> MemoryResult<()> {
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = memory.clone();
        Ok(())
    }

    fn status(&self) -> MemoryResult<PersonaStatusSection> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .status())
    }
}

impl PersonaMemoryStore for SqliteMemoryStore {
    fn load(&self) -> MemoryResult<PersonaMemory> {
        self.persona_memory_load()
    }

    fn save(&self, memory: &PersonaMemory) -> MemoryResult<()> {
        self.persona_memory_save(memory)
    }

    fn status(&self) -> MemoryResult<PersonaStatusSection> {
        self.persona_memory_status()
    }
}

impl SqliteMemoryStore {
    /// 读人格记忆聚合 (`persona_memory_*` 三表).
    pub fn persona_memory_load(&self) -> MemoryResult<PersonaMemory> {
        let conn = self.conn()?;
        let mut memory = PersonaMemory::default();
        let mut stmt = conn.prepare(
            "SELECT id, kind, text, sources_json, polarity, created_at, updated_at, protection \
             FROM persona_memory_entries ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        for row in rows {
            let (id, kind, text, sources_json, polarity, created_at, updated_at, protection) = row?;
            let kind = PersonaKind::from_str_value(&kind).ok_or_else(|| {
                MemoryError::Invalid(format!("unknown persona kind in store: {kind}"))
            })?;
            let polarity = PersonaPolarity::from_str_value(&polarity).ok_or_else(|| {
                MemoryError::Invalid(format!("unknown persona polarity in store: {polarity}"))
            })?;
            let protection = PersonaProtection::from_str_value(&protection).ok_or_else(|| {
                MemoryError::Invalid(format!("unknown persona protection in store: {protection}"))
            })?;
            let sources: Vec<String> = serde_json::from_str(&sources_json).unwrap_or_default();
            let entry = PersonaEntry {
                id,
                kind,
                text,
                sources,
                polarity,
                created_at,
                updated_at,
                protection,
            };
            match kind {
                PersonaKind::CoreBelief => memory.core_beliefs.push(entry),
                PersonaKind::Preference => memory.preferences.push(entry),
                PersonaKind::Boundary => memory.boundaries.push(entry),
            }
        }
        drop(stmt);

        let mut stmt = conn.prepare(
            "SELECT id, kind, existing_entry_id, existing_text, incoming_text, note, detected_at, resolved \
             FROM persona_memory_tensions ORDER BY detected_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?;
        for row in rows {
            let (
                id,
                kind,
                existing_entry_id,
                existing_text,
                incoming_text,
                note,
                detected_at,
                resolved,
            ) = row?;
            let kind = PersonaKind::from_str_value(&kind).ok_or_else(|| {
                MemoryError::Invalid(format!("unknown persona tension kind in store: {kind}"))
            })?;
            memory.tensions.push(PersonaTension {
                id,
                kind,
                existing_entry_id,
                existing_text,
                incoming_text,
                note,
                detected_at,
                resolved: resolved != 0,
            });
        }
        drop(stmt);

        let state = conn
            .query_row(
                "SELECT self_narrative, updated_at, last_consolidation_at \
                 FROM persona_memory_state WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((self_narrative, updated_at, last_consolidation_at)) = state {
            memory.self_narrative = self_narrative;
            memory.updated_at = updated_at;
            memory.last_consolidation_at = last_consolidation_at;
        }
        Ok(memory)
    }

    /// 落盘人格记忆聚合 (状态面持久化: 行集 = 当前聚合, 变更经合并策略).
    pub fn persona_memory_save(&self, memory: &PersonaMemory) -> MemoryResult<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM persona_memory_entries", [])?;
        tx.execute("DELETE FROM persona_memory_tensions", [])?;
        for entry in memory.all_entries() {
            let sources_json = serde_json::to_string(&entry.sources)?;
            tx.execute(
                "INSERT INTO persona_memory_entries \
                 (id, kind, text, sources_json, polarity, created_at, updated_at, protection) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    entry.id,
                    entry.kind.as_str(),
                    entry.text,
                    sources_json,
                    entry.polarity.as_str(),
                    entry.created_at,
                    entry.updated_at,
                    entry.protection.as_str(),
                ],
            )?;
        }
        for tension in &memory.tensions {
            tx.execute(
                "INSERT INTO persona_memory_tensions \
                 (id, kind, existing_entry_id, existing_text, incoming_text, note, detected_at, resolved) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    tension.id,
                    tension.kind.as_str(),
                    tension.existing_entry_id,
                    tension.existing_text,
                    tension.incoming_text,
                    tension.note,
                    tension.detected_at,
                    i64::from(tension.resolved),
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO persona_memory_state (singleton, self_narrative, updated_at, last_consolidation_at) \
             VALUES (1, ?1, ?2, ?3) \
             ON CONFLICT(singleton) DO UPDATE SET \
               self_narrative = excluded.self_narrative, \
               updated_at = excluded.updated_at, \
               last_consolidation_at = excluded.last_consolidation_at",
            rusqlite::params![
                memory.self_narrative,
                memory.updated_at,
                memory.last_consolidation_at,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// 自述节 (只计数, 不回内容).
    pub fn persona_memory_status(&self) -> MemoryResult<PersonaStatusSection> {
        Ok(self.persona_memory_load()?.status())
    }
}

// ============================================
// 7. 整编接线 (人格提取步)
// ============================================

/// 人格整编报告 (一次人格提取步的真账).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaConsolidationReport {
    /// 参与提炼的情节数.
    pub episodes_evaluated: usize,
    /// 提炼出的候选条数 (信念 + 偏好 + 边界).
    pub extracted: usize,
    /// 同类合并数.
    pub merged_same_kind: usize,
    /// 新标注人格张力数.
    pub tensions_marked: usize,
    /// 是否产生沉淀变更 (幂等: 重复整编 = false).
    pub changed: bool,
    /// 本次运行时刻 (epoch 秒).
    pub ran_at: i64,
}

/// 人格提取步: 从近期情节提炼人格增量 → 合并策略 → 落库。
///
/// 夜间整编与整编浅接面共用本入口 (同一条管线, 两个触发面)。
/// 素材口径: 调用方传治理可见的情节 (已遗忘情节不应入料)。
pub fn consolidate_persona(
    store: &dyn PersonaMemoryStore,
    episodes: &[Episode],
    now: i64,
) -> MemoryResult<PersonaConsolidationReport> {
    let extract = PersonaExtractor.extract(episodes);
    let mut memory = store.load()?;
    let outcome = memory.apply_extract(&extract, now);
    if outcome.changed {
        store.save(&memory)?;
    }
    Ok(PersonaConsolidationReport {
        episodes_evaluated: episodes.len(),
        extracted: extract.total(),
        merged_same_kind: outcome.merged.len(),
        tensions_marked: outcome.tensions.len(),
        changed: outcome.changed,
        ran_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episode(id: &str, role: &str, content: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: 1_000,
            role: role.into(),
            content: content.into(),
            session_id: "sess-1".into(),
        }
    }

    fn texts(items: &[PersonaStatement]) -> Vec<String> {
        items.iter().map(|item| item.text.clone()).collect()
    }

    fn extract_from(id: &str, content: &str) -> PersonaExtract {
        PersonaExtractor.extract(&[episode(id, "assistant", content)])
    }

    /// ① 人格提炼: 情节 → 信念 / 偏好 / 边界三类.
    #[test]
    fn extraction_classifies_belief_preference_boundary() {
        let extract = extract_from(
            "ep-1",
            "我相信先验证再叙事。我喜欢把计划写清楚。我绝不在未告知的情况下改用户数据。",
        );
        assert_eq!(texts(&extract.core_beliefs), vec!["我相信先验证再叙事"]);
        assert_eq!(texts(&extract.preferences), vec!["我喜欢把计划写清楚"]);
        assert_eq!(
            texts(&extract.boundaries),
            vec!["我绝不在未告知的情况下改用户数据"]
        );
        assert!(extract.narrative_lines.is_empty());
    }

    /// ② 分账纪律: 「关于用户」的句子不入人格层 (它是用户事实层的沉淀).
    #[test]
    fn user_fact_sentences_never_enter_the_persona_layer() {
        let extract = extract_from("ep-1", "主人喜欢乌龙茶。你在学线代。您偏好简短回复。");
        assert!(extract.core_beliefs.is_empty());
        assert!(extract.preferences.is_empty());
        assert!(extract.boundaries.is_empty());
        assert!(extract.narrative_lines.is_empty());

        // 同一句话里的自指陈述照常入层 —— 分账只挡「关于用户」.
        let mixed = extract_from("ep-2", "主人喜欢乌龙茶。我相信先验证再叙事。");
        assert_eq!(
            texts(&mixed.core_beliefs),
            vec!["我相信先验证再叙事"],
            "自指陈述不受用户事实句牵连"
        );
    }

    /// 自述句进自我叙事, 不进三类条目.
    #[test]
    fn narrative_sentences_feed_self_narrative_only() {
        let extract = extract_from("ep-1", "我是一个先验证再叙事的工程伙伴。");
        assert_eq!(
            texts(&extract.narrative_lines),
            vec!["我是一个先验证再叙事的工程伙伴"]
        );
        assert!(extract.core_beliefs.is_empty());
    }

    /// 只读 assistant 情节 (原始自述主体), 用户/派生情节不入料.
    #[test]
    fn extraction_reads_assistant_episodes_only() {
        let episodes = vec![
            episode("ep-u", "user", "我喜欢乌龙茶"),
            episode("ep-c", "consolidation", "我喜欢派生洞察回流"),
            episode("ep-a", "assistant", "我喜欢把计划写清楚"),
        ];
        let extract = PersonaExtractor.extract(&episodes);
        assert_eq!(texts(&extract.preferences), vec!["我喜欢把计划写清楚"]);
    }

    /// ③ 同类合并: 同核同极并入既有条目, 原文逐字保留, 无重复条目.
    #[test]
    fn same_kind_statements_merge_without_duplicates() {
        let mut memory = PersonaMemory::default();
        let outcome = memory.apply_extract(&extract_from("ep-1", "我喜欢把计划写清楚"), 10);
        assert_eq!(outcome.added.len(), 1);
        let original_text = memory.preferences[0].text.clone();

        let outcome = memory.apply_extract(&extract_from("ep-2", "我喜欢把计划写得很清楚"), 20);
        assert_eq!(outcome.added.len(), 0);
        assert_eq!(outcome.merged.len(), 1, "同类合并吸收新来源");
        assert_eq!(memory.preferences.len(), 1, "同类合并不留重复条目");
        assert_eq!(
            memory.preferences[0].text, original_text,
            "合并保留既有原文 (逐字)"
        );
        assert_eq!(
            memory.preferences[0].sources,
            vec!["ep-1".to_string(), "ep-2".to_string()],
            "来源并集"
        );
    }

    /// ④ 矛盾标注: 同核反极 → 「人格张力」, 不静默覆盖 (双侧原文都在).
    #[test]
    fn contradictions_are_marked_as_tension_not_overwritten() {
        let mut memory = PersonaMemory::default();
        memory.apply_extract(&extract_from("ep-1", "我绝不说谎"), 10);
        let original = memory.boundaries[0].text.clone();

        let outcome = memory.apply_extract(&extract_from("ep-2", "我习惯说谎"), 20);
        assert_eq!(outcome.tensions.len(), 1, "矛盾必须标注张力");
        assert_eq!(memory.tensions.len(), 1);
        assert_eq!(
            memory.boundaries[0].text, original,
            "既有条目不被矛盾句覆盖"
        );
        assert_eq!(
            memory.tensions[0].incoming_text, "我习惯说谎",
            "矛盾侧原文逐字保留"
        );
        assert!(!memory.tensions[0].resolved);
        assert!(outcome.added.is_empty(), "矛盾句不静默吸收为条目");
    }

    /// ⑤ 张力幂等: 同一矛盾重复整编只标注一次.
    #[test]
    fn tension_marking_is_idempotent() {
        let mut memory = PersonaMemory::default();
        memory.apply_extract(&extract_from("ep-1", "我绝不说谎"), 10);
        memory.apply_extract(&extract_from("ep-2", "我习惯说谎"), 20);
        let outcome = memory.apply_extract(&extract_from("ep-2", "我习惯说谎"), 30);
        assert!(outcome.tensions.is_empty(), "重复矛盾不重复标注");
        assert_eq!(memory.tensions.len(), 1);
        assert!(!outcome.changed);
    }

    /// ⑥ 整编幂等: 同一批情节重复整编零变更, 真值刻度不推进.
    #[test]
    fn consolidation_is_idempotent() {
        let store = InMemoryPersonaMemoryStore::new();
        let episodes = vec![episode(
            "ep-1",
            "assistant",
            "我相信先验证再叙事。我喜欢把计划写清楚。我绝不在未告知的情况下改用户数据。",
        )];
        let first = consolidate_persona(&store, &episodes, 10).unwrap();
        assert!(first.changed);
        let snapshot = store.load().unwrap();

        let second = consolidate_persona(&store, &episodes, 20).unwrap();
        assert!(!second.changed, "重复整编必须零变更");
        assert_eq!(second.merged_same_kind, 0);
        assert_eq!(
            store.load().unwrap(),
            snapshot,
            "重复整编后状态逐字段相等 (含 updated_at / 最后整编真值)"
        );
    }

    /// ⑦ 逐字保留区: 压缩不碰 core_beliefs / boundaries 原文.
    #[test]
    fn compression_never_touches_the_verbatim_zone() {
        let mut memory = PersonaMemory::default();
        let long_belief = format!("我相信{}", "先验证再叙事".repeat(40));
        let long_boundary = format!("我绝不{}", "静默覆盖任何原文".repeat(40));
        memory.apply_extract(
            &PersonaExtract {
                core_beliefs: vec![PersonaStatement {
                    text: long_belief.clone(),
                    source_episode_id: "ep-1".into(),
                }],
                boundaries: vec![PersonaStatement {
                    text: long_boundary.clone(),
                    source_episode_id: "ep-1".into(),
                }],
                preferences: vec![PersonaStatement {
                    text: format!("我喜欢{}", "写很长很长的偏好描述".repeat(80)),
                    source_episode_id: "ep-1".into(),
                }],
                narrative_lines: vec![PersonaStatement {
                    text: format!("我是{}", "一个自述很长的伙伴".repeat(80)),
                    source_episode_id: "ep-1".into(),
                }],
            },
            10,
        );

        let view = compress_persona(&memory, PersonaCompressBudget::default());
        assert_eq!(view.verbatim.len(), 2, "逐字保留区两件都在");
        for entry in &view.verbatim {
            assert!(
                entry.text == long_belief || entry.text == long_boundary,
                "逐字保留区原文逐字不变"
            );
        }
        assert!(view.omitted_chars > 0, "可压缩面确实被压缩");
        assert!(
            view.preferences[0].chars().count()
                <= PersonaCompressBudget::default().max_preference_chars + 40,
            "偏好面在预算内"
        );
    }

    /// ⑧ 恒定注入预算: 超长截断策略 (保头 + 显式省略注记, 不静默裁剪).
    #[test]
    fn constant_injection_truncates_overlong_narrative_with_an_explicit_note() {
        let mut memory = PersonaMemory::default();
        memory.self_narrative = "自述".repeat(1_000);
        let injection = build_constant_injection(&memory);
        assert!(injection.starts_with("【我是谁 · 恒定自述】"));
        assert!(
            injection.contains("字符已省略]"),
            "截断必须显式注记: {injection}"
        );
        let body = injection
            .lines()
            .nth(1)
            .unwrap()
            .chars()
            .filter(|c| *c == '自' || *c == '述')
            .count();
        assert!(
            body <= CONSTANT_INJECTION_BUDGET_CHARS,
            "恒预算硬上限: {body}"
        );
    }

    /// ⑨ 恒定注入恒定性: 与查询无关 (身份不靠检索命中, 它一直在) + 字节稳定.
    #[test]
    fn constant_injection_is_query_independent_and_byte_stable() {
        let mut memory = PersonaMemory::default();
        memory.apply_extract(&extract_from("ep-1", "我是先验证再叙事的伙伴"), 10);
        let first = build_constant_injection(&memory);
        let second = build_constant_injection(&memory);
        assert_eq!(first, second, "字节稳定");
        assert!(!first.is_empty(), "有自述就恒在场");
        // 空人格 = 空注入 (未沉淀不编造).
        assert_eq!(build_constant_injection(&PersonaMemory::default()), "");
    }

    /// ⑩ 检索补充按需: 无匹配即空; 命中即补偏好/边界.
    #[test]
    fn retrieval_supplement_is_on_demand() {
        let mut memory = PersonaMemory::default();
        memory.apply_extract(&extract_from("ep-1", "我喜欢在文档里写清楚取舍"), 10);
        memory.apply_extract(&extract_from("ep-1", "我绝不静默覆盖任何原文"), 10);

        assert_eq!(
            build_retrieval_supplement(&memory, "完全无关的话题"),
            "",
            "无匹配 = 不注入 (按需)"
        );
        let supplement = build_retrieval_supplement(&memory, "静默覆盖原文的处理");
        assert!(supplement.contains("【我是谁 · 按需补充】"));
        assert!(
            supplement.contains("我绝不静默覆盖任何原文"),
            "{supplement}"
        );
        assert!(
            !supplement.contains("我喜欢在文档里写清楚取舍"),
            "无关偏好不随行"
        );
    }

    /// ⑪ 保护语义: protection=on 默认 + 遗忘曲线豁免 (慢速曲线也冲不掉).
    #[test]
    fn protected_entries_are_exempt_from_the_forgetting_curve() {
        let mut memory = PersonaMemory::default();
        memory.apply_extract(&extract_from("ep-1", "我绝不在未告知的情况下改用户数据"), 0);
        memory.apply_extract(&extract_from("ep-1", "我喜欢偶尔换个回复语气"), 0);
        // 手动把偏好改为未保护, 走慢速曲线.
        memory.preferences[0].protection = PersonaProtection::Off;
        assert_eq!(
            memory.boundaries[0].protection,
            PersonaProtection::On,
            "protection=on 是默认"
        );

        // 远超慢速半衰期的时刻: 未保护偏好应淘汰, 受保护边界豁免.
        let far_future = (PERSONA_SLOW_HALF_LIFE_HOURS * 3600.0 * 40.0) as i64;
        let report = sweep_persona(&mut memory, far_future, 0.5);
        assert_eq!(report.skipped_protected, 1, "受保护条目豁免遗忘曲线");
        assert_eq!(report.dropped, 1, "未保护条目走慢速曲线淘汰");
        assert_eq!(memory.boundaries.len(), 1, "底线不被时间冲淡");
        assert!(memory.preferences.is_empty());
    }

    /// ⑫ 自述节: 信念数 / 偏好数 / 张力数 / 最后整编真值.
    #[test]
    fn status_section_reports_counts_and_the_last_consolidation_truth() {
        let mut memory = PersonaMemory::default();
        assert_eq!(
            memory.status().last_consolidation_at,
            None,
            "尚未整编如实 None"
        );

        memory.apply_extract(&extract_from("ep-1", "我相信先验证再叙事"), 10);
        memory.apply_extract(&extract_from("ep-1", "我喜欢把计划写清楚"), 10);
        memory.apply_extract(&extract_from("ep-1", "我绝不说谎"), 10);
        memory.apply_extract(&extract_from("ep-2", "我习惯说谎"), 10);

        let status = memory.status();
        assert_eq!(status.core_beliefs, 1);
        assert_eq!(status.preferences, 1);
        assert_eq!(status.boundaries, 1);
        assert_eq!(status.tensions, 1);
        assert_eq!(status.last_consolidation_at, Some(10), "最后整编真值");
    }

    /// 保护语义强度: 受保护恒 1.0, 未保护按慢速曲线衰减.
    #[test]
    fn persona_decay_strength_is_exempt_or_slow() {
        assert_eq!(
            PERSONA_SLOW_HALF_LIFE_HOURS,
            24.0 * 90.0,
            "persona 未保护层慢速半衰期 = 90 天 (设计裁定 R1⑩)"
        );
        assert_eq!(
            PERSONA_SLOW_HALF_LIFE_HOURS,
            crate::retention::BASELINE_HALF_LIFE_HOURS,
            "口径统一: persona 未保护层与未保护记忆同一遗忘半衰期"
        );
        let entry = PersonaEntry {
            id: "persona:x".into(),
            kind: PersonaKind::CoreBelief,
            text: "我相信先验证再叙事".into(),
            sources: Vec::new(),
            polarity: PersonaPolarity::Positive,
            created_at: 0,
            updated_at: 0,
            protection: PersonaProtection::On,
        };
        assert_eq!(persona_decay_strength(&entry, 10_000_000), 1.0);
        let mut unprotected = entry.clone();
        unprotected.protection = PersonaProtection::Off;
        let one_half_life = (PERSONA_SLOW_HALF_LIFE_HOURS * 3600.0) as i64;
        let slow = persona_decay_strength(&unprotected, one_half_life);
        assert!(
            (slow - 0.5).abs() < 1e-3,
            "慢速曲线: 一个慢速半衰期 (90 天) 后强度应为 0.5, 实测 {slow}"
        );
    }

    /// 条目 id 确定性: 同文本同类 → 同 id (跨轮幂等的键).
    #[test]
    fn entry_ids_are_deterministic() {
        assert_eq!(
            entry_id(PersonaKind::Boundary, &normalize_text("我绝不说谎")),
            entry_id(PersonaKind::Boundary, &normalize_text("我绝不说谎"))
        );
        assert_ne!(
            entry_id(PersonaKind::Boundary, &normalize_text("我绝不说谎")),
            entry_id(PersonaKind::CoreBelief, &normalize_text("我绝不说谎"))
        );
    }

    /// 极性判定: 否定极标记 → Negative.
    #[test]
    fn polarity_tracks_negation_markers() {
        assert_eq!(polarity("我绝不说谎"), PersonaPolarity::Negative);
        assert_eq!(polarity("我相信先验证"), PersonaPolarity::Positive);
    }
}
