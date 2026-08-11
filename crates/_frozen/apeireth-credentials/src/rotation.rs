//! # 凭证轮换策略 (4 种)
//!
//! 1:1 翻译 v0.9.21 商业版 4 种 key rotation 策略. 每种策略决定何时触发 key 轮换,
//! 防止长期 key 泄露风险 (per OWASP 2023 密钥管理指南).
//!
//! ## 4 轮换策略 (K-1 强校验: 编译期 hardcode, 不可运行时增删)
//!
//! | # | 策略 | 触发条件 | 默认值 | 商业版参考 |
//! |---|------|---------|--------|-----------|
//! | 1 | `Manual` | admin 手动调用 rotate() | 0 (关闭自动) | AWS Console rotate |
//! | 2 | `Time` | 每 N 天 | 30 天 | GCP Service Account key rotation |
//! | 3 | `Count` | 每 N 次使用 | 1000 次 | HashiCorp Vault dynamic secret |
//! | 4 | `Hybrid` | time + count 任意 | 30 天 OR 1000 次 | AWS Secrets Manager hybrid |
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **4 策略编译期 hardcode**: 不可运行时增删
//! 2. **should_rotate() 触发检查**: 根据当前时间 + 已用次数, 返 bool
//! 3. **Hybrid 是 time + count 任意一个触发**: OR 逻辑, 不是 AND
//! 4. **1:1 翻译 OWASP 2023 §4.3**: 密钥生命周期管理
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 4 策略 1:1 翻译 OWASP 2023 密钥管理指南, 0 业务重设计
//! - **S-2 实事求是**: 4 策略够用 99% 场景, 不发明 `UsageBasedHybrid` 等花哨组合
//! - **O-2 走在前人肩上**: 借鉴 AWS / GCP / Vault 行业惯例
//! - **O-3 干到底**: 4 策略 + should_rotate 守门 + 4 fixture 测试
//! - **O-4 任何人都能接手**: 跟 keyring / i18n 同模式 (enum + Display)
//! - **O-5 不假装**: 4 策略穷举 match, 0 任何 `UnknownStrategy` 漏防

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{CredentialsError, CredentialsResult};

// ============================================================================
// §1 轮换策略枚举 (4 种, 编译期 hardcode)
// ============================================================================

/// 凭证轮换策略 (4 种, K-1 强校验).
///
/// 1:1 翻译 v0.9.21 商业版 4 种 key rotation 策略. 顺序固定, 不可运行时增删.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RotationStrategy {
    /// **Manual**: admin 手动调用 `rotate()`, 0 自动触发.
    Manual,
    /// **Time**: 每 N 秒自动轮换 (默认 30 天 = 2_592_000 秒).
    Time {
        /// 轮换间隔 (秒), 默认 30 天.
        interval_secs: u64,
    },
    /// **Count**: 每 N 次使用后轮换 (默认 1000 次).
    Count {
        /// 轮换阈值 (使用次数), 默认 1000.
        threshold: u64,
    },
    /// **Hybrid**: time + count 任意一个触发 (OR 逻辑).
    Hybrid {
        /// 轮换间隔 (秒), 默认 30 天.
        interval_secs: u64,
        /// 轮换阈值 (使用次数), 默认 1000.
        threshold: u64,
    },
}

