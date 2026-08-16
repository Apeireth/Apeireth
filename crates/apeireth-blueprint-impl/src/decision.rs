//! # D-01 / D-02 / D-03 / D-04 — 4 决策表实装
//!
//! 4 决策表源自 RIVAL 蓝图 §2.4 (R20 阶段 4 估补):
//!
//! | 决策 | 含义 | 本模块 enum | 默认选择 |
//! |------|------|------------|---------|
//! | **D-01** | 工具 endpoint 真接 vs stub 501 | [`D01Impl`] | R20 阶段 4 = 1 真接 + 5 stub 501 |
//! | **D-02** | 6 工具子路径 vs 单 endpoint | [`D02Routing`] | 6 工具子路径 (per tool 白名单) |
//! | **D-03** | WS 鉴权 = 链接 token 5min TTL | [`D03WsAuth`] | 链接 token + 5min TTL |
//! | **D-04** | 限流 = token bucket 走 `apeireth-constraint` | [`D04RateLimit`] | token bucket (capacity=60, refill=1s) |
//!
//! 4 enum 都是 `Serialize + Deserialize`, 可作配置文件 / 决策快照持久化.
//!
//! ## 6 哲学锚穿透
//!
//! - S-1 主 22:33 — 4 决策为 ASI 北极星服务, 限流保稳定 / 鉴权保安全 / 路由保清晰 / stub 守诚实.
//! - S-2 主 17:43 — 4 决策都有实装路径, 不停留在 "设计" 阶段.
//! - O-5 主 17:58 — stub 决策 = 501, 必须明说 "未实装", 不假装真接.
//! - O-2 主 19:33 — 借鉴 v0.9.21 商业版 5 决策, 加 4 维 (1 鉴权 + 1 限流 + 1 路由 + 1 stub).
//! - O-3 主 23:44 — 4 enum 一次写齐, 含默认构造器 + 验证器.
//! - O-4 主 00:56 — 4 enum 都可 `to_string()` 打印, 接手者能直接看决策快照.
//!
//! ## 8 项不修改承诺
//!
//! 1. D-01..D-04 编号不变 (跟 RIVAL §2.4 对齐).
//! 2. D-03 WS 鉴权 TTL 不可超过 5 min (链接 token 5min LOCKED).
//! 3. D-04 默认 token bucket 容量不可超过 1000 (R20 阶段 4 限流上限).
//! 4. 4 enum 都 `#[non_exhaustive]` 禁止外部新增 variant.
//! 5. 4 enum 都实现 `Default` 走推荐选择.
//! 6. 4 enum 都实现 `validate()` 自检, 防止反序列化绕过.
//! 7. 不依赖任何外部 crate 做决策 (纯本 crate 数据结构).
//! 8. 4 enum 互不依赖 — 决策可独立切换.

use crate::error::{BlueprintError, BlueprintResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================
// D-01 — 工具 endpoint 真接 vs stub 501
// ============================================

/// D-01 决策 — 工具 endpoint 是真接还是 stub 501.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum D01Impl {
    /// 真接到 provider SDK
    RealConnect { provider: String, endpoint: String },
    /// Stub 501 (未实装)
    StubNotImplemented { tool: String, planned_stage: String },
}

impl Default for D01Impl {
    fn default() -> Self {
        // R20 阶段 4 默认: 1 真接 (claude-code) + 其他 stub
        Self::StubNotImplemented {
            tool: "<unset>".into(),
            planned_stage: "R20.4".into(),
        }
    }
}

impl D01Impl {
    /// 是否真接
    pub fn is_real(&self) -> bool {
        matches!(self, Self::RealConnect { .. })
    }

    /// 工具名 (用于 audit)
    pub fn tool_name(&self) -> &str {
        match self {
            Self::RealConnect { provider, .. } => provider,
            Self::StubNotImplemented { tool, .. } => tool,
        }
    }

