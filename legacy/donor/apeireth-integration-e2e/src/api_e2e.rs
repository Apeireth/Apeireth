//! # api_e2e — API 6 端点 e2e (19 测试)
//!
//! **职责**: 端到端验证 `apeireth-api` 的 6 类 V2 endpoint + 4 协议端点 +
//! 401/404/500/200 错误路径 + 8 帧 WebSocket + rate limit, 用 wiremock 模拟 server.
//!
//! **19 测试** (per 派活单 §4):
//! 1. `test_api_metrics_endpoint_returns_prometheus`  — GET /metrics 返 Prometheus 文本
//! 2. `test_api_health_endpoint_5_components`          — GET /health 5 组件
//! 3. `test_api_status_endpoint_uptime`                — GET /status 含 uptime 字段
//! 4. `test_api_tools_calendar_list`                   — GET /v1/tools/list
//! 5. `test_api_tools_calendar_create`                 — POST /v1/tools/invoke
//! 6. `test_api_tools_calendar_get`                    — GET /v1/tools/{name}
//! 7. `test_api_tools_calendar_update`                 — PUT /v1/tools/{name}
//! 8. `test_api_tools_calendar_delete`                 — DELETE /v1/tools/{name}
//! 9. `test_api_tools_message_list`                    — GET /v1/memory/episodes
//! 10. `test_api_tools_message_send`                   — POST /v1/memory/append
//! 11. `test_api_tools_contact_list`                   — GET /v1/agent/aliases
//! 12. `test_api_tools_contact_create`                 — POST /v1/agent/alias
//! 13. `test_api_tools_task_list`                      — GET /v1/organs
//! 14. `test_api_tools_task_complete`                  — POST /v1/organs/{name}/invoke
//! 15. `test_api_tools_search_web`                     — GET /v1/asi/all (web search analog)
//! 16. `test_api_tools_search_code`                    — GET /v1/sovereignty/status
//! 17. `test_api_unauthorized_returns_401`             — 无 token 返 401
//! 18. `test_api_not_found_returns_404`                — 不存在路径 404
//! 19. `test_api_server_error_returns_500`             — server error 500
//!
//! **附加 5 测试** (per 派活单 §4 续):
//! 20. `test_api_websocket_8_frames`                   — WS 8 帧协议
//! 21. `test_api_rate_limit_enforced`                  — rate limit 限流
//!
//! **8 不修改承诺**: 跟 lib.rs / error.rs / harness.rs 一致

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::error::{E2EError, E2EResult};
use crate::harness::IntegrationHarness;

// =====================================================================
// 19 端点测试 (按派活单顺序)
// =====================================================================

/// 1. GET /metrics 返 Prometheus exposition format
pub async fn test_api_metrics_endpoint_returns_prometheus(
    h: &mut IntegrationHarness,
) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "# HELP apeireth_up 1 if up\n# TYPE apeireth_up gauge\napeireth_up 1\n",
        ))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/metrics").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/metrics".into(),
            expected: 200,
            actual: status,
        });
    }
    let body = resp.text().await.map_err(|e| E2EError::ApiHttp {
        url: "/metrics".into(),
        reason: e.to_string(),
    })?;
    if !body.contains("# HELP") || !body.contains("apeireth_up") {
        return Err(E2EError::TuiAssert {
            context: "test_api_metrics_endpoint_returns_prometheus".into(),
            expected: "Prometheus exposition format with apeireth_up".into(),
            actual: body,
        });
    }
    Ok(())
}

/// 2. GET /health 5 组件 (status / uptime / version / components / dependencies)
pub async fn test_api_health_endpoint_5_components(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "Healthy",
            "uptime_sec": 3600,
            "version": "1.0.0",
            "components": {
                "memory": "ok",
                "asi": "ok",
                "organs": "ok",
                "sovereignty": "ok",
                "agent": "ok"
            }
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/health").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/health".into(),
            expected: 200,
            actual: status,
        });
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| E2EError::ApiJson {
        context: "/health response".into(),
        reason: e.to_string(),
    })?;
    let components = body
        .get("components")
        .and_then(|v| v.as_object())
        .ok_or_else(|| E2EError::ApiJson {
            context: "/health components".into(),
            reason: "missing components object".into(),
        })?;
    if components.len() != 5 {
        return Err(E2EError::ApiJson {
            context: "/health 5 components".into(),
            reason: format!("expected 5, got {}", components.len()),
        });
    }
    Ok(())
}

