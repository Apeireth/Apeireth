//! `apeireth-governance::rate_limit` — 工具调用频率限制与安全黑名单守门 (A6 规则增强 / Rate Limit Hook).
//!
//! ## 核心哲学 (O-1 安全优先 + S-3 质量工程化)
//! 即使工具已获得执行许可，失控的死循环或高频外部调用依然可能耗尽 API 配额或导致 DoS。
//! 本模块实现细粒度的滑动窗口频率限制器 (`RateLimitGovernanceHook`) 与静态黑名单守门：
//! - **静态黑名单 (`Blacklist`)**: 永久性阻断高危或违规工具；
//! - **多尺度滑动窗口**: 限制单能力/单会话每分钟与每小时最大调用频次；
//! - **信任等级 (`TrustTier`)**: 结合工具信任度动态调整放行额度；
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 外部不可信 C-FFI。

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Action, Decision, GovernanceHook, GovernanceRequest};

/// 工具/能力信任等级.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustTier {
    /// 低信任度 (默认严格限流).
    Low = 1,
    /// 标准信任度 (常规限流).
    Standard = 2,
    /// 高信任度 (放宽限流).
    High = 3,
    /// 核心原生信任 (无限制).
    Trusted = 4,
}

/// 频率限制配置.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 默认单能力每分钟最大允许调用次数.
    pub default_per_minute: u32,
    /// 默认单能力每小时最大允许调用次数.
    pub default_per_hour: u32,
    /// 特定能力的独立每分钟阈值覆盖 (如 "tool.fetch" -> 10).
    pub capability_per_minute_overrides: HashMap<String, u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_per_minute: 30,
            default_per_hour: 300,
            capability_per_minute_overrides: HashMap::new(),
        }
    }
}

/// 单能力调用历史记录 (滑动窗口计数).
#[derive(Debug, Default)]
struct InvocationWindow {
    timestamps_ms: Vec<i64>,
}

impl InvocationWindow {
    fn record_and_check(
        &mut self,
        now_ms: i64,
        limit_per_minute: u32,
        limit_per_hour: u32,
    ) -> bool {
        // 清理 1 小时前的陈旧时间戳
        let one_hour_ago = now_ms.saturating_sub(3600_000);
        self.timestamps_ms.retain(|&t| t >= one_hour_ago);

        let one_minute_ago = now_ms.saturating_sub(60_000);
        let count_minute = self.timestamps_ms.iter().filter(|&&t| t >= one_minute_ago).count();
        let count_hour = self.timestamps_ms.len();

        if (count_minute as u32) >= limit_per_minute || (count_hour as u32) >= limit_per_hour {
            false
        } else {
            self.timestamps_ms.push(now_ms);
            true
        }
    }
}

/// 频率限制治理钩子.
#[derive(Debug, Clone)]
pub struct RateLimitGovernanceHook {
    config: RateLimitConfig,
    blacklist: Arc<HashSet<String>>,
    trust_tiers: Arc<HashMap<String, TrustTier>>,
    windows: Arc<Mutex<HashMap<String, InvocationWindow>>>,
}

impl RateLimitGovernanceHook {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            blacklist: Arc::new(HashSet::new()),
            trust_tiers: Arc::new(HashMap::new()),
            windows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 设置静态黑名单能力集合.
    pub fn with_blacklist(mut self, blacklist: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let set: HashSet<String> = blacklist.into_iter().map(|s| s.into()).collect();
        self.blacklist = Arc::new(set);
        self
    }

    /// 设置能力的信任等级.
    pub fn with_trust_tier(mut self, capability: impl Into<String>, tier: TrustTier) -> Self {
        let mut map = (*self.trust_tiers).clone();
        map.insert(capability.into(), tier);
        self.trust_tiers = Arc::new(map);
        self
    }
}

#[async_trait]
impl GovernanceHook for RateLimitGovernanceHook {
    fn name(&self) -> &str {
        "rate_limit"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        if let Action::CapabilityDispatch { capability, .. } = &request.action {
            let cap_name = capability.as_str();

            // 1. 黑名单直接拦截
            if self.blacklist.contains(cap_name) {
                return Decision::Deny {
                    reason: format!("能力 '{}' 被静态安全黑名单永久封禁", cap_name),
                };
            }

            // 2. 检查信任等级
            let tier = self
                .trust_tiers
                .get(cap_name)
                .copied()
                .unwrap_or(TrustTier::Standard);

            if tier == TrustTier::Trusted {
                return Decision::Allow;
            }

            // 3. 计算放行配额
            let base_per_min = self
                .config
                .capability_per_minute_overrides
                .get(cap_name)
                .copied()
                .unwrap_or(self.config.default_per_minute);

            let (limit_min, limit_hr) = match tier {
                TrustTier::Low => (base_per_min.min(5), self.config.default_per_hour.min(50)),
                TrustTier::Standard => (base_per_min, self.config.default_per_hour),
                TrustTier::High => (base_per_min.saturating_mul(2), self.config.default_per_hour.saturating_mul(2)),
                TrustTier::Trusted => unreachable!(),
            };

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            let mut lock = self.windows.lock().unwrap();
            let window = lock.entry(cap_name.to_string()).or_default();

            if !window.record_and_check(now_ms, limit_min, limit_hr) {
                return Decision::Deny {
                    reason: format!(
                        "能力 '{}' 触发频率限制保护 (单分钟上限: {} 次)",
                        cap_name, limit_min
                    ),
                };
            }
        }

        Decision::Allow
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};

    #[tokio::test]
    async fn test_blacklist_denies_immediately() {
        let hook = RateLimitGovernanceHook::new(RateLimitConfig::default())
            .with_blacklist(["tool.dangerous_shell"]);

        let cap = CapabilityId::new("tool.dangerous_shell").unwrap();
        let args = serde_json::json!({});
        let req = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );

        let dec = hook.evaluate(&req).await;
        match dec {
            Decision::Deny { reason } => {
                assert!(reason.contains("安全黑名单永久封禁"));
            }
            _ => panic!("Expected Deny"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_throttles_after_threshold() {
        let mut config = RateLimitConfig::default();
        config.default_per_minute = 3;

        let hook = RateLimitGovernanceHook::new(config);
        let cap = CapabilityId::new("tool.fetch").unwrap();
        let args = serde_json::json!({});
        let req = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );

        // 连续调用 3 次成功放行
        assert_eq!(hook.evaluate(&req).await, Decision::Allow);
        assert_eq!(hook.evaluate(&req).await, Decision::Allow);
        assert_eq!(hook.evaluate(&req).await, Decision::Allow);

        // 第 4 次触发限流拦截
        let dec = hook.evaluate(&req).await;
        match dec {
            Decision::Deny { reason } => {
                assert!(reason.contains("触发频率限制保护"));
            }
            _ => panic!("Expected Deny on rate limit breach"),
        }
    }

    #[tokio::test]
    async fn test_trusted_tier_bypasses_limits() {
        let mut config = RateLimitConfig::default();
        config.default_per_minute = 1;

        let hook = RateLimitGovernanceHook::new(config)
            .with_trust_tier("tool.core_read", TrustTier::Trusted);

        let cap = CapabilityId::new("tool.core_read").unwrap();
        let args = serde_json::json!({});
        let req = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );

        // 无论调用多少次都放行
        for _ in 0..10 {
            assert_eq!(hook.evaluate(&req).await, Decision::Allow);
        }
    }
}
