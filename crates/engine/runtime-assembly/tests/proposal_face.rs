//! 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`) 的装配级回归:
//! 注册即闸 (开关默认关 fail-closed) / 只入账不落真面 (工作区零变化) /
//! accepted 标记不执行 / 自述提案节同账 / capabilities-matrix §0.2 口径互钉。
//!
//! 装配走真 [`ProductionModules`] 生产组装根; 提案账落位注入才注册 (注册即闸),
//! 名册 `proposals` 行按实际注册条件取值。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::proposal::ProposalConfig;
use serde_json::{json, Value};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

/// 与生产同款的最小装配: 记忆槽关闭, 工具槽默认开。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-proposal-face-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 工作区 + 数据目录 (分开落: 零变化断言盯工作区, 提案账进数据目录)。
fn workspace_and_data(tag: &str) -> (PathBuf, PathBuf) {
    let root = temp_root(tag);
    let workspace = root.join("workspace");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::write(workspace.join("a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(workspace.join("b.md"), "# b\n").unwrap();
    (workspace, data_dir)
}

/// 目录内容快照 (相对路径 + 字节)。
fn snapshot_dir(root: &PathBuf) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &PathBuf, dir: &PathBuf, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("读目录") {
            let entry = entry.expect("目录项");
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .expect("相对路径")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, std::fs::read(&path).expect("读文件")));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn capability<'a>(modules: &'a ProductionModules, id: &str) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

async fn invoke(tool: &Arc<dyn ToolCapability>, arguments: Value) -> Value {
    let call = ToolCall {
        id: "call_proposal".into(),
        name: "proposal_draft".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("structured json")
}

/// ① 注册即闸 + 开关 fail-closed: 默认配置 (提案账落位缺席) = 工具不注册、
/// 名册 `proposals` 行照实 false; 落位注入 = 注册 + 名册 true。
#[test]
fn the_switch_is_fail_closed_and_registration_follows_the_slot() {
    // 默认关 (fail-closed): env 不开 = 落位不注入 = 注册不成。
    assert!(
        ProposalConfig::from_env(".").is_none(),
        "APEIRETH_ENABLE_PROPOSALS 未开必须不装配 (fail-closed)"
    );
    let modules =
        ProductionModules::build(base_config(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, "tool.proposal_draft").is_none(),
        "默认必须不注册提案面"
    );
    let roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "proposals")
        .expect("名册必须有 proposals 行");
    assert!(!row.enabled, "默认关 = 显式未接线");

    // 落位注入 (开关开的装配结果) = 注册 + 名册真值 true (注册即闸的正向面)。
    let (_workspace, data_dir) = workspace_and_data("register");
    let mut config = base_config();
    config.proposals = Some(ProposalConfig::new(data_dir));
    let modules =
        ProductionModules::build(config.clone(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, "tool.proposal_draft").is_some(),
        "落位注入必须注册提案面"
    );
    let roster = roster_from_config(&config, true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "proposals")
        .expect("名册必须有 proposals 行");
    assert!(row.enabled, "落位注入必须报真值 true");
}

