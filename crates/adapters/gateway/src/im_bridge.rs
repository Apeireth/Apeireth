//! IM 快捷接入: 双向消息桥 (件二)。
//!
//! - **入**: IM 收到的文本消息 → 归一化 [`apeireth_sdk::im::ImInboundMessage`] →
//!   会话回合 (走既有对话链 [`ImTurnHandler`], 生产实现是
//!   [`CanonicalChainHandler`] 直连 canonical 入口) →
//! - **出**: 回复按 [`apeireth_sdk::im::ImSegmentPolicy`] **显式分段/截断**后
//!   流回 IM (每段一次出站, 截断必带标记并计丢弃字符);
//! - **同源**: IM 会话与桌面会话是同一个 [`SessionId`], 映射表
//!   ([`ImSessionMapStore`]) 持久化到数据目录 (存储文档拒开语义);
//! - 审批卡片按钮回调走 [`ImApprovalCloser`] 的四态闭合 (见 [`crate::im_approval`])。
//!
//! HTTP 面: [`im_router`] 提供 `POST /v1/im/channels/{channel_id}/events`
//! (入站回调体 + `X-Im-Signature` 签名头), 未配置 IM 时整面不挂载 (本地零回归)。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::clock::{Clock, SystemClock};
use apeireth_core::kernel::{ApprovalId, SessionId, Timestamp};
use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_runtime::canonical::{ApprovalDecision, ApprovalResolution, Runtime, TurnOutcome};
use apeireth_sdk::im::{
    parse_inbound_event, ImApprovalCard, ImCardAction, ImChannelKind, ImChannelTarget,
    ImInboundEvent, ImInboundMessage, ImOutboundText, ImSegmentPolicy, ImSegmentation,
    ImSendReceipt, ImSender,
};
use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::canonical_entry::{execute_chat, CanonicalChatOutcome, CanonicalChatRequest};
use crate::im_approval::{
    ImApprovalCloser, ImApprovalClosure, ImApprovalClosureResult, ImApprovalNotice,
    ImApprovalRequest, ImApprovalResolution, ImApprovalResolver,
};
use crate::im_channels::ImChannelAssembly;

/// 会话映射表的文件名。
pub const IM_SESSIONS_FILE: &str = "im-sessions.json";

/// 会话映射表的存储文档身份。
pub const IM_SESSIONS_DOC_NAME: &str = "im-sessions";

/// 会话映射表的格式版本。
pub const IM_SESSIONS_DOC_VERSION: u32 = 1;

/// 签名头 (适配器契约: `sha256=<hex(sha256(secret || "\n" || body))>`)。
pub const IM_SIGNATURE_HEADER: &str = "x-im-signature";

// ---------------------------------------------------------------------------
// 会话 id 映射表 (持久)
// ---------------------------------------------------------------------------

/// 一条映射: IM 会话 ↔ 桌面会话 (同源)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImSessionMapping {
    /// 渠道 id。
    pub channel_id: String,
    /// IM 会话 id。
    pub conversation_id: String,
    /// 桌面会话 id (同一会话, 同一历史)。
    pub session: SessionId,
}

/// 映射表主体 (存储文档 body)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImSessionTable {
    /// 全部映射 (插入顺序)。
    pub entries: Vec<ImSessionMapping>,
}

/// 映射表存储 (内存 + 持久; 缺失 = 空表, 坏档 = 拒开)。
pub struct ImSessionMapStore {
    path: Option<PathBuf>,
    table: ImSessionTable,
}

impl ImSessionMapStore {
    /// 纯内存存储 (测试/无数据目录)。
    pub fn in_memory() -> Self {
        Self {
            path: None,
            table: ImSessionTable::default(),
        }
    }

    /// 打开数据目录里的映射表 (存在即按存储文档语义打开, 坏档 fail-closed)。
    pub fn open(data_dir: &std::path::Path) -> Result<Self, ImBridgeError> {
        let path = im_sessions_path(data_dir);
        if !path.exists() {
            return Ok(Self {
                path: Some(path),
                table: ImSessionTable::default(),
            });
        }
        let doc = stored_doc::open_single::<ImSessionTable>(&path, &sessions_doc_compat())
            .map_err(|e| ImBridgeError::Session(e.to_string()))?;
        Ok(Self {
            path: Some(path),
            table: doc.body,
        })
    }

    /// 查映射 (不创建)。
    pub fn lookup(&self, channel_id: &str, conversation_id: &str) -> Option<SessionId> {
        self.table
            .entries
            .iter()
            .find(|entry| {
                entry.channel_id == channel_id && entry.conversation_id == conversation_id
            })
            .map(|entry| entry.session)
    }

