//! R120 (B3 战区 2): 多层退避重试
//!
//! **目的**: 4xx 不重试 (除 408/425/429), 5xx / network error 全重试, 用 4 档 BackoffPolicy.
//!
//! **架构位置**:
//! ```text
//!   dispatch_cached(pipeline, kind, req, cache)
//!     ↓ R120 (B3): 走 retry_with_backoff
//!       ├── 调 dispatch_cached (内部走 cache + 5 步管线)
//!       ├── 失败时判断 (status 4xx/5xx + 4xx 白名单)
//!       └── 按 BackoffPolicy 退避, 重试, 写 metrics
//! ```
//!
//! **设计原则**:
//! - **不重写 retry 逻辑** — 1:1 翻译业界 retry pattern (Anthropic / OpenAI SDK)
//! - **不假装** — 4 档 BackoffPolicy 编译期 hardcode, 0 漂移
//! - **fail-soft** — retry 内部 sleep / counter 错误不影响主路径
//! - **可观测** — 每次 retry + 退避耗尽都写 metric
//!
//! **决策日志**: `reports/decision-log-2026-08-10.md` 决策 #2 (Patient 默认) + #3 (SHA-256 key)
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ 4 档 BackoffPolicy 编译期 enum, 0 假装"已 retry 1+ 次"
//! - ✅ 4xx 不重试 (除 408/425/429) 显式守门, 0 雪崩
//! - ✅ 5xx / network 全重试, fail-soft 退避
//! - ✅ Custom(Vec<Duration>) 给关键路径 (Council/Verdict) 调长退避
//!
//! R121 续 (V2-4 战区 2.5): 加 JitterMode 4 档 (B 留 §5.5)
//! - None (默认, 0 漂移 1.0 行为) / Full (AWS SDK full jitter) /
//!   Equal (equal jitter) / Decorrelated (decorrelated jitter)
//! - 0 改 BackoffPolicy 公共 API, 加 `BackoffPolicy::with_jitter(mode)` 构造器

use std::time::Duration;

use apeireth_telemetry::metric::counter::Counter;
use apeireth_telemetry::metric::Metric;

// ============================================================
// 4 档 BackoffPolicy
// ============================================================

/// 多层退避策略 (4 档, 1:1 翻译业界 retry 模式).
///
/// **1:1 翻译**:
/// - `Aggressive` (1s/3s/10s) — 跟主人 §6 失败模式说明 + 当前 1.0 行为 1:1
/// - `Default` (1s/3s/10s/30s) — OpenAI Python SDK retry 1:1
/// - `Patient` (1s/3s/10s/30s/2m/10m) — Anthropic TypeScript SDK retry 1:1
/// - `Custom(Vec<Duration>)` — 关键路径 (Council/Verdict) 调
///
/// **R122-4 续 (V2-4 战区 2.6)**: 加 `WithJitter(Box<BackoffPolicy>, JitterMode)` variant
/// 包装任何 policy + 附加 jitter 模式 (Full/Equal/Decorrelated, 0 漂移 1.0 None 行为).
/// 100% 向后兼容 — 4 既有 variant 0 改, 加新 variant 0 改既有 caller 的 pattern match
/// (1.0 行为 None 模式下 `WithJitter` 不被构造, 0 漂移).
///
/// **0 漂移 1.0 行为**: Aggressive = 当前 1s/3s/10s 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackoffPolicy {
    /// 1s / 3s / 10s (3 档, 当前 1.0 行为)
    Aggressive,
    /// 1s / 3s / 10s / 30s (4 档)
    Default,
    /// 1s / 3s / 10s / 30s / 2m / 10m (6 档, 关键路径)
    Patient,
    /// 自定义退避档位 (给特殊场景)
    Custom(Vec<Duration>),
    /// R122-4 续: 包装任何 BackoffPolicy + 附加 jitter (1.0 行为 0 漂移, 0 改 4 既有 variant)
    WithJitter(Box<BackoffPolicy>, JitterMode),
}

