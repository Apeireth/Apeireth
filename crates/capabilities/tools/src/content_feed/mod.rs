//! 内容流接入 (J1「内容流接入」——让它看见你的世界) + J2 订阅生命周期。
//!
//! 面拆两层: [`store`] 订阅源面 (`feeds.json` StoredDoc 拒开 + 敏感面拒绝),
//! [`summary`] 结构化摘要面 (标题 / 正文截断 / 来源 / 时间), [`tool`] 生产工具
//! 面 (`tool.content_feed`: 受控拉取 → 摘要 → L2 封套 → 入记忆流)。语义全文见
//! [`tool`] 模块头。
//!
//! # J2 拉取→摄取接缝 + 订阅生命周期
//!
//! 拉来的内容自动走**保鲜回路** (散列门 / 摄取队列 / 增量折入, 只读消费记忆的
//! `enqueue_ingest` 端口, 见 [`ingest`]) + **订阅生命周期** (条目去重账 / 拉取
//! 失败退避 / 订阅健康度, 见 [`lifecycle`]) + **注意力纪律** (单次/每日摄取上限,
//! 超限截断如实)。同条目重复拉取 = **零新记忆** (条目账 + 散列门两道去重)。
//!
//! # 治理档位 (二选一已钉死)
//!
//! **拉取与入库都经审批档**: `pull` (拉取 + 入库同一次调用) 每次都停人工
//! 审批, 无自动放行档; 改订阅 (增/删) 走 write_intent 同族 (改订阅 = 写文件),
//! 同样每次人批; `feed_list` 纯读但**不设免审动作** (双面能力宁严勿松, 只读
//! 自述走 `tool.self_status` 的订阅数/最近拉取, 零审批)。备选「拉取只读档、
//! 入库审批档」被拒: 拉取只读档会让跨源内容在人审之前进入提示词面, 正是本
//! 面要闸的注入面。档位由 [`ContentFeedApprovalHook`] 唯一承载 (与本地审批
//! 面板 / IM 审批卡同一条审批链)。

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use async_trait::async_trait;

use crate::exec_pipeline::ToolExecutionPipeline;
use crate::exec_pipeline::{AroundPolicy, DEFAULT_MAX_TIMEOUT_MS};

pub mod ingest;
pub mod lifecycle;
pub mod store;
pub mod summary;
pub mod tool;

pub use ingest::{
    ingest_port, item_content_hash, item_identity, item_source, stable_ingest_content,
    AttentionLimits, IngestOutcome, IngestPort, IngestRequest, DEFAULT_DAILY_INGESTS,
    DEFAULT_PER_PULL_ITEMS,
};
pub use lifecycle::{
    FeedHealth, FeedLifecycleBody, FeedLifecycleStore, LifecyclePolicy, LifecycleStats,
    FEED_LIFECYCLE_DOC_NAME, FEED_LIFECYCLE_DOC_VERSION, FEED_LIFECYCLE_FILE_NAME, ITEM_LEDGER_MAX,
};
pub use store::{
    feeds_doc_compat, validate_feed_id, validate_feed_url, FeedKind, FeedSource, FeedStore,
    FeedStoreError, FeedWriteFace, PullRecord, FEEDS_DOC_NAME, FEEDS_DOC_VERSION, FEEDS_FILE_NAME,
};
pub use summary::{
    summarize_feed_document, summarize_page, FeedDocumentForm, FeedItemSummary,
    BODY_EXCERPT_MAX_CHARS,
};
pub use tool::{
    episode_for_item, write_face_for_action, ContentFeedConfig, ContentFeedTool, EpisodeWriter,
    PullFeedReport, ACTION_FEED_ADD, ACTION_FEED_LIST, ACTION_FEED_REMOVE, ACTION_PULL,
    CONTENT_FEED_ACTIONS, CONTENT_FEED_CAPABILITY_ID, CONTENT_FEED_DISCLOSURE_NOTE,
    CONTENT_FEED_ENVELOPE_BUDGET_CHARS, CONTENT_FEED_EPISODE_ROLE, CONTENT_FEED_SESSION_LABEL,
    CONTENT_FEED_TOOL_NAME,
};

/// 拉取动作的执行时限 (毫秒): 走阻塞池/受控通道, 到期归 `timeout.*`。
pub const CONTENT_FEED_TIMEOUT_MS: u64 = 120_000;

/// 审批档位理由 (审批卡载荷同文)。按动作如实标注写面:
/// 拉取与入库都经审批档 / 改订阅 = 写文件 (write_intent 同族) / 只读列随面同档。
pub fn content_feed_approval_reason(action: Option<&str>) -> String {
    let face = match action.map(str::as_ref) {
        Some(tool::ACTION_PULL) => "pull = 读网络 + 写记忆流 (双面): 拉取与入库都经审批档",
        Some(tool::ACTION_FEED_ADD) | Some(tool::ACTION_FEED_REMOVE) => {
            "feed_add/feed_remove = 改订阅 = 写文件 (write_intent 同族)"
        }
        Some(tool::ACTION_FEED_LIST) => "feed_list = 只读列订阅, 随双面能力同档 (本能力无免审动作)",
        _ => "动作未识别, fail-closed 停人工审批",
    };
    format!(
        "content_feed ({face}) —— RequireApproval 级: 每次调用都停人工审批, 无自动放行档; \
         跨源内容不得直进提示词"
    )
}

