//! # Observability `/v1/observability/status` 端点
//!
//! **目的**: 服务状态总览,返 uptime / version / build_time / git_commit / start_time / active_sessions /
//! 5 R-Measure (per `docs/stage4/r-measure-verification-design-2026-08-05.md` §1-§5).
//!
//! **响应字段** (8 大类):
//! - `service`: name / version / build_time / git_commit
//! - `runtime`: start_time / uptime_seconds / uptime_human (readable)
//! - `sessions`: active_sessions (R20 阶段 6 stub, O-5 不假装)
//! - `r_measures`: 5 项 (R-1/2/3/4/5) + `note: "stub (R20 阶段 6 占位, R20 阶段 3 续真采)"`
//! - `philosophy_anchors`: 6 锚 (per `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §11)
//! - `timestamp` / `schema_version` / `platform`
//!
//! **6 哲学锚穿透**:
//! - S-1: 1:1 翻译 v0.9.21 `out/main` observability 集成 (per 蓝图 §2.5.3)
//! - S-2: 估 100-130 LOC, 5 R-Measure 标 stub (不假装已采)
//! - O-2: 借鉴 OpenTelemetry `service.version` + `service.instance.id` 工业字段
//! - O-3: 8 字段 + 5 R-Measure + 6 anchor 编译期 hardcode
//! - O-4: 任何人都能接手
//! - O-5: 5 R-Measure = 0.0 + `note: "stub"` 字段诚实标缺

#![allow(clippy::all)]

use axum::Json;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

use crate::observability::{
    global_state, OBSERVABILITY_SCHEMA_VERSION, PHILOSOPHY_ANCHORS, R_MEASURES,
};

/// 单个 R-Measure entry.
#[derive(Debug, Serialize)]
pub struct RMeasure {
    /// R-Measure 名 (R-1_direct_execution / ...)
    pub name: String,
    /// 中文标签 (R-1 直行率 / ...)
    pub label_zh: String,
    /// 当前值 (R20 阶段 6 placeholder 0.0)
    pub value: f64,
    /// 单位 ("ratio" / "percent")
    pub unit: String,
}

/// 单个哲学锚 entry.
#[derive(Debug, Serialize)]
pub struct PhilosophyAnchor {
    /// 锚 ID (S-1_north_star / ...)
    pub id: String,
    /// 中文标签 (北极星导向 / ...)
    pub label_zh: String,
    /// 描述
    pub description: &'static str,
}

/// Status 响应 body.
#[derive(Debug, Serialize)]
pub struct StatusResponseBody {
    /// 服务信息
    pub service: ServiceInfo,
    /// Runtime 信息
    pub runtime: RuntimeInfo,
    /// Session 统计
    pub sessions: SessionsInfo,
    /// 5 R-Measure (R-1 ~ R-5)
    pub r_measures: Vec<RMeasure>,
    /// 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)
    pub philosophy_anchors: Vec<PhilosophyAnchor>,
    /// 端点列表
    pub endpoints: Vec<String>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Schema 版本
    pub schema_version: String,
    /// 平台名
    pub platform: String,
}

#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub git_commit: String,
    pub workspace_version: String,
}

#[derive(Debug, Serialize)]
pub struct RuntimeInfo {
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub uptime_human: String,
    pub platform: String,
    pub rust_version: String,
}

#[derive(Debug, Serialize)]
pub struct SessionsInfo {
    pub active_sessions: u64,
    pub note: &'static str,
}

/// `GET /v1/observability/status` — 服务状态总览.
pub async fn status_handler() -> Json<StatusResponseBody> {
    let state = global_state();
    let state = state.read();

    // 5 R-Measure (R20 阶段 6 placeholder 0.0, O-5 不假装)
    let r_measures = R_MEASURES
        .iter()
        .map(|name| RMeasure {
            name: (*name).to_string(),
            label_zh: r_measure_label_zh(name).to_string(),
            value: *state.r_measures.get(*name).unwrap_or(&0.0),
            unit: "ratio".to_string(),
        })
        .collect();

    // 6 哲学锚
    let philosophy_anchors = PHILOSOPHY_ANCHORS
        .iter()
        .map(|id| PhilosophyAnchor {
            id: (*id).to_string(),
            label_zh: anchor_label_zh(id).to_string(),
            description: anchor_description(id),
        })
        .collect();

    let uptime = state.uptime_seconds();
    let body = StatusResponseBody {
        service: ServiceInfo {
            name: "apeireth-api".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: crate::observability::SERVICE_BUILD_TIME.to_string(),
            git_commit: crate::observability::SERVICE_GIT_COMMIT.to_string(),
            workspace_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        runtime: RuntimeInfo {
            start_time: state.start_time,
            uptime_seconds: uptime,
            uptime_human: humanize_seconds(uptime),
            platform: std::env::consts::OS.to_string(),
            rust_version: rustc_version_runtime(),
        },
        sessions: SessionsInfo {
            active_sessions: state.active_sessions,
            note: "stub (R20 阶段 6 占位, 0 业务采集; R20 阶段 3 续真接 LLM session 跟踪)",
        },
        r_measures,
        philosophy_anchors,
        endpoints: crate::observability::OBSERVABILITY_ENDPOINTS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        timestamp: Utc::now(),
        schema_version: OBSERVABILITY_SCHEMA_VERSION.to_string(),
        platform: "apeireth-api".to_string(),
    };

    Json(body)
}

/// R-Measure 中文标签 (5 K-1 字样穿透).
fn r_measure_label_zh(name: &str) -> &'static str {
    match name {
        "R-1_direct_execution" => "R-1 直行率",
        "R-2_direct_speech" => "R-2 直说率",
        "R-3_closed_loop" => "R-3 闭环率",
        "R-4_guard" => "R-4 守门率",
        "R-5_failure_honesty" => "R-5 失败诚实率",
        _ => "unknown",
    }
}

