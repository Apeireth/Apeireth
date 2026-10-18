//! 关怀面板只读面 (`GET /v1/panel/care`) —— 关怀规则读面的 HTTP 数据源。
//!
//! 数据面 = 组合根注入的 [`CarePanelQuery`] 端口, 其适配实现读
//! `care_rules.json` (与 `tool.care_schedule` 同一规则表同一拒开口径: StoredDoc
//! 拒开 = 失败帧报真因, 不静默降级不编数)。读一次响应 = **一次快照读**
//! (W2-③ 面板一致性): 表体一次装载, 各节同出一刻。
//!
//! 响应帧两段 (稳定合同, 字段改名须同步 `docs/gateway-api-contract.md` §7f 与
//! `docs/02-guides/frontend-data-contract.md` §10.10):
//!
//! - **`rules` / `discipline` / `last_care` = `care_list` 帧同形** —— 与
//!   `tool.care_schedule` `care_list` 动作的投影逐字段同名同义 (规则行 =
//!   `rule_json` 同形 + 两列真值补列 `last_fired_at_ms` / `next_trigger_at_ms`,
//!   台账行 = `record_json` 同形); 触发器投影走 organ `CareTrigger` 的同一
//!   serde 形状 (内联 `kind` 标签), 逐字同形;
//! - **`status` = `tool.self_status` 关怀节真值** (规则数 / 下次触发 / 最近关怀
//!   含判定文案 / **心跳两列**), 与自述关怀节同一表体投影 (口径唯一)。
//!
//! **只读**: 本面零写面 —— 改规则走 `tool.care_schedule` (care_add/care_remove =
//! 写文件同族, 每调用停人工审批), 面板不越权。

use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::panels::GatewayState;

/// 打扰纪律配置 (care_list 帧 discipline 节同形; 显式 null = 关窗)。
#[derive(Debug, Clone, Serialize)]
pub struct CareDisciplineDto {
    /// 勿扰窗口起 (分钟 of day; `null` = 显式关窗)。
    pub quiet_start_minutes: Option<u32>,
    /// 勿扰窗口止 (分钟 of day; 跨午夜 = 起 > 止)。
    pub quiet_end_minutes: Option<u32>,
    /// 频率上限 (滚动窗口档)。
    pub frequency_max: u32,
    /// 频率窗口 (小时)。
    pub frequency_window_hours: u32,
    /// 频率上限 (日档)。
    pub daily_max: u32,
    /// 专注抑制 (分钟)。
    pub focus_gap_minutes: u32,
    /// 作息时刻过点补发窗口 (分钟)。
    pub catchup_minutes: u32,
}

/// 一条关怀规则 (care_list 帧 `rule_json` 同形 + 两列真值补列)。
#[derive(Debug, Clone, Serialize)]
pub struct CareRuleDto {
    /// 规则 id (闭合字符集)。
    pub id: String,
    /// 触发器 (organ `CareTrigger` 同一 serde 形状: 内联 `kind` 标签)。
    pub trigger: serde_json::Value,
    /// 触发器稳定词 (`inactive` / `daily_moment` / `anniversary` / `interval`)。
    pub trigger_kind: String,
    /// 素材 (主题/方向)。
    pub material: String,
    /// 是否启用 (false = 停用留档)。
    pub enabled: bool,
    /// 入表时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 每规则最近发起时刻 (epoch 毫秒); `null` = 尚无发起记录。
    pub last_fired_at_ms: Option<i64>,
    /// 每规则下次触发真值 (`CareTrigger::next_occurrence_ms` 同口径推算)。
    pub next_trigger_at_ms: i64,
}

/// 一条关怀台账 (care_list 帧 `record_json` 同形)。
#[derive(Debug, Clone, Serialize)]
pub struct CareRecordDto {
    /// 触发的规则 id。
    pub rule_id: String,
    /// 发起/判定时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 发起意愿判定档。
    pub judge: String,
    /// 是否真的发起 (false = 显式判定说不, 留痕)。
    pub spoke: bool,
    /// 如实备注。
    pub note: String,
}

/// 关怀节真值 (`tool.self_status` 关怀节同形, 含心跳两列)。
///
/// 帧级读失败走 500 错误帧 (错误即帧), 不降级成「全维 null + reason」;
/// 心跳两列 `null` = 心跳生产者未接线/未运行 (如实, 不冒充在跑)。
#[derive(Debug, Clone, Serialize)]
pub struct CarePanelStatusDto {
    /// 关怀规则数 (真值计数)。
    pub rules: u64,
    /// 下次触发时刻 (epoch 毫秒); `null` = 无启用规则。
    pub next_trigger_at_ms: Option<i64>,
    /// 最近关怀时刻 (epoch 毫秒); `null` = 尚无关怀。
    pub last_care_at_ms: Option<i64>,
    /// 最近关怀判定真值文案 (与自述关怀节同一口径)。
    pub last_care_note: Option<String>,
    /// 心跳态: 下次 poll 时刻; `null` = 心跳生产者未运行/未接线。
    pub heartbeat_next_poll_at_ms: Option<i64>,
    /// 心跳态: 最近心跳命中时刻; `null` = 尚无心跳命中。
    pub heartbeat_last_hit_at_ms: Option<i64>,
}

/// 一次关怀快照 (care_list 帧同形三节 + 关怀节真值, 同一刻读出)。
#[derive(Debug, Clone, Serialize)]
pub struct CarePanelDto {
    /// 关怀节真值 (含心跳两列)。
    pub status: CarePanelStatusDto,
    /// 规则列表 (care_list 帧同形 + 真值补列)。
    pub rules: Vec<CareRuleDto>,
    /// 打扰纪律 (care_list 帧同形)。
    pub discipline: CareDisciplineDto,
    /// 最近关怀台账 (care_list 帧同形; `null` = 尚无关怀)。
    pub last_care: Option<CareRecordDto>,
}

/// 关怀面板只读投影端口 (面板族 bounded-context 端口; 适配实现在组合根,
/// 读 `care_rules.json` —— 与 `tool.care_schedule` 同表同拒开口径)。
#[async_trait]
pub trait CarePanelQuery: Send + Sync {
    /// 一次快照读: care_list 帧同形三节 + 关怀节真值 (同一刻, 不拼两次读)。
    async fn care_frame(&self) -> Result<CarePanelDto, String>;
}

/// `GET /v1/panel/care` —— 关怀规则只读面 (只读; 过令牌门, 读面全员可读)。
///
/// 未接线 (端口未注入) = 501 `unsupported` 如实报, 不假装有数据源。
pub async fn list_care(State(state): State<GatewayState>) -> Response {
    let Some(care) = &state.services.care else {
        return crate::panels::unsupported("care.read");
    };
    match care.care_frame().await {
        Ok(frame) => (StatusCode::OK, Json(frame)).into_response(),
        Err(message) => crate::panels::panel_error(message),
    }
}
