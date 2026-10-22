//! V4-a / V4-b 生产接线旋钮 (`APEIRETH_ENABLE_PERSONA_CONSOLIDATION` /
//! `APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE`) 的治理接线回归: 注册即闸
//! (默认关 fail-closed) + 随整编跑 (整编触发点缺席 = 显式不跑, 名册照实 false)。
//!
//! 覆盖: 旋钮 fail-closed (关 = 显式未接线) / 旋钮开 = 接线生效真值 (名册行
//! = 实际注册条件取值) / 上游槽 (整编) 缺席时静默失效照实报。
//! `std::env` 是进程全局, 全部用例串在一把锁后。

use std::sync::Mutex;

use apeireth_cli::{
    build_canonical_runtime_from_env, consolidation_hash_gate_enabled_from_env,
    persona_consolidation_enabled_from_env, ENABLE_CONSOLIDATION_HASH_GATE_ENV,
    ENABLE_PERSONA_CONSOLIDATION_ENV,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_runtime::canonical::Runtime;
use serde_json::Value;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const GUARDED_KEYS: &[&str] = &[
    "APEIRETH_SESSION_DB",
    "APEIRETH_DATA_DIR",
    "APEIRETH_COGNITIVE_DB",
    "APEIRETH_REFLEXION_DIR",
    "APEIRETH_ENABLE_CONSOLIDATION",
    ENABLE_PERSONA_CONSOLIDATION_ENV,
    ENABLE_CONSOLIDATION_HASH_GATE_ENV,
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
            "apeireth-consolidation-knobs-{label}-{}-{name}",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

async fn build_runtime(label: &str) -> Runtime {
    std::env::set_var("APEIRETH_SESSION_DB", temp_path(label, "session.sqlite3"));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    std::env::set_var("APEIRETH_DATA_DIR", temp_path(label, "data"));
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        temp_path(label, "cognitive.sqlite3"),
    );
    std::env::set_var("APEIRETH_REFLEXION_DIR", temp_path(label, "reflexion"));
    build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network")
}

/// 能力名册现值 (与 `tool.self_status` 自省通道同一真值: 实际注册条件取值)。
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

/// 文档随行断言 (宪章 §四.5): user-manual 旋钮表写明本批四缝的旋钮名与
/// 名册行口径 (能力性判断以代码为准, 旋钮名/档位必须对得上)。
#[test]
fn user_manual_documents_the_production_wiring_knobs() {
    const MANUAL: &str = include_str!("../../../../docs/02-guides/user-manual.md");
    for needle in [
        "APEIRETH_ENABLE_PERSONA_CONSOLIDATION",
        "APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE",
        "APEIRETH_ENABLE_EXPOSURE_AUDIT",
        "persona_consolidation",
        "consolidation_hash_gate",
        "voice_emotion_tune",
        "exposure_audit",
    ] {
        assert!(MANUAL.contains(needle), "user-manual 缺旋钮口径: {needle}");
    }
}

/// V4-a 人格提取步旋钮 fail-closed: 默认关 = 显式未接线 (名册照实 false);
/// 开 = 接线生效真值 (名册 `persona_consolidation` 行 true, 随整编跑)。
#[tokio::test]
async fn persona_consolidation_knob_fails_closed_and_wires_with_the_consolidation_trigger() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var("APEIRETH_ENABLE_CONSOLIDATION", "1");

    assert!(
        !persona_consolidation_enabled_from_env(),
        "默认关 (fail-closed)"
    );
    let runtime = build_runtime("persona-off").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["persona_consolidation"], false,
        "旋钮关 = 显式未接线 (名册照实 false)"
    );

    std::env::set_var(ENABLE_PERSONA_CONSOLIDATION_ENV, "1");
    assert!(persona_consolidation_enabled_from_env(), "开 = 生效");
    let runtime = build_runtime("persona-on").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["persona_consolidation"], true,
        "旋钮开 + 整编开 = 接线生效真值"
    );
}

/// V4-a 人格提取步**随整编跑**: 整编触发点缺席时提取步显式不跑 (上游槽
/// 静默失效, 名册照实 false) —— 单独开本旋钮不产生假接线。
#[tokio::test]
async fn persona_consolidation_stays_inert_without_the_consolidation_trigger() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var(ENABLE_PERSONA_CONSOLIDATION_ENV, "1");

    assert!(persona_consolidation_enabled_from_env(), "旋钮开");
    let runtime = build_runtime("persona-no-consolidation").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["persona_consolidation"], false,
        "整编关 = 提取步显式不跑, 名册照实 false"
    );
}

/// V4-b 整编散列门旋钮 fail-closed: 默认关 = 显式未接线; 开 = 接线生效真值。
#[tokio::test]
async fn consolidation_hash_gate_knob_fails_closed_and_wires_with_the_consolidation_trigger() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var("APEIRETH_ENABLE_CONSOLIDATION", "1");

    assert!(
        !consolidation_hash_gate_enabled_from_env(),
        "默认关 (fail-closed)"
    );
    let runtime = build_runtime("hash-off").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["consolidation_hash_gate"], false,
        "旋钮关 = 显式未接线 (名册照实 false)"
    );

    std::env::set_var(ENABLE_CONSOLIDATION_HASH_GATE_ENV, "1");
    assert!(consolidation_hash_gate_enabled_from_env(), "开 = 生效");
    let runtime = build_runtime("hash-on").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["consolidation_hash_gate"], true,
        "旋钮开 + 整编开 = 接线生效真值"
    );
}

/// V4-b 整编散列门**随整编跑**: 整编触发点缺席时散列门显式不跑 (名册照实
/// false) —— 单独开本旋钮不产生假接线。
#[tokio::test]
async fn consolidation_hash_gate_stays_inert_without_the_consolidation_trigger() {
    let _lock = lock_env();
    let guard = EnvGuard::guard();
    guard.clear_all();
    std::env::set_var(ENABLE_CONSOLIDATION_HASH_GATE_ENV, "1");

    assert!(consolidation_hash_gate_enabled_from_env(), "旋钮开");
    let runtime = build_runtime("hash-no-consolidation").await;
    let roster = read_roster(&runtime).await;
    assert_eq!(
        roster["consolidation_hash_gate"], false,
        "整编关 = 散列门显式不跑, 名册照实 false"
    );
}
