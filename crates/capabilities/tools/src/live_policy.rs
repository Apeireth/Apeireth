//! **进程内共享的 live 权限策略**（宿主授权的唯一真源）。
//!
//! # 为什么要有这个模块（2026-11-01 安全审计 P3-6 的推广）
//!
//! 治理链在 CLI 组装根造**一份** [`PermissionPolicy`]，喂给
//! [`PermissionGovernanceHook`](apeireth_governance::PermissionGovernanceHook)，
//! 面板的热撤销（`POST /v1/panel/grants/revoke`）改的就是**这一份**。
//!
//! **原先错在哪**：各工具面的 stage-1 闸（`PreExecute` / `GovernanceHook`）
//! **各持一份自建策略**，与治理链那一份**不是同一个对象**
//! ⇒ ⇒ 面板上「撤销」了某个能力，**工具面看不到** ⇒ 撤销**形同虚设**。
//!
//! # 现在的口径
//!
//! * 宿主策略装进**本进程唯一的槽**（[`install_shared_live_permission_policy`]），
//!   各工具面**现读**槽里当前指向的策略（不是构造时的快照）
//!   ⇒ 撤销**立刻**可见。
//! * **槽里是 `None` = 宿主策略尚未装入**（例如不经 CLI 组装根的嵌入用法、
//!   单元测试）⇒ [`live_policy_decision`] 返回 `None` = **不表态**
//!   ⇒ 各面**沿用自己原有的档位判定** ⇒ **零行为变化**（不制造回归）。
//!
//! # 为什么用 `Option` 而不是「塞一份种子策略」
//!
//! 种子策略方案有两个问题：
//! * 种子要**包含所有工具面的授权**才能不回归 ⇒ 种子模块就得依赖各面
//!   ⇒ **循环依赖**（各面又要依赖本模块）。
//! * 「种子」本身是**第二份真源** ⇒ 它与宿主策略一旦漂移，行为不可解释。
//!
//! ⇒ 改成 `Option` 语义：**未装入就不表态**，各面保留原口径 ⇒ 零回归。

use apeireth_governance::{Decision, Permission, PermissionPolicy};
use std::sync::{Arc, Mutex};

/// 可换指针的共享槽。
///
/// 外层锁只护「换指针」；内层是策略自己的锁。
/// 内层是 [`Option`]：`None` = 宿主策略**尚未装入** ⇒ 各面**不表态**。
pub type LivePermissionPolicySlot = Arc<Mutex<Option<Arc<Mutex<PermissionPolicy>>>>>;

/// 进程内共享槽（进程内**唯一**）。
///
/// 初始为 `None`（宿主策略未装入）⇒ 各面沿用各自原有的档位判定。
pub fn shared_live_permission_policy_slot() -> LivePermissionPolicySlot {
    static SLOT: std::sync::OnceLock<LivePermissionPolicySlot> = std::sync::OnceLock::new();
    SLOT.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

/// 把**宿主策略**（治理链正在评估的那一份）装进共享槽。
///
/// 装入后各工具面与治理链**同源** ⇒ 面板的热撤销立刻可见。
/// 重复装入以**最后一次**为准（后装的接管）。
pub fn install_shared_live_permission_policy(policy: &Arc<Mutex<PermissionPolicy>>) {
    let slot = shared_live_permission_policy_slot();
    let mut current = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *current = Some(Arc::clone(policy));
}

/// 读槽里当前指向的策略句柄（克隆 `Arc`，**不持锁跨调用**）。
///
/// `None` = 宿主策略**尚未装入**。
pub fn current_live_policy(
    slot: &LivePermissionPolicySlot,
) -> Option<Arc<Mutex<PermissionPolicy>>> {
    let current = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    current.as_ref().map(Arc::clone)
}

/// 宿主对该能力的判决。
///
/// * [`Some`] = 宿主已表态（`Allow` / `RequireApproval` / `Deny`）
///   ⇒ **撤销后会是 `Deny`**（因为撤销就是把 grant 摘掉）。
/// * [`None`] = 宿主策略**尚未装入** ⇒ **不表态** ⇒ 调用方沿用自己原有口径。
pub fn live_policy_decision(slot: &LivePermissionPolicySlot, capability: &str) -> Option<Decision> {
    let policy = current_live_policy(slot)?;
    let policy = policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Some(policy.decision_for_capability(capability))
}

/// 宿主是否授权该能力（**含「已被撤销 ⇒ 未授权」**）。
///
/// * [`Some`]`true` = 已授权；
/// * [`Some`]`false` = 未授权 / **已被撤销**；
/// * [`None`] = 宿主策略**尚未装入** ⇒ **不表态**（调用方沿用自己原有口径）。
pub fn live_policy_allows(slot: &LivePermissionPolicySlot, capability: &str) -> Option<bool> {
    match live_policy_decision(slot, capability) {
        Some(Decision::Allow | Decision::RequireApproval { .. }) => Some(true),
        Some(Decision::Deny { .. }) => Some(false),
        None => None,
    }
}
