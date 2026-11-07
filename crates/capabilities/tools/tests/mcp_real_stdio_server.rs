//! End-to-end proof against a **real third-party stdio MCP server**.
//!
//! # Why this file exists
//!
//! Every other end-to-end proof for the external-tool bridge in this
//! repository drives an **in-process JSON-RPC double that this repository
//! wrote**. A double can only prove that the bridge agrees with itself: the
//! wire shapes, the error surface, the declaration sizes and the result
//! payloads of an **independent implementation** are never exercised. The
//! bridge can be perfectly consistent with a double and still be wrong in
//! front of a real server.
//!
//! This file points the same bridge at an **external server process**:
//!
//! * the server is a third-party implementation, started as a child process
//!   and spoken to over stdio — the repository never looks inside it;
//! * the server answers with **its own** frames: its own `initialize` result,
//!   its own `tools/list` payload, its own result envelopes and its own
//!   error objects.
//!
//! The four rings under test are the production ones:
//!
//! 1. **discovery** — `McpToolBridge::refresh()` (a real `tools/list` over a
//!    real child process);
//! 2. **registration** — the discovered tools are pushed into a registry
//!    exactly the way the startup wiring pushes them into the runtime
//!    capability table;
//! 3. **authorization** — the family grant plus the per-call human approval
//!    marking (the `_for_approval` shape: a grant that is **not** an
//!    exemption from the human);
//! 4. **dispatch** — the model-facing name `mcp:<server>:<tool>` really calls
//!    the external server and the server's own answer comes back.
//!
//! The runtime-level counterpart (registration into the real runtime
//! capability table, a real approval suspension and a real approval
//! resume) lives next to the runtime assembly.
//!
//! # How the external server is named
//!
//! The command line comes from the environment variable
//! [`SERVER_COMMAND_ENV`] (plus the optional JSON argument list in
//! [`SERVER_ARGS_ENV`]) — **never from a literal in this file**: the bridge
//! talks to whatever external server the operator points it at, and no
//! vendor name is baked into the repository. When the variable is not set
//! the tests **skip** (they do not fail): a machine without that server
//! simply has nothing to prove.
//!
//! ```text
//! APEIRETH_MCP_REAL_STDIO_SERVER_CMD=/path/to/the/server \
//!   cargo test -p apeireth-tools-canonical --test mcp_real_stdio_server -- --nocapture
//! ```
//!
//! # What these tests deliberately do not depend on
//!
//! * **No fixed tool list.** The external server's tool set is that
//!   server's business and may change at any time. The tests assert only
//!   that *some* tools were discovered, and they pick a call target by
//!   **preference** (first hit wins, absence is a skip).
//! * **No fixed tool count.** The number of tools is not a protocol
//!   guarantee; only a lower bound is asserted.
//! * **No side effects.** The call target is always a tool that only reads
//!   local state: the test never writes a file, never starts a process of its
//!   own and never changes anything on the machine it runs on.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Decision, PermissionPolicy};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::mcp::McpError;
use apeireth_tools_canonical::mcp_bridge::tool::redaction::{
    DECLARATION_MAX_DESCRIPTION_CHARS, DECLARATION_MAX_SCHEMA_DEPTH, TRUNCATION_MARKER,
};
use apeireth_tools_canonical::mcp_bridge::{
    McpBridgeOptions, McpPermissionMapping, McpServerConfig, McpServerSpec, McpToolBridge,
    McpToolEntry, McpToolRegistry, MCP_FAMILY_ROOT,
};

/// Environment variable naming the external stdio MCP server executable.
const SERVER_COMMAND_ENV: &str = "APEIRETH_MCP_REAL_STDIO_SERVER_CMD";

/// Environment variable naming the external server's arguments (JSON array).
const SERVER_ARGS_ENV: &str = "APEIRETH_MCP_REAL_STDIO_SERVER_ARGS";

/// The namespace the external server is registered under.
///
/// A neutral, product-free name: it is the server's **namespace** in the
/// model-facing names (`mcp:<server>:<tool>`), not a vendor claim.
const SERVER_NAME: &str = "realstdio";