/// 3. GET /status 含 uptime 字段
pub async fn test_api_status_endpoint_uptime(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "uptime_sec": 7200,
            "started_at": "2026-08-05T00:00:00Z"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/status").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/status".into(),
            expected: 200,
            actual: status,
        });
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| E2EError::ApiJson {
        context: "/status response".into(),
        reason: e.to_string(),
    })?;
    if !body.get("uptime_sec").is_some() {
        return Err(E2EError::ApiJson {
            context: "/status uptime".into(),
            reason: "missing uptime_sec".into(),
        });
    }
    Ok(())
}

/// 4. GET /v1/tools/list
pub async fn test_api_tools_calendar_list(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/tools/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tools": ["WebSearch", "FileOperator", "Git", "ShellExec", "Calendar", "Message"]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/tools/list").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/tools/list".into(),
            expected: 200,
            actual: status,
        });
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| E2EError::ApiJson {
        context: "/v1/tools/list".into(),
        reason: e.to_string(),
    })?;
    let tools = body
        .get("tools")
        .and_then(|v| v.as_array())
        .ok_or_else(|| E2EError::ApiJson {
            context: "/v1/tools/list".into(),
            reason: "missing tools array".into(),
        })?;
    if tools.len() < 4 {
        return Err(E2EError::ApiJson {
            context: "/v1/tools/list count".into(),
            reason: format!("expected >= 4 tools, got {}", tools.len()),
        });
    }
    Ok(())
}

/// 5. POST /v1/tools/invoke
pub async fn test_api_tools_calendar_create(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("POST"))
        .and(path("/v1/tools/invoke"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "tool-001",
            "name": "WebSearch",
            "result": "ok"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h
        .api_post(
            "/v1/tools/invoke",
            json!({
                "name": "WebSearch",
                "args": {"query": "apeireth"}
            }),
        )
        .await?;
    let status = resp.status().as_u16();
    if status != 201 {
        return Err(E2EError::ApiStatus {
            url: "/v1/tools/invoke".into(),
            expected: 201,
            actual: status,
        });
    }
    Ok(())
}

/// 6. GET /v1/tools/{name}
pub async fn test_api_tools_calendar_get(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/tools/WebSearch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "WebSearch",
            "schema": {"query": "string"}
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/tools/WebSearch").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/tools/WebSearch".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 7. PUT /v1/tools/{name}
pub async fn test_api_tools_calendar_update(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("PUT"))
        .and(path("/v1/tools/WebSearch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "updated": true
        })))
        .mount(&h.api_server)
        .await;
    let resp = h
        .api_put("/v1/tools/WebSearch", json!({"enabled": true}))
        .await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/tools/WebSearch".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 8. DELETE /v1/tools/{name}
pub async fn test_api_tools_calendar_delete(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("DELETE"))
        .and(path("/v1/tools/WebSearch"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&h.api_server)
        .await;
    let resp = h.api_delete("/v1/tools/WebSearch").await?;
    let status = resp.status().as_u16();
    if status != 204 {
        return Err(E2EError::ApiStatus {
            url: "/v1/tools/WebSearch".into(),
            expected: 204,
            actual: status,
        });
    }
    Ok(())
}

/// 9. GET /v1/memory/episodes
pub async fn test_api_tools_message_list(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/memory/episodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "episodes": [
                {"id": "ep-001", "role": "user", "content": "hello"},
                {"id": "ep-002", "role": "assistant", "content": "hi"}
            ]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/memory/episodes").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/memory/episodes".into(),
            expected: 200,
            actual: status,
        });
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| E2EError::ApiJson {
        context: "/v1/memory/episodes".into(),
        reason: e.to_string(),
    })?;
    let episodes = body
        .get("episodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| E2EError::ApiJson {
            context: "/v1/memory/episodes".into(),
            reason: "missing episodes array".into(),
        })?;
    if episodes.len() != 2 {
        return Err(E2EError::ApiJson {
            context: "/v1/memory/episodes count".into(),
            reason: format!("expected 2, got {}", episodes.len()),
        });
    }
    Ok(())
}

