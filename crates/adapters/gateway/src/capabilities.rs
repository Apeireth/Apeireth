//! 能力清单面 (`GET /v1/apeireth/capabilities`): 网关/面板面能力 manifest。
//!
//! 本模块自 `crate::panels` 拆出 (架构尺寸棘轮压回 · 行为等价搬迁): 清单字段、
//! `supported`/`available` 口径、`schema_version` 与端点路径一律不变。

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::panels::GatewayState;

// ---------------------------------------------------------------------------
// Capability manifest（口径：只描述**网关/面板面**；`not_assembled` = 该 HTTP 面未装配，不等于能力不存在——`subagents.orchestration` 网关无端点但 CLI `apeireth subagent` 可用）
// ---------------------------------------------------------------------------

pub(crate) fn cap(
    id: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
) -> serde_json::Value {
    cap_with_reason(id, read, write, ops, supported, available, None)
}

fn cap_with_reason(
    id: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
    reason: Option<&str>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": id,
        "supported": supported,
        "read": read,
        "write": write,
        "version": 1,
        "operations": ops,
        "available": available,
    });
    if !available {
        value["reason"] = serde_json::json!(reason.unwrap_or(if supported {
            "provider_not_configured"
        } else {
            "platform_unsupported"
        }));
    }
    value
}

fn cap_alias(
    id: &str,
    alias_of: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
) -> serde_json::Value {
    let mut value = cap(id, read, write, ops, supported, available);
    value["alias_of"] = serde_json::json!(alias_of);
    value
}

