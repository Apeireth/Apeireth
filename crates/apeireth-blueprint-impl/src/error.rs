//! Blueprint 统一错误类型 — 8-10 种 enum variant
//!
//! 涵盖 4 风险类 (K-1/K-2/K-3/K-4) + 4 决策表 (D-01/D-02/D-03/D-04) + 模板/指标失败场景.
//!
//! 设计原则 (主 17:58 不假装 / 主 17:43 实事求是):
//! - 任何"假装通过守门"的尝试, 必须以 `Err` 返回, 而不是 `Ok(false)` 蒙混.
//! - 错误类型必须能让调用方分辨"哪一关没通过", 而不是吞成 `anyhow`.

use thiserror::Error;

/// Blueprint 统一错误 — 跨 5 模块共享.
#[derive(Debug, Error)]
pub enum BlueprintError {
    // --- K-1 强校验失败 (用户输入 / api key / model name / scope) ---
    /// K-1 强校验失败 — 用户输入不合法 (e.g. 空 / 超长 / 控制字符)
    #[error("K-1 strong validation failed: {field}={value} (reason: {reason})")]
    K1StrongValidationFailed {
        field: String,
        value: String,
        reason: String,
    },

    // --- K-2 弱校验失败 (容错处理 / 默认行为退化) ---
    /// K-2 弱校验失败 — 输入异常, 默认行为也不可执行
    #[error("K-2 weak validation failed: {field} (fallback exhausted: {reason})")]
    K2WeakValidationFailed { field: String, reason: String },

    // --- K-3 监督失败 (trace log / audit 写不出去) ---
    /// K-3 监督失败 — audit log 通道断开 / 写盘失败
    #[error("K-3 audit failed: channel={channel} (reason: {reason})")]
    K3AuditFailed { channel: String, reason: String },

    // --- K-4 守门失败 (deny/allow 决策不一致) ---
    /// K-4 守门拒绝 — deny 决策被强制执行
    #[error("K-4 guard denied: subject={subject} (rule: {rule})")]
    K4GuardDenied { subject: String, rule: String },

    // --- D-01 工具 endpoint 真接 vs stub 501 ---
    /// D-01 决策 — 工具 endpoint 是 stub (501 Not Implemented)
    #[error("D-01 stub not implemented: tool={tool} (501 from {endpoint})")]
    D01StubNotImplemented { tool: String, endpoint: String },

    // --- D-02 6 工具子路径 vs 单 endpoint ---
    /// D-02 决策 — 工具子路径未注册
    #[error("D-02 route missing: tool={tool} (sub_path={sub_path})")]
    D02RouteMissing { tool: String, sub_path: String },

    // --- D-03 WS 鉴权 = 链接 token 5min TTL ---
    /// D-03 决策 — WS 鉴权失败 (token 过期 / 格式错)
    #[error("D-03 WS auth failed: reason={reason} (ttl_at_failure={ttl_seconds}s)")]
    D03WsAuthFailed {
        reason: String,
        ttl_seconds: i64,
    },

    // --- D-04 限流 = token bucket 走 apeireth-constraint ---
    /// D-04 决策 — 限流超限 (token bucket 空)
    #[error("D-04 rate limit exceeded: bucket={bucket} (retry_after={retry_after_ms}ms)")]
    D04RateLimitExceeded {
        bucket: String,
        retry_after_ms: u64,
    },

    // --- 模板 / 指标失败 ---
    /// 模板 A-F 实装缺失 (R20 阶段 4 估补时不假装)
    #[error("template {template_id} not implemented (stage: {stage})")]
    TemplateNotImplemented { template_id: String, stage: String },

    /// 评估指标越界 (Q1/Q2/Q3 都限定 [0.0, 1.0])
    #[error("Q-metric out of range: metric={metric} value={value} (must be in [0.0, 1.0])")]
    QMetricOutOfRange { metric: String, value: f64 },

    // --- 通用 ---
    /// I/O 错误 (文件读写 / 网络)
    #[error("I/O error: {0}")]
    Io(String),

    /// 序列化错误 (serde_json / postcard)
    #[error("serialization error: {0}")]
    Serialization(String),

    /// 其他 (兜底, 不应频繁使用 — 否则说明错误分类不够细)
    #[error("other blueprint error: {0}")]
    Other(String),
}