    /// 自检
    pub fn validate(&self) -> BlueprintResult<()> {
        match self {
            Self::RealConnect { provider, endpoint } => {
                if provider.trim().is_empty() {
                    return Err(BlueprintError::D01StubNotImplemented {
                        tool: "<empty provider>".into(),
                        endpoint: endpoint.clone(),
                    });
                }
                if endpoint.trim().is_empty() {
                    return Err(BlueprintError::D01StubNotImplemented {
                        tool: provider.clone(),
                        endpoint: "<empty>".into(),
                    });
                }
                Ok(())
            }
            Self::StubNotImplemented {
                tool,
                planned_stage,
            } => {
                if tool.trim().is_empty() {
                    return Err(BlueprintError::D01StubNotImplemented {
                        tool: "<empty>".into(),
                        endpoint: format!("planned:{planned_stage}"),
                    });
                }
                Ok(())
            }
        }
    }
}

// ============================================
// D-02 — 6 工具子路径 vs 单 endpoint
// ============================================

/// D-02 决策 — 工具是 6 子路径还是单 endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum D02Routing {
    /// 6 工具子路径 (per tool 白名单)
    SubPath { tool: String, sub_path: String },
    /// 单 endpoint (所有工具走同一路径)
    SingleEndpoint { endpoint: String },
}

impl Default for D02Routing {
    fn default() -> Self {
        // R20 阶段 4 默认: 6 工具子路径
        Self::SubPath {
            tool: "<unset>".into(),
            sub_path: "/v1/<tool>".into(),
        }
    }
}

impl D02Routing {
    pub fn is_subpath(&self) -> bool {
        matches!(self, Self::SubPath { .. })
    }

    pub fn route(&self) -> String {
        match self {
            Self::SubPath { tool, sub_path } => {
                if tool == "<unset>" {
                    sub_path.clone()
                } else {
                    sub_path.replace("<tool>", tool)
                }
            }
            Self::SingleEndpoint { endpoint } => endpoint.clone(),
        }
    }

    pub fn validate(&self) -> BlueprintResult<()> {
        match self {
            Self::SubPath { tool, sub_path } => {
                if tool.trim().is_empty() {
                    return Err(BlueprintError::D02RouteMissing {
                        tool: "<empty>".into(),
                        sub_path: sub_path.clone(),
                    });
                }
                if sub_path.trim().is_empty() {
                    return Err(BlueprintError::D02RouteMissing {
                        tool: tool.clone(),
                        sub_path: "<empty>".into(),
                    });
                }
                Ok(())
            }
            Self::SingleEndpoint { endpoint } => {
                if endpoint.trim().is_empty() {
                    return Err(BlueprintError::D02RouteMissing {
                        tool: "<any>".into(),
                        sub_path: "<empty endpoint>".into(),
                    });
                }
                Ok(())
            }
        }
    }
}

// ============================================
// D-03 — WS 鉴权 = 链接 token 5min TTL
// ============================================

/// D-03 WS 鉴权策略.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum D03WsAuth {
    /// 链接 token (5 min TTL 默认, 不超过)
    LinkToken { ttl: Duration },
    /// query param (不推荐, 容易泄漏到日志)
    QueryParam,
    /// 无鉴权 (仅 dev)
    None,
}

impl Default for D03WsAuth {
    fn default() -> Self {
        Self::LinkToken {
            ttl: Duration::from_secs(5 * 60), // 5 min per 8 项承诺 #2
        }
    }
}

impl D03WsAuth {
    pub fn ttl(&self) -> Option<Duration> {
        match self {
            Self::LinkToken { ttl } => Some(*ttl),
            _ => None,
        }
    }

    pub fn validate(&self) -> BlueprintResult<()> {
        match self {
            Self::LinkToken { ttl } => {
                if ttl.as_secs() > 5 * 60 {
                    return Err(BlueprintError::D03WsAuthFailed {
                        reason: format!("TTL {}s exceeds 5min LOCKED", ttl.as_secs()),
                        ttl_seconds: ttl.as_secs() as i64,
                    });
                }
                if ttl.as_secs() == 0 {
                    return Err(BlueprintError::D03WsAuthFailed {
                        reason: "TTL=0 makes LinkToken useless".into(),
                        ttl_seconds: 0,
                    });
                }
                Ok(())
            }
            Self::QueryParam => Ok(()),
            Self::None => Ok(()), // dev only
        }
    }
}

