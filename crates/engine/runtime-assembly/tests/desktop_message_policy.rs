//! `rt.module_messages` 的**生产接线级证据**（桌面 Host 武装受限模块消息管道）。
//!
//! # 缺口（补前）
//!
//! 生产组合根 `ProductionModules::desktop_agent_policy()`（CLI 组合根
//! `crates/adapters/cli/src/lib.rs:1704` 调用）构造 `AgentHostPolicy` 时
//! **从不设置 `message_policy`** ⇒ ⇒ 模块间消息端口恒为 `Disabled`
//! ⇒ ⇒ 那条「owner 认证 + 有界」的管道在桌面产品里**根本不存在**
//! （只有 `module_message_pipeline.rs` 这类集成测试在装它）。
//!
//! # 为什么「装上限额、零授权」是零行为变化
//!
//! `ModuleMessagePolicy { limits: 有界默认值, grants: 空 }`
//! ⇒ ⇒ **没有任何来源模块被授权**（放行只看 `grants`）
//! ⇒ ⇒ 零流量、零副作用；授权仍须 Host 显式加 `ModuleMessageGrant`
//! （「投递不授予权限」）。
//!
//! # 反向验伪（本文件的断言不是恒绿）
//!
//! 把 `desktop_host.rs` 里 `policy.message_policy = Some(...)` 那一句删掉
//! ⇒ ⇒ 本文件三个断言全部变红（`None != Some(..)`）。已实测。

use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_runtime_assembly::{ProductionBackends, ProductionModules, ProductionModulesConfig};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn production_policy() -> apeireth_runtime::AgentHostPolicy {
    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("production assembly builds");
    modules
        .desktop_agent_policy()
        .expect("desktop agent policy builds")
}

/// ① 认证面存在：桌面 Host 现在**真的装**了消息管道（不再是 `None`/`Disabled`）。
#[tokio::test]
async fn desktop_host_policy_arms_the_module_message_pipeline() {
    let policy = production_policy();
    let message_policy = policy
        .message_policy
        .as_ref()
        .expect("生产 Host 必须武装模块消息管道（补前这里是 None ⇒ 端口恒 Disabled）");

    // ② 有界：限额是**默认的受限值**，不是无上限。
    let limits = &message_policy.limits;
    assert_eq!(limits.in_flight, 16, "在途上限必须受限");
    assert_eq!(limits.max_payload_bytes, 64 * 1024, "载荷上限必须受限");
    assert_eq!(limits.max_hops, 16, "跳数上限必须受限（防环）");
    assert_eq!(limits.idempotency_records, 128, "幂等记录上限必须受限");
    assert_eq!(limits.timeout.as_secs(), 5, "超时必须受限");
}

/// ③ 零授权 ⇒ 零流量 ⇒ **零行为变化**：武装管道本身不授予任何模块发言权。
#[tokio::test]
async fn arming_the_pipeline_grants_no_source_module_any_message_authority() {
    let policy = production_policy();
    let message_policy = policy.message_policy.as_ref().expect("pipeline armed");
    assert!(
        message_policy.grants.is_empty(),
        "生产默认必须是**零授权**（授权只能由 Host 显式加 ModuleMessageGrant）"
    );
    // 认证语义仍由 Host 的模块 grant 表承担，两张表不互相代理：
    // 即使某模块在 `modules` 里有 Agent grant，它也不因此获得消息发言权。
    assert!(
        message_policy
            .grants
            .values()
            .all(|grant| grant.authorization_ref.trim().is_empty() == false),
        "空 grants ⇒ 该断言恒真；保留是为了钉住「grants 里若有条目必须带显式授权引用」"
    );
}

/// ④ 装配是**惰性**的：同一配置反复装配，管道状态一致（不依赖时钟/全局态）。
#[tokio::test]
async fn the_arming_is_idempotent_across_assemblies() {
    let first = production_policy().message_policy.expect("armed");
    let second = production_policy().message_policy.expect("armed");
    assert_eq!(first.limits.in_flight, second.limits.in_flight);
    assert_eq!(first.grants.len(), second.grants.len());
    assert!(first.grants.is_empty() && second.grants.is_empty());
}