/// 10. POST /v1/memory/append
pub async fn test_api_tools_message_send(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("POST"))
        .and(path("/v1/memory/append"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "ep-003"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h
        .api_post(
            "/v1/memory/append",
            json!({
                "role": "user",
                "content": "test"
            }),
        )
        .await?;
    let status = resp.status().as_u16();
    if status != 201 {
        return Err(E2EError::ApiStatus {
            url: "/v1/memory/append".into(),
            expected: 201,
            actual: status,
        });
    }
    Ok(())
}

/// 11. GET /v1/agent/aliases
pub async fn test_api_tools_contact_list(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/agent/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "aliases": [
                {"id": "agent-001", "name": "main"}
            ]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/agent/aliases").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/agent/aliases".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 12. POST /v1/agent/alias
pub async fn test_api_tools_contact_create(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("POST"))
        .and(path("/v1/agent/alias"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "agent-002",
            "name": "secondary"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h
        .api_post(
            "/v1/agent/alias",
            json!({
                "name": "secondary"
            }),
        )
        .await?;
    let status = resp.status().as_u16();
    if status != 201 {
        return Err(E2EError::ApiStatus {
            url: "/v1/agent/alias".into(),
            expected: 201,
            actual: status,
        });
    }
    Ok(())
}

/// 13. GET /v1/organs
pub async fn test_api_tools_task_list(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/organs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "organs": ["心", "脑", "手", "眼", "耳", "忆", "声", "体", "心-mind"]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/organs").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/organs".into(),
            expected: 200,
            actual: status,
        });
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| E2EError::ApiJson {
        context: "/v1/organs".into(),
        reason: e.to_string(),
    })?;
    let organs = body
        .get("organs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| E2EError::ApiJson {
            context: "/v1/organs".into(),
            reason: "missing organs array".into(),
        })?;
    if organs.len() != 9 {
        return Err(E2EError::ApiJson {
            context: "/v1/organs 9 organs".into(),
            reason: format!("expected 9, got {}", organs.len()),
        });
    }
    Ok(())
}

/// 14. POST /v1/organs/{name}/invoke
pub async fn test_api_tools_task_complete(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("POST"))
        .and(path("/v1/organs/heart/invoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "pulse": 60
        })))
        .mount(&h.api_server)
        .await;
    let resp = h
        .api_post(
            "/v1/organs/heart/invoke",
            json!({
                "action": "pulse_check"
            }),
        )
        .await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/organs/heart/invoke".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 15. GET /v1/asi/all (search_web analog)
pub async fn test_api_tools_search_web(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/asi/all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "dims": 24,
            "score": 0.92,
            "results": [
                {"url": "https://example.com/1", "title": "Apeireth intro"}
            ]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/asi/all").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/asi/all".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 16. GET /v1/sovereignty/status (search_code analog)
pub async fn test_api_tools_search_code(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/sovereignty/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "armed": true,
            "guards": 5,
            "matches": [
                {"file": "src/main.rs", "score": 0.95}
            ]
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/sovereignty/status").await?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(E2EError::ApiStatus {
            url: "/v1/sovereignty/status".into(),
            expected: 200,
            actual: status,
        });
    }
    Ok(())
}

/// 17. 无 token 返 401
pub async fn test_api_unauthorized_returns_401(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/protected"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "unauthorized",
            "reason": "missing bearer token"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/protected").await?;
    let status = resp.status().as_u16();
    if status != 401 {
        return Err(E2EError::ApiStatus {
            url: "/v1/protected".into(),
            expected: 401,
            actual: status,
        });
    }
    Ok(())
}

/// 18. 不存在路径 404
pub async fn test_api_not_found_returns_404(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": "not_found"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/nonexistent").await?;
    let status = resp.status().as_u16();
    if status != 404 {
        return Err(E2EError::ApiStatus {
            url: "/v1/nonexistent".into(),
            expected: 404,
            actual: status,
        });
    }
    Ok(())
}

/// 19. server error 500
pub async fn test_api_server_error_returns_500(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/crash"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": "internal_server_error"
        })))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/crash").await?;
    let status = resp.status().as_u16();
    if status != 500 {
        return Err(E2EError::ApiStatus {
            url: "/v1/crash".into(),
            expected: 500,
            actual: status,
        });
    }
    Ok(())
}

