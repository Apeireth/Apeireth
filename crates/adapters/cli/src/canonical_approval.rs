//! 审批面指令分派 (W3.7 从 `lib.rs` 拆出: 装配根文件尺寸棘轮已到顶, 审批分派天然
//! 是独立入口)。
//!
//! 本模块只做一件事: 把一次审批决定落到运行时, 并在**真的落账** (resolve 成功) 后
//! 经装配根交付的审批社交源把该人工决定发布到神经总线 (R1b-2c: 发布是唯一真源,
//! 不再直写调质状态; `APEIRETH_ENABLE_BIO_TONE` 开关关 = 无句柄 = 零发布、零变化,
//! 与未接线逐字节一致)。

use apeireth_core::kernel::{ApprovalId, SessionId};
use apeireth_runtime::canonical::{ApprovalDecision, ApprovalResolution};

use crate::{build_canonical_runtime_from_env_with_observability, resolve_canonical_cli_approval};

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
