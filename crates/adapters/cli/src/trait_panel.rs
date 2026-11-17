//! 性格面板面适配 (组合根): 成长计既有账 + 锁定档 → 四滑杆现值 / 锁定态 /
//! 最近移动。
//!
//! 本模块自 `crate::gateway_panels` 拆出 (架构尺寸棘轮压回 · 行为等价搬迁):
//! 四滑杆口径、锁定档来源、最近移动条数与账本来源一律不变; 公开路径经
//! `crate::gateway_panels` 再导出保持不变。
//!
//! 刻意不做什么: 不做锁定/解锁 (那是 CLI 显式动作面 `trait_command`), 不改成长计
//! 既有账的读法。

use std::sync::Arc;

use apeireth_gateway::{
    GatewayServices, TraitLocksDto, TraitMoveDto, TraitPanelDto, TraitPanelQuery, TraitValuesDto,
    TuningRecordDto,
};
use apeireth_orchestration::trait_report::{recent_moves, TRAIT_RECENT_MOVES};
use apeireth_runtime_assembly::{TraitGrowthFacts, TraitGrowthPaths};
use async_trait::async_trait;

/// 性格面板面适配 (组合根): 成长计既有账 + 锁定档 → 四滑杆现值 / 锁定态 /
/// 最近移动。
///
/// 与 `tool.trait_report` / 自述调参节**同账同源**: 同一本成长计账
/// (`tuning-log.jsonl`) + 同一份锁定档 (`trait-locks.json`), 同一套判据
/// (`trait_report::recent_moves`); 锁定档坏档 = 失败帧即报真因 (绝不读成
/// 全解锁)。一次快照读: 账面一次扫描, 各节同出一刻。
///
/// **零写面**: 锁定/解锁走 CLI 显式动作 (`apeireth trait lock|unlock`),
/// 调参走工具面/自动调参, 本适配只读。
pub struct CliTraitPanel {
    paths: TraitGrowthPaths,
}

impl CliTraitPanel {
    /// 以成长计既有账路径为锚 (锁定档 = 同目录 `trait-locks.json`, 与
    /// `tool.trait_report` 同一落位)。
    pub fn new(paths: TraitGrowthPaths) -> Self {
        Self { paths }
    }
}

#[async_trait]
impl TraitPanelQuery for CliTraitPanel {
    async fn trait_frame(&self) -> Result<TraitPanelDto, String> {
        // 一次快照读: 账面一次扫描 (成长计既有账 + 锁定档), 各节同出一刻。
        let facts = TraitGrowthFacts::read(&self.paths)?;
        let now_ms = apeireth_runtime_assembly::now_epoch_ms();
        let moves = recent_moves(&facts.records, now_ms, TRAIT_RECENT_MOVES)
            .into_iter()
            .map(|movement| TraitMoveDto {
                param: movement.param.id().to_string(),
                label: movement.label,
                at_epoch_ms: movement.at_epoch_ms,
                delta: movement.delta,
                reason: movement.reason,
            })
            .collect();
        let records = facts
            .records
            .iter()
            .map(|record| TuningRecordDto {
                seq: record.seq,
                param: record.param.id().to_string(),
                previous: record.previous,
                next: record.next,
                reason: record.reason.clone(),
                at_epoch_ms: record.at_epoch_ms,
            })
            .collect();
        Ok(TraitPanelDto {
            values: TraitValuesDto {
                memory_fade: facts.values.memory_fade,
                curiosity_strength: facts.values.curiosity_strength,
                tone_saturation: facts.values.tone_saturation,
                consolidation_cadence: facts.values.consolidation_cadence,
            },
            locked: TraitLocksDto {
                memory_fade: facts.locks.memory_fade,
                curiosity_strength: facts.locks.curiosity_strength,
                tone_saturation: facts.locks.tone_saturation,
                consolidation_cadence: facts.locks.consolidation_cadence,
            },
            recent_moves: moves,
            records,
            skipped_records: facts.skipped,
        })
    }
}

/// 把性格面板只读面接进网关服务容器 (组合根接线)。
pub(crate) fn wire_trait_panel(services: &mut GatewayServices) {
    // 性格面板只读面 (GET /v1/panel/traits): 四滑杆现值/锁定态/最近移动 ——
    // 与 `tool.trait_report` **同账同源** (同一成长计既有账 tuning-log.jsonl +
    // 同一锁定档 trait-locks.json)。只读零写面 (锁定/解锁走 CLI 显式动作)。
    services.traits = Some(Arc::new(CliTraitPanel::new(TraitGrowthPaths::new(
        apeireth_runtime_assembly::tuning_log_path(),
    ))));
}