impl BlueprintError {
    /// 返回风险类编号 (K-1..K-4 / D-01..D-04 / 模板 / 指标 / 通用).
    /// 用于 K-3 audit log 标签.
    pub fn category(&self) -> &'static str {
        match self {
            Self::K1StrongValidationFailed { .. } => "K-1",
            Self::K2WeakValidationFailed { .. } => "K-2",
            Self::K3AuditFailed { .. } => "K-3",
            Self::K4GuardDenied { .. } => "K-4",
            Self::D01StubNotImplemented { .. } => "D-01",
            Self::D02RouteMissing { .. } => "D-02",
            Self::D03WsAuthFailed { .. } => "D-03",
            Self::D04RateLimitExceeded { .. } => "D-04",
            Self::TemplateNotImplemented { .. } => "TEMPLATE",
            Self::QMetricOutOfRange { .. } => "Q-METRIC",
            Self::Io(_) => "IO",
            Self::Serialization(_) => "SERIALIZATION",
            Self::Other(_) => "OTHER",
        }
    }

    /// 是否可重试 (K-4 / 模板 / I/O 通常可重试; K-1 不可).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::K2WeakValidationFailed { .. }
                | Self::K3AuditFailed { .. }
                | Self::D04RateLimitExceeded { .. }
                | Self::Io(_)
        )
    }
}

/// Blueprint 专用 Result 类型别名.
pub type BlueprintResult<T> = Result<T, BlueprintError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_category_k1() {
        let e = BlueprintError::K1StrongValidationFailed {
            field: "api_key".to_string(),
            value: "sk-***".to_string(),
            reason: "empty".to_string(),
        };
        assert_eq!(e.category(), "K-1");
        assert!(!e.is_retryable());
    }

    #[test]
    fn error_category_k2_k3_k4() {
        let k2 = BlueprintError::K2WeakValidationFailed {
            field: "scope".to_string(),
            reason: "fallback to read-only".to_string(),
        };
        assert_eq!(k2.category(), "K-2");
        assert!(k2.is_retryable());

        let k3 = BlueprintError::K3AuditFailed {
            channel: "trace".to_string(),
            reason: "disk full".to_string(),
        };
        assert_eq!(k3.category(), "K-3");
        assert!(k3.is_retryable());

        let k4 = BlueprintError::K4GuardDenied {
            subject: "tool:bash".to_string(),
            rule: "deny_rm_rf".to_string(),
        };
        assert_eq!(k4.category(), "K-4");
        assert!(!k4.is_retryable());
    }

    #[test]
    fn error_category_d01_to_d04() {
        let d01 = BlueprintError::D01StubNotImplemented {
            tool: "WebFetch".to_string(),
            endpoint: "/tools/webfetch".to_string(),
        };
        assert_eq!(d01.category(), "D-01");

        let d02 = BlueprintError::D02RouteMissing {
            tool: "Grep".to_string(),
            sub_path: "/v1/grep".to_string(),
        };
        assert_eq!(d02.category(), "D-02");

        let d03 = BlueprintError::D03WsAuthFailed {
            reason: "token expired".to_string(),
            ttl_seconds: -30,
        };
        assert_eq!(d03.category(), "D-03");

        let d04 = BlueprintError::D04RateLimitExceeded {
            bucket: "global".to_string(),
            retry_after_ms: 1000,
        };
        assert_eq!(d04.category(), "D-04");
        assert!(d04.is_retryable());
    }

    #[test]
    fn error_category_template_and_metric() {
        let t = BlueprintError::TemplateNotImplemented {
            template_id: "A".to_string(),
            stage: "R20.4".to_string(),
        };
        assert_eq!(t.category(), "TEMPLATE");

        let q = BlueprintError::QMetricOutOfRange {
            metric: "Q1".to_string(),
            value: 1.5,
        };
        assert_eq!(q.category(), "Q-METRIC");
    }

    #[test]
    fn error_category_io_serde_other() {
        assert_eq!(BlueprintError::Io("disk".into()).category(), "IO");
        assert_eq!(BlueprintError::Serialization("json".into()).category(), "SERIALIZATION");
        assert_eq!(BlueprintError::Other("oops".into()).category(), "OTHER");
    }

    #[test]
    fn error_display_contains_field_name() {
        let e = BlueprintError::K1StrongValidationFailed {
            field: "model_name".to_string(),
            value: "gpt-X".to_string(),
            reason: "unknown model".to_string(),
        };
        let s = format!("{e}");
        assert!(s.contains("model_name"));
        assert!(s.contains("unknown model"));
    }
}
