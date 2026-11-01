//! **设备面：宿主拍板 ⇒ 决定账落账**（2026-11-01 安全审计 P3-1/P3-2 的闭环）。
//!
//! # 为什么单独一个模块
//!
//! `canonical_entry.rs` 受**单文件尺寸棘轮**约束（0 容差）⇒ 这段逻辑放这里，
//! 让大文件只留**一行调用**。
//!
//! # 为什么需要它
//!
//! 修复后设备面的放行**只认宿主决定账**（模型自称的授权人不作数），
//! 但决定账原先**只有 IM 设备审批卡链会写** ⇒ 面板批了却不落账
//! ⇒ 执行面查无批准 ⇒ **永远拒绝**（人批了也执行不了 = 流程断链）。
//! ⇒ 本模块把「宿主拍板」这一步**接上**决定账 ⇒ 闭环。

use apeireth_core::kernel::{ApprovalId, SessionId};
use apeireth_runtime::canonical::{ApprovalDecision, Runtime};

use crate::canonical_entry::CanonicalEntryError;

/// 宿主拍板 ⇒ 设备决定账落账（**不是设备面调用则什么都不做**）。
///
/// 设备面的审批发生在 `execute` 这一拍 ⇒ pending approval 的工具调用参数
/// **自带 `plan_id`** ⇒ 直接取用、不重算（重算要设备表 + 分级词表，易与执行面漂移）。
///
/// ⚠️ 顺序：**先落账、后续跑**（续跑会执行工具 ⇒ 决定账必须已就位）。
/// ⚠️ 落账失败 ⇒ **不续跑**（与 IM 面同口径：「拒绝落账 ⇒ 拒绝即帧」）。
pub(crate) async fn record_device_host_decision(
    runtime: &Runtime,
    session: SessionId,
    approval: ApprovalId,
    decision: &ApprovalDecision,
) -> Result<(), CanonicalEntryError> {
    let pending = runtime
        .pending_approvals(session)
        .await
        .map_err(|error| CanonicalEntryError::InvalidRequest(error.to_string()))?;
    let Some(view) = pending.iter().find(|view| view.approval_id == approval) else {
        // 查无 pending（可能已过期/已决）⇒ 由 resolve_approval 自己报错 ⇒ 这里不拦。
        return Ok(());
    };
    let decisions =
        apeireth_tools_canonical::device::approval::DeviceApprovalDecisionLedger::under_data_dir(
            // 数据根的**单一真源**在 `apeireth_core::paths`（CLI 的 `default_panel_data_dir`
            // 与 runtime-assembly 的 `paths` 都只是转发它）⇒ 面板与 CLI 落决定账
            // 指向**同一个文件**。
            apeireth_core::paths::default_data_root(),
        );
    let approved = matches!(decision, ApprovalDecision::Approve);
    apeireth_tools_canonical::device::approval::record_host_decision_for_call(
        &decisions,
        &view.tool_call.name,
        &view.tool_call.arguments,
        approved,
        "本地审批面板",
        None,
    )
    .map_err(|error| {
        CanonicalEntryError::InvalidRequest(format!(
            "宿主决定落账失败（真因如实，不静默）⇒ 不执行该次调用: {error}"
        ))
    })?;
    Ok(())
}