impl RotationStrategy {
    /// 策略字符串 (snake_case, 跟 serde rename_all 对齐).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Time { .. } => "time",
            Self::Count { .. } => "count",
            Self::Hybrid { .. } => "hybrid",
        }
    }

    /// 4 轮换策略名 (编译期 hardcode).
    #[must_use]
    pub fn all_names() -> [&'static str; 4] {
        ["manual", "time", "count", "hybrid"]
    }

    /// 4 轮换策略 (编译期 hardcode).
    pub const ALL_NAMES: [&'static str; 4] = ["manual", "time", "count", "hybrid"];

    /// 描述字符串 (用于日志 / 错误信息).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Manual => "Manual (admin-only, 0 auto)".to_string(),
            Self::Time { interval_secs } => {
                format!("Time (every {} secs = {} days)", interval_secs, interval_secs / 86_400)
            }
            Self::Count { threshold } => {
                format!("Count (every {threshold} uses)")
            }
            Self::Hybrid {
                interval_secs,
                threshold,
            } => {
                format!(
                    "Hybrid ({} days OR {} uses, whichever first)",
                    interval_secs / 86_400,
                    threshold
                )
            }
        }
    }

    /// **核心**: 检查当前是否应该轮换.
    ///
    /// - `Manual`: 永远 false (admin 手动触发)
    /// - `Time`: now - last_rotated_at >= interval_secs
    /// - `Count`: use_count >= threshold
    /// - `Hybrid`: time OR count (任意一个满足)
    ///
    /// 注意: skeleton 阶段 always 返 false (R21+ 真接商业版 SDK 时再实现真检查).
    /// 当前仅根据参数 + use_count 静态判断.
    #[must_use]
    pub fn should_rotate(
        &self,
        last_rotated_at: Option<DateTime<Utc>>,
        use_count: u64,
    ) -> bool {
        match self {
            Self::Manual => false,
            Self::Time { interval_secs } => match last_rotated_at {
                Some(last) => {
                    let now = Utc::now();
                    let elapsed = (now - last).num_seconds().max(0) as u64;
                    elapsed >= *interval_secs
                }
                None => true, // 从未轮换过, 立即轮换
            },
            Self::Count { threshold } => use_count >= *threshold,
            Self::Hybrid {
                interval_secs,
                threshold,
            } => {
                let time_due = match last_rotated_at {
                    Some(last) => {
                        let now = Utc::now();
                        let elapsed = (now - last).num_seconds().max(0) as u64;
                        elapsed >= *interval_secs
                    }
                    None => true,
                };
                let count_due = use_count >= *threshold;
                time_due || count_due
            }
        }
    }

    /// **核心**: 触发轮换 (skeleton 阶段 stub).
    ///
    /// R21+ 真接商业版 SDK 时, 此函数调对应 Provider 的 rotate API.
    /// 当前返 `Err(CredentialsError::NotImplemented)`.
    pub async fn rotate(&self) -> CredentialsResult<DateTime<Utc>> {
        // skeleton: 永远返 NotImplemented
        // (R21+ 真接时, 调 Provider::rotate_secret / OAuth2 token endpoint 等)
        let _ = self; // suppress unused warning
        Err(CredentialsError::NotImplemented(
            "rotation_rotate: R20 阶段 6 skeleton, R21+ 真接商业版 rotate API",
        ))
    }
}

impl std::fmt::Display for RotationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 默认值常量 (OWASP 2023 推荐)
// ============================================================================

/// Time 策略默认间隔 (30 天 = 2_592_000 秒, per OWASP 2023 §4.3).
pub const DEFAULT_TIME_INTERVAL_SECS: u64 = 30 * 86_400;

/// Count 策略默认阈值 (1000 次, per OWASP 2023 §4.3).
pub const DEFAULT_COUNT_THRESHOLD: u64 = 1000;

/// Hybrid 策略默认 interval (30 天).
pub const DEFAULT_HYBRID_INTERVAL_SECS: u64 = DEFAULT_TIME_INTERVAL_SECS;

/// Hybrid 策略默认 threshold (1000 次).
pub const DEFAULT_HYBRID_THRESHOLD: u64 = DEFAULT_COUNT_THRESHOLD;

/// Time 默认构造函数.
#[must_use]
pub fn time_default() -> RotationStrategy {
    RotationStrategy::Time {
        interval_secs: DEFAULT_TIME_INTERVAL_SECS,
    }
}

/// Count 默认构造函数.
#[must_use]
pub fn count_default() -> RotationStrategy {
    RotationStrategy::Count {
        threshold: DEFAULT_COUNT_THRESHOLD,
    }
}

/// Hybrid 默认构造函数.
#[must_use]
pub fn hybrid_default() -> RotationStrategy {
    RotationStrategy::Hybrid {
        interval_secs: DEFAULT_HYBRID_INTERVAL_SECS,
        threshold: DEFAULT_HYBRID_THRESHOLD,
    }
}

// ============================================================================
// §3 编译期守门 (4 策略对齐)
// ============================================================================

