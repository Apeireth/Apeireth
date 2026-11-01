//! 审批面指令分派 (W3.7 从 `lib.rs` 拆出: 装配根文件尺寸棘轮已到顶, 审批分派天然
//! 是独立入口)。
//!
//! 本模块只做一件事: 把一次审批决定落到运行时, 并在**真的落账** (resolve 成功) 后
//! 经装配根交付的审批社交源把该人工决定发布到神经总线 (R1b-2c: 发布是唯一真源,
//! 不再直写调质状态; `APEIRETH_ENABLE_BIO_TONE` 开关关 = 无句柄 = 零发布、零变化,
//! 与未接线逐字节一致)。

use apeireth_core::kernel::{ApprovalId, SessionId};
use apeireth_runtime::canonical::{ApprovalDecision, ApprovalResolution};

use crate::{
    build_canonical_runtime_from_env_with_observability, default_panel_data_dir,
    resolve_canonical_cli_approval,
};
use apeireth_runtime::Runtime;

/// 审批面决定分派: 人批/人拒 = 真实社会结果 (发布 `ApprovalSocial` 信号);
/// 取消 = 收回请求, 未形成社会结论 (不发布、不编事件)。
///
/// 只在**该次审批真的落账** (resolve 成功) 后发布 —— 开关关时句柄为 `None`,
/// 逐字节零变化; 发布不改审批结论、不参与权限判定。
pub async fn dispatch_canonical_approval(
    session: String,
    approval: String,
    decision: ApprovalDecision,
) -> Result<ApprovalResolution, String> {
    let session = session
        .parse::<SessionId>()
        .map_err(|error| error.to_string())?;
    let approval = approval
        .parse::<ApprovalId>()
        .map_err(|error| error.to_string())?;
    // 审批延续回合沿用原请求的模型 (模型已在原回合定好), 本路径不重路由。
    let (runtime, observer, _cost_router, bio_approval) =
        build_canonical_runtime_from_env_with_observability().await?;
    let result =
        resolve_canonical_cli_approval(&runtime, session, approval, decision.clone()).await;
    if result.is_ok() {
        if let Some(source) = bio_approval.as_ref() {
            // 投递数由订阅者决定; 发布事实本身已在总线记账 (取消不发)。
            let _ = source.publish(&decision);
        }
    }
    observer.flush().await;
    result
}

/// 宿主拍板 ⇒ 设备决定账落账（**不是设备面调用则什么都不做**）。
///
/// 设备面的审批发生在 `execute` 这一拍 ⇒ pending approval 的工具调用参数
/// **自带 `plan_id`** ⇒ 直接取用、不重算（重算要设备表 + 分级词表，易与执行面漂移）。
pub async fn record_device_host_decision(
    runtime: &Runtime,
    session: SessionId,
    approval: ApprovalId,
    decision: &ApprovalDecision,
) -> Result<(), String> {
    let pending = runtime
        .pending_approvals(session)
        .await
        .map_err(|error| error.to_string())?;
    let Some(view) = pending.iter().find(|view| view.approval_id == approval) else {
        // 查无 pending（可能已过期/已决）⇒ 由 resolve_approval 自己报错 ⇒ 这里不拦。
        return Ok(());
    };
    let decisions =
        apeireth_tools_canonical::device::approval::DeviceApprovalDecisionLedger::under_data_dir(
            default_panel_data_dir(),
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
    .map_err(|error| format!("宿主决定落账失败（真因如实，不静默）⇒ 不执行该次调用: {error}"))?;
    Ok(())
}