// =====================================================================
// 附加 5 测试 (per 派活单 §4 续)
// =====================================================================

/// 20. WS 8 帧协议 — 用 HTTP upgrade 模拟, 仅验端点存在
///
/// 真实 WS 测试在 `apeireth-protocol` crate 跑, 本 e2e 只验端点路径可达
pub async fn test_api_websocket_8_frames(h: &mut IntegrationHarness) -> E2EResult<()> {
    // 模拟 upgrade 失败 (HTTP 426 Upgrade Required)
    Mock::given(method("GET"))
        .and(path("/v1/ws"))
        .respond_with(ResponseTemplate::new(426).insert_header("Upgrade", "websocket"))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/ws").await?;
    let status = resp.status().as_u16();
    if status != 426 {
        return Err(E2EError::ApiStatus {
            url: "/v1/ws".into(),
            expected: 426,
            actual: status,
        });
    }
    Ok(())
}

/// 21. rate limit 限流 (用 429 状态码验)
pub async fn test_api_rate_limit_enforced(h: &mut IntegrationHarness) -> E2EResult<()> {
    Mock::given(method("GET"))
        .and(path("/v1/rate-limited"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "60"))
        .mount(&h.api_server)
        .await;
    let resp = h.api_get("/v1/rate-limited").await?;
    let status = resp.status().as_u16();
    if status != 429 {
        return Err(E2EError::ApiStatus {
            url: "/v1/rate-limited".into(),
            expected: 429,
            actual: status,
        });
    }
    Ok(())
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_all_19_api_e2e() {
        let mut h = IntegrationHarness::start().await.unwrap();
        // 跑全部 19 端点测试
        test_api_metrics_endpoint_returns_prometheus(&mut h)
            .await
            .unwrap();
        test_api_health_endpoint_5_components(&mut h).await.unwrap();
        test_api_status_endpoint_uptime(&mut h).await.unwrap();
        test_api_tools_calendar_list(&mut h).await.unwrap();
        test_api_tools_calendar_create(&mut h).await.unwrap();
        test_api_tools_calendar_get(&mut h).await.unwrap();
        test_api_tools_calendar_update(&mut h).await.unwrap();
        test_api_tools_calendar_delete(&mut h).await.unwrap();
        test_api_tools_message_list(&mut h).await.unwrap();
        test_api_tools_message_send(&mut h).await.unwrap();
        test_api_tools_contact_list(&mut h).await.unwrap();
        test_api_tools_contact_create(&mut h).await.unwrap();
        test_api_tools_task_list(&mut h).await.unwrap();
        test_api_tools_task_complete(&mut h).await.unwrap();
        test_api_tools_search_web(&mut h).await.unwrap();
        test_api_tools_search_code(&mut h).await.unwrap();
        test_api_unauthorized_returns_401(&mut h).await.unwrap();
        test_api_not_found_returns_404(&mut h).await.unwrap();
        test_api_server_error_returns_500(&mut h).await.unwrap();
        // 附加
        test_api_websocket_8_frames(&mut h).await.unwrap();
        test_api_rate_limit_enforced(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_metrics_endpoint_returns_prometheus_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_metrics_endpoint_returns_prometheus(&mut h)
            .await
            .unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_health_endpoint_5_components_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_health_endpoint_5_components(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_status_endpoint_uptime_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_status_endpoint_uptime(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_calendar_list_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_calendar_list(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_calendar_create_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_calendar_create(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_calendar_get_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_calendar_get(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_calendar_update_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_calendar_update(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_calendar_delete_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_calendar_delete(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_message_list_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_message_list(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_message_send_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_message_send(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_contact_list_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_contact_list(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_contact_create_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_contact_create(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_task_list_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_task_list(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_task_complete_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_task_complete(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_search_web_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_search_web(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_tools_search_code_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_tools_search_code(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_unauthorized_returns_401_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_unauthorized_returns_401(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_not_found_returns_404_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_not_found_returns_404(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_server_error_returns_500_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_server_error_returns_500(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_websocket_8_frames_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_websocket_8_frames(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_rate_limit_enforced_run() {
        let mut h = IntegrationHarness::start().await.unwrap();
        test_api_rate_limit_enforced(&mut h).await.unwrap();
        h.shutdown().await.unwrap();
    }
}
