//! 技能卡模型 (`SkillCard`) —— 书→技能沉淀 (I 轴吸收自动化 I1) 的知识沉淀单元。
//!
//! 一张技能卡 = 一次知识吸收的可调用沉淀: 稳定身份 + 名字 + 摘要 + 触发词 +
//! 正文 + 来源 + 沉淀时刻。来源一律是**内容输入** (文档 / 书 / 文章), 由调用方
//! 的模型侧 prompt 模板提炼出草稿 ([`SkillCardDraft`]), 工具侧负责结构化
//! (触发词归一 + 坏卡拒 + 身份/时刻派生) 与落盘 ([`crate::skill::library`])。
//!
//! # 坏卡拒 (结构校验即拒绝即帧)
//!
//! [`card_draft_refusal`] 是纯判定: 名字/摘要/正文/触发词/来源标题任一空、
//! 超长、含控制符或路径分隔符 → 拒绝即帧, 不落盘。技能卡是长期资产,
//! 「先存下来以后再修」的半成品一律不收。

use serde::{Deserialize, Serialize};

/// 卡名长度上限 (字符)。
pub const SKILL_NAME_MAX_CHARS: usize = 100;

/// 摘要长度上限 (字符)。
pub const SKILL_SUMMARY_MAX_CHARS: usize = 400;

/// 正文长度上限 (字符)。
pub const SKILL_BODY_MAX_CHARS: usize = 40_000;

/// 单个触发词长度上限 (字符)。
pub const SKILL_TRIGGER_MAX_CHARS: usize = 64;

/// 触发词个数上限。
pub const SKILL_TRIGGER_MAX_COUNT: usize = 24;

/// 来源标题长度上限 (字符)。
pub const SKILL_SOURCE_TITLE_MAX_CHARS: usize = 200;

/// 来源种类: 知识吸收的内容输入只能来自这三类 (命名即承诺)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSourceKind {
    /// 文档 (说明/规范/笔记类内容输入)。
    Document,
    /// 书 (成书章节类内容输入)。
    Book,
    /// 文章 (短篇/报告类内容输入)。
    Article,
}

impl SkillSourceKind {
    /// 稳定标签 (报告/审计字段同文)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Book => "book",
            Self::Article => "article",
        }
    }
}

/// 技能卡来源: 种类 + 标题 (人读归属, 供追溯「这张卡从哪来」)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillSource {
    /// 来源种类 (文档 / 书 / 文章)。
    pub kind: SkillSourceKind,
    /// 来源标题 (非空)。
    pub title: String,
}

/// 模型侧草稿 (调用方 prompt 模板产物): 结构化前的技能卡内容。
///
/// 工具侧不负责「提炼」语义 (那是模型侧 prompt 模板的活), 只负责管线与
/// 落盘: 归一触发词、校验坏卡、派生身份与时刻。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCardDraft {
    /// 卡名 (技能库内唯一键, 同名即同张卡的版本演进)。
    pub name: String,
    /// 一句话摘要。
    pub summary: String,
    /// 触发词 (检索式注入的命中面)。
    pub triggers: Vec<String>,
    /// 正文 (要点 / 操作步骤 —— 沉淀的本体)。
    pub body: String,
    /// 来源 (文档 / 书 / 文章 + 标题)。
    pub source: SkillSource,
}

/// 技能卡 (知识沉淀单元)。
///
/// 字段即承诺: `id` 稳定身份 (同名演进不变), `created_at` 首次沉淀时刻
/// (epoch 毫秒, 版本演进不变), 其余为内容面。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCard {
    /// 稳定身份 (由卡名确定性派生, 版本演进不变)。
    pub id: String,
    /// 卡名 (技能库内唯一键)。
    pub name: String,
    /// 一句话摘要。
    pub summary: String,
    /// 触发词 (已归一: 去空白项 / 去重复, 保序)。
    pub triggers: Vec<String>,
    /// 正文 (要点 / 操作步骤)。
    pub body: String,
    /// 来源 (文档 / 书 / 文章 + 标题)。
    pub source: SkillSource,
    /// 首次沉淀时刻 (epoch 毫秒)。
    pub created_at: u64,
}

