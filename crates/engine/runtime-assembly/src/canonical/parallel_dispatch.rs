//! 并行工具派发的**生产接线**（`CallScheduler`）。
//!
//! # 缺口
//!
//! 运行时**完整支持**并行派发：`plan_parallel_window` 找出「连续且都被判定
//! 并发安全」的调用窗口，`dispatch_parallel_window` 用滚动池并发执行、结果按
//! **模型顺序**提交。但它整条链都挂在 `RuntimeBuilder::with_call_scheduler`
//! 上 —— 而**生产装配从不构造 `CallScheduler`** ⇒ `plan_parallel_window`
//! 恒返 1 ⇒ **并行派发在生产永不发生**（形同不存在）。
//!
//! # 白名单从哪来（不拍脑袋）
//!
//! `SafetyWhitelist` 默认是**空**的（一切皆不安全）。要开并行，必须能回答
//! 「哪些工具并发安全」。这个判断不能靠列名字拍板 —— 所以本模块从
//! **守卫的能力安全注册表**（单一真源）**派生**：
//!
//! > 只有 [`CapabilitySafetyRegistry::is_provably_read_only`] 判为真的能力
//! > 才进白名单。
//!
//! 该判据要求同时满足：`known`（不是未知能力）、无持久化效果、无破坏性、
//! 无外部副作用、不联网、运行类只含 `Read`/`Compute`。
//! **未知一律不放行**（未知不等于安全）。
//!
//! 于是「哪些工具能并行」这件事有了可审计的出处：想放行新工具，
//! 就得先在守卫注册表里如实登记它的安全描述符 —— 而不是在白名单里加个名字。
//!
//! # 行为影响
//!
//! - 只有**连续 ≥2 个**调用都命中白名单时才走并行窗口；单个调用、或任一调用
//!   不在白名单 ⇒ 保持原串行路径（`plan_parallel_window` 返 1）；
//! - 结果按**模型顺序**提交，不改 transcript 顺序；
//! - 白名单为空 ⇒ 不安装调度器（与接线前逐字节一致）。

use std::sync::Arc;

use apeireth_guard::semantics::CapabilitySafetyRegistry;
use apeireth_orchestration::call_scheduler::{
    CallScheduler, SafetyRule, SafetyWhitelist, SchedulerConfig,
};

/// 从守卫的能力安全注册表派生「并发安全」白名单。
///
/// 只收 [`CapabilitySafetyRegistry::is_provably_read_only`] 判为真的能力；
/// 判据与理由见模块文档。
pub fn concurrency_safe_whitelist(registry: &CapabilitySafetyRegistry) -> SafetyWhitelist {
    let mut whitelist = SafetyWhitelist::new();
    let mut allowed: Vec<&str> = registry
        .descriptors()
        .map(|descriptor| descriptor.capability_id.as_str())
        .filter(|capability_id| registry.is_provably_read_only(capability_id))
        .collect();
    // 确定性：按 id 升序装配（注册表本身是 BTreeMap，这里显式排序防上游改结构）
    allowed.sort_unstable();
    for capability_id in allowed {
        whitelist = whitelist.allow(SafetyRule::for_tool(capability_id));
    }
    whitelist
}

/// 生产用调度器：白名单从守卫注册表派生，配置取 [`SchedulerConfig::default`]。
///
/// 白名单**为空**时返回 `None` —— 不安装调度器，行为与接线前一致
/// （避免装一个「永远不并行」的空壳增加读者误解）。
pub fn production_call_scheduler() -> Option<Arc<CallScheduler>> {
    let registry = CapabilitySafetyRegistry::canonical();
    let whitelist = concurrency_safe_whitelist(&registry);
    if whitelist.rule_count() == 0 {
        return None;
    }
    CallScheduler::new(Arc::new(whitelist), SchedulerConfig::default())
        .ok()
        .map(Arc::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_orchestration::call_scheduler::ConcurrencySafety as _;

    /// 派生出的白名单必须**只含可证明只读**的能力。
    #[test]
    fn whitelist_contains_only_provably_read_only_capabilities() {
        let registry = CapabilitySafetyRegistry::canonical();
        let whitelist = concurrency_safe_whitelist(&registry);
        assert!(
            whitelist.rule_count() > 0,
            "至少应派生出一条（否则接线等于没接）"
        );

        // 只读工具应入选
        for capability in [
            "tool.repo",
            "tool.repo_map",
            "tool.self_status",
            "tool.trait_report",
            "tool.search",
        ] {
            assert!(
                whitelist.is_concurrency_safe(capability, &serde_json::json!({})),
                "{capability} 是只读能力，应允许并行"
            );
        }

        // 有副作用 / 未知的一律不得入选
        for capability in [
            "tool.apply_patch",      // 写文件
            "tool.shell",            // 执行 + 外部副作用
            "tool.git_write",        // 仓库写
            "tool.fetch",            // 联网
            "tool.never-registered", // 未知
        ] {
            assert!(
                !whitelist.is_concurrency_safe(capability, &serde_json::json!({})),
                "{capability} 不该被放进并发白名单"
            );
        }
    }

    /// 生产调度器能构造出来（白名单非空），且**未登记能力仍不放行**。
    #[test]
    fn production_scheduler_is_built_and_stays_conservative() {
        let scheduler = production_call_scheduler().expect("只读能力应能派生白名单");
        assert!(scheduler.is_concurrency_safe("tool.repo", &serde_json::json!({})));
        assert!(!scheduler.is_concurrency_safe("tool.apply_patch", &serde_json::json!({})));
        assert!(!scheduler.is_concurrency_safe("tool.unknown", &serde_json::json!({})));
    }

    /// 空注册表 ⇒ 空白名单 ⇒ 不安装调度器（与接线前一致）。
    #[test]
    fn empty_registry_yields_no_scheduler() {
        let registry = CapabilitySafetyRegistry::default();
        let whitelist = concurrency_safe_whitelist(&registry);
        assert_eq!(whitelist.rule_count(), 0, "空注册表不该派生出任何规则");
    }
}