/// 治理档位钩子: `tool.content_feed` 每次调用停 [`Decision::RequireApproval`]
/// (无自动放行档, 任何预设/设置都不得越人)。
///
/// 与 [`crate::git_write`] 的 L2 出网档同一承载方式: 档位在治理层的钩子上,
/// 与本地审批面板 / IM 审批卡同一条审批链。其它能力一律放行 (本钩子只管
/// 自己的面)。
#[derive(Debug, Clone, Copy, Default)]
pub struct ContentFeedApprovalHook;

impl ContentFeedApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for ContentFeedApprovalHook {
    fn name(&self) -> &str {
        "content_feed_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != CONTENT_FEED_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(serde_json::Value::as_str);
        Decision::require_approval(content_feed_approval_reason(action))
    }
}

/// 组装生产五段流水线: 治理档位走治理链 (审批四态), 执行链只装超时与输出
/// 归一合同 (`action` 字段恒在场)。
pub fn content_feed_pipeline() -> Arc<ToolExecutionPipeline> {
    Arc::new(
        ToolExecutionPipeline::new()
            .with_around(
                AroundPolicy::new().with_timeout(CONTENT_FEED_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(tool::ContentFeedTool::output_schema()),
    )
}

/// 组装生产工具面: 内容流工具 + 五段流水线 (模型侧一个能力)。
pub fn content_feed_capability(config: ContentFeedConfig) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(tool::ContentFeedTool::new(config));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        content_feed_pipeline(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{SessionId, TraceId};
    use serde_json::json;

    fn dispatch(action: Option<&str>) -> GovernanceRequest<'static> {
        let capability: &'static CapabilityId = Box::leak(Box::new(
            CapabilityId::new(CONTENT_FEED_CAPABILITY_ID).expect("capability id"),
        ));
        let arguments: &'static serde_json::Value = Box::leak(Box::new(match action {
            Some(action) => json!({ "action": action }),
            None => json!({}),
        }));
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        )
    }

    /// 治理档语义: 每个动作都停人工审批 (无免审动作), 理由按写面如实标注。
    #[tokio::test]
    async fn every_content_feed_action_stops_at_require_approval() {
        let hook = ContentFeedApprovalHook::new();
        for action in CONTENT_FEED_ACTIONS {
            let request = dispatch(Some(action));
            let decision = hook.evaluate(&request).await;
            let Decision::RequireApproval { reason } = decision else {
                panic!("{action} 必须停人工审批 (拉取与入库都经审批档): {decision:?}");
            };
            assert!(reason.contains("RequireApproval"), "{reason}");
            assert!(reason.contains("无自动放行档"), "{reason}");
            assert!(reason.contains("直进提示词"), "{reason}");
        }
        // 动作缺失/未知 fail-closed 同样停审批 (不静默放行)。
        let decision = hook.evaluate(&dispatch(None)).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "{decision:?}"
        );
        let decision = hook.evaluate(&dispatch(Some("rm -rf /"))).await;
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "{decision:?}"
        );
    }

    /// 写面口径进审批卡载荷: 拉取标双面, 改订阅标写文件同族。
    #[tokio::test]
    async fn approval_reason_names_the_write_face_per_action() {
        let hook = ContentFeedApprovalHook::new();
        let Decision::RequireApproval { reason } =
            hook.evaluate(&dispatch(Some(ACTION_PULL))).await
        else {
            panic!("pull 必须停审批");
        };
        assert!(reason.contains("读网络 + 写记忆流"), "{reason}");
        assert!(reason.contains("拉取与入库都经审批档"), "{reason}");

        let Decision::RequireApproval { reason } =
            hook.evaluate(&dispatch(Some(ACTION_FEED_ADD))).await
        else {
            panic!("feed_add 必须停审批");
        };
        assert!(reason.contains("改订阅 = 写文件"), "{reason}");
        assert!(reason.contains("write_intent 同族"), "{reason}");
    }

    /// 本钩子只管自己的面: 其它能力/其它动作形态一律放行 (不越权误伤)。
    #[tokio::test]
    async fn other_capabilities_are_untouched() {
        use apeireth_core::kernel::{SessionId, TraceId};
        let capability = CapabilityId::new("tool.fetch").expect("capability id");
        let arguments = serde_json::json!({ "url": "https://example.com" });
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let decision = ContentFeedApprovalHook::new().evaluate(&request).await;
        assert_eq!(decision, Decision::Allow, "{decision:?}");
    }
}
