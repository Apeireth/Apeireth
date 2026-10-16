//! 只读检索工具面 (`tool.skill_search`, 声明名 `skill_search`) —— 调用面的
//! 按需拉取档。
//!
//! 治理: **只读档、零审批** —— 零文件系统写、无 ask 档钩子, `read_only`
//! 会话预设不拒只读能力。检索结果进上下文一律走 **L2 封套披露**
//! ([`crate::skill::invocation`]): 触发词命中 / 按需点名的卡按共享预算披露,
//! 未命中卡**零注入**; 卡名轻清单 (catalog) 只含名字、零卡体 —— 绝不
//! 无差别全量注入。

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};
use crate::skill::invocation::{
    build_skill_disclosure, select_for_context, DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS,
};
use crate::skill::library::SkillLibrary;

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const SKILL_SEARCH_CAPABILITY_ID: &str = "tool.skill_search";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const SKILL_SEARCH_TOOL_NAME: &str = "skill_search";

/// 生产执行时限 (毫秒): 检索是本地读 + 纯选择, 到期即归 `timeout.*`。
pub const SKILL_SEARCH_TIMEOUT_MS: u64 = 5_000;

/// 单次检索的默认命中上限。
pub const SKILL_SEARCH_DEFAULT_LIMIT: usize = 5;

/// 单次检索的命中上限 (超出拒绝即帧, 不静默截断)。
pub const SKILL_SEARCH_MAX_LIMIT: usize = 20;

/// 卡名轻清单上限 (只列名字, 零卡体; 超出只报总数)。
pub const SKILL_SEARCH_CATALOG_MAX: usize = 50;

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const SKILL_SEARCH_TOOL_DESCRIPTION: &str = "Search the skill library and \
  disclose matching skill cards for prompt injection. Matching is retrieval \
  style: a card matches when the query text hits one of its trigger words or \
  names the card; matched cards are disclosed through the untrusted reference \
  envelope under the shared per-source budget, and unmatched cards are never \
  injected. The catalog lists card names only (no card bodies). This tool is \
  read-only: it needs no approval and writes nothing.";

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillSearchParams {
    /// 检索文本 (对话上下文 / 点名的卡名)。
    pub query: String,
    /// 命中上限 (缺省 [`SKILL_SEARCH_DEFAULT_LIMIT`]; 超 [`SKILL_SEARCH_MAX_LIMIT`] 拒绝即帧)。
    #[serde(default)]
    pub limit: Option<usize>,
}

/// 检索请求校验 (纯判定): 返回 `Some(拒绝即帧文案)` 即拒绝。
pub fn search_request_refusal(params: &SkillSearchParams) -> Option<String> {
    if params.query.trim().is_empty() {
        return Some("拒绝检索: query 不能为空".to_string());
    }
    if let Some(limit) = params.limit {
        if limit == 0 || limit > SKILL_SEARCH_MAX_LIMIT {
            return Some(format!(
                "拒绝检索: limit 必须在 1..={SKILL_SEARCH_MAX_LIMIT} (不静默截断)"
            ));
        }
    }
    None
}

/// 从调用参数解析检索请求 (解析失败/校验失败返回拒绝文案)。
pub fn validated_search_request(
    arguments: &serde_json::Value,
) -> Result<SkillSearchParams, String> {
    let params: SkillSearchParams = serde_json::from_value(arguments.clone())
        .map_err(|error| format!("invalid skill_search parameters: {error}"))?;
    if let Some(refusal) = search_request_refusal(&params) {
        return Err(refusal);
    }
    Ok(params)
}

/// 只读检索工具 (`tool.skill_search`, 声明名 `skill_search`)。
///
/// 零文件系统写: 技能库只读打开 (拒开的卡如实计数, 不砖化检索)。
pub struct SkillSearchTool {
    id: CapabilityId,
    library: SkillLibrary,
    budget_chars: usize,
}