/// Remote tool names preferred as the dispatch target, in order.
///
/// Every name here is a **read-only listing of local state**. The list is a
/// *preference*, not a requirement: a name the server does not advertise is
/// skipped, and the next candidate is tried.
const PREFERRED_CALL_TARGETS: &[&str] = &[
    "list_documents",
    "current_document",
    "list_segments",
    "list_bookmarks",
    "list_names",
];

/// Name prefixes of tools that only read state.
///
/// The fallback path of the target choice: if none of the preferred names is
/// advertised, any advertised tool whose name says it only reads state and
/// whose schema asks for no argument is used. The prefixes deliberately
/// exclude the write-shaped verbs (extract / import / install / push / pull /
/// set / start / stop / capture / recover): the test must never change
/// anything on the machine it runs on.
const READ_ONLY_NAME_PREFIXES: &[&str] = &["list_", "get_", "current_"];

/// How many call targets one dispatch proof may try.
///
/// A bound, not a requirement: the external server may answer some of these
/// with an error of its own, and the proof stops at the first successful
/// round trip.
const MAX_CALL_TARGETS: usize = 16;

/// The external server's command line, or `None` when the environment names
/// none.
fn real_server_command() -> Option<String> {
    std::env::var(SERVER_COMMAND_ENV)
        .ok()
        .map(|command| command.trim().to_string())
        .filter(|command| !command.is_empty())
}

