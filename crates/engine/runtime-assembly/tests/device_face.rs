//! K 轴 L1 在场感知骨架 (`tool.device_sense` + `tool.weather`) 的装配级回归:
//! 注册即闸 (开关默认关 fail-closed) / 名册行按实际注册条件取值 / 自述在场节
//! 同表同源 / capabilities-matrix §0.2 口径互钉。
//!
//! 装配走真 [`ProductionModules`] 生产组装根; 感知配置注入才注册 (注册即闸),
//! 名册 `device_sense` / `weather` 行按实际注册条件取值。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_plugin::ToolCapability;
use apeireth_runtime_assembly::{
    device_presence_probe, roster_from_config, ProductionBackends, ProductionModules,
    ProductionModulesConfig, ProductionSelfStatusSource,
};
use apeireth_tools_canonical::device::registry::{Device, DeviceKind, DevicesBody};
use apeireth_tools_canonical::device::sense::{
    device_sense_enabled_from_env, DeviceSenseActivity, DeviceSenseConfig,
    DEVICE_SENSE_CAPABILITY_ID, DEVICE_SENSE_ENABLE_ENV,
};
use apeireth_tools_canonical::device::weather::{WeatherConfig, WEATHER_CAPABILITY_ID};
use apeireth_tools_canonical::SelfStatusSource;

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
    let dir =
        std::env::temp_dir().join(format!("apeireth-device-face-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

fn capability<'a>(modules: &'a ProductionModules, id: &str) -> Option<&'a Arc<dyn ToolCapability>> {
    modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == id)
}

/// ① 注册即闸 + 开关 fail-closed: 默认配置 (感知落位缺席) = 两工具都不注册、
/// 名册行照实 false; 落位注入 = 注册 + 名册 true。
#[test]
fn the_switch_is_fail_closed_and_registration_follows_the_slot() {
    // 默认关 (fail-closed): env 不开 = 落位不注入 = 注册不成。
    assert!(
        DeviceSenseConfig::from_env(".").is_none(),
        "APEIRETH_ENABLE_DEVICE_SENSE 未开必须不装配 (fail-closed)"
    );
    assert!(
        WeatherConfig::from_env(".").is_none(),
        "天气面同族同闸 (默认关 fail-closed)"
    );
    let modules =
        ProductionModules::build(base_config(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, DEVICE_SENSE_CAPABILITY_ID).is_none(),
        "默认必须不注册设备感知面"
    );
    assert!(
        capability(&modules, WEATHER_CAPABILITY_ID).is_none(),
        "默认必须不注册天气面"
    );
    let roster = roster_from_config(&base_config(), true, false, false, false, &[]);
    for row_name in ["device_sense", "weather"] {
        let row = roster
            .iter()
            .find(|row| row.name == row_name)
            .expect("名册必须有在场感知行");
        assert!(!row.enabled, "{row_name} 默认关 = 显式未接线");
    }

    // 落位注入 (开关开的装配结果) = 注册 + 名册真值 true (注册即闸的正向面)。
    let data_dir = temp_root("register");
    let mut config = base_config();
    config.device_sense = Some(DeviceSenseConfig::new(&data_dir));
    config.weather = Some(WeatherConfig::mock(&data_dir));
    let modules =
        ProductionModules::build(config.clone(), ProductionBackends::default(), clock()).unwrap();
    assert!(
        capability(&modules, DEVICE_SENSE_CAPABILITY_ID).is_some(),
        "落位注入必须注册设备感知面"
    );
    assert!(
        capability(&modules, WEATHER_CAPABILITY_ID).is_some(),
        "落位注入必须注册天气面"
    );
    let roster = roster_from_config(&config, true, false, false, false, &[]);
    for row_name in ["device_sense", "weather"] {
        let row = roster
            .iter()
            .find(|row| row.name == row_name)
            .expect("名册必须有在场感知行");
        assert!(row.enabled, "{row_name} 落位注入 = 真生效");
    }
}

