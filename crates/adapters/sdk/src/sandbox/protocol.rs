//! # Sandbox wire protocol — 协议编解码
//!
//! 客户端 ↔ 外部编排服务之间的请求/响应帧编解码。本模块只做**纯编解码 + 校验**:
//!
//! - 帧结构: `schema_version` + `request_id` + body (op 标签联合)。
//! - schema 版本不符一律 [`SandboxError::Protocol`](crate::sandbox::SandboxError::Protocol),
//!   不做静默兼容猜测。
//! - 线上错误码是**闭合词表** ([`WireErrorCode`]): 未知错误码字符串一律收口进
//!   [`SandboxErrorCode::Protocol`], 不做自由字符串扩散。
//!
//! 传输本身在 [`crate::sandbox::transport::OrchestrationTransport`] 边界外;
//! 编排服务在本仓库内以 mock 边界存在 (见 [`crate::sandbox::mock`]), 本模块
//! 不假设任何具体传输介质。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sandbox::error::{SandboxError, SandboxErrorCode, SandboxResult};
use crate::sandbox::runtime::SandboxStatus;
use crate::sandbox::{
    ExitCode, LogStreamEvent, SandboxConfig, SandboxHandle, SANDBOX_SCHEMA_VERSION,
};

/// 帧 schema 版本 (与 [`SANDBOX_SCHEMA_VERSION`] 同源)。
pub const WIRE_SCHEMA_VERSION: &str = SANDBOX_SCHEMA_VERSION;

/// 线上错误码闭合词表 (9 类)。编排服务只允许返回这些码; 客户端解析侧对
/// 未知字符串收口成 [`SandboxErrorCode::Protocol`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireErrorCode {
    /// 配置被服务端拒绝。
    InvalidConfig,
    /// 状态迁移被服务端拒绝。
    InvalidState,
    /// 服务端无此沙箱。
    NotFound,
    /// 服务端配额超限。
    QuotaExceeded,
    /// 服务端等待超时。
    Timeout,
    /// 服务端资源超限。
    ResourceExhausted,
    /// 服务端策略拒绝 (权限)。
    PermissionDenied,
    /// 服务端运行时错误。
    Runtime,
    /// 服务端内部错误。
    Internal,
}

impl WireErrorCode {
    /// 稳定线上字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "invalid_config",
            Self::InvalidState => "invalid_state",
            Self::NotFound => "not_found",
            Self::QuotaExceeded => "quota_exceeded",
            Self::Timeout => "timeout",
            Self::ResourceExhausted => "resource_exhausted",
            Self::PermissionDenied => "permission_denied",
            Self::Runtime => "runtime",
            Self::Internal => "internal",
        }
    }

    /// 闭合词表全集。
    pub const ALL: &'static [WireErrorCode] = &[
        Self::InvalidConfig,
        Self::InvalidState,
        Self::NotFound,
        Self::QuotaExceeded,
        Self::Timeout,
        Self::ResourceExhausted,
        Self::PermissionDenied,
        Self::Runtime,
        Self::Internal,
    ];

    /// 闭合解析: 词表外字符串 → None (调用方收口成 [`SandboxErrorCode::Protocol`])。
    pub fn parse_closed(word: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|code| code.as_str() == word)
    }

    /// 归类到本层闭合词表 (一一映射, 无自由字符串)。
    pub const fn classify(self) -> SandboxErrorCode {
        match self {
            Self::InvalidConfig => SandboxErrorCode::InvalidConfig,
            Self::InvalidState => SandboxErrorCode::InvalidState,
            Self::NotFound => SandboxErrorCode::NotFound,
            Self::QuotaExceeded => SandboxErrorCode::QuotaExceeded,
            Self::Timeout => SandboxErrorCode::Timeout,
            Self::ResourceExhausted => SandboxErrorCode::ResourceExhausted,
            Self::PermissionDenied => SandboxErrorCode::PermissionDenied,
            Self::Runtime => SandboxErrorCode::Runtime,
            Self::Internal => SandboxErrorCode::Other,
        }
    }
}