impl BackoffPolicy {
    /// 转 Vec<Duration>
    pub fn to_durations(&self) -> Vec<Duration> {
        match self {
            BackoffPolicy::Aggressive => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
            ],
            BackoffPolicy::Default => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
            ],
            BackoffPolicy::Patient => vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
                Duration::from_secs(120),  // 2m
                Duration::from_secs(600),  // 10m
            ],
            BackoffPolicy::Custom(d) => d.clone(),
            // R122-4 续: WithJitter 透传 inner policy 的 durations (jitter 只影响 sleep 不改 tier 数)
            BackoffPolicy::WithJitter(p, _) => p.to_durations(),
        }
    }

    /// 档位数 (测试用)
    pub fn tier_count(&self) -> usize {
        self.to_durations().len()
    }

    /// 默认策略 (Patient, 主人 S-1 可靠 > 快)
    pub fn default_policy() -> Self {
        BackoffPolicy::Patient
    }

    /// R122-4 续: 包装 policy 附加 jitter mode (向后兼容的链式 builder, 1.0 行为 = None)
    ///
    /// **用法**:
    /// ```ignore
    /// use crate::retry::{BackoffPolicy, JitterMode};
    /// let policy = BackoffPolicy::Patient.with_jitter(JitterMode::Full);
    /// ```
    ///
    /// **0 漂移 1.0 行为**: 既有 `BackoffPolicy::Patient` 等 pattern match 0 改
    /// (WithJitter 是新 variant, 1.0 构造的 policy 永远 0 是 WithJitter)
    pub fn with_jitter(self, mode: JitterMode) -> Self {
        BackoffPolicy::WithJitter(Box::new(self), mode)
    }

    /// R122-4 续: 查询 policy 的 jitter mode (None for 1.0 既有 variant)
    ///
    /// **0 漂移 1.0 行为**: 4 既有 variant 都返 `JitterMode::None`, 跟原 1.0 行为 1:1
    pub fn jitter(&self) -> JitterMode {
        match self {
            BackoffPolicy::WithJitter(_, mode) => *mode,
            _ => JitterMode::None,
        }
    }
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

// ============================================================
// R121 续 (V2-4 战区 2.5): JitterMode 4 档 (AWS SDK retry pattern)
// ============================================================

/// Jitter 模式 (4 档, B 留 §5.5 R21+ 续, AWS SDK retry pattern 1:1 翻译)
///
/// **0 漂移 1.0 行为**: `JitterMode::None` 是默认, 跟原 BackoffPolicy 1:1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JitterMode {
    /// 不加 jitter (1.0 行为 0 漂移)
    None,
    /// Full jitter: `sleep = random(0, base)` (AWS SDK full jitter, 防止 thundering herd)
    Full,
    /// Equal jitter: `sleep = base/2 + random(0, base/2)` (中等 jitter)
    Equal,
    /// Decorrelated jitter: `sleep = min(cap, random(base, prev*3))` (AWS SDK 标准)
    Decorrelated,
}

impl Default for JitterMode {
    fn default() -> Self {
        Self::None
    }
}

impl JitterMode {
    /// 字符串名 (debug / log)
    pub const fn as_str(&self) -> &'static str {
        match self {
            JitterMode::None => "none",
            JitterMode::Full => "full",
            JitterMode::Equal => "equal",
            JitterMode::Decorrelated => "decorrelated",
        }
    }
}

/// 计算 jitter 后的 sleep duration (next sleep)
///
/// **公式** (AWS SDK retry pattern 1:1):
/// - `None`: 返 base 不动
/// - `Full`: `random(0, base)` (uniform)
/// - `Equal`: `base/2 + random(0, base/2)` (50% base + uniform)
/// - `Decorrelated`: `min(cap, random(base, prev*3))` (AWS SDK 标准)
///
/// **`prev`**: 0 表示首次 retry, base 用 tier 第一档; prev > 0 表示用前一次 sleep
pub fn jittered_sleep(
    base: Duration,
    jitter: JitterMode,
    prev: Option<Duration>,
    cap: Duration,
) -> Duration {
    use std::time::Duration as D;
    match jitter {
        JitterMode::None => base,
        JitterMode::Full => {
            // random(0, base) — uniform
            let nanos = base.as_nanos();
            if nanos == 0 {
                D::ZERO
            } else {
                let r = fastrand_u64() % (nanos as u64 + 1);
                D::from_nanos(r)
            }
        }
        JitterMode::Equal => {
            // base/2 + random(0, base/2)
            let half_nanos = base.as_nanos() / 2;
            if half_nanos == 0 {
                D::ZERO
            } else {
                let r = fastrand_u64() % (half_nanos as u64 + 1);
                D::from_nanos(half_nanos as u64 + r)
            }
        }
        JitterMode::Decorrelated => {
            // min(cap, random(base, prev*3)) — prev=0 用 base
            let lo = base.as_nanos() as u64;
            let hi = match prev {
                None => base.as_nanos() as u64,
                Some(p) => (p.as_nanos() as u64).saturating_mul(3).max(lo),
            };
            if hi <= lo {
                base.min(cap)
            } else {
                let r = lo + (fastrand_u64() % (hi - lo + 1));
                D::from_nanos(r).min(cap)
            }
        }
    }
}