/// ② 自述在场节与工具面同表同源: 设备数 / 白名单数 / 最近感知真值入帧,
/// 缺省表如实注「默认零设备」。
#[test]
fn the_presence_probe_reads_the_same_table_as_the_tool_face() {
    let data_dir = temp_root("presence");
    let registry =
        apeireth_tools_canonical::device::registry::DeviceRegistry::under_data_dir(&data_dir);
    registry
        .save(&DevicesBody {
            devices: vec![
                Device {
                    id: "thermo-1".into(),
                    kind: DeviceKind::Sensor,
                    name: "温湿度".into(),
                    adapter: "mock".into(),
                    enabled: true,
                },
                Device {
                    id: "cam-1".into(),
                    kind: DeviceKind::Camera,
                    name: "门口".into(),
                    adapter: "mock".into(),
                    enabled: false,
                },
            ],
        })
        .unwrap();
    let activity = Arc::new(DeviceSenseActivity::new());
    activity.record_sense(
        1_700_000_000_000,
        "1 台白名单设备已感知 (mock 档 = 固定 fixture 数据)",
    );

    let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_device_presence_probe(device_presence_probe(data_dir.clone(), activity));
    let snapshot = SelfStatusSource::snapshot(&source);
    let presence = snapshot.device_presence.expect("在场节在场");
    assert_eq!(presence.devices, Some(2), "设备数 = 表内行数 (含停用)");
    assert_eq!(presence.whitelisted, Some(1), "白名单数 = enabled 行数");
    assert_eq!(presence.last_sense_at_ms, Some(1_700_000_000_000));
    assert!(presence
        .last_sense_note
        .as_deref()
        .unwrap()
        .contains("已感知"));

    // 探测口缺位 = 显式 null + 原因 (0 装诚实, 不编数)。
    let bare = ProductionSelfStatusSource::new(Vec::new(), false, None);
    let bare_snapshot = SelfStatusSource::snapshot(&bare);
    assert!(bare_snapshot.device_presence.is_none());
    assert!(bare_snapshot
        .device_presence_reason
        .as_deref()
        .unwrap()
        .contains("not wired"));
}

/// ③ 口径互钉: capabilities-matrix §0.2 的在场感知开关行 (env 旋钮名 + 名册行名)
/// 与实况一致。
#[test]
fn matrix_device_rows_match_the_live_roster() {
    const MATRIX: &str = include_str!("../../../../docs/03-reference/capabilities-matrix.md");
    let start = MATRIX.find("### 0.2").expect("§0.2 present");
    let end = MATRIX[start..]
        .find("\n### ")
        .map(|offset| start + offset)
        .unwrap_or(MATRIX.len());
    let section = &MATRIX[start..end];
    for needle in [
        "APEIRETH_ENABLE_DEVICE_SENSE",
        "`device_sense`",
        "`weather`",
        "tool.device_sense",
        "tool.weather",
    ] {
        assert!(section.contains(needle), "§0.2 缺在场感知行口径: {needle}");
    }
    // 名册行实况: 开关名与文档一致 (注册条件取值)。
    let mut config = base_config();
    config.device_sense = Some(DeviceSenseConfig::new(temp_root("matrix")));
    config.weather = Some(WeatherConfig::mock(temp_root("matrix-weather")));
    let roster = roster_from_config(&config, true, false, false, false, &[]);
    for row_name in ["device_sense", "weather"] {
        assert!(
            roster.iter().any(|row| row.name == row_name && row.enabled),
            "名册 {row_name} 行必须与文档口径一致"
        );
    }
    // 旋钮名实况与文档一致 (fail-closed 语义同源)。
    assert!(
        !device_sense_enabled_from_env(),
        "默认关 (测试环境不开旋钮)"
    );
    assert_eq!(DEVICE_SENSE_ENABLE_ENV, "APEIRETH_ENABLE_DEVICE_SENSE");
}
