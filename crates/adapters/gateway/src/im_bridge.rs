//! IM 快捷接入: 双向消息桥 (件二)。
//!
//! - **入**: IM 收到的文本消息 → 归一化 [`apeireth_sdk::im::ImInboundMessage`] →
//!   会话回合 (走既有对话链 [`ImTurnHandler`], 生产实现是
//!   [`CanonicalChainHandler`] 直连 canonical 入口) →
//! - **出**: 回复按 [`apeireth_sdk::im::ImSegmentPolicy`] **显式分段/截断**后
//!   流回 IM (每段一次出站, 截断必带标记并计丢弃字符);
//! - **同源**: IM 会话与桌面会话是同一个 [`SessionId`], 映射表
//!   ([`ImSessionMapStore`]) 持久化到数据目录 (存储文档拒开语义);
//! - 审批卡片按钮回调走 [`ImApprovalCloser`] 的四态闭合 (见 [`crate::im_approval`]);
//! - 提案卡出站 + 按钮落账走 [`crate::im_proposal`] (自主度提案面接线): 提案
//!   入账 → 人审卡**同链同信封**推到 IM 渠道 (「建议你亲自确认」行随卡到手机),
//!   批准/拒绝按钮 → `proposal accept/reject` 落账 (decided_by = IM 用户 + 渠道
//!   标注); 推送幂等键 = 提案 id, 推送失败留账待重试 (IM 失败不丢提案)。
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
use apeireth_tools_canonical::device::approval::{
    DeviceApprovalPushOutcome, DeviceApprovalPushRecord,
};
use apeireth_tools_canonical::device::device_control::DeviceApprovalCard;
use apeireth_tools_canonical::proposal_push::{ProposalPushOutcome, ProposalPushRecord};
use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::canonical_entry::{
    execute_chat_attributed, CanonicalChatOutcome, CanonicalChatRequest, SessionAttribution,
};
use crate::im_approval::{
    ImApprovalCloser, ImApprovalClosure, ImApprovalClosureResult, ImApprovalNotice,
    ImApprovalRequest, ImApprovalResolution, ImApprovalResolver,
};
use crate::im_channels::ImChannelAssembly;
use crate::im_device::{
    device_card_envelope, is_device_approval_payload, DeviceApprovalPushResult,
    ImDeviceApprovalFace, ImDeviceClosure, ImDeviceClosureResult,
};
use crate::im_proposal::{
    is_proposal_payload, proposal_card_envelope, ImProposalClosure, ImProposalClosureResult,
    ImProposalFace, ProposalPushResult,
};

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

    /// W2-③: 审批续跑写同一本会话账本 —— 过会话写闸串行 (与在途回合零交叠,
    /// 也与设置补丁/删除串行)。
    async fn resolve_serialized(
        &self,
        session: SessionId,
        approval: ApprovalId,
        decision: ApprovalDecision,
    ) -> Result<ApprovalResolution, apeireth_runtime::canonical::RuntimeError> {
        let permit = self.runtime.sessions().write_permit(session).await;
        let resolved = self
            .runtime
            .resolve_approval(session, approval, decision)
            .await;
        drop(permit);
        resolved
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
        // W2-③: IM 桥是进程内客户端 `im-bridge` —— 回合同样过会话写闸串行
        // (与桌面/TUI/web 的回合排队不交叠); 具名审计面留 W3 (见残余表)。
        let attribution = SessionAttribution::new("im-bridge", None);
        match execute_chat_attributed(&self.runtime, request, attribution).await {
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
        match self.resolve_serialized(session, approval, decision).await {
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
    /// 提案记账失败 (提案账/推送账读写失败 —— 留账失败必须报真因, 不静默)。
    #[error("im proposal push bookkeeping failed: {0}")]
    PushBookkeeping(String),
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
    /// 提案卡按钮已落账 (`proposal accept/reject`; 一提案一决定, 恒不执行)。
    ProposalDecided {
        /// 落账记录 (`decided_by` 如实署名)。
        closure: ImProposalClosure,
        /// 回执段数。
        reply_segments: usize,
    },
    /// 提案卡按钮被拒即帧 (重复点击 / 查无 / 缺署名: 真因如实, 0 静默)。
    ProposalRefused {
        /// 拒绝的真实原因。
        reason: String,
        /// 回执段数。
        reply_segments: usize,
    },
    /// 设备审批卡按钮已批准/拒绝落账 (decided_by 如实署名; 批准即执行)。
    DeviceDecided {
        /// 落账记录 (批准 = 状态账记录 id 在场)。
        closure: ImDeviceClosure,
        /// 回执段数。
        reply_segments: usize,
    },
    /// 设备审批卡按钮被拒即帧 (重复点击 / 查无送达 / 缺署名: 真因如实, 0 静默)。
    DeviceRefused {
        /// 拒绝的真实原因。
        reason: String,
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
    /// 提案面接线 (提案卡出站 + 按钮落账; `None` = 未接线, 提案面 inert)。
    proposals: Option<Arc<ImProposalFace>>,
    /// 设备审批面接线 (设备审批卡出站 + 按钮批准/拒绝落账; `None` = 未接线,
    /// 设备面 inert)。
    device_approvals: Option<Arc<ImDeviceApprovalFace>>,
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
            proposals: None,
            device_approvals: None,
            segment_override: None,
            clock: Arc::new(SystemClock),
        }
    }

    /// 接入提案面 (自主度提案面接线: 提案卡出站 + `proposal accept/reject`
    /// 落账; 缺省 = 未接线, 提案按钮回调照实报「未接线」, 既有链零回归)。
    pub fn with_proposal_face(mut self, face: Arc<ImProposalFace>) -> Self {
        self.proposals = Some(face);
        self
    }

    /// 接入设备审批面 (设备审批卡→IM 推送链: 卡出站 + 按钮批准/拒绝落账;
    /// 缺省 = 未接线, 设备按钮回调照实报「未接线」, 既有链零回归)。
    pub fn with_device_approval_face(mut self, face: Arc<ImDeviceApprovalFace>) -> Self {
        self.device_approvals = Some(face);
        self
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
            // 提案入账 → 推送 (提案卡出站接线): 本回合落地的提案草稿随人审卡
            // 推到同一 IM 会话 (幂等 + 失败留账, IM 失败不丢提案)。
            if self.proposals.is_some() {
                self.push_proposal_cards(&target.id, &message.conversation_id)
                    .await?;
            }
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
        // 提案入账 → 推送 (提案卡出站接线, 同上)。
        if self.proposals.is_some() {
            self.push_proposal_cards(&target.id, &message.conversation_id)
                .await?;
        }
        Ok(ImInboundResult::Replied {
            session,
            segments: reply_segments,
            truncated: segmentation.truncated,
            dropped_chars: segmentation.dropped_chars,
        })
    }

    /// 卡片按钮 → 四态闭合 (审计配对原子) → 回执回流。
    ///
    /// 提案卡按钮 (subject = `tool.proposal_draft`) 走 [`crate::im_proposal`]:
    /// `proposal accept/reject` 落账 (一提案一决定), 不进审批四态闭合。
    pub async fn handle_card_action(
        &self,
        target: &ImChannelTarget,
        _kind: ImChannelKind,
        action: ImCardAction,
    ) -> Result<ImInboundResult, ImBridgeError> {
        if is_proposal_payload(&action.payload) {
            return self.handle_proposal_click(target, action).await;
        }
        if is_device_approval_payload(&action.payload) {
            return self.handle_device_click(target, action).await;
        }
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

    /// 提案卡按钮 → `proposal accept/reject` 落账 (decided_by = IM 用户 + 渠道
    /// 标注) → 回执回流。
    ///
    /// 一提案一决定: 重复点击走工具面的拒绝即帧 (真因如实); 提案不设超时,
    /// 什么时候点都按账面判 (一提案一决定只看提案账)。未接提案面 = 照实报
    /// 「未接线」(拒绝即帧, 不假装能落账)。
    async fn handle_proposal_click(
        &self,
        target: &ImChannelTarget,
        action: ImCardAction,
    ) -> Result<ImInboundResult, ImBridgeError> {
        let Some(face) = &self.proposals else {
            let reason = "提案按钮回调未接线: 本桥未配置提案面 (ImProposalFace), \
                          落账面缺位"
                .to_string();
            let reply_segments = self
                .deliver_segments(target, &action.conversation_id, &self.segment_text(&reason))
                .await?;
            return Ok(ImInboundResult::ProposalRefused {
                reason,
                reply_segments,
            });
        };
        match face
            .decider
            .decide_from_click(&target.id, &action.operator_id, &action.payload)
            .await
        {
            ImProposalClosureResult::Decided(closure) => {
                let reply_segments = self
                    .deliver_segments(
                        target,
                        &action.conversation_id,
                        &self.segment_text(&closure.reply_text),
                    )
                    .await?;
                Ok(ImInboundResult::ProposalDecided {
                    closure,
                    reply_segments,
                })
            }
            ImProposalClosureResult::Refused { reason } => {
                let reply_segments = self
                    .deliver_segments(target, &action.conversation_id, &self.segment_text(&reason))
                    .await?;
                Ok(ImInboundResult::ProposalRefused {
                    reason,
                    reply_segments,
                })
            }
        }
    }

    /// 设备审批卡按钮 → 设备动作批准/拒绝落账 (decided_by = IM 用户 + 渠道
    /// 标注) → 回执回流。
    ///
    /// 批准即执行 (与模型侧/面板侧同一台执行核心); 一计划一决定, 重复点击走
    /// 拒绝即帧 (真因如实); 查无送达记录的点击不可信, 拒绝即帧 (money 档无
    /// 送达回执 = 无 IM 卡)。未接设备审批面 = 照实报「未接线」(拒绝即帧,
    /// 不假装能落账)。
    async fn handle_device_click(
        &self,
        target: &ImChannelTarget,
        action: ImCardAction,
    ) -> Result<ImInboundResult, ImBridgeError> {
        let Some(face) = &self.device_approvals else {
            let reason = "设备审批卡按钮回调未接线: 本桥未配置设备审批面 \
                          (ImDeviceApprovalFace), 落账面缺位"
                .to_string();
            let reply_segments = self
                .deliver_segments(target, &action.conversation_id, &self.segment_text(&reason))
                .await?;
            return Ok(ImInboundResult::DeviceRefused {
                reason,
                reply_segments,
            });
        };
        match face.decide_from_click(&target.id, &action.operator_id, &action.payload) {
            ImDeviceClosureResult::Decided(closure) => {
                let reply_segments = self
                    .deliver_segments(
                        target,
                        &action.conversation_id,
                        &self.segment_text(&closure.reply_text),
                    )
                    .await?;
                Ok(ImInboundResult::DeviceDecided {
                    closure,
                    reply_segments,
                })
            }
            ImDeviceClosureResult::Refused { reason } => {
                let reply_segments = self
                    .deliver_segments(target, &action.conversation_id, &self.segment_text(&reason))
                    .await?;
                Ok(ImInboundResult::DeviceRefused {
                    reason,
                    reply_segments,
                })
            }
        }
    }

    /// 提案卡出站: 提案入账 → 人审卡 (**同链同信封**: 与 IM 审批卡同一条
    /// [`Self::deliver_card`] 出站链、同一个 [`ImApprovalCard`] 信封) → IM 会话。
    ///
    /// - **推送幂等键 = 提案 id**: 已推送 (推送账 `pushed` 行) 的不再推 ——
    ///   同提案不重复推送;
    /// - **失败留账**: 推送失败写 `failed` 行 (带真因) 待重试 —— IM 失败不丢
    ///   提案, 提案本体永远在提案账 (`proposals.jsonl`);
    /// - 未接提案面 = 空结果 (inert), 既有链零回归。
    ///
    /// 逐提案结果照实返回 (回执引用 / 失败真因), 0 静默降级。
    pub async fn push_proposal_cards(
        &self,
        channel_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<ProposalPushResult>, ImBridgeError> {
        let Some(face) = &self.proposals else {
            return Ok(Vec::new());
        };
        let target = self
            .channels
            .get(channel_id)
            .ok_or_else(|| ImBridgeError::UnknownChannel(channel_id.to_string()))?;
        let drafts = face
            .pending_drafts()
            .map_err(ImBridgeError::PushBookkeeping)?;
        let now_ms = Timestamp::from_clock(self.clock.as_ref()).epoch_millis();
        let mut results = Vec::new();
        for draft in drafts {
            let envelope = proposal_card_envelope(&draft);
            match self.deliver_card(target, conversation_id, &envelope).await {
                Ok(receipt) => {
                    face.record_push(&ProposalPushRecord {
                        proposal_id: draft.id.clone(),
                        channel_id: channel_id.to_string(),
                        conversation_id: conversation_id.to_string(),
                        outcome: ProposalPushOutcome::Pushed,
                        message_ref: Some(receipt.message_ref.clone()),
                        reason: None,
                        at_ms: now_ms,
                    })
                    .map_err(ImBridgeError::PushBookkeeping)?;
                    results.push(ProposalPushResult {
                        proposal_id: draft.id,
                        outcome: ProposalPushOutcome::Pushed,
                        detail: receipt.message_ref,
                    });
                }
                Err(error) => {
                    // 失败留账 (真因) 待重试: IM 失败不丢提案。
                    let reason = error.to_string();
                    face.record_push(&ProposalPushRecord {
                        proposal_id: draft.id.clone(),
                        channel_id: channel_id.to_string(),
                        conversation_id: conversation_id.to_string(),
                        outcome: ProposalPushOutcome::Failed,
                        message_ref: None,
                        reason: Some(reason.clone()),
                        at_ms: now_ms,
                    })
                    .map_err(ImBridgeError::PushBookkeeping)?;
                    results.push(ProposalPushResult {
                        proposal_id: draft.id,
                        outcome: ProposalPushOutcome::Failed,
                        detail: reason,
                    });
                }
            }
        }
        Ok(results)
    }

    /// 推一张设备审批卡并留推送账 (送达 = `pushed` 行 + 回执; 失败 = `failed` 行
    /// 带真因, 留账待重试)。留账失败必须报真因 ([`ImBridgeError::PushBookkeeping`]),
    /// 不静默。
    async fn push_one_device_card(
        &self,
        face: &ImDeviceApprovalFace,
        target: &ImChannelTarget,
        channel_id: &str,
        conversation_id: &str,
        card: &DeviceApprovalCard,
        now_ms: i64,
    ) -> Result<DeviceApprovalPushResult, ImBridgeError> {
        let envelope = device_card_envelope(card, now_ms);
        match self.deliver_card(target, conversation_id, &envelope).await {
            Ok(receipt) => {
                let message_ref = receipt.message_ref.clone();
                face.record_push(&DeviceApprovalPushRecord {
                    card: card.clone(),
                    channel_id: channel_id.to_string(),
                    conversation_id: conversation_id.to_string(),
                    outcome: DeviceApprovalPushOutcome::Pushed,
                    message_ref: Some(message_ref.clone()),
                    reason: None,
                    at_ms: now_ms,
                })
                .map_err(ImBridgeError::PushBookkeeping)?;
                Ok(DeviceApprovalPushResult {
                    plan_id: card.plan_id.clone(),
                    outcome: DeviceApprovalPushOutcome::Pushed,
                    detail: message_ref,
                })
            }
            Err(error) => {
                // 失败留账 (真因) 待重试: IM 失败不丢卡。
                let reason = error.to_string();
                face.record_push(&DeviceApprovalPushRecord {
                    card: card.clone(),
                    channel_id: channel_id.to_string(),
                    conversation_id: conversation_id.to_string(),
                    outcome: DeviceApprovalPushOutcome::Failed,
                    message_ref: None,
                    reason: Some(reason.clone()),
                    at_ms: now_ms,
                })
                .map_err(ImBridgeError::PushBookkeeping)?;
                Ok(DeviceApprovalPushResult {
                    plan_id: card.plan_id.clone(),
                    outcome: DeviceApprovalPushOutcome::Failed,
                    detail: reason,
                })
            }
        }
    }

    /// 设备审批卡出站: 卡载荷 → 动作计划卡 (**同链同信封**: 与 IM 审批卡同一条
    /// [`Self::deliver_card`] 出站链、同一个 [`ImApprovalCard`] 信封) → IM 会话。
    ///
    /// - **推送幂等键 = 动作 plan_id**: 已送达 (推送账 `pushed` 行) 的不再推 ——
    ///   同计划不重复推送 (结果集不含幂等跳过项);
    /// - **失败留账**: 推送失败写 `failed` 行 (带真因 + 卡载荷) 待重试
    ///   ([`Self::retry_device_approval_pushes`]) —— IM 失败不丢卡;
    /// - 未接设备审批面 = 空结果 (inert), 既有链零回归。
    ///
    /// 逐卡结果照实返回 (回执引用 / 失败真因), 0 静默降级。
    pub async fn push_device_approval_cards(
        &self,
        channel_id: &str,
        conversation_id: &str,
        cards: &[DeviceApprovalCard],
    ) -> Result<Vec<DeviceApprovalPushResult>, ImBridgeError> {
        let Some(face) = &self.device_approvals else {
            return Ok(Vec::new());
        };
        let target = self
            .channels
            .get(channel_id)
            .ok_or_else(|| ImBridgeError::UnknownChannel(channel_id.to_string()))?;
        let pushed = face
            .pushes
            .pushed_plan_ids()
            .map_err(ImBridgeError::PushBookkeeping)?;
        let now_ms = Timestamp::from_clock(self.clock.as_ref()).epoch_millis();
        let mut results = Vec::new();
        for card in cards {
            if pushed.contains(&card.plan_id) {
                continue;
            }
            let result = self
                .push_one_device_card(face, target, channel_id, conversation_id, card, now_ms)
                .await?;
            results.push(result);
        }
        Ok(results)
    }

    /// 设备审批卡重试: 失败留账的卡凭**推送账行内载荷**重建出站 (不需要外部
    /// 记忆, 重启后同账重试); 已送达的计划不进重试集 (幂等键挡)。渠道未配置
    /// 也照实留 `failed` 行 (真因), 不静默。
    pub async fn retry_device_approval_pushes(
        &self,
    ) -> Result<Vec<DeviceApprovalPushResult>, ImBridgeError> {
        let Some(face) = &self.device_approvals else {
            return Ok(Vec::new());
        };
        let pending = face
            .pushes
            .pending_retry()
            .map_err(ImBridgeError::PushBookkeeping)?;
        let now_ms = Timestamp::from_clock(self.clock.as_ref()).epoch_millis();
        let mut results = Vec::new();
        for record in pending {
            let Some(target) = self.channels.get(&record.channel_id) else {
                let reason = format!(
                    "设备审批卡重试失败: 渠道 {:?} 未配置 (留账待下次重试)",
                    record.channel_id
                );
                face.record_push(&DeviceApprovalPushRecord {
                    card: record.card.clone(),
                    channel_id: record.channel_id.clone(),
                    conversation_id: record.conversation_id.clone(),
                    outcome: DeviceApprovalPushOutcome::Failed,
                    message_ref: None,
                    reason: Some(reason.clone()),
                    at_ms: now_ms,
                })
                .map_err(ImBridgeError::PushBookkeeping)?;
                results.push(DeviceApprovalPushResult {
                    plan_id: record.card.plan_id.clone(),
                    outcome: DeviceApprovalPushOutcome::Failed,
                    detail: reason,
                });
                continue;
            };
            let result = self
                .push_one_device_card(
                    face,
                    target,
                    &record.channel_id,
                    &record.conversation_id,
                    &record.card,
                    now_ms,
                )
                .await?;
            results.push(result);
        }
        Ok(results)
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