/// ② 只入账不落真面 (装配级): 经**注册后的工具面**起草提案, 工作区逐字节
/// 零变化, 真面里只有提案账。
#[tokio::test]
async fn drafting_through_the_registered_face_lands_only_in_the_ledger() {
    let (workspace, data_dir) = workspace_and_data("zero-change");
    let mut config = base_config();
    config.proposals = Some(ProposalConfig::new(data_dir.clone()));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(workspace.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let tool = capability(&modules, "tool.proposal_draft").expect("提案面已注册");
    let before = snapshot_dir(&workspace);

    let value = invoke(
        tool,
        json!({
            "action": "draft",
            "kind": "tidy",
            "title": "待整理: 两处临时文件该归档",
            "detail": "重复劳动留下的边角料。",
            "evidence": ["a.rs 临时注释", "b.md 旧段落"],
            "draft_payload": {"files": ["a.rs", "b.md"]},
        }),
    )
    .await;
    assert_eq!(value["status"], "proposed");
    assert_eq!(value["executed"], false);
    assert_eq!(
        before,
        snapshot_dir(&workspace),
        "提案不落任何真面: 工作区必须逐字节零变化"
    );
    let data_entries: Vec<String> = std::fs::read_dir(&data_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(data_entries, ["proposals.jsonl"], "真面里只有提案账");
}

/// ③ accepted 标记 (装配级): 点头 = 账面标记 accepted, `executed: false`,
/// 执行仍走既有工具面的正常审批链; 工作区零变化。
#[tokio::test]
async fn accepting_through_the_registered_face_never_executes() {
    let (workspace, data_dir) = workspace_and_data("accept");
    let mut config = base_config();
    config.proposals = Some(ProposalConfig::new(data_dir));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(workspace.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let tool = capability(&modules, "tool.proposal_draft").expect("提案面已注册");
    let drafted = invoke(
        tool,
        json!({
            "action": "draft",
            "kind": "fix_suggest",
            "title": "小错漏: 拼写修正建议",
            "detail": "一处拼写错了。",
            "evidence": ["b.md:2 拼写"],
            "draft_payload": {"files": ["b.md"]},
        }),
    )
    .await;
    let id = drafted["proposal"]["id"].as_str().expect("id").to_string();
    let before = snapshot_dir(&workspace);

    let value = invoke(
        tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;
    assert_eq!(value["status"], "accepted");
    assert_eq!(value["executed"], false);
    let note = value["execute_note"].as_str().expect("执行说明在场");
    assert!(
        note.contains("existing tool faces"),
        "执行仍走既有工具面: {note}"
    );
    assert_eq!(before, snapshot_dir(&workspace), "标记 accepted 零真面变化");
}

/// ④ 自述提案节 (装配级): `tool.self_status` 的 `proposal` 与工具面同一本
/// 提案账 —— 待审数/分态计数/最近提案读真值。
#[tokio::test]
async fn self_status_proposal_section_reads_the_same_ledger() {
    let (workspace, data_dir) = workspace_and_data("self-status");
    let mut config = base_config();
    config.proposals = Some(ProposalConfig::new(data_dir));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(workspace),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let proposal_tool = capability(&modules, "tool.proposal_draft").expect("提案面已注册");
    let drafted = invoke(
        proposal_tool,
        json!({
            "action": "draft",
            "kind": "list",
            "title": "清单: 待查项",
            "detail": "边角料清单。",
            "evidence": ["a.rs", "b.md"],
            "draft_payload": {"items": ["a.rs", "b.md"]},
        }),
    )
    .await;
    let id = drafted["proposal"]["id"].as_str().unwrap().to_string();
    invoke(
        proposal_tool,
        json!({"action": "accept", "proposal_id": id, "decided_by": "owner"}),
    )
    .await;

    let self_status = capability(&modules, "tool.self_status").expect("自述面已注册");
    let call = ToolCall {
        id: "call_self_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let frame: Value = serde_json::from_str(&result.render()).expect("structured json");
    assert_eq!(frame["proposal"]["pending"], 0);
    assert_eq!(frame["proposal"]["accepted"], 1);
    assert_eq!(frame["proposal"]["total"], 1);
    assert_eq!(frame["proposal"]["recent"][0]["id"], id);
    assert_eq!(frame["proposal"]["recent"][0]["state"], "accepted");
    assert_eq!(frame["proposal"]["reason"], Value::Null);
}

/// ⑤ 提案节缺探测口 = 显式 null + 原因 (0 装诚实, 不编待审数)。
#[tokio::test]
async fn missing_proposal_probe_is_explicit_null_with_a_reason() {
    let modules =
        ProductionModules::build(base_config(), ProductionBackends::default(), clock()).unwrap();
    let self_status = capability(&modules, "tool.self_status").expect("自述面已注册");
    let call = ToolCall {
        id: "call_self_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let frame: Value = serde_json::from_str(&result.render()).expect("structured json");
    assert_eq!(frame["proposal"]["pending"], Value::Null);
    assert_eq!(frame["proposal"]["recent"], Value::Null);
    assert!(
        frame["proposal"]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "缺探测口必须给原因: {}",
        frame["proposal"]
    );
}

/// ⑥ 口径互钉: capabilities-matrix §0.2 的提案开关行 (env 旋钮名 + 名册行名)
/// 与名册实况同真值。
#[test]
fn matrix_proposal_row_matches_the_live_roster() {
    const MATRIX: &str = include_str!("../../../../docs/03-reference/capabilities-matrix.md");
    let start = MATRIX.find("### 0.2").expect("§0.2 present");
    let end = MATRIX[start..]
        .find("\n---")
        .map(|offset| start + offset)
        .unwrap_or(MATRIX.len());
    let section = &MATRIX[start..end];
    for needle in [
        "APEIRETH_ENABLE_PROPOSALS",
        "proposals",
        "tool.proposal_draft",
    ] {
        assert!(section.contains(needle), "§0.2 缺提案开关行口径: {needle}");
    }

    let enabled = |roster: &[apeireth_tools_canonical::CapabilitySwitch], name: &str| {
        roster
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("名册缺行 {name}"))
            .enabled
    };
    let default_roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    assert!(
        !enabled(&default_roster, "proposals"),
        "落位缺席 = 照实 false"
    );
    let (_workspace, data_dir) = workspace_and_data("matrix");
    let mut wired = base_config();
    wired.proposals = Some(ProposalConfig::new(data_dir));
    let wired_roster = roster_from_config(&wired, true, false, false, false, &[]);
    assert!(
        enabled(&wired_roster, "proposals"),
        "落位注入必须报真值 true"
    );
}