/// The external server's arguments (a JSON array of strings).
fn real_server_args() -> Vec<String> {
    let Ok(raw) = std::env::var(SERVER_ARGS_ENV) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

/// The spec of the external stdio MCP server, or `None` when the environment
/// names none.
fn real_server_spec() -> Option<McpServerSpec> {
    let command = real_server_command()?;
    let mut spec = McpServerSpec::stdio(SERVER_NAME, command);
    for argument in real_server_args() {
        spec = spec.with_arg(argument);
    }
    Some(spec)
}

/// The bridge options in force: the production defaults, unchanged.
fn real_server_options() -> McpBridgeOptions {
    McpBridgeOptions::default()
}

/// A bridge over the external stdio MCP server, or `None` when the
/// environment names none.
fn real_server_bridge() -> Option<McpToolBridge> {
    let spec = real_server_spec()?;
    let config = McpServerConfig::new(vec![spec]);
    McpToolBridge::new(config, real_server_options()).ok()
}

/// The nesting depth of a converged schema (the top object counts as 1).
fn object_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => 1 + map.values().map(object_depth).max().unwrap_or(0),
        serde_json::Value::Array(items) => items.iter().map(object_depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// The property names one declared schema marks required.
fn required_properties(entry: &McpToolEntry) -> Vec<String> {
    entry
        .input_schema
        .as_ref()
        .and_then(|schema| schema.get("required"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The call targets: tools that only read state and ask for no argument.
///
/// The choice is a **preference**, never a requirement: when the external
/// server advertises none of the read-only, argument-free tools, the caller
/// skips the dispatch proof instead of failing on a tool set that is not the
/// repository's to control.
fn pick_call_targets(entries: &[McpToolEntry]) -> Vec<McpToolEntry> {
    let mut targets: Vec<McpToolEntry> = Vec::new();
    let mut push = |entry: &McpToolEntry| {
        if !targets
            .iter()
            .any(|chosen| chosen.model_name == entry.model_name)
        {
            targets.push(entry.clone());
        }
    };
    for name in PREFERRED_CALL_TARGETS {
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.remote_name == *name && required_properties(entry).is_empty())
        {
            push(entry);
        }
    }
    let mut fallback: Vec<&McpToolEntry> = entries
        .iter()
        .filter(|entry| {
            READ_ONLY_NAME_PREFIXES
                .iter()
                .any(|prefix| entry.remote_name.starts_with(prefix))
                && required_properties(entry).is_empty()
        })
        .collect();
    fallback.sort_by(|left, right| left.remote_name.cmp(&right.remote_name));
    for entry in fallback {
        push(entry);
    }
    targets.truncate(MAX_CALL_TARGETS);
    targets
}

/// A registry recording what the bridge pushes into it; refuses duplicate
/// capability ids exactly like the live module bag does.
#[derive(Default)]
struct RecordingRegistry {
    tools: Mutex<BTreeMap<String, Arc<dyn ToolCapability>>>,
}

impl McpToolRegistry for RecordingRegistry {
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        let mut tools = self.tools.lock().unwrap();
        if tools.contains_key(tool.id().as_str()) {
            return Err(format!("duplicate capability id {}", tool.id()));
        }
        tools.insert(tool.id().as_str().to_string(), tool);
        Ok(())
    }

    fn unregister_tool(&self, capability_id: &CapabilityId) {
        self.tools.lock().unwrap().remove(capability_id.as_str());
    }
}

/// **Ring 1 — discovery**: a real `tools/list` over a real child process.
///
/// The lower bound is the only number asserted: how many tools the external
/// server advertises is that server's business, not a protocol guarantee.
#[tokio::test]
async fn discovery_against_a_real_stdio_server_finds_tools() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    assert!(
        !report.registered.is_empty(),
        "the external server must expose at least one tool"
    );
    assert!(
        !report.live.is_empty(),
        "the dynamic catalog must hold the discovered tools"
    );
    for name in report.registered_names() {
        assert!(
            name.starts_with("mcp:realstdio:"),
            "the model-facing name must stay namespaced: {name}"
        );
    }
    eprintln!(
        "discovered {} tools from the external stdio server (lower bound only)",
        report.registered.len()
    );
}

/// **Ring 2 — registration**: the discovered tools reach a registry the way
/// the startup wiring reaches the runtime capability table.
#[tokio::test]
async fn real_server_tools_register_into_a_registry() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    let registry = RecordingRegistry::default();
    report
        .apply_to(&registry)
        .expect("the registry accepts the discovered tools");

    let registered = registry.tools.lock().unwrap();
    assert_eq!(
        registered.len(),
        report.registered.len(),
        "every discovered tool must reach the registry"
    );
    for capability_id in registered.keys() {
        assert!(
            capability_id.starts_with("tool.mcp.realstdio."),
            "the capability identity must stay namespaced: {capability_id}"
        );
    }
    eprintln!(
        "registered {} dynamic tools from the external stdio server",
        registered.len()
    );
}

/// **Bounded convergence on real declarations**: the external server's own
/// description and schema sizes are pushed through the model-facing bound.
///
/// The catalog keeps the server's **original** declaration; the model-facing
/// copy is the converged one. Both are compared here, so the bound is proved
/// against the server's real payload rather than against a synthetic one.
#[tokio::test]
async fn real_server_declarations_stay_inside_the_convergence_bounds() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    let entries = bridge.catalog().list();
    assert!(
        !entries.is_empty(),
        "the catalog must hold the discovered tools"
    );

    let mut truncated_descriptions = 0usize;
    let mut depth_capped_schemas = 0usize;
    for entry in &entries {
        let tool = report
            .registered
            .iter()
            .find(|tool| tool.declaration().name == entry.model_name)
            .unwrap_or_else(|| {
                panic!(
                    "the discovered tool {} must be registered",
                    entry.model_name
                )
            });
        let declaration = tool.declaration();

        // The description the model reads is bounded ...
        if let Some(description) = &declaration.description {
            assert!(
                description.chars().count() <= DECLARATION_MAX_DESCRIPTION_CHARS,
                "the description must stay inside its cap: {}",
                description.chars().count()
            );
        }
        // ... and a description that was cut says so.
        let raw_length = entry
            .description
            .as_deref()
            .map(|description| description.chars().count())
            .unwrap_or(0);
        if raw_length > DECLARATION_MAX_DESCRIPTION_CHARS {
            truncated_descriptions += 1;
            let description = declaration.description.as_deref().unwrap_or("");
            assert!(
                description.ends_with(TRUNCATION_MARKER),
                "a cut description must say so: {description}"
            );
        }

        // The schema the model reads is bounded in depth.
        let depth = object_depth(&serde_json::Value::Object(declaration.parameters.clone()));
        assert!(
            depth <= DECLARATION_MAX_SCHEMA_DEPTH + 3,
            "the converged schema must stay inside its depth bound: {depth}"
        );
        if let Some(schema) = entry.input_schema.as_ref() {
            if object_depth(schema) > DECLARATION_MAX_SCHEMA_DEPTH + 3 {
                depth_capped_schemas += 1;
            }
        }
    }
    eprintln!(
        "converged {} over-long descriptions and {} over-deep schemas",
        truncated_descriptions, depth_capped_schemas
    );
}

