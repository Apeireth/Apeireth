//! V5-b 暴露面体检 (`tool.exposure_audit`) 名册接线的治理接线回归: 注册即闸
//! (`APEIRETH_ENABLE_EXPOSURE_AUDIT`, 默认关 fail-closed) + 只读档 (grant 即用
//! 零审批, 与 `tool.self_status` 同档)。
//!
//! 覆盖: 旋钮 fail-closed (关 = 不注册 + 拒绝即帧 + 名册照实 false) / 旋钮开
//! = 注册 + 授权同源 (名册行 = 实际注册条件取值) / 工具真能跑 (第一律: 能力
//! 真能用, 只读体检端到端出报告)。
//! `std::env` 是进程全局, 全部用例串在一把锁后。

use std::sync::Mutex;

use apeireth_cli::{
    build_canonical_runtime_from_env, build_production_governance_from_env,
    exposure_audit_config_from_env, exposure_audit_enabled_from_env, ENABLE_EXPOSURE_AUDIT_ENV,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_runtime::canonical::Runtime;
use apeireth_tools_canonical::EXPOSURE_AUDIT_CAPABILITY_ID;
use serde_json::Value;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const GUARDED_KEYS: &[&str] = &[
    "APEIRETH_SESSION_DB",
    "APEIRETH_COGNITIVE_DB",
    "APEIRETH_DATA_DIR",
    "APEIRETH_REFLEXION_DIR",
    ENABLE_EXPOSURE_AUDIT_ENV,
];

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard() -> Self {
        let previous = GUARDED_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self {
            keys: GUARDED_KEYS,
            previous,
        }
    }

    fn clear_all(&self) {
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

fn temp_path(label: &str, name: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "apeireth-exposure-knob-{label}-{}-{name}",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

async fn build_runtime(label: &str) -> Runtime {
    std::env::set_var("APEIRETH_SESSION_DB", temp_path(label, "session.sqlite3"));
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        temp_path(label, "cognitive.sqlite3"),
    );
    std::env::set_var("APEIRETH_REFLEXION_DIR", temp_path(label, "reflexion"));
    std::env::set_var("APEIRETH_DATA_DIR", temp_path(label, "data"));
    build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network")
}

fn tool_ids(runtime: &Runtime) -> Vec<String> {
    runtime
        .tools()
        .iter()
        .map(|tool| tool.id().as_str().to_string())
        .collect()
}

async fn governance_verdict(capability_id: &str) -> Decision {
    let governance = build_production_governance_from_env();
    let capability = CapabilityId::new(capability_id).expect("capability id");
    let arguments = serde_json::json!({});
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate_verbose(&request).await.decision
}

async fn read_roster(runtime: &Runtime) -> Value {
    let tool = runtime
        .capability_registry()
        .entries()
        .into_iter()
        .map(|(_owner, capability)| capability)
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .expect("tool.self_status must be registered");
    let call = ToolCall {
        id: "call_self_status".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result: ToolResult = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value =
        serde_json::from_str(&result.render()).expect("structured self-report json");
    report["capabilities"].clone()
}

/// 旋钮 fail-closed + 注册即闸 (开关开 = 注册 + 授权同源): 默认关 = 工具不
/// 注册 + 拒绝即帧 + 名册照实 false; 开 = 注册 + grant + 名册照实 true。
#[tokio::test]
async fn exposure_audit_knob_fails_closed_and_grants_with_registration() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();

    assert!(!exposure_audit_enabled_from_env(), "默认关 (fail-closed)");
    assert!(
        exposure_audit_config_from_env()
            .expect("config resolves")
            .is_none(),
        "旋钮关 = 不注入体检配置 (注册即闸)"
    );
    let runtime = build_runtime("off").await;
    assert!(
        !tool_ids(&runtime).contains(&EXPOSURE_AUDIT_CAPABILITY_ID.to_string()),
        "旋钮关 = 工具不注册"
    );
    let roster = read_roster(&runtime).await;
    assert_eq!(roster["exposure_audit"], false, "名册照实 false");
    let decision = governance_verdict(EXPOSURE_AUDIT_CAPABILITY_ID).await;
    assert!(
        matches!(decision, Decision::Deny { .. }),
        "未开旋钮必须拒绝即帧 (fail-closed): {decision:?}"
    );

    std::env::set_var(ENABLE_EXPOSURE_AUDIT_ENV, "1");
    assert!(exposure_audit_enabled_from_env(), "开 = 生效");
    assert!(
        exposure_audit_config_from_env()
            .expect("config resolves")
            .is_some(),
        "旋钮开 = 注入体检配置"
    );
    let runtime = build_runtime("on").await;
    assert!(
        tool_ids(&runtime).contains(&EXPOSURE_AUDIT_CAPABILITY_ID.to_string()),
        "旋钮开 = 工具注册"
    );
    let roster = read_roster(&runtime).await;
    assert_eq!(roster["exposure_audit"], true, "名册照实 true");
    let decision = governance_verdict(EXPOSURE_AUDIT_CAPABILITY_ID).await;
    assert!(
        matches!(decision, Decision::Allow),
        "只读档: grant 即用 (零审批): {decision:?}"
    );
}

/// 只读档 = 零审批: 判决不许是 RequireApproval (与 `tool.self_status` 同档)。
#[tokio::test]
async fn exposure_audit_is_read_only_with_zero_approval() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var(ENABLE_EXPOSURE_AUDIT_ENV, "1");

    let decision = governance_verdict(EXPOSURE_AUDIT_CAPABILITY_ID).await;
    assert!(
        !matches!(decision, Decision::RequireApproval { .. }),
        "只读体检不该停人工审批: {decision:?}"
    );
    assert!(decision.is_allowed(), "只读档 grant 即用: {decision:?}");
}

/// 第一律 (能力真能用): 工具面真能跑出体检报告 —— 只读自查端到端出
/// `apeireth.exposure_report` 报告, 测不了的项显式「未测」不冒充干净。
#[tokio::test]
async fn exposure_audit_tool_runs_end_to_end_and_reports_honestly() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var(ENABLE_EXPOSURE_AUDIT_ENV, "1");
    let runtime = build_runtime("run").await;

    let tool = runtime
        .capability_registry()
        .entries()
        .into_iter()
        .map(|(_owner, capability)| capability)
        .find(|capability| capability.id().as_str() == EXPOSURE_AUDIT_CAPABILITY_ID)
        .expect("tool.exposure_audit must be registered");
    let call = ToolCall {
        id: "call_exposure".into(),
        name: "exposure_audit".into(),
        arguments: serde_json::json!({}),
    };
    let result: ToolResult = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).expect("exposure report json");
    assert_eq!(report["schema"], "apeireth.exposure_report");
    let items = report["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "体检项不许空报");
    let external = items
        .iter()
        .find(|item| item["id"] == "external_surface")
        .expect("外网暴露面项在场");
    assert_eq!(
        external["status"], "not_measured",
        "本地自查测不了的项必须显式未测, 不冒充干净"
    );
    assert!(report["overall"].is_object(), "总评对象在场");
}
