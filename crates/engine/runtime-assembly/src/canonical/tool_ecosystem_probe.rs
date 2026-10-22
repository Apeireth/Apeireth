//! SG-25 工具生态健康 → 生物机制自述格的**迟绑定只读探测口**。
//!
//! ## 数据流 (只读、零副作用)
//!
//! 装配根把「能力表装配完毕后的静态工具集合」填入 [`ToolEcosystemSlot`]；
//! 自述开关开时 [`install_bio_self_status`](super::bio_production_wiring::install_bio_self_status)
//! 注册本 probe。读取时：槽快照 → [`project_capabilities`](super::tool_taxonomy::project_capabilities)
//! 的已投影画像 → 内核 [`analyze`] 公式 → 一格读数（值 = 纯观测 JSON）。
//!
//! ## 口径 (0 装诚实)
//!
//! - **只观测，不剪枝**：读数只含分数/阈值/覆盖/重叠对，没有任何删除/注销/
//!   改写工具的动作（与内核「只给建议」同口径）；
//! - **迟绑定未填 = 未接线**：槽空时回 `Err` → 自述该格 `null` + 真因，
//!   不编分数；零注册 / 零命中策展表同样回 `Err`（覆盖率 0 不算健康）；
//! - **分数只在已收录工具上计算**：未命中策展表的工具不猜标签，只计入
//!   `untagged` 与 `registered` 如实上报，避免「未收录」被洗成低多样性；
//! - **静态快照**：运行期动态发现的 MCP 工具不在快照内（发现发生在装配之后，
//!   属如实缺口，读数的 `taxonomy_version` 与 `untagged` 会暴露覆盖不全）。

use std::sync::{Arc, Mutex};

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::ecosystem_health::{
    analyze, EcosystemConfig, ToolProfile, DEFAULT_DIVERSITY_WEIGHT, DEFAULT_MAX_PAIRS,
    DEFAULT_OVERLAP_THRESHOLD, DEFAULT_OVERLAP_WEIGHT, DEFAULT_REDUNDANCY_TOLERANCE,
    DEFAULT_REDUNDANCY_WEIGHT, DYSBIOSIS_ALERT_THRESHOLD,
};
use apeireth_tools_canonical::self_status::{BioMechanismProbe, BioMechanismReading};
use serde_json::json;

use super::tool_taxonomy::{project_capabilities, TOOL_TAXONOMY_VERSION};

/// 内核分数/阈值是 `f32`; 进 JSON 前四舍五入到 4 位小数, 避免
/// `0.8f32 -> 0.800000011920929` 这类二进制噪声污染可读读数.
fn f32_as_f64(value: f32) -> f64 {
    (f64::from(value) * 10_000.0).round() / 10_000.0
}

/// 装配末端填入的静态工具生态剖面（只读快照）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolEcosystemSnapshot {
    /// 参与投影的注册工具总数（含未收录）。
    pub registered: usize,
    /// 命中策展表的工具画像（内核公式的输入）。
    pub profiles: Vec<ToolProfile>,
    /// 未命中策展表的工具身份（去重、字典序；如实上报，不猜标签）。
    pub untagged: Vec<String>,
    /// 生成该快照时的策展表版本。
    pub taxonomy_version: u32,
}

impl ToolEcosystemSnapshot {
    /// 从装配面能力集合生成快照（纯投影，不调用任何工具）。
    #[must_use]
    pub fn from_capabilities(capabilities: &[Arc<dyn ToolCapability>]) -> Self {
        let projection = project_capabilities(capabilities);
        Self {
            registered: projection.registered,
            profiles: projection.profiles,
            untagged: projection.untagged,
            taxonomy_version: TOOL_TAXONOMY_VERSION,
        }
    }

    /// 覆盖率读数：`已收录 / 注册总数`；零注册 = `None`（不除零、不编 0）。
    #[must_use]
    pub fn coverage(&self) -> Option<f64> {
        if self.registered == 0 {
            return None;
        }
        Some(self.profiles.len() as f64 / self.registered as f64)
    }
}

/// 工具生态剖面的迟绑定槽（装配根填一次，probe 只读）。
///
/// 与 [`BioProbeSlots`](super::bio_production_wiring::BioProbeSlots) 的其它槽
/// 同口径：默认空、填一次、重复填覆盖为最后一个快照；槽锁被毒化时按内容恢复
/// 而非级联 panic。
#[derive(Clone, Default)]
pub struct ToolEcosystemSlot {
    inner: Arc<Mutex<Option<Arc<ToolEcosystemSnapshot>>>>,
}