/// **Ring 3 — authorization**: the family grant plus the per-call human
/// approval marking.
///
/// The grant says "this capability is inside the authorization surface"; the
/// approval marking says "every call still waits for a human". A **bare**
/// grant would turn the external tool into an automatic pass — the exact
/// thing the MCP safety invariants forbid — so the `_for_approval` shape is
/// the one under test.
#[tokio::test]
async fn authorizing_a_real_tool_keeps_the_per_call_human_approval() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let _report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    let entry = bridge
        .catalog()
        .list()
        .first()
        .cloned()
        .expect("the external server must expose at least one tool");

    // The family grant (production shape): the family root covers every
    // capability the external server will ever discover.
    let mut policy = PermissionPolicy::new();
    policy.grant_family(MCP_FAMILY_ROOT);
    policy.require_approval_for_family(MCP_FAMILY_ROOT);

    let decision = policy.decision_for_capability(entry.capability_id.as_str());
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "the family grant must not exempt the external family from the human: {decision:?}"
    );
    assert!(
        !decision.is_allowed(),
        "an external tool must never be an automatic pass"
    );

    // The per-capability grant rides the same shape: granted **and** marked.
    let mut policy = PermissionPolicy::new();
    McpPermissionMapping::authorize_for_approval(&mut policy, &entry);
    let decision = policy.decision_for_capability(entry.capability_id.as_str());
    assert!(
        matches!(decision, Decision::RequireApproval { .. }),
        "a per-capability grant must keep the per-call human: {decision:?}"
    );

    // Without any grant the capability stays refused (fail-closed).
    let mut policy = PermissionPolicy::new();
    let decision = policy.decision_for_capability(entry.capability_id.as_str());
    assert!(
        matches!(decision, Decision::Deny { .. }),
        "an unauthorized external capability must be refused: {decision:?}"
    );
    eprintln!(
        "authorization of {} stays at the per-call human approval level",
        entry.capability_id.as_str()
    );
}

