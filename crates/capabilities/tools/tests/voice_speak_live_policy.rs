//! 发声工具面 (`tool.voice_speak`) 的**宿主授权同源**行为钉子
//! (2026-11-01 安全审计 P3-6)。
//!
//! 原先错在哪: stage-1 闸 (风险映射) 查的是**工具自建**的一份策略, 与治理链
//! 正在评估的那一份**不是同一个对象** ⇒ 面板上的热撤销
//! (`PermissionPolicy::revoke`) 对这个面**无效**, 且从外面**看不出来**。
//!
//! 现在钉住的行为 (口径钉死处都断言精确值, 演进须改测试):
//! * 宿主策略装进共享槽 ⇒ **撤销 ⇒ 拒绝即帧 (不发声)**;
//! * 宿主策略**未装入** ⇒ 沿用本面原有口径 (grant 即发声) ⇒ 零回归。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_protocol::canonical::ToolCall;
use serde_json::json;

use apeireth_tools_canonical::live_policy::LivePermissionPolicySlot;
use apeireth_tools_canonical::voice::{
    fixed_clock, QuietHours, VoiceSpeakRiskMappingHook, VOICE_SPEAK_CAPABILITY_ID,
};
use apeireth_tools_canonical::{PreExecuteHook, PreExecuteRequest, PreVerdict};

// ---------------------------------------------------------------------------
// 夹具 (策略 / 槽 / 请求)
// ---------------------------------------------------------------------------

/// 宿主策略: 授权 `tool.voice_speak` (可随时热撤销)。
fn host_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            VOICE_SPEAK_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 槽: 宿主策略**尚未装入** ⇒ 各面**不表态** ⇒ 零回归。
fn empty_slot() -> LivePermissionPolicySlot {
    Arc::new(Mutex::new(None))
}

/// 槽: 装入宿主策略 (与治理链同源 ⇒ 热撤销可见)。
fn slot_with(policy: &Arc<Mutex<PermissionPolicy>>) -> LivePermissionPolicySlot {
    Arc::new(Mutex::new(Some(Arc::clone(policy))))
}

/// 热撤销 (面板 `POST /v1/panel/grants/revoke` 同款)。
fn revoke(policy: &Arc<Mutex<PermissionPolicy>>) {
    assert!(
        policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .revoke(&Permission::ExecuteTool(
                VOICE_SPEAK_CAPABILITY_ID.to_string()
            )),
        "热撤销必须真的撤掉那份授权"
    );
}

fn pre_request() -> (ToolCall, CapabilityId) {
    (
        ToolCall {
            id: "call_1".into(),
            name: "voice_speak".into(),
            arguments: json!({ "text": "一句" }),
        },
        CapabilityId::new(VOICE_SPEAK_CAPABILITY_ID).expect("capability id"),
    )
}

/// 白昼窗口 (勿扰窗口不掺和: 本文件只钉授权闸)。
fn day() -> (QuietHours, apeireth_tools_canonical::voice::QuietClock) {
    (
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    )
}

// ---------------------------------------------------------------------------
// 用例
// ---------------------------------------------------------------------------

/// ① 宿主策略**未装入** ⇒ stage-1 闸沿用本面自建口径 (grant 即发声) ⇒ 零回归。
#[test]
fn stage_one_gate_keeps_its_verdict_when_the_host_policy_is_not_installed() {
    let (quiet, clock) = day();
    let hook = VoiceSpeakRiskMappingHook::with_slot(empty_slot(), quiet, clock);
    let (call, capability) = pre_request();
    assert_eq!(
        hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow,
        "宿主策略未装入时 stage-1 必须照旧放行 (零回归)"
    );
}

/// ② 宿主撤销授权 ⇒ stage-1 拒绝即帧 (热撤销对本面生效, 不发声)。
#[test]
fn stage_one_gate_denies_after_the_host_revokes() {
    let policy = host_policy();
    let (quiet, clock) = day();
    let hook = VoiceSpeakRiskMappingHook::with_slot(slot_with(&policy), quiet, clock);
    let (call, capability) = pre_request();

    assert_eq!(
        hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow,
        "宿主已授权时 stage-1 放行"
    );

    revoke(&policy);

    let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
    let PreVerdict::Deny { reason } = verdict else {
        panic!("撤销后 stage-1 必须拒绝即帧: {verdict:?}");
    };
    assert!(reason.contains(VOICE_SPEAK_CAPABILITY_ID), "{reason}");
    assert!(reason.contains("撤销"), "{reason}");
}

/// ③ `new(policy)` 构造器: **签名不变**, 读的仍是自己那份策略。
///
/// 口径钉死: 该策略说「否」⇒ **拒绝即帧** (fail-closed, 注册即闸), 该策略说
/// 「行」⇒ 放行; 显式标了人工审批 ⇒ 停 ask 档 (同一条人工审批链)。
#[test]
fn risk_mapping_new_constructor_still_reads_the_policy_it_was_given() {
    let (call, capability) = pre_request();
    let request = PreExecuteRequest::new(&call, &capability);

    let (quiet, clock) = day();
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    let hook = VoiceSpeakRiskMappingHook::new(Arc::clone(&policy), quiet, clock);
    assert!(
        matches!(hook.pre_verdict(&request), PreVerdict::Deny { .. }),
        "构造时给的那份策略未授权 ⇒ 必须拒绝即帧 (fail-closed)"
    );

    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(Permission::ExecuteTool(
            VOICE_SPEAK_CAPABILITY_ID.to_string(),
        ));
    assert_eq!(
        hook.pre_verdict(&request),
        PreVerdict::Allow,
        "构造时给的那份策略被授予后必须放行 (证明读的是它)"
    );
}
