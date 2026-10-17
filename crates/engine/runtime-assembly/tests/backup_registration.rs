//! G5 备份恢复 (`tool.backup_export`) 的装配级接线证明: 注册即闸 / 名册如实 /
//! 只读预设执法面 / 工具面真产出 manifest 摘要包 / self_status 备份节真值。

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    is_write_or_execute_capability, roster_from_config, ProductionBackends, ProductionModules,
    ProductionModulesConfig, ProductionSelfStatusSource,
};
use apeireth_tools_canonical::backup::{
    verify_backup, BackupCategory, BackupExportConfig, BackupHooks, BackupItem, BackupSources,
    MANIFEST_FILE_NAME,
};
use apeireth_tools_canonical::{
    render_snapshot, BackupStatus, SelfStatusIdentity, SelfStatusSnapshot, SelfStatusSource,
    SelfStatusTool, StatusProbe, BACKUP_EXPORT_CAPABILITY_ID,
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

fn sources_at(root: &std::path::Path) -> BackupSources {
    std::fs::create_dir_all(root.join("skills")).expect("fixture dirs");
    std::fs::write(root.join("users.json"), br#"{"users":[]}"#).expect("fixture users");
    std::fs::write(root.join("skills/card.json"), b"card-v1").expect("fixture skill");
    BackupSources::new(
        root,
        vec![
            BackupItem::file(BackupCategory::Users, root.join("users.json"), "users.json"),
            BackupItem::tree(BackupCategory::Skills, root.join("skills"), "skills"),
        ],
    )
}

/// 注册即闸: 默认关 (配置缺席 = 不注册); 配置在场 = 注册, 声明名实相符。
#[tokio::test]
async fn production_registers_the_backup_export_tool_only_when_configured() {
    // 默认关 (fail-closed): 无配置 = 无工具面。
    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("build");
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    assert!(
        !ids.iter().any(|id| id == BACKUP_EXPORT_CAPABILITY_ID),
        "默认不得注册: {ids:?}"
    );

    // 配置在场 = 注册, 声明面名实相符。
    let temp = tempfile::tempdir().expect("temp dir");
    let mut config = base_config();
    config.backup = Some(BackupExportConfig::new(
        sources_at(temp.path()),
        BackupHooks::none(),
    ));
    let modules =
        ProductionModules::build(config, ProductionBackends::default(), clock()).expect("build");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == BACKUP_EXPORT_CAPABILITY_ID)
        .expect("tool.backup_export");
    assert_eq!(tool.declaration().name, "backup_export");

    // 名册如实 (自述能力名册按实际注册条件报告)。
    let mut with_roster = base_config();
    with_roster.backup = Some(BackupExportConfig::new(
        sources_at(&temp.path().join("second")),
        BackupHooks::none(),
    ));
    let roster = roster_from_config(&with_roster, true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|switch| switch.name == "backup_export")
        .expect("backup_export roster row");
    assert!(row.enabled, "配置在场时名册必须报 true");
    let roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    let row = roster
        .iter()
        .find(|switch| switch.name == "backup_export")
        .expect("backup_export roster row");
    assert!(!row.enabled, "默认关时名册必须报 false");
}

/// 工具面真产出: 一次调用 = 一个带 manifest 的全量 bundle (引擎可独立校验)。
#[tokio::test]
async fn the_backup_export_tool_produces_a_manifest_checked_bundle() {
    let temp = tempfile::tempdir().expect("temp dir");
    let data = temp.path().join("data");
    let bundle = temp.path().join("bundle");
    let mut config = base_config();
    config.backup = Some(BackupExportConfig::new(
        sources_at(&data),
        BackupHooks::none(),
    ));
    let modules =
        ProductionModules::build(config, ProductionBackends::default(), clock()).expect("build");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == BACKUP_EXPORT_CAPABILITY_ID)
        .cloned()
        .expect("tool.backup_export");

    let call = ToolCall {
        id: "call_backup".into(),
        name: "backup_export".into(),
        arguments: serde_json::json!({ "target_dir": bundle.to_string_lossy() }),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: serde_json::Value =
        serde_json::from_str(&result.render()).expect("tool output json");
    assert_eq!(
        report["bundle"],
        serde_json::json!(bundle.to_string_lossy())
    );
    assert_eq!(report["manifest"]["entries"], serde_json::json!(2));
    assert_eq!(
        report["manifest"]["format_version"],
        serde_json::json!(apeireth_tools_canonical::BACKUP_FORMAT_VERSION)
    );

    // 引擎独立校验同一 bundle (工具面与 CLI 面同一引擎同一清单口径)。
    let manifest = verify_backup(&bundle).expect("verify tool-produced bundle");
    assert_eq!(manifest.entry_count(), 2);
    assert!(bundle.join(MANIFEST_FILE_NAME).is_file());
}

/// 只读预设执法面: 备份导出是写面 (打包落盘 + 外带记忆), read_only 会话不得执行。
#[test]
fn backup_export_is_a_write_face_for_the_read_only_preset() {
    assert!(is_write_or_execute_capability(
        &CapabilityId::new(BACKUP_EXPORT_CAPABILITY_ID).expect("capability id")
    ));
}

/// self_status 备份节: 探测口在场 = 真值; 缺位 = 显式 null + 原因 (0 装诚实)。
#[test]
fn self_status_backup_section_reports_truth_or_explicit_null() {
    // 缺位: 显式 null + 原因。
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None);
    let snapshot = source.snapshot();
    assert!(snapshot.backup.is_none());
    assert!(snapshot.backup_reason.is_some());
    let value = render_snapshot(&snapshot);
    assert_eq!(
        value["backup"]["last_backup_at_ms"],
        serde_json::json!(null)
    );
    assert_eq!(value["backup"]["manifest_entries"], serde_json::json!(null));

    // 在场: 真值直通 (探测口读「最近备份」记录)。
    let probe: StatusProbe<BackupStatus> = Arc::new(|| {
        Ok(BackupStatus {
            last_backup_at_ms: Some(1_728_000_600_000),
            manifest_entries: Some(8),
            reason: None,
        })
    });
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None).with_backup_probe(probe);
    let value = render_snapshot(&source.snapshot());
    assert_eq!(
        value["backup"]["last_backup_at_ms"],
        serde_json::json!(1_728_000_600_000i64)
    );
    assert_eq!(value["backup"]["manifest_entries"], serde_json::json!(8));
    assert_eq!(value["backup"]["reason"], serde_json::json!(null));

    // 形状合同: 备份节恒在场 (缺失也显式 null)。
    let mut snapshot = SelfStatusSnapshot::minimal(
        apeireth_tools_canonical::SelfStatusIdentity::default(),
        Vec::new(),
    );
    snapshot.backup = Some(BackupStatus {
        last_backup_at_ms: None,
        manifest_entries: None,
        reason: Some("尚无备份记录".into()),
    });
    let value = render_snapshot(&snapshot);
    assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
}
