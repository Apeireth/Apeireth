//! 限流守门（`RateLimitGovernanceHook`）的生产接线。
//!
//! # 为什么需要这个能力
//!
//! 即使工具已获得执行许可，**失控的死循环或高频外部调用**依然可能
//! 耗尽 API 配额或造成 DoS。`apeireth_governance::rate_limit` 早就有完整的
//! 实现（静态黑名单 + 多尺度滑动窗口 + 信任等级分档），
//! 但**生产治理链没挂它** ⇒ 这道闸一直没接电。
//!
//! # 为什么默认挂载（而不是「默认关 + env 开关」）
//!
//! 架构契约的 env 棘轮是**只减不增**（`max_distinct_names = 160`，
//! 见 `docs/architecture/contract.toml`）—— 已满额，再加开关就得提额或走例外，
//! 都不该为「接一道闸」而做。
//!
//! 更重要的是：**「默认关 + 开关」等于这闸依然没接电** ——
//! 那正是本轮要消灭的「能力齐全但没接电」本身。
//!
//! 默认挂载的**行为影响可控**：
//!
//! - 配额内**完全放行**（`Decision::Allow`），不改动任何判定或证据；
//! - 只在**同一 (能力, 会话)** 一分钟内超过 30 次、或一小时内超过 300 次时拒绝；
//! - 这两个阈值（[`RateLimitConfig::default`]）宽松到正常对话/工具使用碰不到，
//!   触发它的场景恰恰就是「失控」—— 那种情况下拒绝是对的。
//!
//! 这与治理链里既有的 `CredentialDisclosureHook` / `PromptInjectionHook`
//! 同一惯例（默认挂载、零变化）。

use apeireth_governance::rate_limit::{RateLimitConfig, RateLimitGovernanceHook};
use apeireth_governance::GovernancePipeline;

/// 构造限流 hook（默认阈值）。
///
/// 阈值取 [`RateLimitConfig::default`]：单 (能力, 会话) 每分钟 30 次、
/// 每小时 300 次。部署要调就改这里 —— 走 env 开关会撞 env 棘轮（只减不增）。
pub fn rate_limit_hook() -> RateLimitGovernanceHook {
    RateLimitGovernanceHook::new(RateLimitConfig::default())
}

/// 把限流 hook 挂到治理链。
///
/// 挂在**授权之后、内容风险之前**（与审批 hook 同位）——
/// 权限许可**不等于**可以无限调：失控的死循环或高频外部调用会耗尽
/// API 配额 / 造成 DoS。
pub fn attach_rate_limit(pipeline: GovernancePipeline) -> GovernancePipeline {
    pipeline.with(std::sync::Arc::new(rate_limit_hook()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 限流**真的生效**：超限后拒绝。
    ///
    /// 这条测试证明「接上了」不是「挂了个摆设」——
    /// hook 必须在同一 (能力, 会话) 超过配额后给出 `Deny`。
    #[tokio::test]
    async fn rate_limit_hook_denies_over_quota() {
        use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
        use apeireth_governance::rate_limit::RateLimitConfig;
        use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};

        let mut config = RateLimitConfig::default();
        config.default_per_minute = 2;
        config.default_per_hour = 100;
        let hook = RateLimitGovernanceHook::new(config);

        let capability = CapabilityId::new("tool.fetch").expect("valid capability");
        let session = SessionId::new();
        let trace = TraceId::new();
        let arguments = serde_json::json!({"url": "https://example.com"});

        // 前两次在配额内 ⇒ 放行
        for round in 1..=2 {
            let request = GovernanceRequest::new(
                Action::CapabilityDispatch {
                    capability: &capability,
                    arguments: &arguments,
                },
                session,
                trace,
                round,
            );
            let decision = hook.evaluate(&request).await;
            assert!(
                matches!(decision, Decision::Allow),
                "第 {round} 次应在配额内: {decision:?}"
            );
        }

        // 第三次超限 ⇒ 拒绝
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            session,
            trace,
            3,
        );
        let decision = hook.evaluate(&request).await;
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "超限必须拒绝: {decision:?}"
        );
    }

    /// 配额内**零行为变化** —— 限流不应改动任何判定。
    ///
    /// 这是默认挂载的前提：正常路径下（配额内）hook 完全放行。
    #[tokio::test]
    async fn quota_inside_passes_through_untouched() {
        use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
        use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};

        let hook = rate_limit_hook();
        let capability = CapabilityId::new("tool.self_status").expect("valid capability");
        let arguments = serde_json::json!({});
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let decision = hook.evaluate(&request).await;
        assert_eq!(decision, Decision::Allow);
    }
}
