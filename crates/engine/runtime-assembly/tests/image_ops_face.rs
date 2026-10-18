//! 创作工具面 W1 (`tool.image_ops`) 装配级钉子: 开关 fail-closed (注册即闸 =
//! 旋钮 + `file_write` 唯一写总闸 + 工作区根) + 名册行照实 + 自述创作节
//! (最近操作计数) 与工具面同一计数器同源接线。
//!
//! 自报读值统一经 `tool.self_status` 真调用取证 (与生产同一条投影路径)。

use std::path::Path;
use std::sync::Arc;

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    BuildInstrumentation, CountingBuildProbe, ModuleBuildProbe, ModuleFamily, ProductionBackends,
    ProductionModules, ProductionModulesConfig,
};
use serde_json::Value;

/// 构造探针的插桩句柄 (显式 unsized coercion, 逐用例复用)。
fn instrumentation(probe: &Arc<CountingBuildProbe>) -> BuildInstrumentation {
    let concrete = Arc::clone(probe);
    let handle: Arc<dyn ModuleBuildProbe> = concrete;
    BuildInstrumentation::default().with_probe(handle)
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

/// 与生产同款的最小装配: 记忆/偏好槽全关 (逐用例显式打开)。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn backends(root: &Path) -> ProductionBackends {
    ProductionBackends {
        workspace_root: Some(root.to_path_buf()),
        ..ProductionBackends::default()
    }
}

async fn invoke_named(
    modules: &ProductionModules,
    capability_id: &str,
    name: &str,
    arguments: Value,
) -> Value {
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == capability_id)
        .cloned()
        .unwrap_or_else(|| panic!("{capability_id} must be registered"));
    let call = ToolCall {
        id: "call_image_ops_face".into(),
        name: name.into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{capability_id}: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured json")
}

/// 1x1 24bpp BMP fixture (纯手工字节, 测试不引图像库): 一像素 (B=50,G=100,R=200)。
fn tiny_bmp() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&58u32.to_le_bytes()); // 文件大小
    bytes.extend_from_slice(&0u32.to_le_bytes()); // 保留位
    bytes.extend_from_slice(&54u32.to_le_bytes()); // 像素数据偏移
    bytes.extend_from_slice(&40u32.to_le_bytes()); // 信息头大小
    bytes.extend_from_slice(&1i32.to_le_bytes()); // 宽
    bytes.extend_from_slice(&1i32.to_le_bytes()); // 高
    bytes.extend_from_slice(&1u16.to_le_bytes()); // 平面数
    bytes.extend_from_slice(&24u16.to_le_bytes()); // 位深
    bytes.extend_from_slice(&0u32.to_le_bytes()); // 无压缩
    bytes.extend_from_slice(&4u32.to_le_bytes()); // 像素数据大小
    bytes.extend_from_slice(&2835i32.to_le_bytes()); // 水平分辨率
    bytes.extend_from_slice(&2835i32.to_le_bytes()); // 垂直分辨率
    bytes.extend_from_slice(&0u32.to_le_bytes()); // 调色板色数
    bytes.extend_from_slice(&0u32.to_le_bytes()); // 重要色数
    bytes.extend_from_slice(&[50, 100, 200, 0]); // BGR + 行对齐
    bytes
}