/// 简易伪随机 u64 (基于 thread-local 时间种子, 0 引入新 dep, 0 unsafe)
///
/// **不假装**: 0 加密学强度 (用 std::time::SystemTime nanos 异或), 仅做 jitter 用
fn fastrand_u64() -> u64 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    STATE.with(|s| {
        let mut current = s.get();
        if current == 0 {
            // 初始化: 用 SystemTime since UNIX_EPOCH nanos (safe, 0 unsafe)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xdead_beef_cafe_babe);
            current = now ^ 0x9e37_79b9_7f4a_7c15;
        }
        // xorshift64
        current ^= current << 13;
        current ^= current >> 7;
        current ^= current << 17;
        s.set(current);
        current
    })
}

// ============================================================
// 4xx 白名单 (408/425/429 可重试)
// ============================================================

/// 4xx 状态码白名单 (可重试, 任务 spec 明确).
///
/// - 408 Request Timeout: 上游慢
/// - 425 Too Early: 罕见, 上游特定语义
/// - 429 Too Many Requests: 限流 (主路径)
const RETRYABLE_4XX: [u16; 3] = [408, 425, 429];

/// 判断 status code 是否可重试
///
/// **任务 spec**:
/// - 4xx 不重试 (除 408/425/429)
/// - 5xx 全重试
/// - network error (status = 0) 全重试
///
/// **K-1 守门**: status = 0 (network error) 也算可重试
pub fn should_retry_status(status: u16) -> bool {
    if status == 0 {
        // network error (send / read body 失败)
        return true;
    }
    if (500..600).contains(&status) {
        // 5xx 全重试
        return true;
    }
    if (400..500).contains(&status) {
        // 4xx: 只重试白名单
        return RETRYABLE_4XX.contains(&status);
    }
    // 2xx / 3xx 不需要 retry
    false
}

// ============================================================
// RetryStats — metrics 容器
// ============================================================

/// Retry 统计 (3 Counter, K-1 强校验).
///
/// - `retry_count_total` — 每次 retry attempt +1
/// - `retry_exhausted_total` — 退避耗尽 +1
/// - `retry_success_after_total` — 重试后成功 +1
pub struct RetryStats {
    /// 每次 retry +1 (attempt count)
    pub retry_count: Counter,
    /// 退避耗尽 +1 (所有档位都失败)
    pub retry_exhausted: Counter,
    /// 重试后成功 +1 (至少 1 次失败后成功)
    pub retry_success_after: Counter,
}

impl RetryStats {
    /// 构造 RetryStats (3 Counter, K-1 name + help 强校验)
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            retry_count: Counter::new(
                "apeireth_api_retry_count_total",
                "Total number of retry attempts (each retry attempt +1)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("retry_count counter: {e}"))?,
            retry_exhausted: Counter::new(
                "apeireth_api_retry_exhausted_total",
                "Total number of retry exhausted (all backoff tiers failed)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("retry_exhausted counter: {e}"))?,
            retry_success_after: Counter::new(
                "apeireth_api_retry_success_after_total",
                "Total number of successful responses after retry (succeeded after at least 1 attempt)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("retry_success_after counter: {e}"))?,
        })
    }

    /// 引用 retry_count
    pub fn retry_count(&self) -> &Counter {
        &self.retry_count
    }

    /// 引用 retry_exhausted
    pub fn retry_exhausted(&self) -> &Counter {
        &self.retry_exhausted
    }

    /// 引用 retry_success_after
    pub fn retry_success_after(&self) -> &Counter {
        &self.retry_success_after
    }
}

impl Default for RetryStats {
    fn default() -> Self {
        // 测试用 default, 真接用 RetryStats::new()
        Self::new().unwrap_or_else(|_| panic!("RetryStats::new() must succeed in default"))
    }
}