impl ToolEcosystemSlot {
    /// 空槽（`fill` 之前 probe 一律回「未接线」）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 槽是否为空（默认关 / 尚未填充的证据）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().is_none()
    }

    /// 用装配面能力集合填充快照；返回是否真的填了（空槽 → 有快照）。
    pub fn fill(&self, capabilities: &[Arc<dyn ToolCapability>]) -> bool {
        let snapshot = Arc::new(ToolEcosystemSnapshot::from_capabilities(capabilities));
        *self.lock() = Some(snapshot);
        true
    }

    /// 读当前快照（`None` = 未填）。
    #[must_use]
    pub fn snapshot(&self) -> Option<Arc<ToolEcosystemSnapshot>> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Arc<ToolEcosystemSnapshot>>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 工具生态健康探测口：把静态剖面喂给内核公式，产出一格纯观测读数。
///
/// 只读；不修改任何工具注册表；不在读数里给「删除建议」之外的动作。
pub struct ToolEcosystemProbe {
    slot: Arc<ToolEcosystemSlot>,
}

impl ToolEcosystemProbe {
    /// 绑定迟绑定槽（槽空 = 运行时该格 `null` + 真因）。
    #[must_use]
    pub fn new(slot: Arc<ToolEcosystemSlot>) -> Self {
        Self { slot }
    }
}

impl BioMechanismProbe for ToolEcosystemProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        let snapshot = self.slot.snapshot().ok_or_else(|| {
            "未接线: 工具生态剖面未填 (装配末端尚未交付能力表快照, 不编值)".to_string()
        })?;
        if snapshot.registered == 0 {
            return Err("未接线: 装配面零工具注册 (无剖面可算, 不编读数)".to_string());
        }
        if snapshot.profiles.is_empty() {
            return Err(format!(
                "未接线: 已注册 {} 个工具但零命中策展表 (SG-25 taxonomy v{}, 覆盖率 0), 不编读数",
                snapshot.registered, snapshot.taxonomy_version
            ));
        }
        let config = EcosystemConfig::default();
        let report = analyze(&snapshot.profiles, &config);
        let alert = report.dysbiosis_score >= DYSBIOSIS_ALERT_THRESHOLD;
        let overlap_pairs: Vec<serde_json::Value> = report
            .overlap_pairs
            .iter()
            .map(|pair| {
                json!({
                    "left": pair.left,
                    "right": pair.right,
                    "similarity": f32_as_f64(pair.similarity),
                })
            })
            .collect();
        let coverage = snapshot.coverage();
        let value = json!({
            "registered": snapshot.registered,
            "tagged": snapshot.profiles.len(),
            "untagged": snapshot.untagged.len(),
            "untagged_names": snapshot.untagged,
            "coverage": coverage,
            "tool_count": report.tool_count,
            "distinct_capabilities": report.distinct_capabilities,
            "diversity_score": f32_as_f64(report.diversity_score),
            "redundancy_score": f32_as_f64(report.redundancy_score),
            "dysbiosis_score": f32_as_f64(report.dysbiosis_score),
            "alert": alert,
            "overlap_pairs": overlap_pairs,
            "recommendations": report.recommendations,
            "taxonomy_version": snapshot.taxonomy_version,
            "weights": {
                "diversity": f32_as_f64(DEFAULT_DIVERSITY_WEIGHT),
                "redundancy": f32_as_f64(DEFAULT_REDUNDANCY_WEIGHT),
                "overlap": f32_as_f64(DEFAULT_OVERLAP_WEIGHT),
            },
            "thresholds": {
                "overlap": f32_as_f64(DEFAULT_OVERLAP_THRESHOLD),
                "redundancy_tolerance": DEFAULT_REDUNDANCY_TOLERANCE,
                "max_pairs": DEFAULT_MAX_PAIRS,
                "dysbiosis_alert": f32_as_f64(DYSBIOSIS_ALERT_THRESHOLD),
            },
        });
        Ok(BioMechanismReading::wired(
            Some(value),
            Some(f64::from(DYSBIOSIS_ALERT_THRESHOLD)),
            None,
        ))
    }
}

