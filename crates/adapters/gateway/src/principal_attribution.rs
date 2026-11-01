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

/// 登记本次会话的**主体归属**（**旁路**：失败不阻断请求）。
///
/// * 无域上下文 / 无 `user_id` ⇒ **不登记**（返回 `false`）。
/// * 未装入写面实现 ⇒ **不登记**（返回 `false`）。
/// * 写失败 ⇒ 返回 `Err`（真因如实）⇒ 调用方记审计。
pub fn record_principal_attribution(
    domain: Option<&crate::domain_context::DomainContext>,
    session: SessionId,
) -> Result<bool, String> {
    let Some(context) = domain else {
        return Ok(false);
    };
    let Some(user_id) = context.user_id() else {
        return Ok(false);
    };
    if user_id.trim().is_empty() {
        return Ok(false);
    }
    // `SessionId` 是 `generated_id!` 生成的（**没有** `as_str()`）⇒ 用 `Display`。
    principal_sessions::record_principal_session(user_id, &session.to_string())
}
