//! 「**主体 → 会话**」归属映射的**端口**（2026-11-01 安全审计 P1 收口）。
//!
//! # 为什么端口在 runtime
//!
//! 归属映射要写进**记忆库**（`apeireth-memory`），而：
//! * `apeireth-gateway`（请求面）**不依赖** memory；
//! * `apeireth-runtime` 被 gateway / runtime-assembly / CLI **都依赖**
//!   ⇒ ⇒ 端口放 runtime ⇒ **无循环依赖**，且三方都能用。
//!
//! ⇒ 实现（持有 memory store 的那个）放**装配层**（CLI）注入 ⇒ 端口与实现分离。
//!
//! # 主体语义（**与 memory 侧一致**）
//!
//! `principal_id` = **人 / 账户级**标识（域面 `DomainUserId` = 令牌绑定的 `user_id`）。
//! ⇒ **不是**客户端署名键（令牌名 / `anonymous` / `unattributed`）
//! ⇒ ⇒ 调用方**只应在能解析出真实 `user_id` 时**登记 ⇒
////! **无主体 ⇒ 不登记**（宁可「归属不可判定」，也不要错归属 ⇒ 误删风险）。
//!
//! # 失败口径
//!
//! 写失败 ⇒ **不阻断主流程**（这是**旁路留痕**：主体忘了 ⇒ episode 归不到主体
//! ⇒ 那一维的「被遗忘权」不生效 ⇒ 由 [`crate::canonical::universal_forget`]
//! 那一侧的 `AttributionStatus` 把缺口**显式暴露**给自述面）。
//! ⇒ ⇒ 调用方**记审计**，不抛错。

use std::sync::{Arc, RwLock};

/// 归属映射的写面（实现方持有 memory store）。
pub trait PrincipalSessionRecorder: Send + Sync {
    /// 登记一条「主体 → 会话」映射（**幂等**）。
    ///
    /// `principal_id` / `session_id` 为空 ⇒ 实现方**必须拒**（fail-closed）。
    fn record(&self, principal_id: &str, session_id: &str) -> Result<(), String>;
}

/// 进程内的写面槽（**未装入 = 不登记** ⇒ 零行为变化）。
///
/// 用 `RwLock<Option<..>>`（不是 `OnceLock`）⇒ **可换指针**：
/// 装配面重装时以**最后一次**为准（与 `live_policy` 的共享槽语义一致）。
static RECORDER: RwLock<Option<Arc<dyn PrincipalSessionRecorder>>> = RwLock::new(None);

/// 装入写面实现（装配层调用；重复装入以**最后一次**为准）。
pub fn install_principal_session_recorder(recorder: Arc<dyn PrincipalSessionRecorder>) {
    let mut slot = RECORDER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(recorder);
}

/// 当前装入的写面（`None` = 未装入 ⇒ 调用方**不登记**）。
pub fn current_recorder() -> Option<Arc<dyn PrincipalSessionRecorder>> {
    let slot = RECORDER
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.clone()
}

/// 登记一条「主体 → 会话」映射（**旁路**：失败不抛错，只返回结果）。
///
/// * `principal_id` 或 `session_id` 为空 ⇒ **不登记**（返回 `Ok(false)`；
///   理由：**空主体会把所有人的 episode 归到同一主体** ⇒ 误删风险）。
/// * 未装入写面 ⇒ **不登记**（返回 `Ok(false)`）。
/// * 写失败 ⇒ 返回 `Err`（真因如实）⇒ 调用方**记审计**，**不阻断主流程**。
pub fn record_principal_session(principal_id: &str, session_id: &str) -> Result<bool, String> {
    let principal_id = principal_id.trim();
    let session_id = session_id.trim();
    if principal_id.is_empty() || session_id.is_empty() {
        return Ok(false);
    }
    let Some(recorder) = current_recorder() else {
        return Ok(false);
    };
    recorder.record(principal_id, session_id)?;
    Ok(true)
}

/// 卸下写面实现（回到「未装入 ⇒ 不登记」）。
///
/// # 为什么需要
///
/// 写面是**进程级**槽 ⇒ 测试里装入后**必须能卸下** ⇒ 否则
/// **同进程内其它用例会继承这个假写面**（跨用例污染 ⇒ 顺序敏感 ⇒ 偶发红）。
///
/// ⇒ 装配面正常流程**不需要**它（进程生命周期内装一次即可）⇒
/// ⇒ 本函数是给**测试隔离**与**热重装**用的。
pub fn clear_principal_session_recorder() {
    let mut slot = RECORDER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = None;
}
