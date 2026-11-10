//! **主体 → 会话**归属映射的请求面接线（2026-11-01 安全审计 P1 收口）。
//!
//! # 为什么要有这个接线
//!
//! [`apeireth_memory::universal_forget`] 的「被遗忘权」在 **episode 维度**要成立，
//! 必须能回答「这个 episode 属于**哪个主体**」⇒ 靠
//! `memory_principal_sessions`（主体 → 会话 映射）连接
//! （`episodes.session_id` ⇄ 映射表的 `session_id` ⇒ `principal_id`）。
//!
//! ⇒ **生产侧原先没有写入方** ⇒ episode **归不到主体**
//! ⇒ ⇒ 那一维的「被遗忘权」**无法生效**（且旧返回体里看不见 —— `episode: true`
//! 曾是常量）⇒ ⇒ 现在由 `AttributionStatus` 把缺口**显式暴露**。
//!
//! # 主体从哪来
//!
//! **域面 `user_id`**（令牌条目绑定的 `user_id` = 人 / 账户级标识）。
//!
//! ⇒ **不是**客户端署名键（`ClientIdentity::client`：令牌名 / `anonymous` / `unattributed`）
//! ⇒ ⇒ **无 `user_id` ⇒ 不登记**（宁可「归属不可判定」，也不要错归属 ⇒ 误删风险）。
//!
//! # 失败口径
//!
//! 写失败 ⇒ **不阻断主流程**（这是**旁路留痕**）⇒ 只记审计。

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::principal_sessions;

use crate::client_tokens::ClientIdentity;
use crate::panels::GatewayState;

/// 登记本次会话的**主体归属**（**旁路**：失败不阻断请求）。
///
/// # 主体取自哪里（**这是本函数的要害**）
///
/// 主体 = **令牌条目绑定的 `user_id`**（`state.client_tokens.entry_for(client).user_id`）。
///
/// ⇒ **不是**域 id（`DomainContext::user_id()`）—— 因为：
/// * 令牌**有** `user_id` 绑定 ⇒ 域 = 该 user_id 的域（碰巧一样）；
/// * 令牌**无** `user_id` 绑定 ⇒ 域解析**回落到默认域** ⇒ `user_id()` = **"default"**
///   ⇒ ⇒ 若拿它当主体 ⇒ **所有无绑定请求都归到 `"default"` 一个主体**
///   ⇒ ⇒ 不同用户的 episode 混在一起 ⇒ ⇒ **误删风险**。
///
/// ⇒ ⇒ **无 `user_id` 绑定 ⇒ 不登记**（fail-closed：宁可归属不可判定，也不要错归属）。
///
/// # 其它不登记的情形
///
/// * 未装入写面实现 ⇒ 不登记（返回 `false`）；
/// * 没有客户端身份 ⇒ 不登记。
///
/// # 失败口径
///
/// 写失败 ⇒ 返回 `Err`（真因如实）⇒ 调用方**记审计**，**不阻断主流程**。
pub fn record_principal_attribution(
    state: &GatewayState,
    identity: Option<&ClientIdentity>,
    session: SessionId,
) -> Result<bool, String> {
    let Some(identity) = identity else {
        return Ok(false);
    };
    let Some(spec) = state.client_tokens.entry_for(&identity.client) else {
        return Ok(false);
    };
    let Some(user_id) = spec.user_id.as_deref() else {
        // **无 user_id 绑定 ⇒ 不登记**（理由见上）。
        return Ok(false);
    };
    if user_id.trim().is_empty() {
        return Ok(false);
    }
    // `SessionId` 是 `generated_id!` 生成的（**没有** `as_str()`）⇒ 用 `Display`。
    principal_sessions::record_principal_session(user_id, &session.to_string())
}