impl SkillCard {
    /// 披露载荷 (L2 封套内的卡体渲染): 名字 / 摘要 / 触发词 / 正文。
    pub fn disclosure_payload(&self) -> String {
        format!(
            "技能: {}\n摘要: {}\n触发词: {}\n{}",
            self.name,
            self.summary,
            self.triggers.join(" / "),
            self.body
        )
    }
}

/// 卡名归一键 (唯一性判定 / 身份派生): 去首尾空白 + 大小写折叠。
pub fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// 由卡名确定性派生稳定身份 (`skill-` + FNV-1a 64 hex)。
///
/// **非密码学**, 仅身份/文件名用 (与改单 `plan_id` 同口径): 同名必同 id
/// (版本演进保持身份), 改名 = 新卡 (旧名走版本演进留审计)。hex 输出天然
/// 文件名安全。
pub fn skill_card_id(name: &str) -> String {
    format!(
        "skill-{:016x}",
        crate::worktree_plan::fnv1a64(name_key(name).as_bytes())
    )
}

/// 触发词归一: 去首尾空白 / 弃空项 / 大小写折叠去重 (保首次出现序)。
///
/// 归一不做静默截断: 超长超量由 [`card_draft_refusal`] 拒绝即帧。
pub fn normalize_triggers(triggers: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for trigger in triggers {
        let trimmed = trigger.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(trimmed.to_string());
    }
    out
}

/// 坏卡拒 (纯判定): 返回 `Some(拒绝即帧文案)` 即这张卡不许入库。
///
/// 逐项: ① 名字非空 + 限长 + 禁控制符/路径分隔符; ② 摘要非空 + 限长;
/// ③ 正文非空 + 限长; ④ 触发词归一后 ≥1 且逐词非空/限长 + 总量限;
/// ⑤ 来源标题非空 + 限长。
pub fn card_draft_refusal(draft: &SkillCardDraft) -> Option<String> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Some("拒绝技能卡: 卡名 (name) 不能为空".to_string());
    }
    if name.chars().count() > SKILL_NAME_MAX_CHARS {
        return Some(format!(
            "拒绝技能卡: 卡名超长 (上限 {SKILL_NAME_MAX_CHARS} 字符)"
        ));
    }
    if name
        .chars()
        .any(|ch| ch.is_control() || ch == '/' || ch == '\\')
    {
        return Some("拒绝技能卡: 卡名含控制符或路径分隔符 (名字不是路径)".to_string());
    }
    let summary = draft.summary.trim();
    if summary.is_empty() {
        return Some("拒绝技能卡: 摘要 (summary) 不能为空".to_string());
    }
    if summary.chars().count() > SKILL_SUMMARY_MAX_CHARS {
        return Some(format!(
            "拒绝技能卡: 摘要超长 (上限 {SKILL_SUMMARY_MAX_CHARS} 字符)"
        ));
    }
    let body = draft.body.trim();
    if body.is_empty() {
        return Some("拒绝技能卡: 正文 (body) 不能为空".to_string());
    }
    if body.chars().count() > SKILL_BODY_MAX_CHARS {
        return Some(format!(
            "拒绝技能卡: 正文超长 (上限 {SKILL_BODY_MAX_CHARS} 字符)"
        ));
    }
    let triggers = normalize_triggers(&draft.triggers);
    if triggers.is_empty() {
        return Some("拒绝技能卡: 触发词 (triggers) 归一后不能为空".to_string());
    }
    if triggers.len() > SKILL_TRIGGER_MAX_COUNT {
        return Some(format!(
            "拒绝技能卡: 触发词超量 (上限 {SKILL_TRIGGER_MAX_COUNT} 个)"
        ));
    }
    for trigger in &triggers {
        if trigger.chars().count() > SKILL_TRIGGER_MAX_CHARS {
            return Some(format!(
                "拒绝技能卡: 触发词「{trigger}」超长 (上限 {SKILL_TRIGGER_MAX_CHARS} 字符)"
            ));
        }
        if trigger.chars().any(|ch| ch.is_control()) {
            return Some(format!("拒绝技能卡: 触发词「{trigger}」含控制符"));
        }
    }
    let title = draft.source.title.trim();
    if title.is_empty() {
        return Some("拒绝技能卡: 来源标题 (source.title) 不能为空".to_string());
    }
    if title.chars().count() > SKILL_SOURCE_TITLE_MAX_CHARS {
        return Some(format!(
            "拒绝技能卡: 来源标题超长 (上限 {SKILL_SOURCE_TITLE_MAX_CHARS} 字符)"
        ));
    }
    None
}

