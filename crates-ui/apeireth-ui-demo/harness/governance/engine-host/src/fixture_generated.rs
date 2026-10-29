//! 固定数据（**自动生成** —— 勿手改）。
//!
// 来源：`crates-ui/apeireth-ui-demo/harness/governance/fixtures/governance-cases.json`
// 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/extract-governance-assets.mjs`
//!
// ⚠️ 时刻一律以**相对基准时刻的偏移**（毫秒）保存 ⇒ 运行期由宿主换算成
//    RFC 3339 文本（参考侧与引擎侧的基准时刻由对照 harness 统一注入）。

use apeireth_ui_kit::governance_tab::{Capability, CapabilityGroup, CapabilityManifest};

/// 会话（固定数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSession {
    /// 会话 id。
    pub id: String,
    /// 会话标题（空 ⇒ `None`）。
    pub title: Option<String>,
}

/// 待签文书（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureApproval {
    /// 文书 id。
    pub approval_id: String,
    /// 所属会话 id。
    pub session: String,
    /// 工具名。
    pub tool_name: String,
    /// 治理理由（空串 ⇒ 缺省文案）。
    pub governance_reason: String,
    /// 命令文本。
    pub command_text: Option<String>,
    /// 参数摘要。
    pub arguments_summary: Option<String>,
    /// 创建时刻偏移（毫秒；`0` ⇒ 无）。
    pub created_offset_ms: i64,
    /// 过期时刻偏移（毫秒；`0` ⇒ 无）。
    pub expires_offset_ms: i64,
}

/// 授权（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureGrant {
    /// 授权 id。
    pub id: String,
    /// 授权名。
    pub name: String,
    /// 入账时刻偏移（毫秒；`0` ⇒ 未记录）。
    pub activated_offset_ms: i64,
}

/// span（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSpan {
    /// span id。
    pub span_id: String,
    /// 所属轨迹 id。
    pub trace_id: String,
    /// 父 span id。
    pub parent_span_id: Option<String>,
    /// kind。
    pub kind: String,
    /// actor。
    pub actor: String,
    /// status。
    pub status: String,
    /// 摘要。
    pub summary: Option<String>,
    /// 开始时刻偏移（毫秒）。
    pub started_offset_ms: i64,
    /// 结束时刻偏移（毫秒；`None` ⇒ 进行中）。
    pub ended_offset_ms: Option<i64>,
    /// 会话 id。
    pub session_id: Option<String>,
}

/// 轨迹（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureTrace {
    /// 轨迹 id。
    pub trace_id: String,
    /// span 数。
    pub span_count: i64,
    /// 开始时刻偏移（毫秒）。
    pub started_offset_ms: i64,
    /// 根 span。
    pub root: FixtureSpan,
}

/// 事件留痕（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureAuditEvent {
    /// 时刻偏移（毫秒）。
    pub ts_offset_ms: i64,
    /// 事件名。
    pub event: String,
    /// 服务名。
    pub service: String,
    /// 明细。
    pub detail: Option<String>,
}

/// 守卫状态（固定数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureGuardStatus {
    /// 守卫是否启用。
    pub enabled: bool,
    /// 快速守卫是否装配。
    pub fast_guard_active: bool,
    /// 链式守卫是否装配。
    pub chain_guard_active: bool,
    /// 意图守卫是否装配。
    pub intent_guard_active: bool,
    /// 跨轮监测是否装配。
    pub cross_turn_monitoring_active: bool,
    /// 评估总数。
    pub total_evaluations: i64,
    /// 放行数。
    pub total_allowed: i64,
    /// 否决数。
    pub total_denied: i64,
    /// 转人工数。
    pub total_approval_required: i64,
    /// ML 分类器是否可用。
    pub ml_classifier_available: bool,
    /// ML 模型版本。
    pub ml_model_version: Option<String>,
}

/// 守卫事件（固定数据；时刻为相对基准的偏移）。
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureGuardEvent {
    /// 时刻偏移（毫秒）。
    pub timestamp_offset_ms: i64,
    /// 会话 id。
    pub session_id: String,
    /// 轨迹 id。
    pub trace_id: String,
    /// 轮次。
    pub round: i64,
    /// 能力 id。
    pub capability_id: String,
    /// 守卫阶段。
    pub stage: String,
    /// 决策。
    pub decision: String,
    /// 风险分。
    pub risk_score: f64,
    /// 理由。
    pub reasons: Vec<String>,
    /// 证据。
    pub evidence: Vec<String>,
}

/// 对照用例（固定数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureCase {
    /// 用例 id。
    pub id: String,
    /// 能力清单键（`None` ⇒ 清单未到）。
    pub manifest: Option<String>,
    /// 数据行为（`default` / `empty` / `error` / `loading`）。
    pub data: String,
    /// 动作序列（DOM 点击）。
    pub actions: Vec<FixtureAction>,
}

