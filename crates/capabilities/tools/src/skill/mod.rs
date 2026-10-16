//! 书→技能沉淀 (P 轴吸收自动化 P1, 原 I 轴/I1 —— 2026-10-16 字母双占统一改名, 见
//! `docs/04-internal/backend-enhancement/00-backlog.md` D13) —— 知识吸收成可调用技能的流水线。
//!
//! 四件套 (管线图与测试证据见 `docs/04-internal/backend-enhancement/I1-skill-distill-delivery.md`):
//!
//! 1. **技能卡模型** ([`card`]): `SkillCard{id, name, summary, triggers[],
//!    body, source, created_at}` —— 知识沉淀单元; 来源只收内容输入
//!    (文档 / 书 / 文章);
//! 2. **技能库** ([`library`]): 数据目录 `skills/` 每卡一 StoredDoc (拒开
//!    语义), 创建重复名拒绝, 版本演进 = 同名更新留旧版审计;
//! 3. **沉淀管线** ([`distill`], `tool.skill_distill`): 内容输入 (正文 /
//!    文件只读通道) + 模型侧草稿 → 结构化技能卡 → 入技能库。提炼 (触发词 /
//!    要点 / 操作步骤) 由调用方的 prompt 模板完成, 工具侧管管线与落盘;
//! 4. **调用面** ([`invocation`] + [`search`], `tool.skill_search`): 技能卡
//!    进入模型上下文的唯一方式 = **L2 封套披露** (检索式注入: 触发词命中 /
//!    按需拉取), 绝不无差别全量注入 (上下文预算纪律)。
//!
//! # 治理 (命名即承诺)
//!
//! | 工具 | 档位 | 语义 |
//! |---|---|---|
//! | `tool.skill_distill` | 写文件面 (file_write 总闸族 + 审批档) | 注册即闸 (与写总闸同开); 每次沉淀都人批, 无自动放行档 |
//! | `tool.skill_search` | 只读档 | 零审批、零写盘; `read_only` 会话预设不拒 |

pub mod card;
pub mod distill;
pub mod invocation;
pub mod library;
pub mod search;

pub use card::{
    card_draft_refusal, name_key, normalize_triggers, skill_card_id, structure_card, SkillCard,
    SkillCardDraft, SkillSource, SkillSourceKind,
};
pub use distill::{
    authorized_skill_distill_policy, distill_into, distill_request_refusal, read_source,
    skill_distill_capability, skill_distill_pipeline, skill_distill_summary,
    source_request_refusal, SkillDistillApprovalHook, SkillDistillBoundaryGuard,
    SkillDistillParams, SkillDistillReport, SkillDistillRiskMappingHook, SkillDistillTool,
    SkillSourceInput, SKILL_DISTILL_CAPABILITY_ID, SKILL_DISTILL_TIMEOUT_MS,
    SKILL_DISTILL_TOOL_DESCRIPTION, SKILL_DISTILL_TOOL_NAME, SKILL_SOURCE_INLINE_REF,
    SKILL_SOURCE_MAX_BYTES,
};
pub use invocation::{
    build_skill_disclosure, name_pull, select_for_context, trigger_hit_count,
    DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS, SKILL_DISCLOSURE_HEADER, SKILL_NAME_QUERY_MIN_CHARS,
};
pub use library::{
    SkillCardRefusal, SkillIntake, SkillLibrary, SkillLibraryError, SkillLibraryScan,
    SkillRevisionAudit, SKILLS_SUBDIR, SKILL_AUDIT_DOC_NAME, SKILL_AUDIT_DOC_VERSION,
    SKILL_AUDIT_FILE, SKILL_CARD_DOC_VERSION,
};
pub use search::{
    search_request_refusal, skill_search_capability, skill_search_pipeline,
    validated_search_request, SkillSearchBoundaryGuard, SkillSearchParams, SkillSearchTool,
    SKILL_SEARCH_CAPABILITY_ID, SKILL_SEARCH_CATALOG_MAX, SKILL_SEARCH_DEFAULT_LIMIT,
    SKILL_SEARCH_MAX_LIMIT, SKILL_SEARCH_TIMEOUT_MS, SKILL_SEARCH_TOOL_DESCRIPTION,
    SKILL_SEARCH_TOOL_NAME,
};