// ============================================
// D-04 — 限流 = token bucket 走 apeireth-constraint
// ============================================

/// D-04 限流策略.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum D04RateLimit {
    /// Token bucket (capacity + refill interval)
    TokenBucket {
        capacity: u32,
        refill_interval: Duration,
    },
    /// 不限流 (dev / test)
    None,
}

impl Default for D04RateLimit {
    fn default() -> Self {
        Self::TokenBucket {
            capacity: 60,
            refill_interval: Duration::from_secs(1),
        }
    }
}

impl D04RateLimit {
    pub fn validate(&self) -> BlueprintResult<()> {
        match self {
            Self::TokenBucket {
                capacity,
                refill_interval,
            } => {
                if *capacity == 0 {
                    return Err(BlueprintError::D04RateLimitExceeded {
                        bucket: "zero-capacity".into(),
                        retry_after_ms: 0,
                    });
                }
                if *capacity > 1000 {
                    return Err(BlueprintError::D04RateLimitExceeded {
                        bucket: format!("capacity={capacity}"),
                        retry_after_ms: 0,
                    });
                }
                if refill_interval.as_millis() == 0 {
                    return Err(BlueprintError::D04RateLimitExceeded {
                        bucket: "zero-refill-interval".into(),
                        retry_after_ms: 0,
                    });
                }
                Ok(())
            }
            Self::None => Ok(()),
        }
    }
}

// ============================================
// DecisionBundle — 4 决策打包 + 验证
// ============================================

/// 4 决策打包 — 一次校验全部.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecisionBundle {
    pub d01: D01Impl,
    pub d02: D02Routing,
    pub d03: D03WsAuth,
    pub d04: D04RateLimit,
}

impl DecisionBundle {
    pub fn new(d01: D01Impl, d02: D02Routing, d03: D03WsAuth, d04: D04RateLimit) -> Self {
        Self { d01, d02, d03, d04 }
    }

    /// 自检全部 4 决策.
    pub fn validate(&self) -> BlueprintResult<()> {
        self.d01.validate()?;
        self.d02.validate()?;
        self.d03.validate()?;
        self.d04.validate()?;
        Ok(())
    }

