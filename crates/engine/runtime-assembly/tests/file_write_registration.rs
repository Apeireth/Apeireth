//! 受控文件写入 (`tool.apply_patch`) 的装配级接线证明 (件一/件二)。
//!
//! 覆盖: 生产名册注册可见 (第七件, 默认关 opt-in) / 声明面模型可见 (含
//! git 写边界文案) / 自省通道名册如实报告两个开关的生效值。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::{
    TrustedShellConfig, APPLY_PATCH_CAPABILITY_ID, APPLY_PATCH_TOOL_NAME, GIT_WRITE_BOUNDARY_NOTE,
};

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

/// 注册可见: 主开关开 = 第七件生产工具进名册 (模型可见); 默认关 / 无工作区
/// 根 = 不注册。
#[tokio::test]
async fn production_registers_the_file_write_tool_only_when_enabled() {
    let mut config = base_config();
    config.file_write = true;
    let backends = ProductionBackends {
        workspace_root: Some(PathBuf::from(".")),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
    let ids = registered_ids(&modules);
    assert!(
        ids.iter().any(|id| id == APPLY_PATCH_CAPABILITY_ID),
        "the file-write tool must be registered: {ids:?}"
    );

    // 声明面 (模型看到的工具) 携带补丁合同与 git 写边界文案。
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == APPLY_PATCH_CAPABILITY_ID)
        .expect("tool.apply_patch");
    let declaration = tool.declaration();
    assert_eq!(declaration.name, APPLY_PATCH_TOOL_NAME);
    let description = declaration.description.clone().expect("description");
    assert!(
        description.contains(GIT_WRITE_BOUNDARY_NOTE),
        "the registered declaration must state the git write boundary: {description}"
    );

    // 默认关 (opt-in): 不开主开关 = 不注册 (模型看不到、设置无开关)。
    let off = ProductionModules::build(
        base_config(),
        ProductionBackends {
            workspace_root: Some(PathBuf::from(".")),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .expect("assembly builds");
    assert!(registered_ids(&off)
        .iter()
        .all(|id| id != APPLY_PATCH_CAPABILITY_ID));

    // 无工作区根 = 本地写工具没注册成 (与 filesystem/search 同口径)。
    let mut no_root = base_config();
    no_root.file_write = true;
    let without_root =
        ProductionModules::build(no_root, ProductionBackends::default(), clock()).expect("build");
    assert!(registered_ids(&without_root)
        .iter()
        .all(|id| id != APPLY_PATCH_CAPABILITY_ID));
}

/// 自省通道名册: 两个写入开关按真实生效值报告 (主开关未注册成时子档照实 false)。
#[tokio::test]
async fn roster_reports_file_write_switches_from_effective_config() {
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
    on.file_write_auto_pass = true;
    let capabilities = roster_flags(on).await;
    assert_eq!(capabilities["file_write"], true);
    assert_eq!(capabilities["file_write_auto_pass"], true);

    // 主开关关: 两个开关都照实 false (子档依赖主开关)。
    let mut sub_only = base_config();
    sub_only.file_write_auto_pass = true;
    let capabilities = roster_flags(sub_only).await;
    assert_eq!(capabilities["file_write"], false);
    assert_eq!(capabilities["file_write_auto_pass"], false);
}

/// 组装根补充行与配置行互不覆盖: 名册投影保持稳定排序、无重复键。
#[test]
fn roster_projection_is_stable_for_the_file_write_rows() {
    let mut config = ProductionModulesConfig::default();
    config.file_write = true;
    config.file_write_auto_pass = true;
    let roster = roster_from_config(&config, true, false, true, &[]);
    let file_write: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("file_write"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        file_write,
        vec![("file_write", true), ("file_write_auto_pass", true)]
    );
    // 无工作区根: 注册条件不成立, 两行都照实 false。
    let roster = roster_from_config(&config, false, false, true, &[]);
    let file_write: Vec<_> = roster
        .iter()
        .filter(|row| row.name.starts_with("file_write"))
        .map(|row| (row.name.as_str(), row.enabled))
        .collect();
    assert_eq!(
        file_write,
        vec![("file_write", false), ("file_write_auto_pass", false)]
    );
}

/// 文件写入总闸接线 (宪法级, 内测整改): `file_write` 是**唯一写总闸** ——
/// 组装根把同一开关值注入 shell 配置, shell 写命令与 apply_patch 同受此闸。
/// 关 = 写命令拒绝即帧 (`pipeline.pre_deny`, 文案明示同受一闸); 开 = 写命令
/// 照常冻结 (行为不变, 审批链照旧走 shell 既有风险映射)。
#[test]
fn production_wires_the_file_write_gate_into_the_shell_tool() {
    let write_call = ToolCall {
        id: "call_gate".into(),
        name: "shell".into(),
        arguments: serde_json::json!({ "command": "echo xxx > file.txt" }),
    };
    let shell_of = |modules: &ProductionModules| {
        modules
            .capabilities()
            .iter()
            .find(|capability| capability.id().as_str() == "tool.shell")
            .cloned()
            .expect("tool.shell must be registered")
    };
    let backends = || ProductionBackends {
        workspace_root: Some(PathBuf::from(".")),
        ..ProductionBackends::default()
    };

    // 总闸关 (默认, fail-closed): shell 写命令拒绝即帧。
    let mut off = base_config();
    off.shell = Some(TrustedShellConfig::new("."));
    assert!(!off.file_write, "默认必须 fail-closed");
    let modules = ProductionModules::build(off, backends(), clock()).expect("assembly builds");
    let rendered = match shell_of(&modules).freeze_invocation(&write_call) {
        Err(result) => result.render(),
        Ok(_) => panic!("file_write 关时 shell 写命令必须拒绝即帧"),
    };
    assert!(rendered.contains("pipeline.pre_deny"), "{rendered}");
    assert!(
        rendered.contains("文件写入开关未开——shell 写命令受同一总闸管辖"),
        "{rendered}"
    );

    // 总闸开: 同一写命令照常冻结 (行为不变, 审批链照旧)。
    let mut on = base_config();
    on.file_write = true;
    on.shell = Some(TrustedShellConfig::new("."));
    let modules = ProductionModules::build(on, backends(), clock()).expect("assembly builds");
    let frozen = shell_of(&modules)
        .freeze_invocation(&write_call)
        .expect("总闸开时写命令冻结必须放行 (行为不变)");
    assert!(frozen.is_some(), "总闸开时写命令冻结必须有载荷");
}
