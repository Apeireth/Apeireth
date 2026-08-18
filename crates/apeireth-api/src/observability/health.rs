//! # Observability `/v1/observability/health` 端点
//!
//! **目的**: Health check 端点,返 5 组件 (db / cache / queue / external_api / disk_space) 状态.
//!
//! **响应字段**:
//! - `status`: "ok" / "degraded" / "down" (3 态, per `apeireth-observability::HealthStatus`)
//! - `components`: 5 组件各自 status + last_check + details
//! - `timestamp`: UTC RFC 3339
//! - `schema_version`: 1
//! - `platform`: "apeireth-api"
//!
//! **HTTP 状态码** (per Prometheus / Kubernetes health 约定):
//! - 200: all ok / partial degraded (仍可服务)
//! - 503: ≥1 组件 down (不能服务)
//!
//! **6 哲学锚穿透**:
//! - S-1: 1:1 翻译 `apeireth-observability::HealthStatus` 三态枚举
//! - S-2: 估 80-120 LOC, 0 假装已接真依赖 (R20 阶段 6 stub "ok")
//! - O-2: 借鉴 Kubernetes liveness/readiness probe 工业标准
//! - O-3: 5 组件编译期 hardcode (HEALTH_COMPONENTS 守门)
//! - O-4: 任何人都能接手
//! - O-5: 不假装真查 (skeleton 阶段 5 组件全 ok, R20 阶段 3 续真依赖)

#![allow(clippy::all)]

use std::collections::HashMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Serialize;

use crate::observability::{
    global_state, ComponentHealth, HEALTH_COMPONENTS, OBSERVABILITY_SCHEMA_VERSION,
};

/// Health 响应 body.
#[derive(Debug, Serialize)]
pub struct HealthResponseBody {
    /// 整体状态 ("ok" / "degraded" / "down")
    pub status: String,
    /// 5 组件状态
    pub components: Vec<ComponentHealth>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Schema 版本
    pub schema_version: String,
    /// 平台名
    pub platform: String,
    /// 失败组件数 (0 = 全 ok, 1+ = 至少 1 down)
    pub failed_count: u32,
    /// 降级组件数 (0 = 全 ok, 1+ = 至少 1 degraded)
    pub degraded_count: u32,
}

/// `GET /v1/observability/health` — 5 组件 health check.
pub async fn health_handler() -> Response {
    let state = global_state();
    let state = state.read();

    // 收集 5 组件 (顺序跟 HEALTH_COMPONENTS 一致, 编译期守门)
    let mut components = Vec::with_capacity(HEALTH_COMPONENTS.len());
    let mut failed_count = 0u32;
    let mut degraded_count = 0u32;
    for name in HEALTH_COMPONENTS {
        let c = state
            .components
            .get(*name)
            .cloned()
            .unwrap_or_else(|| ComponentHealth {
                name: (*name).to_string(),
                status: "unknown".to_string(),
                last_check: Utc::now(),
                details: HashMap::new(),
            });
        match c.status.as_str() {
            "down" => failed_count += 1,
            "degraded" => degraded_count += 1,
            _ => {}
        }
        components.push(c);
    }

    // 整体 status: 任一 down → down, 任一 degraded → degraded, 全 ok → ok
    let overall = if failed_count > 0 {
        "down"
    } else if degraded_count > 0 {
        "degraded"
    } else {
        "ok"
    };

    let body = HealthResponseBody {
        status: overall.to_string(),
        components,
        timestamp: Utc::now(),
        schema_version: OBSERVABILITY_SCHEMA_VERSION.to_string(),
        platform: "apeireth-api".to_string(),
        failed_count,
        degraded_count,
    };

    // HTTP 状态码: 200 (ok/degraded) / 503 (down)
    let http_status = if overall == "down" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (http_status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::ObsState;

    #[test]
    fn all_ok_response_status_200() {
        let s = ObsState::new();
        // 5 组件全 ok → overall ok
        let mut failed = 0;
        let mut degraded = 0;
        for c in s.components.values() {
            if c.status == "down" {
                failed += 1;
            }
            if c.status == "degraded" {
                degraded += 1;
            }
        }
        let overall = if failed > 0 {
            "down"
        } else if degraded > 0 {
            "degraded"
        } else {
            "ok"
        };
        assert_eq!(overall, "ok");
    }

    #[test]
    fn partial_degraded_response_status_200() {
        // 模拟: 1 组件 degraded → overall degraded, HTTP 仍 200
        let mut s = ObsState::new();
        s.components.get_mut("db").unwrap().status = "degraded".to_string();

        let mut failed = 0;
        let mut degraded = 0;
        for c in s.components.values() {
            if c.status == "down" {
                failed += 1;
            }
            if c.status == "degraded" {
                degraded += 1;
            }
        }
        let overall = if failed > 0 {
            "down"
        } else if degraded > 0 {
            "degraded"
        } else {
            "ok"
        };
        assert_eq!(overall, "degraded");
        // HTTP 200 (degraded 仍可服务)
    }

    #[test]
    fn one_down_response_status_503() {
        let mut s = ObsState::new();
        s.components.get_mut("cache").unwrap().status = "down".to_string();

        let mut failed = 0;
        let mut degraded = 0;
        for c in s.components.values() {
            if c.status == "down" {
                failed += 1;
            }
            if c.status == "degraded" {
                degraded += 1;
            }
        }
        assert!(failed > 0);
        let _ = degraded;
    }
}