/// 绑定槽的工具生态探测口（装配安装点用）。
#[must_use]
pub fn tool_ecosystem_bio_probe(slot: Arc<ToolEcosystemSlot>) -> Arc<dyn BioMechanismProbe> {
    Arc::new(ToolEcosystemProbe::new(slot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::CapabilityId;
    use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
    use async_trait::async_trait;

    struct StubTool {
        id: CapabilityId,
        name: &'static str,
    }

    #[async_trait]
    impl ToolCapability for StubTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> NormalizedTool {
            NormalizedTool::new(self.name)
        }

        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, serde_json::json!({ "stub": true }))
        }
    }

    fn stub(capability_id: &str, name: &'static str) -> Arc<dyn ToolCapability> {
        Arc::new(StubTool {
            id: CapabilityId::new(capability_id).expect("static capability id is valid"),
            name,
        })
    }

    fn slot_with(capabilities: &[Arc<dyn ToolCapability>]) -> Arc<ToolEcosystemSlot> {
        let slot = Arc::new(ToolEcosystemSlot::new());
        slot.fill(capabilities);
        slot
    }

    #[test]
    fn an_empty_slot_reports_not_wired_instead_of_inventing_scores() {
        let probe = ToolEcosystemProbe::new(Arc::new(ToolEcosystemSlot::new()));
        let error = probe.snapshot(0).expect_err("空槽必须回真因");
        assert!(error.contains("未接线"), "真因必须显式说未接线: {error}");
        assert!(error.contains("不编值"), "真因必须说清楚不编值: {error}");
    }

    #[test]
    fn zero_registered_or_zero_tagged_is_an_honest_error() {
        let empty = slot_with(&[]);
        let error = ToolEcosystemProbe::new(empty)
            .snapshot(0)
            .expect_err("零注册不得编读数");
        assert!(error.contains("零工具注册"), "{error}");

        let untagged_only = slot_with(&[stub("tool.mystery", "mystery_tool")]);
        let error = ToolEcosystemProbe::new(untagged_only)
            .snapshot(0)
            .expect_err("零命中策展表不得编读数");
        assert!(error.contains("零命中策展表"), "{error}");
    }

    #[test]
    fn a_filled_slot_yields_observation_only_json_with_honest_coverage() {
        let slot = slot_with(&[
            stub("tool.filesystem", "filesystem"),
            stub("tool.search", "search"),
            stub("tool.shell", "shell"),
            stub("tool.mystery", "mystery_tool"),
        ]);
        let reading = ToolEcosystemProbe::new(slot)
            .snapshot(0)
            .expect("三段已收录应产出读数");
        assert!(reading.wired);
        let value = reading.value.expect("读数必须有值");
        assert_eq!(value["registered"], json!(4));
        assert_eq!(value["tagged"], json!(3));
        assert_eq!(value["untagged"], json!(1));
        assert_eq!(value["untagged_names"], json!(["mystery_tool"]));
        assert_eq!(value["coverage"], json!(0.75));
        assert_eq!(value["taxonomy_version"], json!(TOOL_TAXONOMY_VERSION));
        assert_eq!(value["tool_count"], json!(3), "分数只覆盖已收录工具");
        assert!(value["diversity_score"].is_f64());
        assert!(value["dysbiosis_score"].is_f64());
        assert!(value["recommendations"].is_array());
        assert_eq!(
            reading.threshold,
            Some(f64::from(DYSBIOSIS_ALERT_THRESHOLD))
        );
        // 只观测：读数里没有任何写动作字段。
        for forbidden in ["removed", "pruned", "deleted", "unregistered"] {
            assert!(
                value.get(forbidden).is_none(),
                "读数不得包含写动作字段 {forbidden}"
            );
        }
    }

    #[test]
    fn the_alert_flag_follows_the_kernel_threshold() {
        // 三个工具全部落在同一能力族 → 多样性 1/3、重叠压力高 → 失调分越阈值。
        let slot = slot_with(&[
            stub("tool.play_start", "play_start"),
            stub("tool.play_finish", "play_finish"),
            stub("tool.ritual_done", "ritual_done"),
        ]);
        let reading = ToolEcosystemProbe::new(slot).snapshot(0).expect("读数");
        let value = reading.value.expect("值");
        assert_eq!(
            value["alert"],
            json!(true),
            "同族三工具应报失调告警: {value}"
        );
    }

    #[test]
    fn fill_overwrites_the_previous_snapshot_and_reports_its_own_counts() {
        let slot = Arc::new(ToolEcosystemSlot::new());
        assert!(slot.is_empty());
        slot.fill(&[stub("tool.filesystem", "filesystem")]);
        assert_eq!(slot.snapshot().expect("已填").registered, 1);
        slot.fill(&[stub("tool.search", "search"), stub("tool.shell", "shell")]);
        assert_eq!(slot.snapshot().expect("覆盖后在").registered, 2);
        assert!(!slot.is_empty());
    }
}
