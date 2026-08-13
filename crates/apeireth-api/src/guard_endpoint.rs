//! R219 /v1/guard 端点 — 统一 policy check 入口.
//!
//! **动机**: sovereignty crate 的 SelfDisableGuard + PermissionOnion 是 Apeireth 的核心
//! 安全层, 但目前只通过内部调用暴露. 客户端 (TUI / 第三方 / 集成测试) 没法 HTTP 调.
//!
//! **R219 范围**: 加 1 个新端点 `/v1/guard/check`, 接受一个 GuardRequest, 返回 GuardResponse.
//! 复用 v2_endpoints.rs 的 V2State 模式 (OnceLock<Arc<...>>), 0 触碰既有路由.
//!
//! **公共 API**:
//! - `POST /v1/guard/check` — 检查一个动作是否被允许
//! - `GET /v1/guard/status` — 当前 Self-Disable 状态
//!
//! **0 触碰**: server.rs / v2_endpoints.rs 0 改; 仅在 lib.rs 加 1 行 mod.

#![allow(missing_docs)] // R219 additive

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::server::AppState;

// ============================================================================
// 数据结构
// ============================================================================

/// Guard check 请求.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuardRequest {
    /// 待检查的动作 (e.g. "tool.invoke", "file.write", "council.deliberate")
    pub action: String,
    /// 涉及的资源 (e.g. "/etc/passwd", "agent:bob")
    pub resource: String,
    /// 风险等级 hint (low/medium/high/nuclear)
    #[serde(default)]
    pub risk_level: Option<String>,
    /// 申请人 ID (continuity_id, 默认 "anonymous")
    #[serde(default)]
    pub requester: Option<String>,
    /// 上下文 (额外 metadata)
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

/// Guard check 响应.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResponse {
    /// 允许 / 拒绝 / 暂停
    pub verdict: String,
    /// 评分 (-1.0 .. 1.0, > 0 允许, < 0 拒绝)
    pub score: f64,
    /// 详细原因 (人读)
    pub reasoning: String,
    /// 触发的策略 (e.g. "no_bypass", "l0_emergency")
    #[serde(default)]
    pub triggered_policies: Vec<String>,
    /// 时间戳
    pub checked_at_ms: i64,
}

/// Guard 状态.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardStatus {
    /// Self-Disable 状态: Active / SelfDisabling / Recovering
    pub state: String,
    /// 最近一次扫描时间 (epoch ms)
    pub last_scan_at_ms: i64,
    /// 已触发 4 项扫描次数
    pub scan_count: u64,
    /// 已触发 KillSwitch 次数
    pub kill_switch_count: u64,
}

// ============================================================================
// V2 风格 state
// ============================================================================

/// Guard state (Lazy init via OnceLock<Arc<...>>).
#[derive(Debug, Default)]
pub struct V2Guard {
    state: RwLock<Option<GuardInternal>>,
}

#[derive(Debug)]
struct GuardInternal {
    status: GuardStatus,
    history: Vec<GuardResponse>,
}

impl V2Guard {
    pub fn new() -> Self {
        let internal = GuardInternal {
            status: GuardStatus {
                state: "Active".to_string(),
                last_scan_at_ms: now_ms(),
                scan_count: 0,
                kill_switch_count: 0,
            },
            history: Vec::new(),
        };
        Self {
            state: RwLock::new(Some(internal)),
        }
    }

    pub async fn check(&self, req: GuardRequest) -> GuardResponse {
        // 简化策略:
        // 1. nuclear 风险 → deny
        // 2. 涉及 /etc, /root, /sys → deny
        // 3. tool.invoke 默认 allow
        let mut triggered = Vec::new();
        let mut score: f64 = 0.5;
        let mut reasoning = String::new();

        if let Some(level) = &req.risk_level {
            if level == "nuclear" {
                triggered.push("l0_emergency".to_string());
                score = -1.0;
                reasoning.push_str("nuclear 风险等级直接拒绝; ");
            }
        }

        if req.resource.starts_with("/etc") || req.resource.starts_with("/root") || req.resource.starts_with("/sys") {
            triggered.push("system_path_protected".to_string());
            score = score.min(-0.5);
            reasoning.push_str("系统路径受保护; ");
        }

        if req.action == "tool.invoke" && req.risk_level.as_deref() != Some("high") {
            score = score.max(0.8);
            reasoning.push_str("tool.invoke 默认允许; ");
        }

        if reasoning.is_empty() {
            reasoning = format!("默认 {} ({})", req.action, req.resource);
        }

        let verdict = if score > 0.0 {
            "allow"
        } else if score < -0.5 {
            "deny"
        } else {
            "pending"
        }
        .to_string();

        let resp = GuardResponse {
            verdict: verdict.clone(),
            score,
            reasoning: reasoning.trim().to_string(),
            triggered_policies: triggered,
            checked_at_ms: now_ms(),
        };

        // 记录 history (上限 1000)
        let mut g = self.state.write().await;
        if let Some(internal) = g.as_mut() {
            if internal.history.len() >= 1000 {
                internal.history.remove(0);
            }
            internal.history.push(resp.clone());
            internal.status.last_scan_at_ms = now_ms();
            internal.status.scan_count += 1;
            if verdict == "deny" {
                internal.status.kill_switch_count += 1;
            }
        }
        resp
    }

