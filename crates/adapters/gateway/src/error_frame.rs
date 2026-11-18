//! Unified JSON error frame for every gateway error response.
//!
//! Every HTTP error the gateway produces uses one envelope:
//!
//! ```json
//! {"error": {"message": "<human>", "code": "<error_code>", "solution": "<action>"}}
//! ```
//!
//! The `message` field is the forward-compatible continuation of the legacy
//! flat `{"error": "..."}` string: clients that only read the human message can
//! switch from `error` to `error.message` without losing information. `code`
//! is the machine-readable contract and `solution` is an actionable Chinese
//! hint shown by the desktop companion.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Stable machine-readable error codes.
///
/// This catalog is closed: the frontend and the desktop settings surface key
/// their recovery UX off these exact strings, so new codes are a contract
/// change, not a local convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// No API key has been configured.
    AuthMissingKey,
    /// The configured API key was rejected by the provider.
    AuthInvalidKey,
    /// The upstream provider could not be reached.
    ProviderUnreachable,
    /// The upstream provider answered with an error.
    ProviderError,
    /// The request parameters were invalid.
    InvalidRequest,
    /// The authenticated user's role may not perform this operation (W3-② 角色门).
    PermissionDenied,
    /// 域级越权 (W3-① 多户数据隔离): 点名他域 / 面未按用户域开放 —— 帧恒定,
    /// 不泄露他域存在。
    DomainForbidden,
    /// 本域不可用 (W3-①): 域配置坏/拒开 —— 真因可报 (本域信息, 不涉他域存在性)。
    DomainRefused,
    /// The referenced session does not exist.
    SessionNotFound,
    /// The request was rate limited.
    RateLimited,
    /// A cognitive review module (judge/council) rejected the candidate.
    ReviewRejected,
    /// The turn hit the structural round limit without reaching an answer.
    TurnNotConverged,
    /// 工作区不存在或对调用方不可见 (A2 合同面): 帧恒定, 不泄露他域/他人存在性。
    WorkspaceNotFound,
    /// 工作区修订号冲突 (A2 合同面): 客户端带来的 `expected_revision` 与当前修订号不符。
    WorkspaceConflict,
    /// 工作区已吊销 (A2 合同面): 状态不可用, 不是并发冲突 —— 故不复用 conflict 码。
    WorkspaceRevoked,
    /// 联系人不存在或对调用方不可见 (B3a 合同面): 帧恒定, 不泄露他人存在性。
    ContactNotFound,
    /// 联系人修订号冲突 (B3a 合同面): 客户端带来的 `expected_revision` 与当前修订号不符。
    ContactConflict,
    /// 联系人已归档 (B3a 合同面): 终态 (软删除墓碑), 不是并发冲突 —— 故不复用 conflict 码。
    ContactArchived,
    /// 工作区绑定冲突 (B3a 合同面): 绑定的工作区不存在、已吊销或修订号不匹配。
    ContactBindingConflict,
    /// 会话绑定被拒 (B3b 合同面 / 回合闸): 会话的 agent 绑定已不再成立 ——
    /// 联系人缺失/已归档、工作区缺失/已吊销或修订号已变 —— 故不复用 conflict 码。
    AgentBindingRejected,
    /// 会话版本冲突 (B3b 合同面): 客户端带来的 `expected_revision` 与会话当前修订号不符。
    SessionBindingConflict,
    /// 操作幂等键载荷冲突 (C2-0 目录登记): 同一个操作键上已经落过**不同载荷**,
    /// 与「同键在飞」是两回事 —— 故不复用 `operation_in_progress`。
    /// /v1/chat 带 x-apeireth-operation 头时产生 (C2-1 起接线)。
    OperationConflict,
    /// 操作幂等键在飞 (C2-0 目录登记): 同一个操作键的请求还在处理中,
    /// 不得再起一次 —— 与「同键不同载荷」是两回事, 故不复用 `operation_conflict`。
    /// /v1/chat 带 x-apeireth-operation 头时产生 (C2-1 起接线)。
    OperationInProgress,
    /// An unexpected internal error.
    Internal,
}

