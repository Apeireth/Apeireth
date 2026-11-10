//! 问责闭环第 ③ 步 · 自动回填的**端到端**（CLI 显式登记 → 运行时面回填 → 报告可见）。
//!
//! 链路：**已声明影响 → `apeireth egress bind` 登记归属 → 工具调用失败 →
//! 账目变 `failed` → `apeireth egress impact` 显示 `后果=failed`**。
//!
//! 覆盖的红线：
//! - 工具**成功** ⇒ 后果**不变**（绝不自动填 `confirmed`）；
//! - **没登记** ⇒ 不动（不编造归属）；
//! - **没有已声明影响** ⇒ `bind` 拒（归属只能指向已声明的影响）；
//! - 人工回填（`egress outcome`）**不受影响**。

use std::path::{Path, PathBuf};

use apeireth_cli::egress_command::run_egress_at;
use apeireth_protocol::canonical::{ToolOutcome, ToolResult};
use apeireth_runtime_assembly::accountability::{
    AccountabilityLedger, Affected, Boundary, ImpactStatement, Outcome,
};
use apeireth_runtime_assembly::consequence_backfill::ConsequenceBackfillModule;

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "apeireth-egress-bind-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).expect("mkdir");
    root
}

/// 落一条已声明影响（行动 id = `act-1`）。
fn declare_impact(root: &Path) {
    let ledger = AccountabilityLedger::new(Some(root.join("impact-ledger.jsonl")));
    assert!(
        ledger.append(&ImpactStatement {
            action_id: "act-1".into(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::UserToOrg,
            facts_used: Vec::new(),
            inferences: Vec::new(),
            grants: Vec::new(),
            affected: Affected::NamedParty,
            reversible: false,
            destination: "smtp:hr@example.com".into(),
            outcome: Outcome::Unknown,
        }),
        "影响声明应落盘"
    );
}

fn failed(call_id: &str) -> ToolResult {
    ToolResult {
        tool_call_id: call_id.to_string(),
        name: Some("shell.exec".into()),
        outcome: ToolOutcome::Error {
            message: "boom".into(),
            retryable: false,
        },
    }
}

fn succeeded(call_id: &str) -> ToolResult {
    ToolResult::ok(call_id, serde_json::json!({"sent": true}))
}

fn args(verb: &str, root: &Path, rest: &[&str]) -> Vec<String> {
    let mut out = vec![verb.to_string()];
    out.extend(rest.iter().map(|value| value.to_string()));
    out.push("--path".to_string());
    out.push(root.join("impact-ledger.jsonl").display().to_string());
    out
}

/// 端到端：工具失败 ⇒ 账目变 `failed`，报告面看得见。
#[test]
fn bind_then_tool_failure_turns_the_impact_into_failed() {
    let root = temp_root("e2e");
    declare_impact(&root);

    // ① 登记前：后果未知。
    let before = run_egress_at(&root, &args("impact", &root, &[])).expect("impact");
    assert!(before.contains("后果未知=1"), "{before}");

    // ② 显式登记归属（调用方知道两个 id；系统不推断）。
    let bind = run_egress_at(&root, &args("bind", &root, &["act-1", "call_1"])).expect("bind");
    assert!(bind.contains("已登记归属"), "{bind}");

    // ③ 工具失败 ⇒ 自动回填（运行时面）。
    let module = ConsequenceBackfillModule::from_data_root(&root);
    assert_eq!(
        module.observe_tool_result(&failed("call_1")).as_deref(),
        Some("act-1")
    );

    // ④ 报告面：后果=failed、未知=0。
    let after = run_egress_at(&root, &args("impact", &root, &[])).expect("impact");
    assert!(after.contains("后果=failed"), "{after}");
    assert!(after.contains("后果未知=0"), "{after}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 工具**成功** ⇒ 后果**不变**（绝不自动填 `confirmed`）。
#[test]
fn bind_then_tool_success_leaves_the_impact_unknown() {
    let root = temp_root("success");
    declare_impact(&root);
    run_egress_at(&root, &args("bind", &root, &["act-1", "call_1"])).expect("bind");

    let module = ConsequenceBackfillModule::from_data_root(&root);
    assert_eq!(module.observe_tool_result(&succeeded("call_1")), None);

    let report = run_egress_at(&root, &args("impact", &root, &[])).expect("impact");
    assert!(report.contains("后果=unknown"), "{report}");
    assert!(report.contains("后果未知=1"), "{report}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 没有登记 ⇒ **不动**（不编造归属）。
#[test]
fn unregistered_tool_failure_changes_nothing() {
    let root = temp_root("unregistered");
    declare_impact(&root);

    let module = ConsequenceBackfillModule::from_data_root(&root);
    assert_eq!(module.observe_tool_result(&failed("call_1")), None);

    let report = run_egress_at(&root, &args("impact", &root, &[])).expect("impact");
    assert!(report.contains("后果=unknown"), "{report}");
    assert!(report.contains("后果未知=1"), "{report}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 归属只能指向**已声明**的影响：查不到 ⇒ 拒（不编造归属）。
#[test]
fn bind_refuses_an_action_without_a_declared_impact() {
    let root = temp_root("no-declaration");
    declare_impact(&root);

    let error = run_egress_at(&root, &args("bind", &root, &["act-404", "call_1"]))
        .expect_err("没有已声明影响 ⇒ 拒");
    assert!(error.contains("没有已声明的影响"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 1:1：一次工具调用只能对应一条已声明影响（冲突 ⇒ 拒）。
#[test]
fn bind_refuses_breaking_the_one_to_one_mapping() {
    let root = temp_root("one-to-one");
    declare_impact(&root);

    run_egress_at(&root, &args("bind", &root, &["act-1", "call_1"])).expect("bind");
    let error = run_egress_at(&root, &args("bind", &root, &["act-1", "call_2"]))
        .expect_err("一条影响只能对应一次工具调用");
    assert!(error.contains("1:1"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 人工回填（`egress outcome`）**不受影响**（不回归）。
#[test]
fn manual_outcome_backfill_still_works() {
    let root = temp_root("manual");
    declare_impact(&root);

    let outcome = run_egress_at(
        &root,
        &[
            "outcome".to_string(),
            "act-1".to_string(),
            "confirmed".to_string(),
            "--path".to_string(),
            root.join("consequence-ledger.jsonl").display().to_string(),
        ],
    )
    .expect("人工回填应成功");
    assert!(outcome.contains("已回填"), "{outcome}");

    let report = run_egress_at(&root, &args("impact", &root, &[])).expect("impact");
    assert!(report.contains("后果=confirmed"), "{report}");
    assert!(report.contains("后果未知=0"), "{report}");
    let _ = std::fs::remove_dir_all(&root);
}