/// **Ring 4 — dispatch**: the model-facing name really reaches the external
/// server and the server's own answer comes back.
///
/// The bridge-level family release is what the production startup wiring
/// does (`wire_external_tools`); the per-capability grant keeps the human
/// at the per-call level. The call therefore walks the **whole** pipeline:
/// risk mapping, the deny guard, the around deadline and the output
/// normalization contract.
#[tokio::test]
async fn dispatching_to_a_real_server_returns_the_servers_own_answer() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    let entries = bridge.catalog().list();
    let targets = pick_call_targets(&entries);
    if targets.is_empty() {
        eprintln!("SKIP: the external server advertises no read-only, argument-free tool to call");
        return;
    }

    // The bridge-level family release (production shape).
    bridge.family_grants().grant_mcp_family();

    let mut structured_answers = 0usize;
    let mut ok_calls = 0usize;
    let mut error_calls = 0usize;
    for (index, entry) in targets.iter().enumerate() {
        // The per-capability grant with the per-call human marking.
        McpPermissionMapping::authorize_for_approval(&mut bridge.policy().lock().unwrap(), entry);
        let Some(tool) = report
            .registered
            .iter()
            .find(|tool| tool.declaration().name == entry.model_name)
        else {
            continue;
        };
        let call = ToolCall {
            id: format!("call_real_{index}"),
            name: entry.model_name.clone(),
            arguments: serde_json::json!({}),
        };
        let result = tool.invoke(&call).await;

        match &result.outcome {
            ToolOutcome::Ok { value } => {
                ok_calls += 1;
                // **结构化通道在真机上的落地**（2026-11-07 复验）：真服务器每个工具都返回
                // `structuredContent` ⇒ 修好之前它会被 serde 的「忽略未知字段」**静默丢弃**
                // ⇒ 模型面**看不到**这个键；修好之后它作为**一等通道**到达模型面。
                // ⇒ 断言写成**条件式**：服务器给了 ⇒ 模型面必须能拿到、且必须是**结构化值**
                // （对象/数组，不是裸标量）；服务器没给 ⇒ 不作要求（**不假红**）。
                if let Some(structured) = value.get("structured") {
                    structured_answers += 1;
                    assert!(
                        matches!(
                            structured,
                            serde_json::Value::Object(_) | serde_json::Value::Array(_)
                        ),
                        "the structured channel must arrive as a structured value, got {:?}",
                        structured
                    );
                }
                let text = value["text"].as_str().unwrap_or_default();
                assert!(
                    !text.is_empty(),
                    "the external server answered with an empty text"
                );
                assert!(
                    value["content"].is_array(),
                    "the result contract carries the content blocks"
                );
                assert_eq!(
                    value["is_error"],
                    serde_json::json!(false),
                    "the server's answer was not an error"
                );
                eprintln!(
                    "dispatched {}: the external server answered with {} chars",
                    entry.model_name,
                    text.chars().count()
                );
            }
            ToolOutcome::Error { message, retryable } => {
                // The server answered; the answer was an error of its own.
                // That is a **real** answer, and it must reach the model
                // intact — not as a transport failure, not as an empty
                // message.
                error_calls += 1;
                assert!(
                    !message.contains("mcp transport failure"),
                    "the round trip must have completed: {message}"
                );
                assert!(
                    !*retryable,
                    "a server-side answer is not a retryable transport failure: {message}"
                );
                assert_ne!(
                    message, "external tool reported an error without detail",
                    "the server's own error text must reach the model"
                );
                assert!(
                    !message.is_empty(),
                    "the server's error text must reach the model"
                );
            }
        }
        // One successful round trip is the proof; the rest is noise.
        if ok_calls > 0 {
            break;
        }
    }
    assert!(
        ok_calls + error_calls > 0,
        "at least one call must have reached the external server"
    );
    eprintln!(
        "dispatched against the external stdio server: {ok_calls} successful answer(s), \
         {structured_answers} of them carried the structured channel, \
         {error_calls} server-side error answer(s)"
    );
}

/// **Error surface**: a protocol error from the real server must be
/// classified as a protocol error — not as a transport failure and not as a
/// silent success.
///
/// The tool name below cannot exist on any server, so the answer is the
/// server's own "unknown tool" rejection.
#[tokio::test]
async fn a_protocol_error_from_a_real_server_is_classified_as_a_protocol_error() {
    let Some(bridge) = real_server_bridge() else {
        eprintln!("SKIP: {SERVER_COMMAND_ENV} names no external stdio MCP server");
        return;
    };

    let _report = bridge
        .refresh()
        .await
        .expect("the real server must answer tools/list");
    let connection = bridge
        .connections()
        .first()
        .expect("the bridge owns one connection per enabled server");

    let error = connection
        .call_tool("__apeireth_absent_tool__", serde_json::json!({}))
        .await
        .expect_err("an unknown tool must not succeed");
    match error {
        McpError::JsonRpc { code, message } => {
            assert!(
                !message.is_empty(),
                "the server's own error text must survive the classification"
            );
            eprintln!(
                "unknown tool answered with a JSON-RPC error: code={code}, message_len={}",
                message.chars().count()
            );
        }
        other => panic!("an unknown tool must be classified as a protocol error, got: {other:?}"),
    }
}