/// 用例动作（**DOM 点击** ⇒ 两侧同名同义）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureAction {
    /// 选择器。
    pub click: String,
    /// 第几个匹配项。
    pub index: usize,
}

/// 全部能力清单（键 ⇒ 清单）。
pub fn manifests() -> Vec<(&'static str, CapabilityManifest)> {
    vec![
        (
            "full",
            CapabilityManifest {
                capabilities: vec![
                    CapabilityGroup {
                        capabilities: vec![
                            Capability {
                                id: String::from("permissions.approval.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("permissions.approval.resolve"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("permissions.grants.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("permissions.revoke"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                        ],
                    },
                    CapabilityGroup {
                        capabilities: vec![Capability {
                            id: String::from("sessions.read"),
                            supported: true,
                            available: Some(true),
                            reason: None,
                            alias_of: None,
                        }],
                    },
                    CapabilityGroup {
                        capabilities: vec![Capability {
                            id: String::from("activity.sse"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        }],
                    },
                    CapabilityGroup {
                        capabilities: vec![
                            Capability {
                                id: String::from("trace.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("audit.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                        ],
                    },
                    CapabilityGroup {
                        capabilities: vec![
                            Capability {
                                id: String::from("safety.guard.status.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("safety.guard.events.read"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                            Capability {
                                id: String::from("safety.guard.evaluate"),
                                supported: true,
                                available: Some(true),
                                reason: None,
                                alias_of: None,
                            },
                        ],
                    },
                ],
            },
        ),
        (
            "no_approval_read",
            CapabilityManifest {
                capabilities: vec![CapabilityGroup {
                    capabilities: vec![
                        Capability {
                            id: String::from("permissions.approval.read"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                        Capability {
                            id: String::from("permissions.approval.resolve"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                    ],
                }],
            },
        ),
        (
            "grants_read_only",
            CapabilityManifest {
                capabilities: vec![CapabilityGroup {
                    capabilities: vec![
                        Capability {
                            id: String::from("permissions.grants.read"),
                            supported: true,
                            available: Some(true),
                            reason: None,
                            alias_of: None,
                        },
                        Capability {
                            id: String::from("permissions.revoke"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                    ],
                }],
            },
        ),
        (
            "audit_unavailable",
            CapabilityManifest {
                capabilities: vec![CapabilityGroup {
                    capabilities: vec![
                        Capability {
                            id: String::from("trace.read"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                        Capability {
                            id: String::from("audit.read"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                    ],
                }],
            },
        ),
        (
            "guard_unavailable",
            CapabilityManifest {
                capabilities: vec![CapabilityGroup {
                    capabilities: vec![
                        Capability {
                            id: String::from("safety.guard.status.read"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                        Capability {
                            id: String::from("safety.guard.events.read"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                        Capability {
                            id: String::from("safety.guard.evaluate"),
                            supported: false,
                            available: Some(false),
                            reason: Some(String::from("not_implemented")),
                            alias_of: None,
                        },
                    ],
                }],
            },
        ),
    ]
}

/// 会话清单。
pub fn sessions() -> Vec<FixtureSession> {
    vec![
        FixtureSession {
            id: String::from("sess-alpha"),
            title: Some(String::from("主线程 · 装配")),
        },
        FixtureSession {
            id: String::from("sess-beta"),
            title: None,
        },
    ]
}

/// 某会话的待签 inbox。
pub fn inbox(session_id: &str) -> Vec<FixtureApproval> {
    let items: &[FixtureApproval] = match session_id {
        "sess-alpha" => &[
            FixtureApproval {
                approval_id: String::from("ap-0001"),
                session: String::from("sess-alpha"),
                tool_name: String::from("fs.write"),
                governance_reason: String::from("写工作区文件需要主人点头"),
                command_text: Some(String::from("fs.write(\"notes.md\", \"…\")")),
                arguments_summary: Some(String::from("path=notes.md, bytes=128")),
                created_offset_ms: -180000,
                expires_offset_ms: 900000,
            },
            FixtureApproval {
                approval_id: String::from("ap-0002"),
                session: String::from("sess-alpha"),
                tool_name: String::from("shell.exec"),
                governance_reason: String::from(""),
                command_text: None,
                arguments_summary: Some(String::from("cmd=git status")),
                created_offset_ms: -60000,
                expires_offset_ms: 0,
            },
        ],
        "sess-beta" => &[FixtureApproval {
            approval_id: String::from("ap-0003"),
            session: String::from("sess-beta"),
            tool_name: String::from("net.fetch"),
            governance_reason: String::from("出网请求需要放行"),
            command_text: Some(String::from("net.fetch(\"https://example.invalid/\")")),
            arguments_summary: Some(String::from("net.fetch(\"https://example.invalid/\")")),
            created_offset_ms: -3600000,
            expires_offset_ms: -600000,
        }],
        _ => &[],
    };
    items.to_vec()
}

/// 授权清单。
pub fn grants() -> Vec<FixtureGrant> {
    vec![
        FixtureGrant {
            id: String::from("tool.repo.write"),
            name: String::from("tool.repo.write"),
            activated_offset_ms: -3600000,
        },
        FixtureGrant {
            id: String::from("memory.read"),
            name: String::from("memory.read"),
            activated_offset_ms: 0,
        },
    ]
}

/// 轨迹清单。
pub fn traces() -> Vec<FixtureTrace> {
    vec![
        FixtureTrace {
            trace_id: String::from("trace-0001"),
            span_count: 3,
            started_offset_ms: -300000,
            root: FixtureSpan {
                span_id: String::from("span-root-1"),
                trace_id: String::from("trace-0001"),
                parent_span_id: None,
                kind: String::from("turn"),
                actor: String::from("assistant"),
                status: String::from("ok"),
                summary: Some(String::from("装配工作区")),
                started_offset_ms: -300000,
                ended_offset_ms: Some(-290000),
                session_id: Some(String::from("sess-alpha")),
            },
        },
        FixtureTrace {
            trace_id: String::from("trace-0002"),
            span_count: 2,
            started_offset_ms: -900000,
            root: FixtureSpan {
                span_id: String::from("span-root-2"),
                trace_id: String::from("trace-0002"),
                parent_span_id: None,
                kind: String::from("tool"),
                actor: String::from("shell"),
                status: String::from("error"),
                summary: None,
                started_offset_ms: -900000,
                ended_offset_ms: Some(-895000),
                session_id: Some(String::from("sess-beta")),
            },
        },
    ]
}

/// 某轨迹的 span 树。
pub fn spans(trace_id: &str) -> Vec<FixtureSpan> {
    let items: &[FixtureSpan] = match trace_id {
        "trace-0001" => &[
            FixtureSpan {
                span_id: String::from("span-root-1"),
                trace_id: String::from("trace-0001"),
                parent_span_id: None,
                kind: String::from("turn"),
                actor: String::from("assistant"),
                status: String::from("ok"),
                summary: Some(String::from("装配工作区")),
                started_offset_ms: -300000,
                ended_offset_ms: Some(-290000),
                session_id: Some(String::from("sess-alpha")),
            },
            FixtureSpan {
                span_id: String::from("span-child-1"),
                trace_id: String::from("trace-0001"),
                parent_span_id: Some(String::from("span-root-1")),
                kind: String::from("provider"),
                actor: String::from("provider"),
                status: String::from("ok"),
                summary: None,
                started_offset_ms: -299000,
                ended_offset_ms: Some(-295000),
                session_id: Some(String::from("sess-alpha")),
            },
            FixtureSpan {
                span_id: String::from("span-child-2"),
                trace_id: String::from("trace-0001"),
                parent_span_id: Some(String::from("span-child-1")),
                kind: String::from("tool"),
                actor: String::from("fs.write"),
                status: String::from("ok"),
                summary: Some(String::from("写入 notes.md")),
                started_offset_ms: -294000,
                ended_offset_ms: Some(-290000),
                session_id: Some(String::from("sess-alpha")),
            },
        ],
        "trace-0002" => &[FixtureSpan {
            span_id: String::from("span-root-2"),
            trace_id: String::from("trace-0002"),
            parent_span_id: None,
            kind: String::from("tool"),
            actor: String::from("shell"),
            status: String::from("error"),
            summary: Some(String::from("退出码 1")),
            started_offset_ms: -900000,
            ended_offset_ms: Some(-895000),
            session_id: Some(String::from("sess-beta")),
        }],
        _ => &[],
    };
    items.to_vec()
}

/// 事件留痕。
pub fn audit_events() -> Vec<FixtureAuditEvent> {
    vec![
        FixtureAuditEvent {
            ts_offset_ms: -120000,
            event: String::from("chat.turn.completed"),
            service: String::from("gateway"),
            detail: Some(String::from("回合完成")),
        },
        FixtureAuditEvent {
            ts_offset_ms: -180000,
            event: String::from("governance.approval.resolved"),
            service: String::from("gateway"),
            detail: Some(String::from("审批落笔")),
        },
        FixtureAuditEvent {
            ts_offset_ms: -240000,
            event: String::from("memory.write"),
            service: String::from("gateway"),
            detail: Some(String::from("记忆变动")),
        },
    ]
}

/// 守卫状态。
pub fn guard_status() -> FixtureGuardStatus {
    FixtureGuardStatus {
        enabled: true,
        fast_guard_active: true,
        chain_guard_active: true,
        intent_guard_active: true,
        cross_turn_monitoring_active: false,
        total_evaluations: 128,
        total_allowed: 96,
        total_denied: 12,
        total_approval_required: 20,
        ml_classifier_available: true,
        ml_model_version: Some(String::from("m3")),
    }
}

/// 守卫事件。
pub fn guard_events() -> Vec<FixtureGuardEvent> {
    vec![
        FixtureGuardEvent {
            timestamp_offset_ms: -60000,
            session_id: String::from("sess-alpha-0123456789"),
            trace_id: String::from("trace-0001"),
            round: 3,
            capability_id: String::from("tool.repo.write"),
            stage: String::from("chain_guard"),
            decision: String::from("approval_required"),
            risk_score: 0.72,
            reasons: vec![
                String::from("写操作需要主人点头"),
                String::from("路径在工作区内"),
            ],
            evidence: vec![String::from("policy:repo.write"), String::from("chain:2")],
        },
        FixtureGuardEvent {
            timestamp_offset_ms: -120000,
            session_id: String::from("sess-beta-9876543210"),
            trace_id: String::from("trace-0002"),
            round: 1,
            capability_id: String::from("shell.exec"),
            stage: String::from("fast_guard"),
            decision: String::from("allow"),
            risk_score: 0.12,
            reasons: vec![],
            evidence: vec![],
        },
    ]
}

/// approvals 面的对照用例。
pub fn cases_approvals() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            id: String::from("approvals-ledger"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("approvals-open"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".gov-toolbar .gov-chips button"),
                index: 1,
            }],
        },
        FixtureCase {
            id: String::from("approvals-expired"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".gov-toolbar .gov-chips button"),
                index: 2,
            }],
        },
        FixtureCase {
            id: String::from("approvals-empty"),
            manifest: Some(String::from("full")),
            data: String::from("empty"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("approvals-error"),
            manifest: Some(String::from("full")),
            data: String::from("error"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("approvals-loading"),
            manifest: Some(String::from("full")),
            data: String::from("loading"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("approvals-unsupported"),
            manifest: Some(String::from("no_approval_read")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("approvals-pending"),
            manifest: None,
            data: String::from("default"),
            actions: vec![],
        },
    ]
}

/// grants 面的对照用例。
pub fn cases_grants() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            id: String::from("grants-list"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("grants-confirm"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".gov-list .btn-revoke"),
                index: 0,
            }],
        },
        FixtureCase {
            id: String::from("grants-empty"),
            manifest: Some(String::from("full")),
            data: String::from("empty"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("grants-error"),
            manifest: Some(String::from("full")),
            data: String::from("error"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("grants-read-only"),
            manifest: Some(String::from("grants_read_only")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("grants-pending"),
            manifest: None,
            data: String::from("default"),
            actions: vec![],
        },
    ]
}

/// audit 面的对照用例。
pub fn cases_audit() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            id: String::from("audit-traces"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("audit-trace-detail"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".md-master .md-row"),
                index: 0,
            }],
        },
        FixtureCase {
            id: String::from("audit-events"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".gov-toolbar .gov-chips button"),
                index: 1,
            }],
        },
        FixtureCase {
            id: String::from("audit-event-detail"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![
                FixtureAction {
                    click: String::from(".gov-toolbar .gov-chips button"),
                    index: 1,
                },
                FixtureAction {
                    click: String::from(".md-master .md-row"),
                    index: 0,
                },
            ],
        },
        FixtureCase {
            id: String::from("audit-error"),
            manifest: Some(String::from("full")),
            data: String::from("error"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("audit-unsupported"),
            manifest: Some(String::from("audit_unavailable")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("audit-pending"),
            manifest: None,
            data: String::from("default"),
            actions: vec![],
        },
    ]
}

/// guard 面的对照用例。
pub fn cases_guard() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            id: String::from("guard-status"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("guard-event-expanded"),
            manifest: Some(String::from("full")),
            data: String::from("default"),
            actions: vec![FixtureAction {
                click: String::from(".gov-list .ev-expand"),
                index: 0,
            }],
        },
        FixtureCase {
            id: String::from("guard-empty"),
            manifest: Some(String::from("full")),
            data: String::from("empty"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("guard-error"),
            manifest: Some(String::from("full")),
            data: String::from("error"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("guard-unsupported"),
            manifest: Some(String::from("guard_unavailable")),
            data: String::from("default"),
            actions: vec![],
        },
        FixtureCase {
            id: String::from("guard-pending"),
            manifest: None,
            data: String::from("default"),
            actions: vec![],
        },
    ]
}

/// 错误文案（固定数据）。
pub fn error_text() -> String {
    String::from("网关不可达（对照固定数据）")
}