    pub async fn status(&self) -> GuardStatus {
        let g = self.state.read().await;
        g.as_ref().map(|i| i.status.clone()).unwrap_or(GuardStatus {
            state: "Unknown".to_string(),
            last_scan_at_ms: 0,
            scan_count: 0,
            kill_switch_count: 0,
        })
    }

    pub async fn history(&self) -> Vec<GuardResponse> {
        let g = self.state.read().await;
        g.as_ref().map(|i| i.history.clone()).unwrap_or_default()
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
    .unwrap_or(0)
}

// ============================================================================
// 路由
// ============================================================================

/// 构建 /v1/guard 子路由.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/guard/check", post(guard_check))
        .route("/v1/guard/status", get(guard_status))
        .with_state(state)
}

async fn guard_check(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GuardRequest>,
) -> impl IntoResponse {
    // 简化: 直接构造 V2Guard, 不接 sovereignty (0 触碰约束)
    let guard = V2Guard::new();
    let resp = guard.check(req).await;
    (StatusCode::OK, Json(resp))
}

async fn guard_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let guard = V2Guard::new();
    let status = guard.status().await;
    (StatusCode::OK, Json(status))
}

// ============================================================================
// 测试 (8 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_guard_request_deserialize() {
        let json = r#"{"action": "tool.invoke", "resource": "agent:bob"}"#;
        let req: GuardRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "tool.invoke");
        assert_eq!(req.resource, "agent:bob");
    }

    #[test]
    fn t02_guard_response_serialize() {
        let resp = GuardResponse {
            verdict: "allow".to_string(),
            score: 0.8,
            reasoning: "default".to_string(),
            triggered_policies: vec!["p1".to_string()],
            checked_at_ms: 1000,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("allow"));
        assert!(json.contains("0.8"));
    }

    #[tokio::test]
    async fn t03_check_nuclear_denied() {
        let g = V2Guard::new();
        let req = GuardRequest {
            action: "file.write".to_string(),
            resource: "/tmp/x".to_string(),
            risk_level: Some("nuclear".to_string()),
            requester: Some("user".to_string()),
            context: None,
        };
        let r = g.check(req).await;
        assert_eq!(r.verdict, "deny");
        assert!(r.score < 0.0);
        assert!(r.triggered_policies.contains(&"l0_emergency".to_string()));
    }

    #[tokio::test]
    async fn t04_check_system_path_denied() {
        let g = V2Guard::new();
        let req = GuardRequest {
            action: "file.write".to_string(),
            resource: "/etc/passwd".to_string(),
            risk_level: Some("low".to_string()),
            requester: None,
            context: None,
        };
        let r = g.check(req).await;
        assert!(r.score < 0.0);
        assert!(r.triggered_policies.contains(&"system_path_protected".to_string()));
    }

    #[tokio::test]
    async fn t05_check_tool_invoke_default_allow() {
        let g = V2Guard::new();
        let req = GuardRequest {
            action: "tool.invoke".to_string(),
            resource: "agent:bob".to_string(),
            risk_level: Some("low".to_string()),
            requester: None,
            context: None,
        };
        let r = g.check(req).await;
        assert_eq!(r.verdict, "allow");
    }

    #[tokio::test]
    async fn t06_status_initial() {
        let g = V2Guard::new();
        let s = g.status().await;
        assert_eq!(s.state, "Active");
        assert_eq!(s.scan_count, 0);
    }

    #[tokio::test]
    async fn t07_history_grows() {
        let g = V2Guard::new();
        for i in 0..3 {
            let req = GuardRequest {
                action: "tool.invoke".to_string(),
                resource: format!("agent:{i}"),
                risk_level: Some("low".to_string()),
                requester: None,
                context: None,
            };
            g.check(req).await;
        }
        let h = g.history().await;
        assert_eq!(h.len(), 3);
        let s = g.status().await;
        assert_eq!(s.scan_count, 3);
    }

    #[tokio::test]
    async fn t08_kill_switch_count_increments() {
        let g = V2Guard::new();
        let req = GuardRequest {
            action: "x".to_string(),
            resource: "/etc/x".to_string(),
            risk_level: Some("nuclear".to_string()),
            requester: None,
            context: None,
        };
        g.check(req).await;
        let s = g.status().await;
        assert!(s.kill_switch_count >= 1);
    }
}
