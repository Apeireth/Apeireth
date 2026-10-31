//! Blocking DB work retains an owned native lease until completion, including caller cancellation.
use super::*;
pub(super) fn execute(
    state: &State,
    op: &str,
    call: &ToolCall,
    ctx: &ToolExecutionContext,
) -> ToolResult {
    if ctx.cancellation.is_cancelled() || Instant::now() >= ctx.deadline {
        return fail(call, "cancelled", "call no longer active");
    }
    let mapping = match &state.mappings {
        Some(mappings) => match mappings.lease(ctx) {
            Some(lease) => Some(lease),
            None => {
                return fail(
                    call,
                    "namespace_denied",
                    "Host mapping is missing or revoked",
                )
            }
        },
        None => None,
    };
    let principal = ctx.principal.as_ref().expect("checked execution context");
    let key = namespace_key(principal);
    let _write_guard = if matches!(op, "write" | "update" | "delete") {
        Some(state.writes.lock().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    };
    // A blocking worker waiting for the store mutex must recheck before mutation.
    if ctx.cancellation.is_cancelled() || Instant::now() >= ctx.deadline {
        return fail(call, "cancelled", "call no longer active");
    }
    if mapping.as_ref().is_some_and(|m| !m.authority.is_active()) {
        return fail(call, "namespace_denied", "Host mapping revoked");
    }
    let access = mapping.as_ref().and_then(|m| m.memory.as_ref());
    if mapping.is_some() && access.is_none() {
        return fail(call, "namespace_denied", "no memory store mapping");
    }
    if matches!(op, "write" | "update" | "delete") && access.is_some_and(|a| !a.writable_private) {
        return fail(call, "namespace_denied", "shared source is read-only");
    }
    let result = execute_checked(state, op, call, ctx, &key, access);
    if ctx.cancellation.is_cancelled() {
        return fail(call, "cancelled", "call no longer active");
    }
    if mapping.as_ref().is_some_and(|m| !m.authority.is_active()) {
        return fail(call, "namespace_denied", "Host mapping revoked");
    }
    match result {
        Ok(value) => ToolResult::ok(&call.id, value).with_name(&call.name),
        Err((code, message)) => fail(call, code, message),
    }
}
type Failure = (&'static str, String);
fn store_error(error: impl ToString) -> Failure {
    ("memory_store", error.to_string())
}
fn execute_checked(
    state: &State,
    op: &str,
    call: &ToolCall,
    ctx: &ToolExecutionContext,
    key: &str,
    access: Option<&crate::profile_memory::ProfileMemoryAccess>,
) -> Result<Value, Failure> {
    let c = access.map(|a| &a.coordinator).unwrap_or(&state.coordinator);
    match op {
        "retrieve" | "search" | "compact" => {
            let mut query =
                MemoryRecallQuery::new(key, call.arguments["query"].as_str().unwrap_or(""));
            query.layers = vec![MemoryLayerKind::Working, MemoryLayerKind::Episodic];
            query.visible_scopes = vec![MemoryScope::Session {
                session_id: key.into(),
            }];
            if let Some(access) = access {
                query.visible_scopes =
                    access.scopes(ctx.principal.as_ref().expect("authenticated"));
                query.exact_scope_authorization = true;
                query.layers = vec![
                    MemoryLayerKind::Working,
                    MemoryLayerKind::Episodic,
                    MemoryLayerKind::Semantic,
                    MemoryLayerKind::Relational,
                ];
            }
            query.limit = call.arguments["limit"].as_u64().unwrap_or(8).clamp(1, 20) as usize;
            query.max_chars = state.max_context_chars;
            query.as_of_ms = Some(state.clock.now().timestamp_millis());
            let mut result = c.recall(&query).map_err(store_error)?;
            // Defense in depth: no old metadata, preference or typed/global backend candidate can cross this boundary.
            let mut owned = Vec::with_capacity(result.items.len());
            let mut remaining_chars = state.max_context_chars;
            for mut item in result.items {
                if c.backend()
                    .get_episode(&item.id)
                    .map_err(store_error)?
                    .is_some_and(|e| {
                        if access.is_some() {
                            c.episode_scope_metadata(&e)
                                .0
                                .is_visible_in(&query.visible_scopes)
                        } else {
                            e.session_id == key
                        }
                    })
                {
                    let governed = c
                        .governance()
                        .get_governed(&item.id)
                        .map_err(store_error)?
                        .ok_or_else(|| store_error("missing governance record"))?;
                    if governed.status != apeireth_memory::MemoryGovernanceStatus::Active {
                        continue;
                    }
                    item.content = governed
                        .episode
                        .content
                        .chars()
                        .take(remaining_chars)
                        .collect();
                    remaining_chars = remaining_chars.saturating_sub(item.content.chars().count());
                    if !item.content.is_empty() {
                        owned.push(item);
                    }
                } else if access.is_some()
                    && c.backend()
                        .get_episode(&item.id)
                        .map_err(store_error)?
                        .is_none()
                {
                    item.content = item.content.chars().take(remaining_chars).collect();
                    remaining_chars = remaining_chars.saturating_sub(item.content.chars().count());
                    if !item.content.is_empty() {
                        owned.push(item);
                    }
                }
            }
            result.items = owned;
            if op == "compact" {
                let episodes = result
                    .items
                    .iter()
                    .map(|item| Episode {
                        id: item.id.clone(),
                        timestamp: item.timestamp_ms / 1000,
                        role: "memory".into(),
                        content: item.content.clone(),
                        session_id: key.into(),
                    })
                    .collect::<Vec<_>>();
                let compact = apeireth_memory::ContinuityCompressor::new().compress(
                    key,
                    &episodes,
                    state.max_context_chars,
                );
                return Ok(
                    json!({"untrusted_data":true,"mode":"non_destructive_summary","continuity":compact}),
                );
            }
            loop {
                let mut payload = json!({"untrusted_data":true,"items":result.items});
                let estimate = apeireth_orchestration::token_meter::fold_text(&payload.to_string());
                payload["estimated_tokens"] = json!(estimate);
                if apeireth_orchestration::token_meter::fold_text(&payload.to_string())
                    <= state.max_recall_tokens
                {
                    return Ok(payload);
                }
                if result.items.pop().is_none() {
                    return Err((
                        "memory_budget",
                        "retrieval payload exceeds Host token estimate budget".into(),
                    ));
                }
            }
        }
        "write" => {
            let content = call.arguments["content"]
                .as_str()
                .ok_or_else(|| ("invalid_arguments", "content required".into()))?;
            let fallback = format!(
                "{}:{}:{}",
                ctx.agent,
                ctx.turn.map(|t| t.to_string()).unwrap_or_default(),
                ctx.call_id
            );
            let nonce = call.arguments["idempotency_key"]
                .as_str()
                .unwrap_or(&fallback);
            let id = format!("ep-{}", stable_id("memory.write.v1", &[key, nonce]));
            if let Some(existing) = c.backend().get_episode(&id).map_err(store_error)? {
                if existing.session_id != key || existing.content != content {
                    return Err((
                        "idempotency_conflict",
                        "key already used for another write".into(),
                    ));
                }
                let governed = c
                    .governance()
                    .get_governed(&id)
                    .map_err(store_error)?
                    .ok_or_else(|| ("memory_store", "missing governance record".into()))?;
                if governed.status != apeireth_memory::MemoryGovernanceStatus::Active {
                    return Err((
                        "memory_forgotten",
                        "idempotency key belongs to forgotten memory".into(),
                    ));
                }
                return Ok(json!({"id":id,"revision":governed.revision,"replayed":true}));
            }
            let episode = Episode {
                id: id.clone(),
                timestamp: state.clock.now().timestamp(),
                role: "memory".into(),
                content: content.into(),
                session_id: key.into(),
            };
            let provenance = MemoryProvenance {
                source: format!("module.memory:{}", ctx.call_id),
                source_session: ctx.session.map(|s| s.to_string()),
                source_trace: ctx.trace.map(|s| s.to_string()),
                source_request: ctx.turn.map(|s| s.to_string()),
            };
            c.writeback_scoped_episode(
                &episode,
                &MemoryScope::Session {
                    session_id: key.into(),
                },
                &provenance,
            )
            .map_err(store_error)?;
            // SQLite append-only INSERT OR IGNORE may race another process: verify instead of reporting a false successful write.
            let stored = c
                .backend()
                .get_episode(&id)
                .map_err(store_error)?
                .ok_or_else(|| store_error("write was not persisted"))?;
            if stored.session_id != key || stored.content != content {
                return Err((
                    "idempotency_conflict",
                    "concurrent write used the same key".into(),
                ));
            }
            Ok(json!({"id":id,"revision":0,"replayed":false}))
        }
        "update" | "delete" => {
            let id = call.arguments["id"]
                .as_str()
                .ok_or_else(|| ("invalid_arguments", "id required".into()))?;
            let raw = c.backend().get_episode(id).map_err(store_error)?;
            if !raw.is_some_and(|episode| episode.session_id == key) {
                return Err((
                    "not_found",
                    "memory unavailable in authorized namespace".into(),
                ));
            }
            let revision = call.arguments["expected_revision"]
                .as_i64()
                .ok_or_else(|| ("invalid_arguments", "expected revision required".into()))?;
            let updated = if op == "update" {
                c.update_episode_content(
                    id,
                    call.arguments["content"].as_str().unwrap_or(""),
                    Some("module.memory"),
                    revision,
                )
            } else {
                c.forget_episode(id, call.arguments["reason"].as_str(), revision)
            }
            .map_err(|error| ("memory_conflict", error.to_string()))?;
            Ok(json!({"id":id,"revision":updated.revision,"status":updated.status}))
        }
        _ => Err(("invalid_arguments", "unsupported memory operation".into())),
    }
}