impl SkillSearchTool {
    /// 绑定技能库与披露预算 (字符, 与调用方上下文预算同参)。
    pub fn new(library: SkillLibrary, budget_chars: usize) -> Self {
        Self {
            id: CapabilityId::new(SKILL_SEARCH_CAPABILITY_ID).expect("valid capability id"),
            library,
            budget_chars,
        }
    }

    /// 用缺省披露预算 ( [`DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS`] ) 绑定。
    pub fn with_default_budget(library: SkillLibrary) -> Self {
        Self::new(library, DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS)
    }

    /// 技能库根 (测试/装配检查用)。
    pub fn library_root(&self) -> &std::path::Path {
        self.library.root()
    }

    /// 输出归一合同: 检索报告的形状冻结为六字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("matched", SchemaKind::Array)
            .require("catalog", SchemaKind::Array)
            .require("catalog_total", SchemaKind::Number)
            .require("disclosure", SchemaKind::String)
            .require("refused_cards", SchemaKind::Number)
    }
}

#[async_trait]
impl ToolCapability for SkillSearchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Retrieval text: the conversation context or the name of a card to pull."
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum matched cards to disclose (default 5, hard max 20)."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(SKILL_SEARCH_TOOL_NAME)
            .with_description(SKILL_SEARCH_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = SKILL_SEARCH_TOOL_NAME;
        let params = match validated_search_request(&call.arguments) {
            Ok(params) => params,
            Err(refusal) => return ToolResult::permanent_error(&call.id, refusal).with_name(name),
        };
        let limit = params.limit.unwrap_or(SKILL_SEARCH_DEFAULT_LIMIT);
        let scan = self.library.scan();
        let cards: Vec<crate::skill::card::SkillCard> =
            scan.cards.into_iter().map(|doc| doc.body).collect();
        let matched = select_for_context(&cards, &params.query, limit);
        let disclosure = build_skill_disclosure(&matched, self.budget_chars);
        let matched_names: Vec<String> = matched.iter().map(|card| card.name.clone()).collect();
        let mut catalog: Vec<String> = cards.iter().map(|card| card.name.clone()).collect();
        let catalog_total = catalog.len();
        catalog.truncate(SKILL_SEARCH_CATALOG_MAX);
        let status = if matched.is_empty() {
            "no_match"
        } else {
            "disclosed"
        };
        ToolResult::ok(
            &call.id,
            serde_json::json!({
                "status": status,
                "matched": matched_names,
                "catalog": catalog,
                "catalog_total": catalog_total,
                "disclosure": disclosure,
                "refused_cards": scan.refused.len(),
            }),
        )
        .with_name(name)
    }
}

/// 单调边界 Guard (只拒不放): 空查询 / 越限 limit 一律拒绝, 后续阶段不得翻回。
pub struct SkillSearchBoundaryGuard;

impl SkillSearchBoundaryGuard {
    /// 空构造 (无状态 Guard)。
    pub fn new() -> Self {
        Self
    }
}

impl Default for SkillSearchBoundaryGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard for SkillSearchBoundaryGuard {
    fn name(&self) -> &str {
        "skill_search_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != SKILL_SEARCH_CAPABILITY_ID {
            return None;
        }
        let Ok(params) = serde_json::from_value::<SkillSearchParams>(request.arguments.clone())
        else {
            return None;
        };
        search_request_refusal(&params)
    }
}

/// 组装只读检索五段流水线: 单调边界 Guard → 超时 → 输出归一合同。
///
/// **只读档零审批**: 无 ask 档钩子 (与 `tool.worktree_plan` 同口径)。
pub fn skill_search_pipeline() -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_guard(Arc::new(SkillSearchBoundaryGuard::new()))
            .with_around(
                AroundPolicy::new().with_timeout(SKILL_SEARCH_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(SkillSearchTool::output_schema()),
    )
}

/// 组装只读检索工具面 (模型侧一个能力)。
pub fn skill_search_capability(
    library: SkillLibrary,
    budget_chars: usize,
) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(SkillSearchTool::new(library, budget_chars));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        skill_search_pipeline(),
    ))
}