/// 开关 fail-closed: 只有「旋钮 + `file_write` 写总闸 + 工作区根」三者齐备才
/// 注册 `tool.image_ops`; 名册 `image_ops` 行按实际注册条件取值, 构造探针零构造。
#[tokio::test]
async fn image_ops_switch_registers_only_with_the_write_gate_and_reports_honestly() {
    let dir = tempfile::tempdir().unwrap();

    // 默认关 (fail-closed): 不注册, 名册照实 false, 族零构造。
    let probe = CountingBuildProbe::new();
    let config = base_config();
    let modules = ProductionModules::build_instrumented(
        config,
        backends(dir.path()),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");
    assert!(
        !modules
            .capabilities()
            .iter()
            .any(|capability| capability.id().as_str() == "tool.image_ops"),
        "默认关不得注册"
    );
    assert_eq!(probe.count(ModuleFamily::ImageOps), 0, "开关关 = 零构造");
    let report = invoke_named(
        &modules,
        "tool.self_status",
        "self_status",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(report["capabilities"]["image_ops"], false);

    // 旋钮开但写总闸关: 仍不注册 (输出写归档文件写入族, 子依赖主 fail-closed)。
    let probe = CountingBuildProbe::new();
    let mut config = base_config();
    config.image_ops = true;
    let modules = ProductionModules::build_instrumented(
        config,
        backends(dir.path()),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");
    assert!(
        !modules
            .capabilities()
            .iter()
            .any(|capability| capability.id().as_str() == "tool.image_ops"),
        "写总闸关 = 不注册"
    );
    assert_eq!(probe.count(ModuleFamily::ImageOps), 0);
    let report = invoke_named(
        &modules,
        "tool.self_status",
        "self_status",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(report["capabilities"]["image_ops"], false);

    // 旋钮 + 写总闸 + 工作区根: 真注册 + 名册照实 true + 族构造恰一次。
    let probe = CountingBuildProbe::new();
    let mut config = base_config();
    config.image_ops = true;
    config.file_write = true;
    let modules = ProductionModules::build_instrumented(
        config,
        backends(dir.path()),
        clock(),
        &instrumentation(&probe),
    )
    .expect("assembly");
    assert!(
        modules
            .capabilities()
            .iter()
            .any(|capability| capability.id().as_str() == "tool.image_ops"),
        "三条件齐备必须注册"
    );
    assert_eq!(probe.count(ModuleFamily::ImageOps), 1, "族构造恰一次");
    let report = invoke_named(
        &modules,
        "tool.self_status",
        "self_status",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(report["capabilities"]["image_ops"], true);
}

/// 自述创作节 (最近操作计数) 与 `tool.image_ops` 同一计数器同源: 真调工具后
/// 创作节报真计数; 面没注册 = 显式 null + 原因 (0 装诚实, 不编数)。
#[tokio::test]
async fn self_status_creation_section_reports_recent_operation_counts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("in.bmp"), tiny_bmp()).unwrap();

    let mut config = base_config();
    config.image_ops = true;
    config.file_write = true;
    let modules =
        ProductionModules::build(config, backends(dir.path()), clock()).expect("assembly");

    // 真调一次图像操作 (resize 1x1 -> 2x2)。
    let report = invoke_named(
        &modules,
        "tool.image_ops",
        "image_ops",
        serde_json::json!({
            "input": "in.bmp",
            "output": "out.bmp",
            "operations": [{ "op": "resize", "width": 2, "height": 2 }]
        }),
    )
    .await;
    assert_eq!(report["output_size"]["width"], 2);
    assert!(dir.path().join("out.bmp").exists(), "真落盘");

    // 自述创作节: 同一计数器的最近操作计数 (只计数不回路径)。
    let status = invoke_named(
        &modules,
        "tool.self_status",
        "self_status",
        serde_json::json!({}),
    )
    .await;
    let creation = &status["image_ops"];
    assert_eq!(creation["calls"], 1);
    assert_eq!(creation["ops_applied"], 1);
    assert_eq!(creation["denied"], 0);
    assert_eq!(creation["last_call_ops"], "resize");
    assert!(creation["last_op_at_ms"].is_number(), "最近操作时刻如实");
    let resize = creation["by_operation"]
        .as_array()
        .expect("by_operation rows")
        .iter()
        .find(|row| row["operation"] == "resize")
        .expect("resize row");
    assert_eq!(resize["count"], 1);
    let rendered = creation.to_string();
    assert!(
        !rendered.contains(&dir.path().to_string_lossy().to_string()),
        "创作节零路径 (脱敏口径): {rendered}"
    );

    // 面没注册: 创作节显式 null + 原因 (不编数)。
    let config = base_config();
    let modules =
        ProductionModules::build(config, backends(dir.path()), clock()).expect("assembly");
    let status = invoke_named(
        &modules,
        "tool.self_status",
        "self_status",
        serde_json::json!({}),
    )
    .await;
    let creation = &status["image_ops"];
    assert_eq!(creation["calls"], Value::Null);
    assert_eq!(creation["by_operation"], Value::Null);
    assert!(
        creation["reason"].as_str().unwrap().contains("not wired"),
        "缺来源要如实说: {}",
        creation["reason"]
    );
}