/// 结构化 (沉淀管线第二拍): 草稿 → 技能卡 —— 坏卡拒 → 触发词归一 →
/// 派生稳定身份与沉淀时刻。`now_ms` 由调用方给 (测试可钉死)。
pub fn structure_card(draft: &SkillCardDraft, now_ms: u64) -> Result<SkillCard, String> {
    if let Some(refusal) = card_draft_refusal(draft) {
        return Err(refusal);
    }
    let name = draft.name.trim().to_string();
    Ok(SkillCard {
        id: skill_card_id(&name),
        name,
        summary: draft.summary.trim().to_string(),
        triggers: normalize_triggers(&draft.triggers),
        body: draft.body.trim().to_string(),
        source: SkillSource {
            kind: draft.source.kind,
            title: draft.source.title.trim().to_string(),
        },
        created_at: now_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> SkillCardDraft {
        SkillCardDraft {
            name: "换元法记得换 dx".into(),
            summary: "定积分换元后积分限与微分元同步替换".into(),
            triggers: vec![" 换元 ".into(), "dx".into(), "换元".into()],
            body: "步骤: ① 令 x=φ(t); ② dx=φ'(t)dt; ③ 换积分限。".into(),
            source: SkillSource {
                kind: SkillSourceKind::Book,
                title: "数学分析讲义".into(),
            },
        }
    }

    #[test]
    fn structure_card_normalizes_triggers_and_derives_identity() {
        let card = structure_card(&draft(), 1_700_000_000_000).unwrap();
        assert_eq!(card.triggers, vec!["换元", "dx"], "归一去空白去重复且保序");
        assert_eq!(card.name, "换元法记得换 dx");
        assert_eq!(card.created_at, 1_700_000_000_000);
        assert!(card.id.starts_with("skill-"));
        assert_eq!(card.id, skill_card_id("换元法记得换 dx"));
    }

    #[test]
    fn identity_is_stable_for_the_same_name_key() {
        assert_eq!(
            skill_card_id("  Rust 惯用法 "),
            skill_card_id("rust 惯用法"),
            "同名键必同 id (版本演进保持身份)"
        );
        assert_ne!(skill_card_id("rust a"), skill_card_id("rust b"));
    }

    #[test]
    fn bad_cards_are_refused_not_stored() {
        let mut bad = draft();
        bad.name = "  ".into();
        assert!(card_draft_refusal(&bad).unwrap().contains("卡名"));

        let mut bad = draft();
        bad.triggers = vec!["  ".into()];
        assert!(card_draft_refusal(&bad).unwrap().contains("触发词"));

        let mut bad = draft();
        bad.body = String::new();
        assert!(card_draft_refusal(&bad).unwrap().contains("正文"));

        let mut bad = draft();
        bad.summary = String::new();
        assert!(card_draft_refusal(&bad).unwrap().contains("摘要"));

        let mut bad = draft();
        bad.source.title = " ".into();
        assert!(card_draft_refusal(&bad).unwrap().contains("来源标题"));

        let mut bad = draft();
        bad.name = "../越狱".into();
        assert!(card_draft_refusal(&bad).unwrap().contains("路径分隔符"));
    }

    #[test]
    fn disclosure_payload_carries_name_summary_triggers_and_body() {
        let card = structure_card(&draft(), 0).unwrap();
        let payload = card.disclosure_payload();
        assert!(payload.contains("技能: 换元法记得换 dx"));
        assert!(payload.contains("触发词: 换元 / dx"));
        assert!(payload.contains("dx=φ'(t)dt"));
    }
}
