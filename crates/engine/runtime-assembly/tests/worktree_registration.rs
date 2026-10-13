//! 干活能力轴 P2 (worktree_plan / worktree_apply) 的装配级接线证明 (总装批件一)。
//!
//! 覆盖: 注册条件 (开关 + `file_write` 唯一写总闸 + 工作区根, 注册即闸) /
//! 依赖 fail-closed (子依赖写总闸, 总闸关 = 不注册不生效) / 自省通道名册两行
//! 照实报告生效值 / 默认关 opt-in。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::{WORKTREE_APPLY_CAPABILITY_ID, WORKTREE_PLAN_CAPABILITY_ID};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn registered_ids(modules: &ProductionModules) -> Vec<String> {
    modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect()
}

fn has_both(ids: &[String]) -> bool {
    ids.iter().any(|id| id == WORKTREE_PLAN_CAPABILITY_ID)
        && ids.iter().any(|id| id == WORKTREE_APPLY_CAPABILITY_ID)
}

/// 注册条件 (注册即闸): 两级工具只有在**开关开 + `file_write` 唯一写总闸开 +
/// 工作区根已注入**时才进名册; 默认关 / 总闸关 / 无根三种情况都不注册。
#[tokio::test]
async fn production_registers_the_worktree_tools_only_under_the_file_write_gate() {
    // 三条件齐备 = 两级都注册 (模型可见)。
    let mut on = base_config();
    on.file_write = true;
    on.worktree_plan = true;
    on.worktree_apply = true;
    let modules = ProductionModules::build(
        on,
        ProductionBackends {
            workspace_root: Some(PathBuf::from(".")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("assembly builds");
    let ids = registered_ids(&modules);
    assert!(
        has_both(&ids),
        "开关 + 写总闸 + 工作区根齐备时两级工具必须注册: {ids:?}"
    );

    // 默认关 (opt-in): 不开开关 = 不注册。
    let off = ProductionModules::build(
        base_config(),
        ProductionBackends {
            workspace_root: Some(PathBuf::from(".")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("assembly builds");
    assert!(!has_both(&registered_ids(&off)), "默认必须 fail-closed");

    // 依赖 fail-closed (子依赖主): 开关开但写总闸 `file_write` 关 = 不注册。
    let mut gate_off = base_config();
    gate_off.worktree_plan = true;
    gate_off.worktree_apply = true;
    assert!(!gate_off.file_write, "默认写总闸必须关");
    let modules = ProductionModules::build(
        gate_off,
        ProductionBackends {
            workspace_root: Some(PathBuf::from(".")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("assembly builds");
    assert!(
        !has_both(&registered_ids(&modules)),
        "写总闸关时两级工具不得注册 (注册即闸)"
    );

    // 无工作区根 = 本地写工具没注册成 (与 filesystem/apply_patch 同口径)。
    let mut no_root = base_config();
    no_root.file_write = true;
    no_root.worktree_plan = true;
    no_root.worktree_apply = true;
    let modules = ProductionModules::build(no_root, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(
        !has_both(&registered_ids(&modules)),
        "无工作区根时两级工具不得注册"
    );
}

/// 自省通道名册两行: `worktree_plan` / `worktree_apply` 照实报告生效值 ——
/// 写总闸没开时两行都 false (不复述"配置说开"而注册没成的事实)。
#[tokio::test]
async fn roster_reports_the_worktree_switches_from_the_effective_config() {
    async fn roster_flags(config: ProductionModulesConfig) -> serde_json::Value {
        let backends = ProductionBackends {
            workspace_root: Some(PathBuf::from(".")),
            ..ProductionBackends::default()
        };
        let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
        let tool = modules
            .capabilities()
            .iter()
            .find(|capability| capability.id().as_str() == "tool.self_status")
            .cloned()
            .expect("tool.self_status must be registered");
        let call = ToolCall {
            id: "call_roster".into(),
            name: "self_status".into(),
            arguments: serde_json::json!({}),
        };
        let result = tool.invoke(&call).await;
        assert!(result.is_ok(), "{}", result.render());
        let report: serde_json::Value =
            serde_json::from_str(&result.render()).expect("structured self-report json");
        report["capabilities"].clone()
    }

    let mut on = base_config();
    on.file_write = true;
    on.worktree_plan = true;
    on.worktree_apply = true;
    let capabilities = roster_flags(on).await;
    assert_eq!(capabilities["worktree_plan"], true);
    assert_eq!(capabilities["worktree_apply"], true);

    // 依赖 fail-closed: 写总闸关 = 两行照实 false (子依赖主)。
    let mut gate_off = base_config();
    gate_off.worktree_plan = true;
    gate_off.worktree_apply = true;
    let capabilities = roster_flags(gate_off).await;
    assert_eq!(capabilities["worktree_plan"], false);
    assert_eq!(capabilities["worktree_apply"], false);
}

/// 名册投影稳定: 两行按注册条件取值 (无工作区根 = false), 排序稳定无重复键。
#[test]
fn roster_projection_follows_the_worktree_registration_condition() {
    let mut config = ProductionModulesConfig::default();
    config.file_write = true;
    config.worktree_plan = true;
    config.worktree_apply = true;
    let roster = roster_from_config(&config, true, false, true, &[]);
    let rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("worktree_"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        rows,
        vec![("worktree_apply", true), ("worktree_plan", true)]
    );

    // 无工作区根: 注册条件不成立, 两行都照实 false。
    let roster = roster_from_config(&config, false, false, true, &[]);
    let rows: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("worktree_"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        rows,
        vec![("worktree_apply", false), ("worktree_plan", false)]
    );
}

/// 共享读前观测门禁 (注册条件第三件): `tool.worktree_apply` 与读工具同一张
/// 观测表 —— 读工具的「已读」就是它的覆盖写证据; 未读不改拒绝即帧, 读后同单
/// 放行真落盘 (装配面用的确实是共享门禁, 不是私有空门禁)。
#[tokio::test]
async fn worktree_apply_shares_the_read_observation_gate_with_the_read_tools() {
    let root = std::env::temp_dir().join(format!(
        "apeireth-worktree-shared-gate-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp workspace");
    std::fs::write(root.join("note.txt"), "one\n").expect("seed file");

    let mut config = base_config();
    config.filesystem = true;
    config.file_write = true;
    config.worktree_apply = true;
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("assembly builds");
    let tool_of = |id: &str| {
        modules
            .capabilities()
            .iter()
            .find(|capability| capability.id().as_str() == id)
            .cloned()
            .unwrap_or_else(|| panic!("{id} must be registered"))
    };

    let update = ToolCall {
        id: "call_update".into(),
        name: "worktree_apply".into(),
        arguments: serde_json::json!({
            "plan": {
                "goal": "把 note.txt 换一行",
                "files": [{ "path": "note.txt", "action": "update", "expect": "内容换成 uno" }]
            },
            "patch": "*** Begin Patch\n*** Update File: note.txt\n@@\n-one\n+uno\n*** End Patch",
            "verify": []
        }),
    };

    // 未读不改: 没有读观测 = 拒绝即帧 (观测门禁在装配面真实生效)。
    // 断言钉结构面 (单调边界 Guard 的身份名), 不钉文案字面。
    let refused = tool_of("tool.worktree_apply").invoke(&update).await;
    assert!(!refused.is_ok(), "未读不得覆盖写: {}", refused.render());
    assert!(
        refused.render().contains("worktree_apply_boundary"),
        "拒绝即帧须来自单调边界 Guard: {}",
        refused.render()
    );

    // 读工具一读 (同一张观测表) → 同一改单放行, 四拍真落盘。
    let read = tool_of("tool.filesystem")
        .invoke(&ToolCall {
            id: "call_read".into(),
            name: "filesystem".into(),
            arguments: serde_json::json!({ "operation": "read", "path": "note.txt" }),
        })
        .await;
    assert!(read.is_ok(), "{}", read.render());

    let applied = tool_of("tool.worktree_apply").invoke(&update).await;
    assert!(applied.is_ok(), "{}", applied.render());
    assert_eq!(
        std::fs::read_to_string(root.join("note.txt")).expect("landed"),
        "uno\n"
    );

    let _ = std::fs::remove_dir_all(&root);
}