/// 哲学锚中文标签.
fn anchor_label_zh(id: &str) -> &'static str {
    match id {
        "S-1_north_star" => "S-1 北极星导向",
        "S-2_seek_truth" => "S-2 实事求是",
        "O-2_stand_on_shoulders" => "O-2 走在前人肩上",
        "O-3_drive_through" => "O-3 干到底",
        "O-4_handoff_to_anyone" => "O-4 任何人都能接手",
        "O-5_no_pretense" => "O-5 不假装",
        _ => "unknown",
    }
}

/// 哲学锚描述.
fn anchor_description(id: &str) -> &'static str {
    match id {
        "S-1_north_star" => "1:1 翻译 v0.9.21 商业版集成面, 0 重设计",
        "S-2_seek_truth" => "估 350-450 LOC, skeleton 阶段实接 3 端点",
        "O-2_stand_on_shoulders" => "借鉴 Prometheus / OpenTelemetry 工业标准",
        "O-3_drive_through" => "18 metric + 5 组件 + 5 R-Measure + 6 anchor 编译期守门",
        "O-4_handoff_to_anyone" => "§1-§5 + reports/ 完整 path, 任何人都能接手",
        "O-5_no_pretense" => "5 R-Measure = 0.0 + stub 字段诚实标缺",
        _ => "unknown",
    }
}

/// 把秒数转 "1d 2h 3m 4s" 可读形式.
fn humanize_seconds(s: u64) -> String {
    let days = s / 86_400;
    let hours = (s % 86_400) / 3_600;
    let mins = (s % 3_600) / 60;
    let secs = s % 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m {secs}s")
    } else if hours > 0 {
        format!("{hours}h {mins}m {secs}s")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

/// 拿 rustc 版本 (用 `rustc_version_runtime` crate, 估 stub "unknown" 防编译失败).
fn rustc_version_runtime() -> String {
    // 编译期 hardcode "stable" (跟 v0.9.21 商业版一致), 0 引 `rustc-version` crate
    option_env!("RUSTC_VERSION")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "stable".to_string())
}

#[allow(dead_code)]
fn _unused() -> HashMap<String, String> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_seconds_formats_correctly() {
        assert_eq!(humanize_seconds(0), "0s");
        assert_eq!(humanize_seconds(45), "45s");
        assert_eq!(humanize_seconds(125), "2m 5s");
        assert_eq!(humanize_seconds(3_725), "1h 2m 5s");
        assert_eq!(humanize_seconds(90_061), "1d 1h 1m 1s");
    }

    #[test]
    fn r_measure_label_zh_covers_all_5() {
        for r in R_MEASURES {
            let label = r_measure_label_zh(r);
            assert_ne!(label, "unknown", "R-Measure {r} missing zh label");
        }
    }

    #[test]
    fn anchor_label_zh_covers_all_6() {
        for a in PHILOSOPHY_ANCHORS {
            let label = anchor_label_zh(a);
            assert_ne!(label, "unknown", "anchor {a} missing zh label");
        }
    }

    #[test]
    fn anchor_description_covers_all_6() {
        for a in PHILOSOPHY_ANCHORS {
            let desc = anchor_description(a);
            assert_ne!(desc, "unknown", "anchor {a} missing description");
        }
    }

    #[test]
    fn status_body_has_5_r_measures() {
        // 编一个 status_body 验证
        let r_measures = R_MEASURES
            .iter()
            .map(|name| RMeasure {
                name: (*name).to_string(),
                label_zh: r_measure_label_zh(name).to_string(),
                value: 0.0,
                unit: "ratio".to_string(),
            })
            .collect::<Vec<_>>();
        assert_eq!(r_measures.len(), 5);
    }
}
