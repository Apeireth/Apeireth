//! 创作工具面 K3 · 表格操作 (`tool.sheet_ops`) 的装配级回归:
//! 注册即闸 (开关默认关 fail-closed) / 写总闸注入 (拒写不拒读) / 自述创作节
//! 同源同账 (工具面与自述同一计数器) / 名册 `sheet_ops` 行按实际注册条件取值。
//!
//! 装配走真 [`ProductionModules`] 生产组装根; 配置注入才注册 (注册即闸),
//! 名册 `sheet_ops` 行按实际注册条件取值。真 fixture = 真 CSV 文件。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    roster_from_config, ProductionBackends, ProductionModules, ProductionModulesConfig,
};
use apeireth_tools_canonical::{SheetOpsConfig, SHEET_OPS_CAPABILITY_ID};
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
        "apeireth-sheet-ops-face-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试工作区");
    dir
}

/// 真 fixture CSV (三行销售表)。
fn fixture_csv() -> &'static str {
    "name,qty,price,note\napple,3,1.25,red\npear,10,0.5,fresh\nplum,2,3.4,sweet\n"
}

fn capability<'a>(modules: &'a ProductionModules, id: &str) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

async fn invoke(tool: &Arc<dyn ToolCapability>, arguments: Value) -> Value {
    let call = ToolCall {
        id: "call_sheet_ops".into(),
        name: "sheet_ops".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    serde_json::from_str(&result.render()).expect("structured json")
}

async fn invoke_raw(tool: &Arc<dyn ToolCapability>, arguments: Value) -> (bool, String) {
    let call = ToolCall {
        id: "call_sheet_ops".into(),
        name: "sheet_ops".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    (result.is_ok(), result.render())
}

/// ① 注册即闸 + 开关 fail-closed: 默认配置 (配置落位缺席) = 工具不注册、
/// 名册 `sheet_ops` 行照实 false; 落位注入 = 注册 + 名册真值 true。
#[test]
fn the_switch_is_fail_closed_and_registration_follows_the_slot() {
    assert!(
        SheetOpsConfig::from_env(".").is_none(),
        "APEIRETH_ENABLE_SHEET_OPS 未开必须不装配 (fail-closed)"
    );
    let modules =
        ProductionModules::build(base_config(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, SHEET_OPS_CAPABILITY_ID).is_none(),
        "默认必须不注册表格操作面"
    );
    let roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "sheet_ops")
        .expect("名册必须有 sheet_ops 行");
    assert!(!row.enabled, "默认关 = 显式未接线");

    // 落位注入 (开关开的装配结果) = 注册 + 名册真值 true (注册即闸的正向面)。
    let root = temp_root("register");
    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    let modules =
        ProductionModules::build(config.clone(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, SHEET_OPS_CAPABILITY_ID).is_some(),
        "落位注入必须注册表格操作面"
    );
    let roster = roster_from_config(&config, true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|row| row.name == "sheet_ops")
        .expect("名册必须有 sheet_ops 行");
    assert!(row.enabled, "注册即闸的正向面必须照实 true");
}

/// ② 真读穿全流水线 + 自述创作节同源同账: 工具面一次 summary 之后,
/// `tool.self_status` 创作节必须报同一计数器的真值 (只计数不回路径)。
#[tokio::test]
async fn a_real_read_shows_up_in_the_creation_section_of_self_status() {
    let root = temp_root("creation");
    std::fs::write(root.join("sales.csv"), fixture_csv()).unwrap();
    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();

    let sheet_tool = capability(&modules, SHEET_OPS_CAPABILITY_ID).expect("表格操作面已注册");
    let frame = invoke(
        sheet_tool,
        json!({"action": "summary", "source": "sales.csv"}),
    )
    .await;
    assert_eq!(frame["status"], "ok");
    assert_eq!(frame["row_count"], 3);

    let status_tool = capability(&modules, "tool.self_status").expect("自述面在场");
    let status = invoke(status_tool, json!({})).await;
    assert_eq!(status["sheet_ops"]["calls"], 1);
    assert_eq!(status["sheet_ops"]["rows_touched"], 3);
    assert_eq!(
        status["sheet_ops"]["by_operation"][1]["operation"],
        "summary"
    );
    assert_eq!(status["sheet_ops"]["by_operation"][1]["count"], 1);
    assert!(
        !status["sheet_ops"].to_string().contains(".csv"),
        "创作节零路径零文件名: {}",
        status["sheet_ops"]
    );
}

/// ③ 写总闸 (宪法级 `file_write` 唯一写总闸) 注入: 关 = 拒写不拒读 (拒绝即帧
/// 说明写总闸), 开 = 写面正常 (创建类原子写)。
#[tokio::test]
async fn the_write_gate_refuses_writes_but_never_reads() {
    let root = temp_root("gate");
    std::fs::write(root.join("sales.csv"), fixture_csv()).unwrap();
    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    // `file_write` 默认 false = 写总闸关。
    let modules = ProductionModules::build(
        config.clone(),
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let sheet_tool = capability(&modules, SHEET_OPS_CAPABILITY_ID).expect("表格操作面已注册");
    let frame = invoke(sheet_tool, json!({"action": "read", "source": "sales.csv"})).await;
    assert_eq!(frame["status"], "ok", "读面不受写总闸影响");
    let (ok, rendered) = invoke_raw(
        sheet_tool,
        json!({"action": "export_csv", "source": "sales.csv", "output": "out.csv"}),
    )
    .await;
    assert!(!ok, "写总闸关必须拒写");
    assert!(
        rendered.contains("唯一写总闸"),
        "拒绝帧必须说明写总闸: {rendered}"
    );

    // 写总闸开 = 写面正常 (创建类原子写)。
    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    config.file_write = true;
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let sheet_tool = capability(&modules, SHEET_OPS_CAPABILITY_ID).expect("表格操作面已注册");
    let frame = invoke(
        sheet_tool,
        json!({"action": "export_csv", "source": "sales.csv", "output": "out.csv"}),
    )
    .await;
    assert_eq!(frame["status"], "ok", "写总闸开必须放行写面");
    assert!(root.join("out.csv").exists(), "输出必须真落盘");
}

/// ④ 变换动作带 `output` 也是写面 (受同一写总闸约束): 关 = 拒, 开 = 落新文件。
#[tokio::test]
async fn transform_outputs_follow_the_same_write_gate() {
    let root = temp_root("transform");
    std::fs::write(root.join("sales.csv"), fixture_csv()).unwrap();

    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let sheet_tool = capability(&modules, SHEET_OPS_CAPABILITY_ID).expect("表格操作面已注册");
    let (ok, _) = invoke_raw(
        sheet_tool,
        json!({"action": "sort", "source": "sales.csv", "column": "qty",
               "output": "sorted.csv"}),
    )
    .await;
    assert!(!ok, "写总闸关: 变换动作带 output 必须拒");
    let frame = invoke(
        sheet_tool,
        json!({"action": "sort", "source": "sales.csv", "column": "qty"}),
    )
    .await;
    assert_eq!(
        frame["status"], "ok",
        "写总闸关: 同一动作不带 output 照常读"
    );

    let mut config = base_config();
    config.sheet_ops = Some(SheetOpsConfig::new(&root));
    config.file_write = true;
    let modules = ProductionModules::build(
        config,
        ProductionBackends {
            workspace_root: Some(root.clone()),
            ..ProductionBackends::default()
        },
        clock(),
    )
    .unwrap();
    let sheet_tool = capability(&modules, SHEET_OPS_CAPABILITY_ID).expect("表格操作面已注册");
    let frame = invoke(
        sheet_tool,
        json!({"action": "sort", "source": "sales.csv", "column": "qty",
               "output": "sorted.csv"}),
    )
    .await;
    assert_eq!(frame["status"], "ok");
    assert!(root.join("sorted.csv").exists(), "写总闸开: 输出必须真落盘");
}
