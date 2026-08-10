//! R23 device_code grant_type — RFC 8628 OAuth 2.0 Device Authorization Grant。
//!
//! **8 项承诺**: 全部遵守。**不假装**: skeleton 阶段本地内存状态机，0 引 HTTP。
//! **不修改承诺 (LOCKED)**: 0 改 FlowStep enum 骨；0 改 workspace.version；0 改 LOCKED 文档。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Device-code grant_type 错误 (5 K-1 强校验对应变体).
#[derive(Debug, Error)]
pub enum DeviceCodeError {
    /// client_id 为空字符串 (RFC 8628 §3.1 必须).
    #[error("device_code: client_id {0} is empty")]
    EmptyClientId(String),
    /// scope 列表为空.
    #[error("device_code: scope is empty")]
    EmptyScope,
    /// poll interval 必须 > 0 秒 (RFC 8628 §3.5).
    #[error("device_code: interval {0} must be > 0")]
    NonPositiveInterval(u64),
    /// 状态机非法迁移 (skeleton 阶段 4 步顺序严格).
    #[error("device_code: state transition invalid: {from} -> {to}")]
    InvalidTransition {
        /// 当前 step.
        from: String,
        /// 试图前往的 step.
        to: String,
    },
}

/// Device-code 统一结果类型.
pub type DeviceCodeResult<T> = Result<T, DeviceCodeError>;

/// 4 步 device_code 流程 (per RFC 8628 + 借鉴 Golutra).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceCodeStep {
    /// 0: request_code — 客户端向设备端点请求 device_code + user_code.
    RequestCode,
    /// 1: display_user_code — 展示 user_code + verification_uri 给用户.
    DisplayUserCode,
    /// 2: poll_token — 按 interval 轮询 token 端点.
    PollToken,
    /// 3: complete — 拿到 access_token，结束.
    Complete,
}

impl DeviceCodeStep {
    /// 4 步常量.
    pub const COUNT: usize = 4;
    /// 4 步静态数组 (顺序与 RFC 8628 §3 一致).
    pub const ALL: &'static [DeviceCodeStep] = &[
        DeviceCodeStep::RequestCode,
        DeviceCodeStep::DisplayUserCode,
        DeviceCodeStep::PollToken,
        DeviceCodeStep::Complete,
    ];
    /// step → stable string (per FlowStep enum 命名一致).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequestCode => "request_code",
            Self::DisplayUserCode => "display_user_code",
            Self::PollToken => "poll_token",
            Self::Complete => "complete",
        }
    }
}

/// 设备码响应 (per RFC 8628 §3.2, skeleton 阶段本地生成).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// 设备码 (客户端 secret, 不暴露给用户).
    pub device_code: String,
    /// 用户码 (用户输入到 verification_uri).
    pub user_code: String,
    /// 用户应访问的 URL.
    pub verification_uri: String,
    /// 完成验证的 URI (可选, RFC 8628 §3.3).
    pub verification_uri_complete: Option<String>,
    /// 过期秒数.
    pub expires_in: u64,
    /// 轮询间隔秒数.
    pub interval: u64,
}

/// 本地内存 device_code 会话状态机.
#[derive(Debug, Clone)]
pub struct DeviceCodeSession {
    /// OAuth client_id.
    pub client_id: String,
    /// 申请的 scope 列表.
    pub scope: Vec<String>,
    /// 当前 device_code 响应 (issue_code 后填充).
    pub response: Option<DeviceCodeResponse>,
    /// 当前 step (新会话 = RequestCode).
    pub current_step: DeviceCodeStep,
    /// poll 轮次计数 (调试 + 指数退避基线).
    pub poll_count: u32,
}

impl DeviceCodeSession {
    /// 新建会话 (per 5 K-1 强校验: client_id 非空 + scope 非空).
    pub fn new(client_id: impl Into<String>, scope: Vec<String>) -> DeviceCodeResult<Self> {
        let id = client_id.into();
        if id.trim().is_empty() {
            return Err(DeviceCodeError::EmptyClientId(id));
        }
        if scope.is_empty() {
            return Err(DeviceCodeError::EmptyScope);
        }
        Ok(Self { client_id: id, scope, response: None, current_step: DeviceCodeStep::RequestCode, poll_count: 0 })
    }