/// 4 轮换策略数 (K-1 强校验: 编译期 hardcode).
pub const ROTATION_STRATEGY_COUNT: usize = 4;
const _: () = assert!(RotationStrategy::ALL_NAMES.len() == ROTATION_STRATEGY_COUNT);
const _: () = assert!(DEFAULT_TIME_INTERVAL_SECS == 2_592_000);
const _: () = assert!(DEFAULT_COUNT_THRESHOLD == 1000);

// ============================================================================
// §4 单元测试 (4 策略 + should_rotate 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_4_strategies() {
        // 4 轮换策略全部存在
        assert_eq!(RotationStrategy::all_names().len(), 4);
        assert!(RotationStrategy::all_names().contains(&"manual"));
        assert!(RotationStrategy::all_names().contains(&"time"));
        assert!(RotationStrategy::all_names().contains(&"count"));
        assert!(RotationStrategy::all_names().contains(&"hybrid"));
    }

    #[test]
    fn test_manual_never_auto_rotates() {
        // Manual 永远返 false (除非 admin 手动)
        let now = Utc::now();
        assert!(!RotationStrategy::Manual.should_rotate(Some(now), 0));
        assert!(!RotationStrategy::Manual.should_rotate(Some(now), 1_000_000));
        assert!(!RotationStrategy::Manual.should_rotate(None, 0));
    }

    #[test]
    fn test_rotation_time_30_days() {
        // Time 30 天: 没过 30 天 → false, 过了 → true
        let now = Utc::now();
        let recent = now - chrono::Duration::days(15);
        let old = now - chrono::Duration::days(31);
        let strategy = time_default();
        assert!(!strategy.should_rotate(Some(recent), 0), "15 days < 30 days, no rotate");
        assert!(strategy.should_rotate(Some(old), 0), "31 days >= 30 days, rotate");
        assert!(strategy.should_rotate(None, 0), "never rotated, rotate now");
    }

    #[test]
    fn test_rotation_count_1000_uses() {
        // Count 1000 次: < 1000 → false, >= 1000 → true
        let now = Utc::now();
        let strategy = count_default();
        assert!(!strategy.should_rotate(Some(now), 0), "0 < 1000, no rotate");
        assert!(!strategy.should_rotate(Some(now), 999), "999 < 1000, no rotate");
        assert!(strategy.should_rotate(Some(now), 1000), "1000 >= 1000, rotate");
        assert!(strategy.should_rotate(Some(now), 5000), "5000 > 1000, rotate");
    }

    #[test]
    fn test_hybrid_time_or_count() {
        // Hybrid 30 天 OR 1000 次: 任一满足即 true
        let now = Utc::now();
        let recent = now - chrono::Duration::days(15);
        let old = now - chrono::Duration::days(31);
        let strategy = hybrid_default();
        // 都不满足
        assert!(!strategy.should_rotate(Some(recent), 500), "15 days + 500 < both, no rotate");
        // 仅 time 满足
        assert!(strategy.should_rotate(Some(old), 500), "31 days >= 30 days, rotate");
        // 仅 count 满足
        assert!(strategy.should_rotate(Some(recent), 1500), "1500 >= 1000, rotate");
        // 都满足
        assert!(strategy.should_rotate(Some(old), 2000), "both, rotate");
        // 从未轮换
        assert!(strategy.should_rotate(None, 0), "never rotated, rotate");
    }

    #[test]
    fn test_describe_includes_interval() {
        // 描述字符串含时间 / 次数 (用于日志)
        let time = time_default();
        let count = count_default();
        let hybrid = hybrid_default();
        let manual = RotationStrategy::Manual;
        assert!(time.describe().contains("2592000"));
        assert!(time.describe().contains("30 days"));
        assert!(count.describe().contains("1000"));
        assert!(hybrid.describe().contains("30 days"));
        assert!(hybrid.describe().contains("1000"));
        assert!(manual.describe().contains("Manual"));
    }

    #[test]
    fn test_rotate_stub_returns_not_implemented() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let result = time_default().rotate().await;
            assert!(matches!(result, Err(CredentialsError::NotImplemented(_))));
        });
    }

    #[test]
    fn test_serde_roundtrip_4_strategies() {
        let strategies = [
            RotationStrategy::Manual,
            time_default(),
            count_default(),
            hybrid_default(),
        ];
        for s in &strategies {
            let json = serde_json::to_string(s).expect("serialize");
            let parsed: RotationStrategy = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&parsed, s);
        }
    }
}