    /// 查会话对应的 IM 会话 (桌面侧转发审批卡片用)。
    pub fn conversation_for(&self, session: &SessionId) -> Option<(String, String)> {
        self.table
            .entries
            .iter()
            .find(|entry| entry.session == *session)
            .map(|entry| (entry.channel_id.clone(), entry.conversation_id.clone()))
    }

    /// 查/建映射: 首次见到的 IM 会话铸一个桌面会话并**持久化**。
    pub fn session_for(
        &mut self,
        channel_id: &str,
        conversation_id: &str,
    ) -> Result<SessionId, ImBridgeError> {
        if let Some(session) = self.lookup(channel_id, conversation_id) {
            return Ok(session);
        }
        let session = SessionId::new();
        self.bind(channel_id, conversation_id, session)?;
        Ok(session)
    }

    /// 显式绑定 (桌面会话转发到 IM 会话时同源绑定)。
    pub fn bind(
        &mut self,
        channel_id: &str,
        conversation_id: &str,
        session: SessionId,
    ) -> Result<(), ImBridgeError> {
        if let Some(existing) = self.table.entries.iter().find(|entry| {
            entry.channel_id == channel_id && entry.conversation_id == conversation_id
        }) {
            if existing.session == session {
                return Ok(());
            }
            return Err(ImBridgeError::Session(format!(
                "conversation {conversation_id:?} is already mapped to another session"
            )));
        }
        self.table.entries.push(ImSessionMapping {
            channel_id: channel_id.to_string(),
            conversation_id: conversation_id.to_string(),
            session,
        });
        self.persist()
    }

    /// 全部映射 (读面)。
    pub fn entries(&self) -> &[ImSessionMapping] {
        &self.table.entries
    }

    fn persist(&self) -> Result<(), ImBridgeError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        stored_doc::save_single(
            path,
            &sessions_doc_compat(),
            self.table.clone(),
            stored_doc::DEFAULT_DOC_MODE,
        )
        .map(|_| ())
        .map_err(|e| ImBridgeError::Session(e.to_string()))
    }
}

/// 会话映射表的落盘路径。
pub fn im_sessions_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join(IM_SESSIONS_FILE)
}

/// 会话映射表的读契约 (仅当前版本可读)。
pub fn sessions_doc_compat() -> DocCompat {
    DocCompat::exact(IM_SESSIONS_DOC_NAME, IM_SESSIONS_DOC_VERSION)
}

// ---------------------------------------------------------------------------
// 对话链适配 (走既有 canonical 入口)
// ---------------------------------------------------------------------------

/// 一个会话回合的结果 (回复文本 + 可能的待审批)。
#[derive(Debug, Clone, Default)]
pub struct ImTurnOutcome {
    /// 回复文本 (空 = 无文本回复, 只有卡片)。
    pub text: String,
    /// 回合暂停待人批时的审批通知。
    pub approval: Option<ImApprovalNotice>,
}

/// 对话链 (IM 文本 → 会话回合)。生产实现 [`CanonicalChainHandler`]。
#[async_trait]
pub trait ImTurnHandler: Send + Sync {
    /// 以 `session` 跑一个会话回合。
    async fn handle_text(&self, session: SessionId, text: &str) -> ImTurnOutcome;
}

/// 生产对话链适配: 直连既有 canonical 入口 [`execute_chat`] / canonical 审批路径。
pub struct CanonicalChainHandler {
    runtime: Arc<Runtime>,
}