    /// 步骤 0→1: 模拟服务端响应 (skeleton 阶段本地生成).
    pub fn issue_code(&mut self, interval: u64) -> DeviceCodeResult<DeviceCodeResponse> {
        if interval == 0 {
            return Err(DeviceCodeError::NonPositiveInterval(interval));
        }
        if self.current_step != DeviceCodeStep::RequestCode {
            return Err(DeviceCodeError::InvalidTransition {
                from: self.current_step.as_str().to_string(),
                to: DeviceCodeStep::DisplayUserCode.as_str().to_string(),
            });
        }
        let response = DeviceCodeResponse {
            device_code: format!("dev_{}", self.client_id),
            user_code: format!("USER-{}", self.poll_count + 1),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: None,
            expires_in: 600,
            interval,
        };
        self.response = Some(response.clone());
        self.current_step = DeviceCodeStep::DisplayUserCode;
        Ok(response)
    }

    /// R44: 步骤 0->1 真接 HTTP server response (RFC 8628 §3.2 DeviceCodeResponse).
    /// 跟 issue_code 1:1 镜像, 唯一区别: 用 server 返回的 device_code/user_code/verification_uri/interval
    /// 代替本地生成. 0 改动 4 步状态机骨.
    pub fn issue_code_from_http(
        &mut self,
        device_code: String,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        expires_in: u64,
        interval: u64,
    ) -> DeviceCodeResult<DeviceCodeResponse> {
        if self.current_step != DeviceCodeStep::RequestCode {
            return Err(DeviceCodeError::InvalidTransition {
                from: self.current_step.as_str().to_string(),
                to: DeviceCodeStep::DisplayUserCode.as_str().to_string(),
            });
        }
        let response = DeviceCodeResponse {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete,
            expires_in,
            interval,
        };
        self.response = Some(response.clone());
        self.current_step = DeviceCodeStep::DisplayUserCode;
        Ok(response)
    }

    /// 步骤 1→2: 用户已输入 user_code，开始轮询.
    pub fn user_submitted(&mut self) -> DeviceCodeResult<()> {
        if self.current_step != DeviceCodeStep::DisplayUserCode {
            return Err(DeviceCodeError::InvalidTransition {
                from: self.current_step.as_str().to_string(),
                to: DeviceCodeStep::PollToken.as_str().to_string(),
            });
        }
        self.current_step = DeviceCodeStep::PollToken;
        Ok(())
    }

    /// 步骤 2 内: 每轮 poll 自增计数（不前进到 Complete）.
    pub fn poll(&mut self) -> DeviceCodeResult<u32> {
        if self.current_step != DeviceCodeStep::PollToken {
            return Err(DeviceCodeError::InvalidTransition {
                from: self.current_step.as_str().to_string(),
                to: DeviceCodeStep::PollToken.as_str().to_string(),
            });
        }
        self.poll_count += 1;
        Ok(self.poll_count)
    }

    /// 步骤 2→3: 服务端确认 device_code，发放 access_token.
    pub fn complete(&mut self) -> DeviceCodeResult<String> {
        if self.current_step != DeviceCodeStep::PollToken {
            return Err(DeviceCodeError::InvalidTransition {
                from: self.current_step.as_str().to_string(),
                to: DeviceCodeStep::Complete.as_str().to_string(),
            });
        }
        let token = format!("access_token_for_{}", self.client_id);
        self.current_step = DeviceCodeStep::Complete;
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_count_is_4() {
        assert_eq!(DeviceCodeStep::COUNT, 4);
        assert_eq!(DeviceCodeStep::ALL.len(), 4);
    }

    #[test]
    fn step_strings_are_stable() {
        assert_eq!(DeviceCodeStep::RequestCode.as_str(), "request_code");
        assert_eq!(DeviceCodeStep::Complete.as_str(), "complete");
    }

    #[test]
    fn empty_client_id_rejected() {
        assert!(DeviceCodeSession::new("", vec!["read".into()]).is_err());
    }

    #[test]
    fn empty_scope_rejected() {
        assert!(DeviceCodeSession::new("cid", vec![]).is_err());
    }

    #[test]
    fn full_flow_request_display_poll_complete() {
        let mut s = DeviceCodeSession::new("cid", vec!["read".into()]).unwrap();
        let resp = s.issue_code(5).unwrap();
        assert_eq!(resp.interval, 5);
        s.user_submitted().unwrap();
        assert_eq!(s.poll().unwrap(), 1);
        assert_eq!(s.poll().unwrap(), 2);
        let token = s.complete().unwrap();
        assert!(token.starts_with("access_token_for_"));
    }

    #[test]
    fn invalid_transition_returns_error() {
        let mut s = DeviceCodeSession::new("cid", vec!["read".into()]).unwrap();
        let err = s.poll().unwrap_err();
        assert!(matches!(err, DeviceCodeError::InvalidTransition { .. }));
    }

    #[test]
    fn zero_interval_rejected() {
        let mut s = DeviceCodeSession::new("cid", vec!["read".into()]).unwrap();
        assert!(s.issue_code(0).is_err());
    }
}