impl ErrorCode {
    /// The wire representation used in the `code` field.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthMissingKey => "auth_missing_key",
            Self::AuthInvalidKey => "auth_invalid_key",
            Self::ProviderUnreachable => "provider_unreachable",
            Self::ProviderError => "provider_error",
            Self::InvalidRequest => "invalid_request",
            Self::PermissionDenied => "permission_denied",
            Self::DomainForbidden => "domain_forbidden",
            Self::DomainRefused => "domain_refused",
            Self::SessionNotFound => "session_not_found",
            Self::RateLimited => "rate_limited",
            Self::ReviewRejected => "review_rejected",
            Self::TurnNotConverged => "turn_not_converged",
            Self::WorkspaceNotFound => "workspace_not_found",
            Self::WorkspaceConflict => "workspace_conflict",
            Self::WorkspaceRevoked => "workspace_revoked",
            Self::ContactNotFound => "contact_not_found",
            Self::ContactConflict => "contact_conflict",
            Self::ContactArchived => "contact_archived",
            Self::ContactBindingConflict => "contact_binding_conflict",
            Self::AgentBindingRejected => "agent_binding_rejected",
            Self::SessionBindingConflict => "session_binding_conflict",
            Self::OperationConflict => "operation_conflict",
            Self::OperationInProgress => "operation_in_progress",
            Self::Internal => "internal",
        }
    }

    /// The actionable Chinese hint attached to this code.
    pub fn solution(self) -> &'static str {
        match self {
            // **两种凭据共用这两个码**（模型 provider 密钥 / 网关访问令牌），
            // 所以提示必须把两条路都说出来。此前只写「在设置→模型 保存 API 密钥」——
            // 网关令牌缺失时（401 `auth_missing_key`，message 明说
            // "gateway token required"）却让人去配模型密钥，是**错误建议**
            // （真机实测 2026-10-25）。
            Self::AuthMissingKey => {
                "缺凭据：网关访问令牌由部署者用 APEIRETH_GATEWAY_TOKEN 配置；模型密钥在「设置 → 模型」中保存"
            }
            Self::AuthInvalidKey => {
                "凭据无效或已撤销：网关令牌请核对 APEIRETH_GATEWAY_TOKEN；模型密钥请在「设置 → 模型」重新保存"
            }
            Self::ProviderUnreachable => "检查 base_url 与网络连通性后重试",
            Self::ProviderError => "查看 message 中的上游信息, 必要时联系服务商",
            Self::InvalidRequest => "检查参数后重试",
            // 同一个码被两个不同原因共用：①令牌没绑定用户（客户端身份未归属）
            // ②绑定了但角色不足。此前只讲角色 ⇒ 未绑定用户时给不出正确动作
            // （真机实测 2026-10-25：403 message 说 "not bound to any user"，
            // 提示却让人去找 owner 提权）。两条路都说出来。
            Self::PermissionDenied => {
                "若提示「未绑定用户」：用 apeireth user add --user-id <ID> --clients <令牌名> 绑定；若提示角色不足：需更高角色请 owner 调整用户表"
            }
            Self::DomainForbidden => "该操作不在本用户域的开放面内; 域面按用户域隔离, 越域访问一律拒绝",
            Self::DomainRefused => "本用户域的数据面未打开 (域配置可能损坏); 请联系 owner 检查 users/<id>/domain.json",
            Self::SessionNotFound => "回到会话列表重新选择",
            Self::RateLimited => "稍后重试或降低请求频率",
            Self::ReviewRejected => "回复被认知评审驳回——点「重新生成」再试; 若频繁出现, 在设置 → 高级能力中把认知深度调低",
            Self::TurnNotConverged => "本轮工具链过长未收敛——把问题拆小一点再问, 或点「重新生成」",
            // 工作区面（A2 合同面）：三个码各说一条可执行动作。
            Self::WorkspaceNotFound => "回到工作区列表重新读取; 若该工作区已被关闭或不属于你, 列表里不会再出现",
            Self::WorkspaceConflict => "工作区已被改动——重新读取列表拿到最新修订号, 再带着新的 expected_revision 提交",
            Self::WorkspaceRevoked => "工作区已关闭, 不能再选择或关闭; 请换一个可用工作区",
            // 联系人面 (B3a 合同面): 四个码各说一条可执行动作。
            Self::ContactNotFound => "回到联系人列表重新读取; 若该联系人已被归档或不属于你, 列表里不会再出现",
            Self::ContactConflict => "联系人已被改动——重新读取该联系人拿到最新修订号, 再带着新的 expected_revision 提交",
            Self::ContactArchived => "联系人已归档 (软删除), 不可修改也不可复活; 请新建一个联系人",
            Self::ContactBindingConflict => "绑定的工作区不存在、已吊销或修订号已变——重读工作区列表拿到最新修订号后再声明绑定",
            // 会话绑定面 (B3b 合同面 / 回合闸): 两个码各说一条可执行动作。
            Self::AgentBindingRejected => {
                "会话绑定的联系人/工作区已不可用——重新读取会话绑定, 换一位可用联系人或一个未吊销的工作区后再发起回合"
            }
            Self::SessionBindingConflict => "会话已被改动——重新读取会话拿到最新修订号, 再带着新的 expected_revision 提交",
            // 操作面 (C2-0 目录登记): 两个码各说一条可执行动作, 且彼此语义不混。
            Self::OperationConflict => {
                "同一操作键已经用另一份载荷提交过——按原载荷重试, 或换一个操作键; 不要拿旧键提交新载荷"
            }
            Self::OperationInProgress => {
                "同一操作键的请求还在处理中——等它结束后再重试, 不要重复提交"
            }
            Self::Internal => "重启 Companion 后重试, 若复现请提交日志",
        }
    }
}