impl CanonicalChainHandler {
    /// 构造 (拿到运行时)。
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl ImTurnHandler for CanonicalChainHandler {
    async fn handle_text(&self, session: SessionId, text: &str) -> ImTurnOutcome {
        let request = CanonicalChatRequest {
            session: Some(session),
            input: text.to_string(),
            model: None,
            system: None,
        };
        match execute_chat(&self.runtime, request).await {
            Ok(CanonicalChatOutcome::Completed(response)) => ImTurnOutcome {
                text: response.text,
                approval: None,
            },
            Ok(CanonicalChatOutcome::PendingApproval(view)) => {
                let round = u64::from(view.round);
                ImTurnOutcome {
                    text: String::new(),
                    approval: Some(ImApprovalNotice::from_canonical(&view, round)),
                }
            }
            Err(error) => ImTurnOutcome {
                text: format!("处理失败: {error}"),
                approval: None,
            },
        }
    }
}

#[async_trait]
impl ImApprovalResolver for CanonicalChainHandler {
    async fn resolve(
        &self,
        session: SessionId,
        approval_ref: &str,
        decision: &str,
    ) -> ImApprovalResolution {
        let Ok(approval) = approval_ref.parse::<ApprovalId>() else {
            return ImApprovalResolution::NotFound;
        };
        let (decision, label) = match decision {
            "approve" => (ApprovalDecision::Approve, "approved"),
            "reject" => (ApprovalDecision::Reject { reason: None }, "rejected"),
            "cancel" => (ApprovalDecision::Cancel { reason: None }, "cancelled"),
            _ => return ImApprovalResolution::NotFound,
        };
        match self
            .runtime
            .resolve_approval(session, approval, decision)
            .await
        {
            Ok(ApprovalResolution::Resumed(TurnOutcome::Completed(response))) => {
                ImApprovalResolution::Resumed {
                    text: response.text,
                    label: label.to_string(),
                }
            }
            Ok(ApprovalResolution::Resumed(TurnOutcome::PendingApproval(_))) => {
                // 续跑又撞上下一个待审批: 本按钮回合已完成, 后续卡片另发。
                ImApprovalResolution::Resumed {
                    text: String::new(),
                    label: label.to_string(),
                }
            }
            Ok(ApprovalResolution::Expired) => ImApprovalResolution::Expired,
            Ok(ApprovalResolution::AlreadyResolved { status }) => {
                ImApprovalResolution::AlreadyResolved {
                    label: format!("{status:?}").to_ascii_lowercase(),
                }
            }
            Ok(ApprovalResolution::ExecutionInterrupted { .. }) => {
                ImApprovalResolution::Interrupted
            }
            Ok(ApprovalResolution::NotFound) => ImApprovalResolution::NotFound,
            Err(error) => ImApprovalResolution::Failed {
                reason: error.to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// 消息桥
// ---------------------------------------------------------------------------

/// 消息桥错误 (事实分类)。
#[derive(Debug, thiserror::Error)]
pub enum ImBridgeError {
    /// 渠道未配置。
    #[error("im channel {0:?} is not configured")]
    UnknownChannel(String),
    /// 入站校验失败。
    #[error("im inbound payload failed verification: {0}")]
    Verify(String),
    /// 入站解码失败。
    #[error("im inbound payload could not be decoded: {0}")]
    Decode(String),
    /// 出站投递失败 (含重连预算耗尽)。
    #[error("im outbound delivery failed: {0}")]
    Delivery(String),
    /// 审批审计配对提交失败。
    #[error("im approval audit commit failed: {0}")]
    Audit(String),
    /// 会话映射存储失败。
    #[error("im session mapping store failed: {0}")]
    Session(String),
    /// 会话映射缺失 (卡片回调找不到同源会话)。
    #[error("im conversation is not mapped to a session: {0}")]
    UnmappedConversation(String),
}

/// 入站处理结果 (HTTP 面直接回给对端)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImInboundResult {
    /// 握手挑战 (回显)。
    Verify {
        /// 待回显 challenge。
        challenge: Option<String>,
    },
    /// 已识别但无需处理。
    Ignored {
        /// 事实原因。
        reason: String,
    },
    /// 文本消息已回 (分段投递)。
    Replied {
        /// 同源桌面会话。
        session: SessionId,
        /// 投递段数。
        segments: usize,
        /// 是否截断。
        truncated: bool,
        /// 丢弃字符数。
        dropped_chars: usize,
    },
    /// 回合暂停待人批 (卡片已发)。
    ApprovalRequested {
        /// 同源桌面会话。
        session: SessionId,
        /// 审计配对身份。
        pair_id: String,
        /// 卡片风险级。
        risk_level: String,
    },
    /// 卡片按钮已闭合 (四态 + 一次性执行)。
    Closed {
        /// 闭合记录。
        closure: ImApprovalClosure,
        /// 回执段数。
        reply_segments: usize,
    },
}

/// 双向消息桥 (入站解码 → 会话回合 → 分段回流 / 审批卡片 → 四态闭合)。
pub struct ImBridge {
    channels: HashMap<String, ImChannelTarget>,
    kinds: HashMap<String, ImChannelKind>,
    sender: Arc<dyn ImSender>,
    sessions: Arc<Mutex<ImSessionMapStore>>,
    handler: Arc<dyn ImTurnHandler>,
    approvals: Arc<ImApprovalCloser>,
    segment_override: Option<ImSegmentPolicy>,
    clock: Arc<dyn Clock>,
}

impl ImBridge {
    /// 装配一座桥 (无启用渠道时 `is_active()` = false, 全面 inert)。
    pub fn new(
        assembly: &ImChannelAssembly,
        sender: Arc<dyn ImSender>,
        sessions: Arc<Mutex<ImSessionMapStore>>,
        handler: Arc<dyn ImTurnHandler>,
        approvals: Arc<ImApprovalCloser>,
    ) -> Self {
        let mut channels = HashMap::new();
        let mut kinds = HashMap::new();
        for target in &assembly.targets {
            kinds.insert(target.id.clone(), target.kind);
            channels.insert(target.id.clone(), target.clone());
        }
        Self {
            channels,
            kinds,
            sender,
            sessions,
            handler,
            approvals,
            segment_override: None,
            clock: Arc::new(SystemClock),
        }
    }

    /// 注入时钟 (超时判定口径; 测试与运行时共用同一时钟)。
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// 覆盖分段策略 (默认按 kind 的消息预算)。
    pub fn with_segment_policy(mut self, policy: ImSegmentPolicy) -> Self {
        self.segment_override = Some(policy);
        self
    }

    /// 是否有渠道接入运行。
    pub fn is_active(&self) -> bool {
        !self.channels.is_empty()
    }

    /// 会话映射读面 (同源证据)。
    pub fn session_entries(&self) -> Vec<ImSessionMapping> {
        self.sessions
            .lock()
            .map(|store| store.entries().to_vec())
            .unwrap_or_default()
    }

    /// 处理一条入站回调体 (签名头可选; 渠道有秘密时必须通过校验)。
    pub async fn handle_inbound(
        &self,
        channel_id: &str,
        raw_body: &str,
        signature: Option<&str>,
    ) -> Result<ImInboundResult, ImBridgeError> {
        let target = self
            .channels
            .get(channel_id)
            .ok_or_else(|| ImBridgeError::UnknownChannel(channel_id.to_string()))?;
        if let Some(secret) = &target.secret {
            let provided = signature
                .ok_or_else(|| ImBridgeError::Verify("missing signature header".to_string()))?;
            secret
                .verify_body(raw_body.as_bytes(), provided)
                .map_err(|_| ImBridgeError::Verify("signature mismatch".to_string()))?;
        }
        let kind = self
            .kinds
            .get(channel_id)
            .copied()
            .ok_or_else(|| ImBridgeError::UnknownChannel(channel_id.to_string()))?;
        let event = parse_inbound_event(kind, raw_body)
            .map_err(|e| ImBridgeError::Decode(e.to_string()))?;
        match event {
            ImInboundEvent::Verify { challenge } => Ok(ImInboundResult::Verify { challenge }),
            ImInboundEvent::Ignored { reason } => Ok(ImInboundResult::Ignored {
                reason: reason.to_string(),
            }),
            ImInboundEvent::Message(message) => self.handle_message(target, kind, message).await,
            ImInboundEvent::CardAction(action) => {
                self.handle_card_action(target, kind, action).await
            }
        }
    }

    /// 文本消息 → 会话回合 → 回复分段回流 (或审批卡片)。
    pub async fn handle_message(
        &self,
        target: &ImChannelTarget,
        kind: ImChannelKind,
        message: ImInboundMessage,
    ) -> Result<ImInboundResult, ImBridgeError> {
        let session = {
            let mut store = self
                .sessions
                .lock()
                .map_err(|e| ImBridgeError::Session(format!("session map lock poisoned: {e}")))?;
            store.session_for(&target.id, &message.conversation_id)?
        };
        let outcome = self.handler.handle_text(session, &message.text).await;

        if let Some(notice) = outcome.approval {
            let card = notice.approval_card();
            self.deliver_card(target, &message.conversation_id, &card)
                .await?;
            return Ok(ImInboundResult::ApprovalRequested {
                session,
                pair_id: notice.pair_id.clone(),
                risk_level: notice.risk_level(),
            });
        }

        let segmentation = self.segment(kind, &outcome.text);
        let reply_segments = self
            .deliver_segments(target, &message.conversation_id, &segmentation)
            .await?;
        Ok(ImInboundResult::Replied {
            session,
            segments: reply_segments,
            truncated: segmentation.truncated,
            dropped_chars: segmentation.dropped_chars,
        })
    }

    /// 卡片按钮 → 四态闭合 (审计配对原子) → 回执回流。
    pub async fn handle_card_action(
        &self,
        target: &ImChannelTarget,
        _kind: ImChannelKind,
        action: ImCardAction,
    ) -> Result<ImInboundResult, ImBridgeError> {
        let session = {
            let store = self
                .sessions
                .lock()
                .map_err(|e| ImBridgeError::Session(format!("session map lock poisoned: {e}")))?;
            store
                .lookup(&target.id, &action.conversation_id)
                .ok_or_else(|| {
                    ImBridgeError::UnmappedConversation(action.conversation_id.clone())
                })?
        };
        let now_ms = Timestamp::from_clock(self.clock.as_ref()).epoch_millis();
        let result = self
            .approvals
            .close(ImApprovalRequest {
                session,
                payload: action.payload,
                now_ms,
            })
            .await?;
        match result {
            ImApprovalClosureResult::Closed(closure) => {
                let reply_segments = self
                    .deliver_segments(
                        target,
                        &action.conversation_id,
                        &self.segment_text(&closure.reply_text),
                    )
                    .await?;
                Ok(ImInboundResult::Closed {
                    closure,
                    reply_segments,
                })
            }
            ImApprovalClosureResult::AlreadyClosed { .. } => Ok(ImInboundResult::Ignored {
                reason: "approval round already closed".to_string(),
            }),
        }
    }

    /// 桌面侧待审批 → IM 审批卡片 (同源绑定会话映射)。
    pub async fn notify_approval(
        &self,
        channel_id: &str,
        conversation_id: &str,
        notice: &ImApprovalNotice,
    ) -> Result<ImSendReceipt, ImBridgeError> {
        let target = self
            .channels
            .get(channel_id)
            .ok_or_else(|| ImBridgeError::UnknownChannel(channel_id.to_string()))?;
        {
            let mut store = self
                .sessions
                .lock()
                .map_err(|e| ImBridgeError::Session(format!("session map lock poisoned: {e}")))?;
            store.bind(channel_id, conversation_id, notice.session)?;
        }
        let card = notice.approval_card();
        self.deliver_card(target, conversation_id, &card).await
    }

    fn segment(&self, kind: ImChannelKind, text: &str) -> ImSegmentation {
        let policy = self
            .segment_override
            .clone()
            .unwrap_or_else(|| ImSegmentPolicy::for_kind(kind));
        apeireth_sdk::im::segment_reply(text, &policy)
    }

    fn segment_text(&self, text: &str) -> ImSegmentation {
        let policy = self
            .segment_override
            .clone()
            .unwrap_or_else(|| ImSegmentPolicy::default());
        apeireth_sdk::im::segment_reply(text, &policy)
    }

    async fn deliver_segments(
        &self,
        target: &ImChannelTarget,
        conversation_id: &str,
        segmentation: &ImSegmentation,
    ) -> Result<usize, ImBridgeError> {
        for segment in &segmentation.segments {
            let message = ImOutboundText {
                conversation_id: conversation_id.to_string(),
                text: segment.clone(),
            };
            self.sender
                .send_text(target, &message)
                .await
                .map_err(|e| ImBridgeError::Delivery(e.to_string()))?;
        }
        Ok(segmentation.segments.len())
    }

    async fn deliver_card(
        &self,
        target: &ImChannelTarget,
        conversation_id: &str,
        card: &ImApprovalCard,
    ) -> Result<ImSendReceipt, ImBridgeError> {
        self.sender
            .send_card(target, conversation_id, card)
            .await
            .map_err(|e| ImBridgeError::Delivery(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// HTTP 面 (入站回调端点)
// ---------------------------------------------------------------------------

/// 挂载 IM 入站路由 (`POST /v1/im/channels/{channel_id}/events`)。
///
/// 无启用渠道时调用方应**不挂载**本路由: 本地审批/对话零回归。
pub fn im_router(bridge: Arc<ImBridge>) -> Router {
    Router::new()
        .route(
            "/v1/im/channels/:channel_id/events",
            post(im_events_handler),
        )
        .with_state(bridge)
}

async fn im_events_handler(
    State(bridge): State<Arc<ImBridge>>,
    Path(channel_id): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let raw = match std::str::from_utf8(&body) {
        Ok(raw) => raw,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("body is not UTF-8: {error}")})),
            )
                .into_response()
        }
    };
    let signature = headers
        .get(IM_SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok());
    match bridge.handle_inbound(&channel_id, raw, signature).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "result": result,
        }))
        .into_response(),
        Err(ImBridgeError::Verify(reason)) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"ok": false, "error": reason})),
        )
            .into_response(),
        Err(ImBridgeError::Decode(reason)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": reason})),
        )
            .into_response(),
        Err(ImBridgeError::UnknownChannel(channel)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok": false, "error": format!("unknown channel {channel}")})),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"ok": false, "error": error.to_string()})),
        )
            .into_response(),
    }
}