pub(crate) async fn capabilities(State(state): State<GatewayState>) -> Response {
    let services = &state.services;
    let sessions_supported = services.sessions.is_some();
    let memory_supported = services.memory.is_some();
    let memory_write_supported = services.memory_commands.is_some();
    let memory_governance_supported = services.memory_governance.is_some();
    let tools_supported = services.tools.is_some();
    let trace_supported = services.traces.is_some();
    let audit_supported = services.audit.is_some();
    let permissions_supported = services.grants.is_some();
    let permissions_write_supported = services.grant_commands.is_some();
    let organs_supported = services.modules.is_some();
    let safety_supported = services.safety_guard.is_some();
    let workbench_supported = services.workbench.is_some();
    let user_admin_supported = services.user_admin.is_some();
    let domain_admin_supported = services.domain_admin.is_some();
    let skills_supported = services.skills.is_some();
    let tasks_supported = services.tasks.is_some();
    let care_supported = services.care.is_some();
    let traits_supported = services.traits.is_some();
    let workspaces_supported = services.workspaces.is_some();
    let contacts_supported = services.contacts.is_some();

    let memory_ids = [
        ("memory.read", true, false, &["list", "read"] as &[&str]),
        ("memory.write", false, true, &["append"] as &[&str]),
        ("memory.update", false, true, &["update"] as &[&str]),
        ("memory.forget", false, true, &["forget"] as &[&str]),
        ("memory.protect", false, true, &["protect"] as &[&str]),
        ("memory.unprotect", false, true, &["unprotect"] as &[&str]),
    ];
    let memory = memory_ids
        .iter()
        .map(|(id, read, write, operations)| {
            let (supported, available) = match *id {
                "memory.read" => (memory_supported, memory_supported),
                "memory.write" => (memory_write_supported, memory_write_supported),
                // 纠正与治理面同源：注入 MemoryGovernanceStore 即可用
                // （存储侧 `update_episode_content` 早已实现，此前只是面板没接电）。
                _ => (memory_governance_supported, memory_governance_supported),
            };
            cap(id, *read, *write, operations, supported, available)
        })
        .chain(std::iter::once(cap(
            "memory.graph.read",
            true,
            false,
            &["graph"],
            memory_supported,
            memory_supported,
        )))
        .collect::<Vec<_>>();

    let manifest = serde_json::json!({
        "schema_version": 1,
        "runtime": { "service": "apeireth-gateway-2.0", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": [
            { "name": "health", "capabilities": [ cap("health", true, false, &["check"], true, true) ] },
            { "name": "models", "capabilities": [ cap("models.list", true, false, &["list"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "providers", "capabilities": [ cap("providers.list", true, false, &["list"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "runtime", "capabilities": [ cap("runtime.snapshot.read", true, false, &["read"], true, true) ] },
            { "name": "chat", "capabilities": [ cap("chat.completions", true, true, &["complete", "stream"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "sessions", "capabilities": [
                cap("sessions.read", true, false, &["list"], sessions_supported, sessions_supported),
                cap("sessions.delete", false, true, &["delete"], true, true),
            ] },
            { "name": "memory", "capabilities": memory },
            { "name": "tools", "capabilities": [ cap("tools.list", true, false, &["list"], tools_supported, tools_supported) ] },
            { "name": "skills", "capabilities": [ cap("skills.read", true, false, &["list"], skills_supported, skills_supported) ] },
            { "name": "care", "capabilities": [ cap("care.read", true, false, &["list"], care_supported, care_supported) ] },
            { "name": "traits", "capabilities": [ cap("traits.read", true, false, &["read"], traits_supported, traits_supported) ] },
            { "name": "tasks", "capabilities": [
                cap("tasks.read", true, false, &["list"], tasks_supported, tasks_supported),
                cap("tasks.human_judge_retry", false, true, &["retry"], tasks_supported, tasks_supported),
            ] },
            { "name": "workspaces", "capabilities": [
                cap("workspaces.read", true, false, &["list", "get"], workspaces_supported, workspaces_supported),
                cap("workspaces.write", false, true, &["create", "select", "close"], workspaces_supported, workspaces_supported),
            ] },
            crate::contact_panel::contacts_capability_group(contacts_supported),
            crate::session_binding::session_binding_capability_group(
                state.runtime.contact_registry().is_some()
                    && state.runtime.workspace_registry().is_some(),
                contacts_supported
                    && !state.user_access.is_active()
                    && (matches!(state.token_gate, crate::canonical_entry::GatewayTokenGate::Enforced(_))
                        || !state.client_tokens.is_empty()),
            ),
            { "name": "permissions", "capabilities": [
                cap("permissions.approval.read", true, false, &["list"], true, true),
                cap("permissions.approval.resolve", false, true, &["resolve"], true, true),
                cap_alias("approvals.read", "permissions.approval.read", true, false, &["list"], true, true),
                cap_alias("approvals.resolve", "permissions.approval.resolve", false, true, &["resolve"], true, true),
                cap("permissions.grants.read", true, false, &["list"], permissions_supported, permissions_supported),
                cap("permissions.revoke", false, true, &["revoke"], permissions_write_supported, permissions_write_supported),
            ] },
            { "name": "organs", "capabilities": [
                cap("organs.list", true, false, &["list"], organs_supported, organs_supported),
                cap("modules.list", true, false, &["list"], organs_supported, organs_supported),
            ] },
            { "name": "safety", "capabilities": [
                cap("safety.guard.status.read", true, false, &["read"], safety_supported, safety_supported),
                cap("safety.guard.events.read", true, false, &["list"], safety_supported, safety_supported),
                cap("safety.guard.evaluate", false, true, &["evaluate"], safety_supported, safety_supported),
            ] },
            { "name": "workbench", "capabilities": [
                cap("workbench.turn.read", true, false, &["read"], workbench_supported, workbench_supported),
            ] },
            { "name": "metrics", "capabilities": [
                cap("metrics.read", true, false, &["read"], true, true),
            ] },
            { "name": "users", "capabilities": [
                cap("users.read", true, false, &["list"], user_admin_supported, user_admin_supported),
                cap("users.manage", false, true, &["add", "role", "remove"], user_admin_supported, user_admin_supported),
            ] },
            { "name": "domains", "capabilities": [
                cap("domains.read", true, false, &["list"], domain_admin_supported, domain_admin_supported),
                cap("domains.manage", false, true, &["create"], domain_admin_supported, domain_admin_supported),
            ] },
            { "name": "voice", "capabilities": [
                cap_with_reason("voice.duplex", false, false, &["duplex", "stream"], false, false, Some("not_assembled")),
            ] },
            { "name": "subagents", "capabilities": [
                cap_with_reason("subagents.orchestration", false, false, &["spawn", "coordinate"], false, false, Some("not_assembled")),
            ] },
            { "name": "trace", "capabilities": [ cap("trace.read", true, false, &["list", "detail"], trace_supported, trace_supported) ] },
            { "name": "audit", "capabilities": [ cap("audit.read", true, false, &["list"], audit_supported, audit_supported) ] },
            { "name": "activity", "capabilities": [ cap("activity.sse", true, false, &["subscribe"], true, true) ] },
        ]
    });
    (StatusCode::OK, Json(manifest)).into_response()
}