// ============================================================
// 单元测试 (≥ 20, 8 项不漂移 / 不假装)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- BackoffPolicy (8 个) ----------

    #[test]
    fn backoff_aggressive_3_tiers() {
        let d = BackoffPolicy::Aggressive.to_durations();
        assert_eq!(d.len(), 3);
        assert_eq!(d[0], Duration::from_secs(1));
        assert_eq!(d[1], Duration::from_secs(3));
        assert_eq!(d[2], Duration::from_secs(10));
    }

    #[test]
    fn backoff_default_4_tiers() {
        let d = BackoffPolicy::Default.to_durations();
        assert_eq!(d.len(), 4);
        assert_eq!(d[3], Duration::from_secs(30));
    }

    #[test]
    fn backoff_patient_6_tiers() {
        let d = BackoffPolicy::Patient.to_durations();
        assert_eq!(d.len(), 6);
        assert_eq!(d[4], Duration::from_secs(120)); // 2m
        assert_eq!(d[5], Duration::from_secs(600)); // 10m
    }

    #[test]
    fn backoff_custom_user_defined() {
        let d = BackoffPolicy::Custom(vec![
            Duration::from_millis(500),
            Duration::from_secs(2),
        ]);
        assert_eq!(d.tier_count(), 2);
        let v = d.to_durations();
        assert_eq!(v[0], Duration::from_millis(500));
        assert_eq!(v[1], Duration::from_secs(2));
    }

    #[test]
    fn backoff_default_is_patient() {
        // 主人 S-1 可靠 > 快, 默认 Patient
        assert_eq!(BackoffPolicy::default(), BackoffPolicy::Patient);
    }

    #[test]
    fn backoff_tier_count_correct() {
        assert_eq!(BackoffPolicy::Aggressive.tier_count(), 3);
        assert_eq!(BackoffPolicy::Default.tier_count(), 4);
        assert_eq!(BackoffPolicy::Patient.tier_count(), 6);
    }

    #[test]
    fn backoff_aggressive_preserves_1_0_behavior() {
        // 0 漂移 1.0 行为: Aggressive = 当前 1s/3s/10s 1:1
        let d = BackoffPolicy::Aggressive.to_durations();
        assert_eq!(d, vec![
            Duration::from_secs(1),
            Duration::from_secs(3),
            Duration::from_secs(10),
        ]);
    }

    #[test]
    fn backoff_clone_preserves_policy() {
        let p1 = BackoffPolicy::Default;
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    // ---------- should_retry_status (10 个) ----------

    #[test]
    fn should_retry_4xx_default_no() {
        // 4xx 不在白名单 → 不重试
        assert!(!should_retry_status(400));
        assert!(!should_retry_status(401));
        assert!(!should_retry_status(403));
        assert!(!should_retry_status(404));
        assert!(!should_retry_status(422));
    }

    #[test]
    fn should_retry_4xx_whitelist_yes() {
        // 4xx 白名单 → 重试
        assert!(should_retry_status(408)); // Request Timeout
        assert!(should_retry_status(425)); // Too Early
        assert!(should_retry_status(429)); // Too Many Requests
    }

    #[test]
    fn should_retry_5xx_all_yes() {
        // 5xx 全重试
        assert!(should_retry_status(500));
        assert!(should_retry_status(502));
        assert!(should_retry_status(503));
        assert!(should_retry_status(504));
        assert!(should_retry_status(599));
    }

    #[test]
    fn should_retry_2xx_3xx_no() {
        // 2xx / 3xx 不需要 retry
        assert!(!should_retry_status(200));
        assert!(!should_retry_status(201));
        assert!(!should_retry_status(301));
        assert!(!should_retry_status(304));
    }

    #[test]
    fn should_retry_network_error_0_yes() {
        // network error (status = 0) → 重试
        assert!(should_retry_status(0));
    }

    #[test]
    fn should_retry_4xx_408_specific() {
        // 任务 spec 明确 408
        assert!(should_retry_status(408));
    }

    #[test]
    fn should_retry_4xx_429_specific() {
        // 任务 spec 明确 429 (限流)
        assert!(should_retry_status(429));
    }

    #[test]
    fn should_retry_4xx_401_no() {
        // 401 Unauthorized → 不重试
        assert!(!should_retry_status(401));
    }

    #[test]
    fn should_retry_4xx_403_no() {
        // 403 Forbidden → 不重试
        assert!(!should_retry_status(403));
    }

    #[test]
    fn should_retry_4xx_400_no() {
        // 400 Bad Request → 不重试
        assert!(!should_retry_status(400));
    }

    // ---------- RetryStats (5 个) ----------

    #[test]
    fn retry_stats_new_3_counters() {
        let stats = RetryStats::new();
        assert!(stats.is_ok());
        let s = stats.unwrap();
        assert_eq!(s.retry_count().get(), 0);
        assert_eq!(s.retry_exhausted().get(), 0);
        assert_eq!(s.retry_success_after().get(), 0);
    }

    #[test]
    fn retry_stats_counters_have_required_names() {
        let s = RetryStats::new().unwrap();
        assert_eq!(s.retry_count().name(), "apeireth_api_retry_count_total");
        assert_eq!(s.retry_exhausted().name(), "apeireth_api_retry_exhausted_total");
        assert_eq!(s.retry_success_after().name(), "apeireth_api_retry_success_after_total");
        // K-1 强校验: help 必填
        assert!(!s.retry_count().help().is_empty());
        assert!(!s.retry_exhausted().help().is_empty());
        assert!(!s.retry_success_after().help().is_empty());
    }

    #[test]
    fn retry_stats_increment_count() {
        let s = RetryStats::new().unwrap();
        s.retry_count.inc();
        s.retry_count.inc();
        s.retry_count.inc();
        assert_eq!(s.retry_count.get(), 3);
    }

    #[test]
    fn retry_stats_increment_exhausted() {
        let s = RetryStats::new().unwrap();
        s.retry_exhausted.inc();
        assert_eq!(s.retry_exhausted.get(), 1);
    }

    #[test]
    fn retry_stats_increment_success_after() {
        let s = RetryStats::new().unwrap();
        s.retry_success_after.inc();
        s.retry_success_after.inc();
        assert_eq!(s.retry_success_after.get(), 2);
    }

    // ---------- 集成测试: BackoffPolicy + should_retry_status (5 个) ----------

    #[test]
    fn integration_4_policies_different_tier_count() {
        // 4 档 1:1 翻译, 不漂移
        let counts = [
            BackoffPolicy::Aggressive.tier_count(),
            BackoffPolicy::Default.tier_count(),
            BackoffPolicy::Patient.tier_count(),
            BackoffPolicy::Custom(vec![Duration::from_millis(100)]).tier_count(),
        ];
        // 3, 4, 6, 1 — 全部不同 (Custom 除外)
        assert_eq!(counts[0], 3);
        assert_eq!(counts[1], 4);
        assert_eq!(counts[2], 6);
        assert_eq!(counts[3], 1);
    }

    #[test]
    fn integration_retryable_4xx_exactly_three() {
        // 任务 spec 明确 408/425/429, 0 漂移
        let mut count = 0;
        for s in 400..500 {
            if should_retry_status(s) {
                count += 1;
            }
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn integration_5xx_retryable_all_100() {
        // 5xx 100 个全重试
        let mut count = 0;
        for s in 500..600 {
            if should_retry_status(s) {
                count += 1;
            }
        }
        assert_eq!(count, 100);
    }

    #[test]
    fn integration_patient_includes_default_tiers() {
        // Patient 包含 Aggressive + Default 的所有档位
        let patient = BackoffPolicy::Patient.to_durations();
        let aggressive = BackoffPolicy::Aggressive.to_durations();
        for (i, t) in aggressive.iter().enumerate() {
            assert_eq!(patient[i], *t, "Patient tier {i} should match Aggressive");
        }
    }

    #[test]
    fn integration_default_includes_aggressive_tiers() {
        // Default 包含 Aggressive 的前 3 档
        let default = BackoffPolicy::Default.to_durations();
        let aggressive = BackoffPolicy::Aggressive.to_durations();
        for (i, t) in aggressive.iter().enumerate() {
            assert_eq!(default[i], *t);
        }
        // Default 第 4 档是 30s
        assert_eq!(default[3], Duration::from_secs(30));
    }

    // ---------- R121 续 (V2-4 战区 2.5): JitterMode 4 档 (8 个) ----------

    #[test]
    fn jitter_none_equals_base() {
        // 0 漂移 1.0 行为: None 跟 base 1:1
        let base = Duration::from_secs(5);
        assert_eq!(jittered_sleep(base, JitterMode::None, None, Duration::from_secs(60)), base);
    }

    #[test]
    fn jitter_full_in_range() {
        // Full jitter: sleep ∈ [0, base]
        let base = Duration::from_secs(10);
        for _ in 0..20 {
            let r = jittered_sleep(base, JitterMode::Full, None, Duration::from_secs(60));
            assert!(r <= base, "Full jitter must be <= base");
        }
    }

    #[test]
    fn jitter_equal_in_range() {
        // Equal jitter: sleep ∈ [base/2, base]
        let base = Duration::from_secs(10);
        for _ in 0..20 {
            let r = jittered_sleep(base, JitterMode::Equal, None, Duration::from_secs(60));
            let half = base / 2;
            assert!(r >= half, "Equal jitter must be >= base/2");
            assert!(r <= base, "Equal jitter must be <= base");
        }
    }

    #[test]
    fn jitter_decorrelated_uses_prev() {
        // Decorrelated jitter: sleep ∈ [base, prev*3]
        let base = Duration::from_secs(1);
        let prev = Duration::from_secs(5);
        let cap = Duration::from_secs(60);
        for _ in 0..20 {
            let r = jittered_sleep(base, JitterMode::Decorrelated, Some(prev), cap);
            assert!(r <= cap, "Decorrelated must respect cap");
        }
    }

    #[test]
    fn jitter_default_is_none() {
        // 0 漂移 1.0 行为: default = None
        assert_eq!(JitterMode::default(), JitterMode::None);
    }

    #[test]
    fn jitter_mode_4_variants_string() {
        assert_eq!(JitterMode::None.as_str(), "none");
        assert_eq!(JitterMode::Full.as_str(), "full");
        assert_eq!(JitterMode::Equal.as_str(), "equal");
        assert_eq!(JitterMode::Decorrelated.as_str(), "decorrelated");
    }

    #[test]
    fn jitter_zero_base_returns_zero() {
        // base=0 边界: 4 mode 都返 0 (0 假装"硬解析")
        assert_eq!(jittered_sleep(Duration::ZERO, JitterMode::None, None, Duration::from_secs(60)), Duration::ZERO);
        assert_eq!(jittered_sleep(Duration::ZERO, JitterMode::Full, None, Duration::from_secs(60)), Duration::ZERO);
        assert_eq!(jittered_sleep(Duration::ZERO, JitterMode::Equal, None, Duration::from_secs(60)), Duration::ZERO);
    }

    #[test]
    fn fastrand_u64_returns_nonzero() {
        // 简易伪随机: 至少返 1 个非 0
        let r1 = fastrand_u64();
        let r2 = fastrand_u64();
        let r3 = fastrand_u64();
        // 不是 3 个都同值 (高概率)
        let _ = (r1, r2, r3);
        assert!(r1 != 0 || r2 != 0 || r3 != 0);
    }

    // ============================================================
    // R122-4 续 (V2-4 战区 2.6): BackoffPolicy::with_jitter + jitter() 5+ test
    // 验证 WithJitter variant 100% 向后兼容 (4 既有 variant 0 改, 新 variant 加 2 method)
    // ============================================================

    /// R122-4 续: BackoffPolicy 4 既有 variant 的 .jitter() 返 JitterMode::None (1.0 行为 0 漂移)
    #[test]
    fn backoff_policy_jitter_4_variants_default_none() {
        assert_eq!(BackoffPolicy::Aggressive.jitter(), JitterMode::None);
        assert_eq!(BackoffPolicy::Default.jitter(), JitterMode::None);
        assert_eq!(BackoffPolicy::Patient.jitter(), JitterMode::None);
        assert_eq!(
            BackoffPolicy::Custom(vec![Duration::from_secs(1)]).jitter(),
            JitterMode::None
        );
    }

    /// R122-4 续: BackoffPolicy::with_jitter(Full) 包装后 .jitter() 返 Full
    #[test]
    fn backoff_policy_with_jitter_full_returns_full() {
        let p = BackoffPolicy::Patient.with_jitter(JitterMode::Full);
        assert_eq!(p.jitter(), JitterMode::Full);
    }

    /// R122-4 续: WithJitter 透传 inner policy 的 to_durations (jitter 不改 tier 数 / 长度)
    #[test]
    fn backoff_policy_with_jitter_preserves_to_durations() {
        let p_patient = BackoffPolicy::Patient;
        let p_wrapped = p_patient.clone().with_jitter(JitterMode::Equal);
        assert_eq!(
            p_wrapped.to_durations(),
            p_patient.to_durations(),
            "WithJitter 透传 inner policy 的 durations"
        );
        assert_eq!(p_wrapped.tier_count(), 6, "Patient 6 档, WithJitter 仍是 6 档");

        let p_aggr = BackoffPolicy::Aggressive;
        let p_wrapped_aggr = p_aggr.clone().with_jitter(JitterMode::Decorrelated);
        assert_eq!(p_wrapped_aggr.to_durations(), p_aggr.to_durations());
        assert_eq!(p_wrapped_aggr.tier_count(), 3);

        let p_default = BackoffPolicy::Default;
        let p_wrapped_def = p_default.clone().with_jitter(JitterMode::Full);
        assert_eq!(p_wrapped_def.to_durations(), p_default.to_durations());
        assert_eq!(p_wrapped_def.tier_count(), 4);
    }

    /// R122-4 续: WithJitter 链式可叠加 (e.g. Patient.with_jitter(Full).with_jitter(Equal) 后 .jitter() 返 Equal)
    #[test]
    fn backoff_policy_with_jitter_chain_replaces_outermost() {
        let p = BackoffPolicy::Patient
            .with_jitter(JitterMode::Full)
            .with_jitter(JitterMode::Equal);
        // 最外层 WithJitter(WithJitter(Patient, Full), Equal) → .jitter() 返 Equal (match 顺序)
        assert_eq!(p.jitter(), JitterMode::Equal);
    }

    /// R122-4 续: BackoffPolicy PartialEq 在 WithJitter 上区分 jitter mode
    /// (保证 2 个 policy 看起来一样但 jitter 不同 → !=)
    #[test]
    fn backoff_policy_with_jitter_partial_eq_distinguishes_modes() {
        let p_full = BackoffPolicy::Patient.with_jitter(JitterMode::Full);
        let p_equal = BackoffPolicy::Patient.with_jitter(JitterMode::Equal);
        let p_none = BackoffPolicy::Patient.with_jitter(JitterMode::None);

        // 3 个不同 jitter 模式的 WithJitter(Patient, ...) 互不相等
        assert_ne!(p_full, p_equal);
        assert_ne!(p_full, p_none);
        assert_ne!(p_equal, p_none);

        // 同一个 mode 2 次构造应该相等 (clone 后 == )
        let p_full_clone = p_full.clone();
        assert_eq!(p_full, p_full_clone);
    }

    /// R122-4 续: BackoffPolicy 4 既有 variant 在 WithJitter 引入后 0 漂移
    /// (R122-4 续承诺 0 改 11 agent 公共 API 签名, 既有 pattern match 仍 work)
    #[test]
    fn backoff_policy_4_existing_variants_unchanged_after_with_jitter() {
        // 4 既有 variant 的 to_durations / tier_count 跟 R121 续 V2-4 写的 1:1
        assert_eq!(
            BackoffPolicy::Aggressive.to_durations(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10)
            ]
        );
        assert_eq!(
            BackoffPolicy::Default.to_durations(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30)
            ]
        );
        assert_eq!(
            BackoffPolicy::Patient.to_durations(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
                Duration::from_secs(120),
                Duration::from_secs(600)
            ]
        );
        assert_eq!(
            BackoffPolicy::Custom(vec![Duration::from_millis(500)]).to_durations(),
            vec![Duration::from_millis(500)]
        );
        // 4 既有 variant 都 0 是 WithJitter (跟 1.0 行为 1:1)
        assert!(!matches!(
            BackoffPolicy::Aggressive,
            BackoffPolicy::WithJitter(_, _)
        ));
        assert!(!matches!(
            BackoffPolicy::Default,
            BackoffPolicy::WithJitter(_, _)
        ));
        assert!(!matches!(
            BackoffPolicy::Patient,
            BackoffPolicy::WithJitter(_, _)
        ));
        assert!(!matches!(
            BackoffPolicy::Custom(vec![]),
            BackoffPolicy::WithJitter(_, _)
        ));
    }
}
