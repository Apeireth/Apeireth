//! 审批面指令分派 (W3.7 从 `lib.rs` 拆出: 装配根文件尺寸棘轮已到顶, 审批分派天然
//! 是独立入口)。
//!
//! 本模块只做一件事: 把一次审批决定落到运行时, 并在**真的落账** (resolve 成功) 后
//! 把该社会结果旁路推进共享调质状态 (`APEIRETH_ENABLE_BIO_TONE` 开关关 = 无句柄 =
//! 不回调、零变化, 与未接线逐字节一致)。

use apeireth_core::kernel::{ApprovalId, SessionId};
use apeireth_runtime::canonical::{ApprovalDecision, ApprovalResolution};
use apeireth_runtime_assembly::canonical::bio_tone::{
    apply_approval_social_feedback, approval_social_feedback,
};

use crate::{build_canonical_runtime_from_env_with_observability, resolve_canonical_cli_approval};

/// 审批面社会反馈源 (W3.7): 人批/人拒 = 真实社会结果; 取消 = 收回请求, 不构成
/// 社会结论 (不编事件)。只在该次审批真的落账 (resolve 成功) 后旁路推进共享调质
/// 状态 —— 开关关时句柄为 None, 逐字节零变化。
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
    let (runtime, observer, _cost_router, bio_tone) =
        build_canonical_runtime_from_env_with_observability().await?;
    let feedback = approval_social_feedback(&decision);
    let result = resolve_canonical_cli_approval(&runtime, session, approval, decision).await;
    if result.is_ok() {
        apply_approval_social_feedback(bio_tone.as_ref(), feedback);
    }
    observer.flush().await;
    result
}
