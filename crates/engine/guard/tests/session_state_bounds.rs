//! **P1-3 的接线证据**：护栏的**会话态有界**（长跑进程不再无界增长）。
//!
//! # 这个测试证明什么
//!
//! 护栏接线审计（`docs/status/guard-wiring-audit-2026-10-27.md`）的 **P1-3**：
//!
//! * 护栏按会话键持有**六张无上界的表**（`session_scopes` / `session_risk_history` /
//!   `session_behavior_summary` / `session_history` / `turn_intents` / `last_immune_signals`）
//!   ⇒ ⇒ 长跑进程里**只增不减** ⇒ ⇒ **无界增长**；
//! * `clear_session` 有实现但**生产零调用点**
//!   ⇒ ⇒（运行时**没有「会话结束」概念** ⇒ ⇒ 那是**协议决策**，护栏不能自己发明）
//!   ⇒ ⇒ 所以这里给这些表**自己**加上界：**会话 LRU** ⇒ 超限**整段驱逐**最久未用的会话。
//!
//! # 三条纪律
//!
//! 1. **整段驱逐**（一个会话的**所有**表**一起**清）⇒ ⇒ 不留「半吊子」状态；
//! 2. **LRU** ⇒ 驱逐的是**最久未用**的（**不是**任意一个）⇒ ⇒ **在用的会话不会被误杀**；
//! 3. **不改变判定结果** ⇒ ⇒ 只影响**历史状态**的可见性。

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, GovernanceHook, GovernanceRequest};
use apeireth_guard::BehaviorChainGuardHook;

/// 被跟踪会话数的上限（**与实现里的常量一致** ⇒ 改实现要同步改这里）。
const MAX_TRACKED_SESSIONS: usize = 256;

fn make_request<'a>(
    session: SessionId,
    trace: TraceId,
    round: u32,
    cap: &'a CapabilityId,
    args: &'a serde_json::Value,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: cap,
            arguments: args,
        },
        session,
        trace,
        round,
    )
}

#[tokio::test]
async fn the_tracked_session_count_stays_bounded() {
    // ⇒ 🎯 **核心**：连续用**几百个不同会话**打 ⇒ ⇒ 被跟踪会话数**不越界**。
    let hook = BehaviorChainGuardHook::new();
    let cap = CapabilityId::new("fs.read").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs" });
    let sessions = MAX_TRACKED_SESSIONS + 64;
    for _ in 0..sessions {
        let session = SessionId::new();
        let request = make_request(session, TraceId::new(), 1, &cap, &args);
        hook.evaluate_verbose(&request).await;
    }
    let tracked = hook.tracked_sessions();
    assert!(
        tracked <= MAX_TRACKED_SESSIONS,
        "被跟踪会话数 {tracked} **不该超过**上限 {MAX_TRACKED_SESSIONS}"
    );
    assert!(
        tracked > 0,
        "裁剪不该把**所有**会话都清掉（tracked = {tracked}）"
    );
}

#[tokio::test]
async fn the_least_recently_used_session_is_evicted_first() {
    // ⇒ **LRU 语义**：最久未用的那个**先被驱逐** ⇒ ⇒ **在用的**不被误杀。
    let hook = BehaviorChainGuardHook::new();
    let cap = CapabilityId::new("fs.read").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs" });
    let oldest = SessionId::new();
    let oldest_request = make_request(oldest, TraceId::new(), 1, &cap, &args);
    hook.evaluate_verbose(&oldest_request).await;
    // ⇒ 再用很多别的会话把额度占满。
    for _ in 0..MAX_TRACKED_SESSIONS + 32 {
        let request = make_request(SessionId::new(), TraceId::new(), 1, &cap, &args);
        hook.evaluate_verbose(&request).await;
    }
    // ⇒ 「最久未用」的那个**已被驱逐** ⇒ ⇒ **不该**还被跟踪。
    assert!(
        !hook.tracks_session(&oldest),
        "最久未用的会话**不该**还被跟踪（LRU 驱逐没生效？）"
    );
}

#[tokio::test]
async fn a_recently_used_session_is_not_evicted() {
    // ⇒ **反证**：**刚用过**的会话**不该**被驱逐。
    let hook = BehaviorChainGuardHook::new();
    let cap = CapabilityId::new("fs.read").unwrap();
    let args = serde_json::json!({ "path": "src/main.rs" });
    let recent = SessionId::new();
    let recent_request = make_request(recent, TraceId::new(), 1, &cap, &args);
    hook.evaluate_verbose(&recent_request).await;
    // ⇒ 再用**别的**会话填满额度（但**每次都碰一下** recent ⇒ ⇒ 它始终是「最新」）。
    for _ in 0..MAX_TRACKED_SESSIONS + 32 {
        let other = make_request(SessionId::new(), TraceId::new(), 1, &cap, &args);
        hook.evaluate_verbose(&other).await;
        let keep_alive = make_request(recent, TraceId::new(), 2, &cap, &args);
        hook.evaluate_verbose(&keep_alive).await;
    }
    // ⇒ recent 一直在被触碰 ⇒ ⇒ 它**不该**被驱逐 ⇒ ⇒ **仍在跟踪**。
    assert!(
        hook.tracks_session(&recent),
        "刚用过的会话**不该**被驱逐（LRU 语义错了？）"
    );
}