    /// 决策快照 (用于 log / debug)
    pub fn snapshot(&self) -> String {
        format!(
            "D-01={} | D-02={} | D-03={} | D-04={}",
            self.d01.tool_name(),
            if self.d02.is_subpath() {
                "subpath"
            } else {
                "single"
            },
            match &self.d03 {
                D03WsAuth::LinkToken { ttl } => format!("link_token({}s)", ttl.as_secs()),
                D03WsAuth::QueryParam => "query_param".to_string(),
                D03WsAuth::None => "none".to_string(),
            },
            match &self.d04 {
                D04RateLimit::TokenBucket {
                    capacity,
                    refill_interval,
                } => {
                    format!("bucket({}/{}s)", capacity, refill_interval.as_secs())
                }
                D04RateLimit::None => "none".to_string(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- D-01 ---
    #[test]
    fn d01_real_connect_validates() {
        let d = D01Impl::RealConnect {
            provider: "claude-code".into(),
            endpoint: "/v1/messages".into(),
        };
        assert!(d.is_real());
        assert!(d.validate().is_ok());
    }

    #[test]
    fn d01_stub_is_not_real() {
        let d = D01Impl::StubNotImplemented {
            tool: "WebFetch".into(),
            planned_stage: "R21".into(),
        };
        assert!(!d.is_real());
        assert!(d.validate().is_ok());
    }

    #[test]
    fn d01_stub_rejects_empty_tool() {
        let d = D01Impl::StubNotImplemented {
            tool: "".into(),
            planned_stage: "R21".into(),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d01_real_rejects_empty_provider() {
        let d = D01Impl::RealConnect {
            provider: "".into(),
            endpoint: "/v1".into(),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d01_default_is_stub() {
        let d = D01Impl::default();
        assert!(!d.is_real());
    }

    // --- D-02 ---
    #[test]
    fn d02_subpath_replaces_placeholder() {
        let d = D02Routing::SubPath {
            tool: "Bash".into(),
            sub_path: "/v1/<tool>".into(),
        };
        assert_eq!(d.route(), "/v1/Bash");
        assert!(d.is_subpath());
    }

    #[test]
    fn d02_single_endpoint() {
        let d = D02Routing::SingleEndpoint {
            endpoint: "/v1/tools".into(),
        };
        assert!(!d.is_subpath());
        assert_eq!(d.route(), "/v1/tools");
    }

    #[test]
    fn d02_subpath_rejects_empty_tool() {
        let d = D02Routing::SubPath {
            tool: "".into(),
            sub_path: "/v1/<tool>".into(),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d02_default_is_subpath() {
        let d = D02Routing::default();
        assert!(d.is_subpath());
    }

    // --- D-03 ---
    #[test]
    fn d03_default_is_5min_link_token() {
        let d = D03WsAuth::default();
        assert_eq!(d.ttl(), Some(Duration::from_secs(300)));
    }

    #[test]
    fn d03_rejects_ttl_over_5min() {
        let d = D03WsAuth::LinkToken {
            ttl: Duration::from_secs(6 * 60),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d03_rejects_zero_ttl() {
        let d = D03WsAuth::LinkToken {
            ttl: Duration::from_secs(0),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d03_query_param_has_no_ttl() {
        let d = D03WsAuth::QueryParam;
        assert_eq!(d.ttl(), None);
        assert!(d.validate().is_ok());
    }

    #[test]
    fn d03_none_validates_for_dev() {
        let d = D03WsAuth::None;
        assert!(d.validate().is_ok());
    }

    #[test]
    fn d03_accepts_5min_exactly() {
        let d = D03WsAuth::LinkToken {
            ttl: Duration::from_secs(300),
        };
        assert!(d.validate().is_ok());
    }

    // --- D-04 ---
    #[test]
    fn d04_default_is_60_per_sec() {
        let d = D04RateLimit::default();
        match d {
            D04RateLimit::TokenBucket {
                capacity,
                refill_interval,
            } => {
                assert_eq!(capacity, 60);
                assert_eq!(refill_interval, Duration::from_secs(1));
            }
            _ => panic!("expected TokenBucket"),
        }
    }

    #[test]
    fn d04_rejects_zero_capacity() {
        let d = D04RateLimit::TokenBucket {
            capacity: 0,
            refill_interval: Duration::from_secs(1),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d04_rejects_over_1000_capacity() {
        let d = D04RateLimit::TokenBucket {
            capacity: 10_000,
            refill_interval: Duration::from_secs(1),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d04_accepts_1000_capacity() {
        let d = D04RateLimit::TokenBucket {
            capacity: 1000,
            refill_interval: Duration::from_secs(1),
        };
        assert!(d.validate().is_ok());
    }

    #[test]
    fn d04_rejects_zero_refill_interval() {
        let d = D04RateLimit::TokenBucket {
            capacity: 60,
            refill_interval: Duration::from_millis(0),
        };
        assert!(d.validate().is_err());
    }

    #[test]
    fn d04_none_validates() {
        let d = D04RateLimit::None;
        assert!(d.validate().is_ok());
    }

    // --- DecisionBundle ---
    #[test]
    fn bundle_validates_all_4() {
        let b = DecisionBundle::default();
        assert!(b.validate().is_ok());
    }

    #[test]
    fn bundle_fails_on_any_invalid() {
        let b = DecisionBundle::new(
            D01Impl::default(),
            D02Routing::default(),
            D03WsAuth::LinkToken {
                ttl: Duration::from_secs(10 * 60),
            }, // invalid: 10min
            D04RateLimit::default(),
        );
        assert!(b.validate().is_err());
    }

    #[test]
    fn bundle_snapshot_is_human_readable() {
        let b = DecisionBundle::default();
        let s = b.snapshot();
        assert!(s.contains("D-01"));
        assert!(s.contains("D-02"));
        assert!(s.contains("D-03"));
        assert!(s.contains("D-04"));
        assert!(s.contains("link_token"));
    }
}