/// 线上错误码字符串 → 本层闭合分类 (未知 → [`SandboxErrorCode::Protocol`])。
pub fn classify_wire_error_code(word: &str) -> SandboxErrorCode {
    WireErrorCode::parse_closed(word)
        .map(WireErrorCode::classify)
        .unwrap_or(SandboxErrorCode::Protocol)
}

/// 客户端 → 编排服务的请求体 (op 标签联合)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OrchestrationRequest {
    /// 创建沙箱 (客户端分配沙箱 ID, 完整配置下发)。
    Create {
        /// 沙箱 ID (客户端分配, 服务端原样建档; 重复 ID = 状态错误)。
        sandbox_id: Uuid,
        /// 沙箱配置 (含安全策略 / 资源限制 / 隔离配置; 装箱避免整帧大对象搬运)。
        config: Box<SandboxConfig>,
    },
    /// 终止运行中沙箱 (graceful, 可带信号)。
    Terminate {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 信号 (None = 服务端默认)。
        signal: Option<i32>,
    },
    /// 销毁并释放资源 (volume / network / 记录)。
    Destroy {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 是否连带释放附属资源。
        release_resources: bool,
    },
    /// 状态巡检单查。
    Inspect {
        /// 目标沙箱。
        sandbox_id: Uuid,
    },
    /// 列举服务端全部沙箱 (巡检对账用)。
    List,
    /// 等待退出 (服务端侧长轮询)。
    Wait {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 服务端等待上限 (毫秒)。
        timeout_ms: u64,
    },
    /// 拉取日志 chunk (断点续传: since_seq 之后)。
    Logs {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 已消费的序号 (取其后)。
        since_seq: u64,
        /// 单次最大 chunk 数。
        max_chunks: u64,
    },
}

/// 编排服务 → 客户端的成功结果 (op 标签联合)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpResult {
    /// 创建成功 (服务端返回完整句柄)。
    Created {
        /// 服务端句柄。
        handle: SandboxHandle,
    },
    /// 终止完成。
    Terminated {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 终止后状态。
        status: SandboxStatus,
    },
    /// 销毁完成。
    Destroyed {
        /// 目标沙箱。
        sandbox_id: Uuid,
    },
    /// 巡检单查结果。
    Inspected {
        /// 服务端句柄快照。
        handle: SandboxHandle,
    },
    /// 列举结果。
    Listed {
        /// 服务端全部句柄。
        handles: Vec<SandboxHandle>,
    },
    /// 等待结果 (finished=false 表示仍在运行)。
    Waited {
        /// 目标沙箱。
        sandbox_id: Uuid,
        /// 退出码 (finished 时有值)。
        exit_code: Option<ExitCode>,
        /// 是否已退出。
        finished: bool,
    },
    /// 日志 chunk 批 (last=true 表示流到此为止)。
    LogsChunks {
        /// 本批 chunk (seq 从请求的 since_seq 起连续)。
        events: Vec<LogStreamEvent>,
        /// 是否最后一批。
        last: bool,
    },
}

/// 请求帧 (带 schema 版本 + 请求 ID)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestFrame {
    /// schema 版本 (必须 == [`WIRE_SCHEMA_VERSION`])。
    pub schema_version: String,
    /// 请求 ID (回声对账)。
    pub request_id: Uuid,
    /// 请求体。
    pub body: OrchestrationRequest,
}

/// 响应体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrchestrationResponse {
    /// 成功。
    Ok {
        /// 成功结果。
        result: OpResult,
    },
    /// 失败 (闭合错误码 + 诊断细节)。
    Err {
        /// 闭合错误码。
        code: WireErrorCode,
        /// 诊断细节 (不参与分类)。
        detail: String,
    },
}

/// 响应帧 (带 schema 版本 + 请求 ID 回声)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFrame {
    /// schema 版本 (必须 == [`WIRE_SCHEMA_VERSION`])。
    pub schema_version: String,
    /// 请求 ID (必须与请求帧一致)。
    pub request_id: Uuid,
    /// 响应体。
    pub body: OrchestrationResponse,
}

fn encode<T: Serialize>(value: &T) -> SandboxResult<String> {
    serde_json::to_string(value).map_err(SandboxError::from)
}