/// One error frame: human message + machine code + actionable hint.
///
/// `details` carries the real budget numbers of a round-limit failure (limit,
/// consumed rounds, and the approval-frozen tool name when one existed), so a
/// `turn_not_converged` frame is diagnosable without a second lookup.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorFrame {
    pub message: String,
    pub code: ErrorCode,
    pub solution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorFrame {
    /// Build a frame for `code`; `solution` is derived from the code.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code,
            solution: code.solution().to_string(),
            details: None,
        }
    }

    /// Attach structured diagnostics to the frame.
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Build a complete HTTP error response (`{"error": {...}}`).
    pub fn response(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Response {
        (
            status,
            Json(ErrorEnvelope {
                error: Self::new(code, message),
            }),
        )
            .into_response()
    }
}

/// The HTTP body: `{"error": {...}}`.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorFrame,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// 目录清单：24 个变体，**顺序 = 枚举声明顺序 = `crate::error_codes` 常量目录顺序**。
    /// 长度由返回类型钉死（`[ErrorCode; 24]`）⇒ 少列/多列一个都编译失败。
    pub(crate) fn all_variants() -> [ErrorCode; 24] {
        [
            ErrorCode::AuthMissingKey,
            ErrorCode::AuthInvalidKey,
            ErrorCode::ProviderUnreachable,
            ErrorCode::ProviderError,
            ErrorCode::InvalidRequest,
            ErrorCode::PermissionDenied,
            ErrorCode::DomainForbidden,
            ErrorCode::DomainRefused,
            ErrorCode::SessionNotFound,
            ErrorCode::RateLimited,
            ErrorCode::ReviewRejected,
            ErrorCode::TurnNotConverged,
            ErrorCode::WorkspaceNotFound,
            ErrorCode::WorkspaceConflict,
            ErrorCode::WorkspaceRevoked,
            ErrorCode::ContactNotFound,
            ErrorCode::ContactConflict,
            ErrorCode::ContactArchived,
            ErrorCode::ContactBindingConflict,
            ErrorCode::AgentBindingRejected,
            ErrorCode::SessionBindingConflict,
            ErrorCode::OperationConflict,
            ErrorCode::OperationInProgress,
            ErrorCode::Internal,
        ]
    }

    /// 编译期漂移守卫：枚举里加了变体而忘了更新上面的清单 ⇒ **编译失败**
    /// （本 match 没有通配臂）。返回值 = 该变体在清单里的位置 ⇒
    /// [`every_variant_appears_once_in_catalog_order`] 拿它对账，顺序漂移也能抓。
    fn variant_index(code: ErrorCode) -> usize {
        match code {
            ErrorCode::AuthMissingKey => 0,
            ErrorCode::AuthInvalidKey => 1,
            ErrorCode::ProviderUnreachable => 2,
            ErrorCode::ProviderError => 3,
            ErrorCode::InvalidRequest => 4,
            ErrorCode::PermissionDenied => 5,
            ErrorCode::DomainForbidden => 6,
            ErrorCode::DomainRefused => 7,
            ErrorCode::SessionNotFound => 8,
            ErrorCode::RateLimited => 9,
            ErrorCode::ReviewRejected => 10,
            ErrorCode::TurnNotConverged => 11,
            ErrorCode::WorkspaceNotFound => 12,
            ErrorCode::WorkspaceConflict => 13,
            ErrorCode::WorkspaceRevoked => 14,
            ErrorCode::ContactNotFound => 15,
            ErrorCode::ContactConflict => 16,
            ErrorCode::ContactArchived => 17,
            ErrorCode::ContactBindingConflict => 18,
            ErrorCode::AgentBindingRejected => 19,
            ErrorCode::SessionBindingConflict => 20,
            ErrorCode::OperationConflict => 21,
            ErrorCode::OperationInProgress => 22,
            ErrorCode::Internal => 23,
        }
    }

    /// 清单 ⇄ 守卫下标 ⇄ 枚举三方对账：清单第 i 位必须正好是守卫认第 i 位的变体
    /// ⇒ 漏项会撞长度/下标, 重项会让某一位置对不上, 顺序漂移同样被抓。
    #[test]
    fn every_variant_appears_once_in_catalog_order() {
        let variants = all_variants();
        assert_eq!(variants.len(), 24, "目录清单必须是 24 个变体");
        for (index, code) in variants.iter().enumerate() {
            assert_eq!(
                variant_index(*code),
                index,
                "{code:?} 在清单里的位置与枚举声明顺序不一致"
            );
        }
    }

    #[test]
    fn frame_serializes_message_code_and_solution_in_contract_order() {
        let body = serde_json::to_value(ErrorEnvelope {
            error: ErrorFrame::new(ErrorCode::AuthMissingKey, "no key configured"),
        })
        .unwrap();

        assert_eq!(body["error"]["message"], "no key configured");
        assert_eq!(body["error"]["code"], "auth_missing_key");
        assert_eq!(
            body["error"]["solution"],
            "缺凭据：网关访问令牌由部署者用 APEIRETH_GATEWAY_TOKEN 配置；模型密钥在「设置 → 模型」中保存"
        );
        // The contract is a stable wire shape; assert the exact JSON.
        assert_eq!(
            serde_json::to_string(&ErrorEnvelope {
                error: ErrorFrame::new(ErrorCode::InvalidRequest, "bad input")
            })
            .unwrap(),
            r#"{"error":{"message":"bad input","code":"invalid_request","solution":"检查参数后重试"}}"#
        );
    }

    #[test]
    fn every_catalog_code_has_a_nonempty_solution() {
        for code in all_variants() {
            assert!(!code.solution().is_empty(), "{code:?}");
            assert!(!code.as_str().is_empty(), "{code:?}");
        }
    }

    /// 枚举 ⇄ 常量目录的双向对齐：每个变体的线串必须能被
    /// [`crate::error_codes::parse_code`] 反解回**同一个**变体；
    /// 且常量目录必须与枚举一一对应（数量相同、顺序相同 ⇒ 不错位）。
    #[test]
    fn every_variant_round_trips_through_the_constant_catalog() {
        let consts = [
            crate::error_codes::AUTH_MISSING_KEY,
            crate::error_codes::AUTH_INVALID_KEY,
            crate::error_codes::PROVIDER_UNREACHABLE,
            crate::error_codes::PROVIDER_ERROR,
            crate::error_codes::INVALID_REQUEST,
            crate::error_codes::PERMISSION_DENIED,
            crate::error_codes::DOMAIN_FORBIDDEN,
            crate::error_codes::DOMAIN_REFUSED,
            crate::error_codes::SESSION_NOT_FOUND,
            crate::error_codes::RATE_LIMITED,
            crate::error_codes::REVIEW_REJECTED,
            crate::error_codes::TURN_NOT_CONVERGED,
            crate::error_codes::WORKSPACE_NOT_FOUND,
            crate::error_codes::WORKSPACE_CONFLICT,
            crate::error_codes::WORKSPACE_REVOKED,
            crate::error_codes::CONTACT_NOT_FOUND,
            crate::error_codes::CONTACT_CONFLICT,
            crate::error_codes::CONTACT_ARCHIVED,
            crate::error_codes::CONTACT_BINDING_CONFLICT,
            crate::error_codes::AGENT_BINDING_REJECTED,
            crate::error_codes::SESSION_BINDING_CONFLICT,
            crate::error_codes::OPERATION_CONFLICT,
            crate::error_codes::OPERATION_IN_PROGRESS,
            crate::error_codes::INTERNAL,
        ];
        let variants = all_variants();
        assert_eq!(consts.len(), variants.len(), "常量目录与枚举数量不一致");

        for (code, wire) in variants.iter().zip(consts.iter()) {
            assert_eq!(code.as_str(), *wire, "{code:?} 的线串与常量目录错位");
            assert_eq!(
                crate::error_codes::parse_code(wire),
                Some(*code),
                "{wire} 无法反解回同一变体"
            );
        }
    }
}
