//! CLI 侧的**被遗忘权**实现（按主体全量遗忘）。
//!
//! # 语义
//!
//! 走治理存储的 `forget_principal`（[`apeireth_memory::MemoryGovernanceStore`]）——
//! **事务语义在 memory crate 里**，本模块只做「窄口 → 全口」的转发，
//! 不复制任何存储逻辑。
//!
//! # 为什么没注入治理存储就报未接线
//!
//! 返回明确错误而不是静默成功 —— 静默成功会让「删了用户但记忆还在」
//! 这件事**不可见**（调用方以为清干净了）。

use apeireth_gateway::PrincipalForgetDto;
use apeireth_memory::MemoryGovernanceStore;

use crate::gateway_panels::CliPanelData;

/// 按主体全量遗忘（被遗忘权）。
///
/// 一个事务清 5 账（episode / preference / commitment / temporal / persona）；
/// protected 人格记录 ⇒ **整体拒绝**（不静默跳过）。
pub async fn forget_principal(
    governance: Option<&std::sync::Arc<dyn MemoryGovernanceStore>>,
    principal_id: &str,
    reason: Option<&str>,
) -> Result<PrincipalForgetDto, String> {
    let Some(governance) = governance else {
        return Err("memory.forget_principal 未接线（未注入治理存储）".to_string());
    };
    let at_ms = chrono::Utc::now().timestamp_millis();
    let outcome = governance
        .forget_principal(principal_id, at_ms, reason)
        .map_err(|error| error.to_string())?;
    Ok(PrincipalForgetDto {
        principal_id: principal_id.to_string(),
        episode: outcome.episode,
        preference: outcome.preference,
        commitment: outcome.commitment,
        temporal: outcome.temporal,
        persona: outcome.persona,
        refused: false,
        reason: None,
    })
}

/// `CliPanelData` 的 `PanelData::forget_principal` 实现（转发到本模块）。
pub async fn cli_forget_principal(
    data: &CliPanelData,
    principal_id: &str,
    reason: Option<&str>,
) -> Result<PrincipalForgetDto, String> {
    forget_principal(data.governance.as_ref(), principal_id, reason).await
}