fn check_version(version: &str) -> SandboxResult<()> {
    if version != WIRE_SCHEMA_VERSION {
        return Err(SandboxError::Protocol(format!(
            "schema version mismatch: frame={version} client={WIRE_SCHEMA_VERSION}"
        )));
    }
    Ok(())
}

/// 编码请求帧为 JSON。
pub fn encode_request(request_id: Uuid, body: OrchestrationRequest) -> SandboxResult<String> {
    encode(&RequestFrame {
        schema_version: WIRE_SCHEMA_VERSION.to_string(),
        request_id,
        body,
    })
}

/// 解码并校验请求帧 (schema 版本必须一致)。
pub fn decode_request(frame: &str) -> SandboxResult<RequestFrame> {
    let parsed: RequestFrame = serde_json::from_str(frame).map_err(SandboxError::from)?;
    check_version(&parsed.schema_version)?;
    Ok(parsed)
}

/// 编码响应帧为 JSON。
pub fn encode_response(request_id: Uuid, body: OrchestrationResponse) -> SandboxResult<String> {
    encode(&ResponseFrame {
        schema_version: WIRE_SCHEMA_VERSION.to_string(),
        request_id,
        body,
    })
}

/// 解码并校验响应帧 (schema 版本必须一致)。
pub fn decode_response(frame: &str) -> SandboxResult<ResponseFrame> {
    let parsed: ResponseFrame = serde_json::from_str(frame).map_err(SandboxError::from)?;
    check_version(&parsed.schema_version)?;
    Ok(parsed)
}

/// 构造错误响应帧 (服务端 mock / 测试用)。
pub fn error_response(
    request_id: Uuid,
    code: WireErrorCode,
    detail: impl Into<String>,
) -> ResponseFrame {
    ResponseFrame {
        schema_version: WIRE_SCHEMA_VERSION.to_string(),
        request_id,
        body: OrchestrationResponse::Err {
            code,
            detail: detail.into(),
        },
    }
}

