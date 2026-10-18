//! 性格面板只读面 (`GET /v1/panel/traits`) —— 四滑杆现值 / 锁定态 / 最近移动的
//! HTTP 数据源。
//!
//! 数据面 = 组合根注入的 [`TraitPanelQuery`] 端口, 其适配实现读**成长计既有账**
//! (`tuning-log.jsonl`, 自校准沉积记录) + **锁定档** (`trait-locks.json`) ——
//! 与 `tool.trait_report` / 自述调参节**同账同源** (同一本账、同一份锁定档、
//! 同一套判据, 不新造账)。读一次响应 = **一次快照读** (W2-③ 面板一致性):
//! 账面一次扫描, 各节同出一刻。
//!
//! 响应帧 (稳定合同, 字段改名须同步 `docs/gateway-api-contract.md` §7f 与
//! `docs/02-guides/frontend-data-contract.md` §10.11):
//!
//! | 字段 | 含义 |
//! |---|---|
//! | `values` | 四滑杆现值 (生效值: 进程内自动调整 > env 旋钮 > 基线) |
//! | `locked` | 四杆锁定态 (`trait-locks.json`; 缺档 = 全解锁默认态, 坏档即帧) |
//! | `recent_moves` | 最近移动 (新 → 旧, ≤4 条; `trait_report::recent_moves` 同判据: 零移动留痕与未来不计) |
//! | `records` | 成长计既有账账面行 (周报 7 天窗同账投影的数据面) |
//! | `skipped_records` | 账面坏行跳过计数 (0 装诚实, 不静默吞) |
//!
//! **只读**: 本面零写面 —— 锁定/解锁是 CLI 显式动作 (`apeireth trait lock|unlock`),
//! 调参走工具面/自动调参, 面板不越权。

use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::panels::GatewayState;

/// 四滑杆现值 (生效值; 与 `tool.self_status` 调参节同源)。
#[derive(Debug, Clone, Serialize)]
pub struct TraitValuesDto {
    /// 遗忘衰减强度倍率当前生效值。
    pub memory_fade: f64,
    /// 好奇强度倍率当前生效值。
    pub curiosity_strength: f64,
    /// 语气情绪饱和倍率当前生效值。
    pub tone_saturation: f64,
    /// 整合间隔 (回合数) 当前生效值。
    pub consolidation_cadence: f64,
}

/// 四杆锁定态 (与 `trait-locks.json` 同真值; 解锁是显式动作)。
#[derive(Debug, Clone, Serialize)]
pub struct TraitLocksDto {
    /// 遗忘衰减强度滑杆锁定态。
    pub memory_fade: bool,
    /// 好奇强度滑杆锁定态。
    pub curiosity_strength: bool,
    /// 语气情绪饱和度滑杆锁定态。
    pub tone_saturation: bool,
    /// 整合节奏滑杆锁定态。
    pub consolidation_cadence: bool,
}

/// 一次滑杆移动 (`trait_report::TraitMove` / 自述调参节同形)。
#[derive(Debug, Clone, Serialize)]
pub struct TraitMoveDto {
    /// 滑杆 id (snake_case 稳定词)。
    pub param: String,
    /// 滑杆中文名 (与调参面板同名)。
    pub label: String,
    /// 移动时刻 (epoch 毫秒)。
    pub at_epoch_ms: i64,
    /// 移动量 (带符号: next - previous)。
    pub delta: f64,
    /// 移动原因 (沉积记录的 reason 原文)。
    pub reason: String,
}

/// 成长计既有账一行 (`self_tuning::TuningRecord` / tuning-log.jsonl 同形)。
#[derive(Debug, Clone, Serialize)]
pub struct TuningRecordDto {
    /// 沉积序号。
    pub seq: u64,
    /// 被调整的滑杆 id。
    pub param: String,
    /// 调整前值。
    pub previous: f64,
    /// 调整后值。
    pub next: f64,
    /// 调整原因 (原文)。
    pub reason: String,
    /// 沉积时刻 (epoch 毫秒)。
    pub at_epoch_ms: i64,
}

/// 一次性格成长账快照 (四滑杆现值 + 锁定态 + 最近移动 + 账面行, 同一刻读出)。
#[derive(Debug, Clone, Serialize)]
pub struct TraitPanelDto {
    /// 四滑杆现值 (生效值)。
    pub values: TraitValuesDto,
    /// 四杆锁定态。
    pub locked: TraitLocksDto,
    /// 最近移动 (新 → 旧, ≤4 条)。
    pub recent_moves: Vec<TraitMoveDto>,
    /// 成长计既有账账面行 (含零移动留痕行)。
    pub records: Vec<TuningRecordDto>,
    /// 账面坏行跳过计数 (不静默吞)。
    pub skipped_records: usize,
}

/// 性格面板只读投影端口 (面板族 bounded-context 端口; 适配实现在组合根,
/// 与 `tool.trait_report` 读同一本成长计账 + 同一份锁定档)。
#[async_trait]
pub trait TraitPanelQuery: Send + Sync {
    /// 一次快照读: 现值 / 锁定态 / 最近移动 / 账面行 (同一刻, 不拼两次读)。
    async fn trait_frame(&self) -> Result<TraitPanelDto, String>;
}

/// `GET /v1/panel/traits` —— 性格成长账只读面 (只读; 过令牌门, 读面全员可读)。
///
/// 未接线 (端口未注入) = 501 `unsupported` 如实报, 不假装有数据源。
pub async fn list_traits(State(state): State<GatewayState>) -> Response {
    let Some(traits) = &state.services.traits else {
        return crate::panels::unsupported("traits.read");
    };
    match traits.trait_frame().await {
        Ok(frame) => (StatusCode::OK, Json(frame)).into_response(),
        Err(message) => crate::panels::panel_error(message),
    }
}
