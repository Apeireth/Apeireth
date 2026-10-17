//! 通用嗓发声旋钮 (`APEIRETH_ENABLE_VOICE_SPEAK`, V3-b) 的治理接线回归:
//! 注册即闸 (默认关 fail-closed) + 发声面档 (grant 即发声, **与 shell 的
//! 「每次人批」执行档分立**, 不借壳) + 发声配置同源 (账本落点 / 勿扰窗口)。
//!
//! 覆盖: 旋钮链 (关 = 不在授权面, 拒绝即帧) / 治理档 (发声面档 vs shell
//! 执行档) / 配置同源 (spoken.jsonl 落点 + 勿扰窗口有窗)。

use std::sync::Mutex;

use apeireth_cli::{
    build_production_governance_from_env, voice_speak_config_from_env,
    voice_speak_enabled_from_env, ENABLE_VOICE_SPEAK_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernancePipeline, GovernanceRequest};
use apeireth_tools_canonical::{SPOKEN_LEDGER_FILE, VOICE_SPEAK_CAPABILITY_ID};
use serde_json::{json, Value};

/// shell 旋钮的 env 名 (未导出常量, 测试按公开口径给同名字面值)。
const SHELL_ENABLE_ENV: &str = "APEIRETH_ENABLE_SHELL";

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self { keys, previous }
    }

    fn clear(&self) {
        for key in self.keys {
            std::env::remove_var(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 旋钮相关 env 全表 (含数据目录: 发声配置的账本/勿扰接线都在数据目录下)。
fn voice_env_guard() -> (EnvGuard, tempfile::TempDir) {
    let data_dir = tempfile::tempdir().expect("tempdir");
    std::env::set_var("APEIRETH_DATA_DIR", data_dir.path());
    let guard = EnvGuard::guard(&[
        SHELL_ENABLE_ENV,
        ENABLE_VOICE_SPEAK_ENV,
        "APEIRETH_VOICE_QUIET_HOURS",
        "APEIRETH_VOICE_INTERRUPT",
        "APEIRETH_DATA_DIR",
    ]);
    guard.clear();
    std::env::set_var("APEIRETH_DATA_DIR", data_dir.path());
    (guard, data_dir)
}

async fn verdict(
    governance: &GovernancePipeline,
    capability_id: &str,
    arguments: Value,
) -> apeireth_governance::GovernanceVerdict {
    let capability = CapabilityId::new(capability_id).expect("capability id");
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate_verbose(&request).await
}

/// 旋钮 fail-closed: 默认关 = 不在授权面 (拒绝即帧); 开 = 注册 + 授权同源
/// (同一开关值同时决定装配侧注册与授权面)。
#[tokio::test]
async fn voice_speak_knob_fails_closed_and_grants_with_registration() {
    let _lock = lock_env();
    let (guard, _data) = voice_env_guard();

    guard.clear();
    assert!(!voice_speak_enabled_from_env(), "默认关 (fail-closed)");
    assert!(
        voice_speak_config_from_env().expect("config").is_none(),
        "旋钮关 = 不注入发声配置 (注册即闸)"
    );
    let governance = build_production_governance_from_env();
    let result = verdict(
        &governance,
        VOICE_SPEAK_CAPABILITY_ID,
        json!({ "text": "一句" }),
    )
    .await;
    assert!(
        matches!(result.decision, Decision::Deny { .. }),
        "未开旋钮必须拒绝即帧 (fail-closed): {:?}",
        result.decision
    );

    std::env::set_var(ENABLE_VOICE_SPEAK_ENV, "1");
    assert!(voice_speak_enabled_from_env(), "开 = 生效");
    let config = voice_speak_config_from_env()
        .expect("config resolves")
        .expect("旋钮开 = 注入发声配置");
    assert_eq!(
        config
            .ledger_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some(SPOKEN_LEDGER_FILE),
        "语句账本落点同源 (数据目录 spoken.jsonl)"
    );
    assert!(
        config.quiet.window().is_some(),
        "勿扰窗口有窗 (fail-closed 朝安静)"
    );

    let governance = build_production_governance_from_env();
    let result = verdict(
        &governance,
        VOICE_SPEAK_CAPABILITY_ID,
        json!({ "text": "一句" }),
    )
    .await;
    assert!(
        matches!(result.decision, Decision::Allow),
        "发声面档: grant 即发声 (免逐次人批): {:?}",
        result.decision
    );
}

/// 治理档分立 (不借壳): shell = grant + require_approval (每次人批的执行档);
/// voice_speak = grant 即发声 (发声面档) —— 两个能力不同档, 各走各的裁决。
#[tokio::test]
async fn voice_speak_tier_is_distinct_from_the_shell_execution_tier() {
    let _lock = lock_env();
    let (guard, _data) = voice_env_guard();
    guard.clear();
    std::env::set_var(SHELL_ENABLE_ENV, "1");
    std::env::set_var(ENABLE_VOICE_SPEAK_ENV, "1");

    let governance = build_production_governance_from_env();

    let shell = verdict(&governance, "tool.shell", json!({ "command": "echo hi" })).await;
    let Decision::RequireApproval { .. } = shell.decision else {
        panic!("shell 是每次人批的执行档: {:?}", shell.decision);
    };

    let voice = verdict(
        &governance,
        VOICE_SPEAK_CAPABILITY_ID,
        json!({ "text": "一句" }),
    )
    .await;
    assert!(
        matches!(voice.decision, Decision::Allow),
        "发声面档不借 shell 的审批档: {:?}",
        voice.decision
    );
}