/// 校验响应帧的 request_id 回声 (错配 = 协议违规)。
pub fn check_response_echo(expected: Uuid, frame: &ResponseFrame) -> SandboxResult<()> {
    if frame.request_id != expected {
        return Err(SandboxError::Protocol(format!(
            "request id echo mismatch: sent={expected} got={}",
            frame.request_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::isolation::IsolationConfig;
    use crate::sandbox::policy::SecurityPolicy;
    use crate::sandbox::resource::ResourceLimits;
    use crate::sandbox::runtime::{IsolationLevel, RuntimeKind};
    use crate::sandbox::SandboxConfig;

    fn config() -> SandboxConfig {
        SandboxConfig::new(
            RuntimeKind::Docker,
            IsolationLevel::Container,
            SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "apeireth",
            ),
            ResourceLimits::default(),
        )
    }

    /// 协议编解码: 请求/响应帧 JSON 往返无损。
    #[test]
    fn request_and_response_frames_round_trip() {
        let id = Uuid::new_v4();
        let body = OrchestrationRequest::Create {
            sandbox_id: id,
            config: Box::new(config()),
        };
        let wire = encode_request(id, body.clone()).expect("encode");
        let frame = decode_request(&wire).expect("decode");
        assert_eq!(frame.request_id, id);
        assert_eq!(frame.schema_version, WIRE_SCHEMA_VERSION);
        assert_eq!(frame.body, body);

        let resp = OrchestrationResponse::Ok {
            result: OpResult::Destroyed { sandbox_id: id },
        };
        let wire = encode_response(id, resp.clone()).expect("encode");
        let frame = decode_response(&wire).expect("decode");
        check_response_echo(id, &frame).expect("echo ok");
        assert_eq!(frame.body, resp);
    }

    /// schema 版本不符一律协议违规, 不静默兼容。
    #[test]
    fn schema_version_mismatch_is_a_protocol_violation() {
        let id = Uuid::new_v4();
        let wire = encode_request(id, OrchestrationRequest::List).expect("encode");
        let tampered = wire.replace(WIRE_SCHEMA_VERSION, "99");
        let err = decode_request(&tampered).unwrap_err();
        assert!(matches!(err, SandboxError::Protocol(_)));
        assert_eq!(err.code(), SandboxErrorCode::Protocol);
    }

    /// 错误分类: 9 个线上码一一映射到本层闭合词表。
    #[test]
    fn wire_error_codes_classify_into_the_closed_vocabulary() {
        let expected = [
            (
                WireErrorCode::InvalidConfig,
                SandboxErrorCode::InvalidConfig,
            ),
            (WireErrorCode::InvalidState, SandboxErrorCode::InvalidState),
            (WireErrorCode::NotFound, SandboxErrorCode::NotFound),
            (
                WireErrorCode::QuotaExceeded,
                SandboxErrorCode::QuotaExceeded,
            ),
            (WireErrorCode::Timeout, SandboxErrorCode::Timeout),
            (
                WireErrorCode::ResourceExhausted,
                SandboxErrorCode::ResourceExhausted,
            ),
            (
                WireErrorCode::PermissionDenied,
                SandboxErrorCode::PermissionDenied,
            ),
            (WireErrorCode::Runtime, SandboxErrorCode::Runtime),
            (WireErrorCode::Internal, SandboxErrorCode::Other),
        ];
        assert_eq!(expected.len(), WireErrorCode::ALL.len());
        for (wire, class) in expected {
            assert_eq!(wire.classify(), class);
            assert_eq!(
                classify_wire_error_code(wire.as_str()),
                class,
                "wire word {} must classify to {class}",
                wire.as_str()
            );
            // 字符串往返 (闭合词表稳定)。
            assert_eq!(WireErrorCode::parse_closed(wire.as_str()), Some(wire));
        }
    }

    /// 未知线上错误码字符串收口成 protocol, 不扩散自由分类。
    #[test]
    fn unknown_wire_error_words_close_to_protocol() {
        for word in ["", "mystery", "QUOTA_EXCEEDED", "quota_exceeded "] {
            assert_eq!(
                classify_wire_error_code(word),
                SandboxErrorCode::Protocol,
                "word {word:?} must close to protocol"
            );
            assert_eq!(WireErrorCode::parse_closed(word), None);
        }
    }

    /// 非法 JSON / 错配回声都收口成 protocol。
    #[test]
    fn malformed_json_and_echo_mismatch_are_protocol_violations() {
        let err = decode_request("{not json").unwrap_err();
        assert!(matches!(err, SandboxError::Protocol(_)));

        let sent = Uuid::new_v4();
        let other = Uuid::new_v4();
        let frame = ResponseFrame {
            schema_version: WIRE_SCHEMA_VERSION.to_string(),
            request_id: other,
            body: OrchestrationResponse::Ok {
                result: OpResult::Listed { handles: vec![] },
            },
        };
        let err = check_response_echo(sent, &frame).unwrap_err();
        assert!(matches!(err, SandboxError::Protocol(_)));
    }

    /// IsolationConfig 参与请求帧编解码 (类型边界无损)。
    #[test]
    fn full_config_with_isolation_round_trips() {
        let mut cfg = config();
        cfg.isolation_config = IsolationConfig {
            level: IsolationLevel::Container,
            runtime: RuntimeKind::Gvisor,
            pid_namespace: true,
            network_namespace: true,
            mount_namespace: true,
            seccomp_profile: Some("default".into()),
            cgroup_slice: Some("sandbox.slice".into()),
            capabilities: vec!["CAP_NET_BIND_SERVICE".into()],
        };
        let sandbox_id = Uuid::new_v4();
        let wire = encode_request(
            Uuid::new_v4(),
            OrchestrationRequest::Create {
                sandbox_id,
                config: Box::new(cfg.clone()),
            },
        )
        .unwrap();
        match decode_request(&wire).unwrap().body {
            OrchestrationRequest::Create {
                sandbox_id: decoded_id,
                config,
            } => {
                assert_eq!(decoded_id, sandbox_id);
                assert_eq!(*config, cfg);
            }
            other => panic!("unexpected body {other:?}"),
        }
    }
}
