//! Dynamic tools: one external tool, one model-facing declaration.
//!
//! Every tool a server exposes becomes an [`McpDynamicTool`] under the
//! `mcp:<server>:<tool>` namespace. Execution is the remote `tools/call`; the
//! result structure is normalized here into the output contract's structured
//! value ({text, content, is_error}) before the pipeline's normalization
//! stage freezes it into a `ToolOutcome`. The tool itself is pipeline-neutral:
//! the bridge wraps it in `PipelinedCapability`, so risk mapping, guards, the
//! deadline, and the correction channel all wrap this call.
//!
//! # Bounded convergence at this boundary
//!
//! Everything the server authored reaches the model through this module: the
//! declaration (description + input schema) and the result (text + content
//! blocks + the server's **structured** payload). Both are bounded and
//! converged by [`redaction`] before they
//! leave: a length cap on every string, a bounded shape for the declared
//! schema, and a clean of the invisible characters that carry injections.
//! The convergence is *bounded*, never total — a normal schema keeps its
//! properties, and a normal description is kept verbatim.
//!
//! # Frozen invocations (the approval card's payload)
//!
//! The tool freezes its execution inputs ([`McpDynamicTool::freeze_invocation`])
//! so the runtime's approval card shows **what is about to run**, and resumes
//! the approved call from that frozen payload
//! ([`McpDynamicTool::invoke_frozen`]) instead of from the ambient
//! environment. The payload carries the server, the remote tool name, and the
//! arguments; the display carries the same call's *shape* only — the
//! arguments' values can carry credentials, so the card shows their key
//! names, never their values.
//!
//! # Content-level credential sweep on the result face
//!
//! Bounding the result is not the same as recognizing a credential in it: a
//! remote server can hand back a key, a password assignment, a private key
//! block, or a bearer token, and a length cap keeps that answer small, not
//! harmless. So the result face **sweeps** what the server authored before it
//! becomes the model-facing result: the placeholder family (a true value
//! becomes an irreversible alias, registered in the ledger) followed by the
//! existing one-way credential tripwire (a credential shape becomes a
//! `[REDACTED_*]` label). The sweep is the repository's existing egress
//! mechanism ([`crate::guardrail::ToolGuardrail::sweep_output_for_egress`])
//! — it rewrites in place, it does not refuse the call and it does not
//! truncate the answer.
//!
//! The sweep covers **text** surfaces: the joined result text, every text
//! block, a resource block's URI and body, and every string leaf of the
//! server's **structured** payload. It does **not** cover inline
//! image data: that is a base64 payload, rewriting it would corrupt it, and
//! the shape matchers misfire on base64.
//!
//! What the sweep did is observable: the rewritten markers are in the output
//! the model reads, and a result that was swept carries an `egress_sweep`
//! sidecar (hit counts and credential kinds) next to its content.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::mcp::{content_block_to_wire, ContentBlock, ToolCallResult};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::guardrail::{EgressSweepOutcome, LeakedCredentialKind};
use crate::mcp::McpError;

use super::catalog::McpToolEntry;
use super::connection::McpServerConnection;
use super::SessionRedactionDomain;

// The bounded-convergence helpers live in their own file. The bridge's
// module root is frozen for this batch, so the file is wired in here — the
// model-facing boundary that uses it.
#[path = "redaction.rs"]
pub mod redaction;

/// The frozen payload version of the external-tool invocation.
///
/// Bumped whenever the payload shape changes; a resumed call carrying another
/// version is refused instead of executed with a half-understood payload.
///
/// **Version 2** — the payload carries the **descriptor receipt** next to the
/// call inputs: the frozen server, the frozen remote tool name, the frozen
/// arguments, **and** the descriptor the human approved (server, tool, schema,
/// read-only declaration). A resumed call whose descriptor no longer matches
/// the live one is refused — the human approved *that* descriptor, not a
/// different one wearing the same name.
const MCP_FROZEN_VERSION: u32 = 2;

/// The exact, versioned execution inputs frozen at approval time.
///
/// This is the bridge's own payload schema. The runtime treats it as an
/// opaque `serde_json::Value`; the bridge owns deserialization and must
/// execute these fields — and only these fields — when resuming an approved
/// operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpFrozenInvocation {
    version: u32,
    /// The server the approved call runs against.
    server: String,
    /// The server-side tool name.
    remote_name: String,
    /// The arguments the approved call carries.
    arguments: serde_json::Value,
    /// The descriptor receipt the human approved (server, tool, schema,
    /// read-only declaration).
    ///
    /// The resumed call compares it against the live receipt: a descriptor
    /// that changed since the approval is a **different** call wearing the
    /// same name, and it needs a new approval.
    descriptor: serde_json::Value,
}

/// The declared output shape every MCP result must normalize into.
pub fn mcp_output_schema() -> OutputSchema {
    OutputSchema::new()
        .require("text", SchemaKind::String)
        .require("content", SchemaKind::Array)
}

/// What one result-face sweep found (the observability sidecar).
///
/// The counts are per call and carry no true value: the placeholder family
/// reports how many spans it aliased, the credential tripwire reports which
/// credential shapes it tagged. A result whose sweep found nothing carries
/// no sidecar at all — an untouched answer stays exactly the answer it was.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSweepReport {
    /// How many spans the placeholder family aliased.
    pub redaction_hits: usize,
    /// Which credential shapes the one-way tripwire tagged, in hit order.
    pub tripwire_kinds: Vec<LeakedCredentialKind>,
    /// Whether the answer came from a **class B** tool (a tool the operator
    /// declared as "its result face legitimately is credential material").
    ///
    /// A class B answer is swept by the **reversible** channel: the model sees
    /// placeholders and the true value only leaves the domain through the
    /// explicit local consumption face. The flag is in the sidecar so the
    /// difference is **observable**, not a claim in a comment.
    pub class_b: bool,
}

impl McpSweepReport {
    /// Whether this sweep found anything at all.
    pub fn is_hit(&self) -> bool {
        self.redaction_hits > 0 || !self.tripwire_kinds.is_empty()
    }

    /// Fold one sweep outcome into this report.
    fn absorb(&mut self, outcome: &EgressSweepOutcome) {
        self.redaction_hits += outcome.report.redaction.hits.len();
        self.tripwire_kinds
            .extend(outcome.report.tripwire.leaked_kinds.iter().copied());
    }
}

/// Normalize one MCP result structure into the contract's structured value:
/// joined text, the wire content blocks, the server's error flag, and the
/// server's **structured** payload when it sent one.
///
/// The server's answer is bounded on the way in: the joined text, the block
/// count and every block payload carry a length cap (see [`redaction`]), so
/// one remote answer can never flood the model's context.
pub fn normalized_mcp_value(result: &ToolCallResult) -> serde_json::Value {
    normalized_mcp_value_with_sweep(result, None)
}

/// The normalized structured value, plus what the result-face sweep found.
///
/// A sweep that found nothing leaves the value exactly as
/// [`normalized_mcp_value`] builds it — the sidecar appears only when the
/// sweep actually rewrote something, so an untouched answer keeps its shape.
pub fn normalized_mcp_value_with_sweep(
    result: &ToolCallResult,
    sweep: Option<&McpSweepReport>,
) -> serde_json::Value {
    let text = redaction::truncate_text(&result.extract_text(), redaction::RESULT_MAX_TEXT_CHARS);
    let content: Vec<serde_json::Value> = result
        .content
        .iter()
        .take(redaction::RESULT_MAX_BLOCKS)
        .map(|block| content_block_to_wire(&redaction::bound_content_block(block)))
        .collect();
    let mut value = serde_json::json!({
        "text": text,
        "content": content,
        "is_error": result.is_error,
    });
    // The structured channel: the server's own structured payload reaches the
    // model — bounded like every other server-authored surface. A server
    // that sent none adds no key: such an answer keeps its old shape.
    if let Some(structured) = &result.structured {
        value["structured"] = redaction::bound_structured_value(structured);
    }
    if let Some(report) = sweep {
        if report.is_hit() {
            let mut sidecar = serde_json::json!({
                "redaction_hits": report.redaction_hits,
                "tripwire_kinds": report.tripwire_kinds,
            });
            // 类 B 标记: 只在类 B 工具的结果面出现（**默认面逐字节不变**）.
            if report.class_b {
                sidecar["class_b"] = serde_json::json!(true);
            }
            value["egress_sweep"] = sidecar;
        }
    }
    value
}

/// Freeze one MCP result into a model-facing tool result.
///
/// A server-side error (`isError`) becomes a permanent tool error carrying
/// the server's own text; a success becomes the normalized structure the
/// output contract validates. The server's text is bounded in both cases.
pub fn tool_result_from_mcp(call_id: &str, tool_name: &str, result: ToolCallResult) -> ToolResult {
    tool_result_from_mcp_swept(call_id, tool_name, result, None)
}

/// Freeze one MCP result into a model-facing tool result, carrying what the
/// result-face sweep found.
///
/// The error face has no structured sidecar to carry the report (the error
/// shape is fixed by the protocol), so an errored call shows the sweep only
/// through the rewritten markers in its message.
pub fn tool_result_from_mcp_swept(
    call_id: &str,
    tool_name: &str,
    result: ToolCallResult,
    sweep: Option<&McpSweepReport>,
) -> ToolResult {
    let text = redaction::truncate_text(&result.extract_text(), redaction::RESULT_MAX_TEXT_CHARS);
    if result.is_error {
        let message = if text.is_empty() {
            "external tool reported an error without detail".to_string()
        } else {
            text
        };
        ToolResult::permanent_error(call_id, message).with_name(tool_name)
    } else {
        ToolResult::ok(call_id, normalized_mcp_value_with_sweep(&result, sweep))
            .with_name(tool_name)
    }
}

/// Sweep one server-authored structured payload: every string leaf goes
/// through the sweep, the shape is kept.
///
/// The structured channel is server-authored content like the text blocks, so
/// it is swept the same way: a credential hiding in a string leaf of the
/// payload must not reach the model either.
fn sweep_structured_value(
    value: &serde_json::Value,
    domain: &mut SessionRedactionDomain,
    report: &mut McpSweepReport,
    class_b: bool,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(text) => {
            let outcome = sweep_text_surface(domain, text, class_b);
            report.absorb(&outcome);
            serde_json::Value::String(outcome.text().to_string())
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|item| sweep_structured_value(item, domain, report, class_b))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, item)| {
                    (
                        key.clone(),
                        sweep_structured_value(item, domain, report, class_b),
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// 扫一个文本面: **类 B** 工具走可逆通道, 其余走既有占位族 + 单向绊线.
///
/// 两条路的差别只有一处 —— **真值登记在哪本簿**:
/// * 占位族那一本 ⇒ 受控回程面（**自动**）能还原 ⇒ 真值可能进 transcript;
/// * 类 B 那一本 ⇒ 只有**显式**的本地消费面能还原 ⇒ 真值不进 transcript.
fn sweep_text_surface(
    domain: &mut SessionRedactionDomain,
    text: &str,
    class_b: bool,
) -> EgressSweepOutcome {
    if class_b {
        domain.sweep_class_b_result_face(text)
    } else {
        domain.sweep_result_face(text)
    }
}

/// Sweep one content block: the text surfaces go through the sweep, the
/// inline image payload does not.
///
/// An inline image is base64 data, not prose: rewriting it would corrupt it,
/// and the shape matchers misfire on base64 (a payload can carry the letters
/// of a credential prefix by accident). This is a stated boundary of the
/// result face, not a silent omission.
fn sweep_block(
    domain: &mut SessionRedactionDomain,
    block: ContentBlock,
    report: &mut McpSweepReport,
    class_b: bool,
) -> ContentBlock {
    match block {
        ContentBlock::Text { text, mime_type } => {
            let outcome = sweep_text_surface(domain, &text, class_b);
            report.absorb(&outcome);
            ContentBlock::Text {
                text: outcome.text().to_string(),
                mime_type,
            }
        }
        ContentBlock::Image { data, mime_type } => ContentBlock::Image { data, mime_type },
        ContentBlock::Resource {
            uri,
            text,
            mime_type,
        } => {
            let outcome = sweep_text_surface(domain, &uri, class_b);
            report.absorb(&outcome);
            let uri = outcome.text().to_string();
            let text = text.map(|body| {
                let outcome = sweep_text_surface(domain, &body, class_b);
                report.absorb(&outcome);
                outcome.text().to_string()
            });
            ContentBlock::Resource {
                uri,
                text,
                mime_type,
            }
        }
    }
}

/// One external tool, callable like any builtin tool.
pub struct McpDynamicTool {
    entry: McpToolEntry,
    connection: Arc<McpServerConnection>,
    /// The session redaction domain the result face is swept in.
    ///
    /// One domain is one redaction scope: inside it the same value always
    /// gets the same alias, across domains it gets a different one. It is
    /// shared behind a mutex because one tool serves concurrent calls and the
    /// sweep needs the registry and the ledger of a single domain.
    ///
    /// **The domain is a session-level thing, not a tool-level one**: the
    /// holder is injected ([`McpDynamicTool::new_with_domain`]) so the tools
    /// of one session share it — and so the provider-facing egress gate can
    /// hold the *same* domain and recognise what this tool aliased.
    domain: Arc<Mutex<SessionRedactionDomain>>,
    /// Whether this tool is **declared class B** — 「本工具的结果面合法地就是
    /// 凭据」.
    ///
    /// 声明为类 B ⇒ 结果面走**可逆占位族**（真值登记进**类 B 登记簿**）⇒
    /// 模型侧只见占位符, 真值只由**显式的本地消费面**交还.
    /// 未声明 ⇒ 走既有占位族 + 单向绊线（**行为逐字节不变**）.
    class_b: bool,
    /// The module generation this tool was built in.
    ///
    /// A tool whose module generation expired is not callable any more: the
    /// generation is the module's aliveness token, and a call on a dead
    /// generation would run against a link nobody owns any more.
    generation: u64,
    /// The task scope that bounds this tool's calls to its generation.
    generation_scope: apeireth_plugin::task_scope::ModuleTaskScope,
}

impl McpDynamicTool {
    /// A dynamic tool for `entry`, executing through `connection`.
    ///
    /// The tool gets its own sweep domain, so every external tool is swept on
    /// the result face without the caller wiring anything.
    ///
    /// **This is the un-wired default**: one domain per tool. A caller that
    /// wants one session one domain injects a shared domain instead
    /// ([`McpDynamicTool::new_with_domain`]).
    pub fn new(entry: McpToolEntry, connection: Arc<McpServerConnection>) -> Self {
        Self::new_with_domain(
            entry,
            connection,
            Arc::new(Mutex::new(SessionRedactionDomain::new())),
        )
    }

    /// A dynamic tool whose result face is swept in a caller-held domain.
    ///
    /// Sharing one domain across the tools of one session keeps the alias of
    /// one value stable across those tools (the model can correlate them),
    /// and lets the provider-facing egress gate hold the *same* domain — so
    /// the two faces recognise each other's aliases. Separate domains keep
    /// the aliases unrelated.
    pub fn new_with_domain(
        entry: McpToolEntry,
        connection: Arc<McpServerConnection>,
        domain: Arc<Mutex<SessionRedactionDomain>>,
    ) -> Self {
        Self::new_with_domain_and_class_b(entry, connection, domain, false)
    }

    /// A **class B** dynamic tool: its result face legitimately *is* credential
    /// material.
    ///
    /// The class B result face is swept by the **reversible** channel: the
    /// model sees placeholders, and the true value only leaves the domain
    /// through the **explicit** local consumption face
    /// ([`SessionRedactionDomain::restore_for_local_consumer`]).
    ///
    /// The domain is the caller's: the class B placeholders must be
    /// recognisable by the same local consumer that owns the domain.
    pub fn new_class_b(
        entry: McpToolEntry,
        connection: Arc<McpServerConnection>,
        domain: Arc<Mutex<SessionRedactionDomain>>,
    ) -> Self {
        Self::new_with_domain_and_class_b(entry, connection, domain, true)
    }

    /// The shared constructor: a tool, its domain, and whether the tool is
    /// declared class B.
    pub fn new_with_domain_and_class_b(
        entry: McpToolEntry,
        connection: Arc<McpServerConnection>,
        domain: Arc<Mutex<SessionRedactionDomain>>,
        class_b: bool,
    ) -> Self {
        let generation = connection.generation();
        let generation_scope = connection.generation_scope();
        Self {
            entry,
            connection,
            domain,
            class_b,
            generation,
            generation_scope,
        }
    }

    /// Whether this tool is **declared class B**.
    pub fn is_class_b(&self) -> bool {
        self.class_b
    }

    /// The receipt of the descriptor this tool exposes.
    ///
    /// The receipt is what a human approved: the server, the remote tool
    /// name, the declared input schema and the read-only declaration. It is
    /// frozen into the approval payload and compared again when the approved
    /// call resumes — a descriptor that changed in the meantime needs a new
    /// approval.
    fn descriptor_receipt(&self) -> serde_json::Value {
        serde_json::json!({
            "server": self.entry.server,
            "tool": self.entry.remote_name,
            "schema": self.entry.input_schema,
            "read_only": self.entry.declared_read_only,
        })
    }

    /// The catalog entry backing this tool.
    pub fn entry(&self) -> &McpToolEntry {
        &self.entry
    }

    /// The connection this tool executes through.
    pub fn connection(&self) -> &Arc<McpServerConnection> {
        &self.connection
    }

    /// The session redaction domain this tool sweeps its results in.
    ///
    /// The domain is also the observability face of the sweep: its ledger
    /// records every span the sweep rewrote (category and fingerprint, never
    /// a true value).
    pub fn sweep_domain(&self) -> &Arc<Mutex<SessionRedactionDomain>> {
        &self.domain
    }

    /// The egress sweep domain this tool sweeps its results in.
    ///
    /// The historical name of [`McpDynamicTool::sweep_domain`]; it returns the
    /// same handle.
    pub fn sweeper(&self) -> &Arc<Mutex<SessionRedactionDomain>> {
        &self.domain
    }

    /// The execution inputs of one call, frozen.
    ///
    /// The freeze takes the **current** entry (server, remote tool name) and
    /// the call's arguments — this is the shape a not-yet-approved call
    /// executes in. The resumed call uses the frozen payload instead.
    ///
    /// The payload carries the **descriptor receipt** too: the human approved
    /// that descriptor, and the resumed call compares it against the live one
    /// before it runs.
    fn build_frozen(&self, call: &ToolCall) -> McpFrozenInvocation {
        McpFrozenInvocation {
            version: MCP_FROZEN_VERSION,
            server: self.entry.server.clone(),
            remote_name: self.entry.remote_name.clone(),
            arguments: call.arguments.clone(),
            descriptor: self.descriptor_receipt(),
        }
    }

    /// The human-facing approval card face of one frozen call.
    ///
    /// The card shows the **shape** of the call, never its values: a remote
    /// tool's arguments can carry credentials, and the remote
    /// `input_schema` can describe sensitive fields — so neither the values
    /// nor the schema belong here. The payload carries the real arguments.
    fn display_invocation(frozen: &McpFrozenInvocation) -> serde_json::Value {
        let (argument_keys, argument_shape) = match frozen.arguments.as_object() {
            Some(arguments) => (arguments.keys().cloned().collect::<Vec<String>>(), "object"),
            None => (Vec::new(), "non_object"),
        };
        serde_json::json!({
            "version": frozen.version,
            "server": frozen.server,
            "tool": frozen.remote_name,
            "argument_shape": argument_shape,
            "argument_keys": argument_keys,
            "argument_count": argument_keys.len(),
        })
    }

    /// Sweep one server-authored result: the text surfaces go through the
    /// egress sweep, the inline image payload does not.
    ///
    /// The sweep runs **after** the answer is bounded (the same caps the
    /// declaration face uses), so the cost of scanning stays bounded by the
    /// answer's own limit; the normalization stage bounds the swept text
    /// once more, because an alias is longer than the value it replaced.
    ///
    /// This is the right place for the sweep: the result is complete, and it
    /// has not been handed to the caller yet.
    fn sweep_remote_result(&self, result: &ToolCallResult) -> (ToolCallResult, McpSweepReport) {
        let mut report = McpSweepReport::default();
        report.class_b = self.class_b;
        let bounded: Vec<ContentBlock> = result
            .content
            .iter()
            .take(redaction::RESULT_MAX_BLOCKS)
            .map(|block| redaction::bound_content_block(block))
            .collect();
        let mut domain = self
            .domain
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let swept: Vec<ContentBlock> = bounded
            .into_iter()
            .map(|block| sweep_block(&mut domain, block, &mut report, self.class_b))
            .collect();
        (
            ToolCallResult {
                content: swept,
                is_error: result.is_error,
                // The structured channel is server-authored too: it is swept
                // here and bounded on the model-facing side.
                structured: result.structured.as_ref().map(|structured| {
                    sweep_structured_value(structured, &mut domain, &mut report, self.class_b)
                }),
            },
            report,
        )
    }

    /// Sweep one server-authored text (the error face).
    ///
    /// A server's error message is server-authored content too, and it
    /// reaches the model context just like a result does — so it is swept the
    /// same way.
    fn sweep_text(&self, text: &str) -> String {
        let mut domain = self
            .domain
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sweep_text_surface(&mut domain, text, self.class_b)
            .text()
            .to_string()
    }

    /// Execute one frozen call: the frozen server, the frozen remote tool
    /// name, the frozen arguments.
    ///
    /// The call runs **inside the module's generation scope** and through
    /// the descriptor-checked path: the scope stops the call when the module
    /// generation died while it waited, and the checked path refuses a
    /// descriptor that changed since the discovery.
    async fn execute_frozen(&self, call: &ToolCall, frozen: &McpFrozenInvocation) -> ToolResult {
        let name = self.entry.model_name.as_str();
        match self
            .generation_scope
            .run(self.connection.call_tool_checked(
                &frozen.remote_name,
                frozen.arguments.clone(),
                &self.entry.input_schema,
                self.entry.declared_read_only,
            ))
            .await
            .unwrap_or_else(|_| Err(McpError::Transport("module generation stopped".into())))
        {
            Ok(result) => {
                let (swept, report) = self.sweep_remote_result(&result);
                tool_result_from_mcp_swept(&call.id, name, swept, Some(&report))
            }
            Err(McpError::Transport(reason)) => {
                let reason = self.sweep_text(&reason);
                ToolResult::retryable_error(
                    &call.id,
                    format!("mcp transport failure for {name}: {reason}"),
                )
                .with_name(name)
            }
            Err(McpError::JsonRpc { code, message }) => {
                let message = self.sweep_text(&message);
                ToolResult::permanent_error(
                    &call.id,
                    format!("mcp error for {name} (code {code}): {message}"),
                )
                .with_name(name)
            }
            Err(other) => {
                let other = self.sweep_text(&other.to_string());
                ToolResult::permanent_error(
                    &call.id,
                    format!("mcp call failed for {name}: {other}"),
                )
                .with_name(name)
            }
        }
    }
}

#[async_trait]
impl ToolCapability for McpDynamicTool {
    fn id(&self) -> &CapabilityId {
        &self.entry.capability_id
    }

    /// The model-facing declaration of one external tool.
    ///
    /// The server's description and input schema are converged before they
    /// reach the model (see [`redaction`]): the description is cleaned and
    /// bounded, the schema keeps its properties but loses its unbounded
    /// shape, and a schema that is not an object schema degrades to the
    /// empty object schema.
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.entry.model_name.clone(),
            description: redaction::converge_description(self.entry.description.as_deref()),
            parameters: redaction::converge_input_schema(self.entry.input_schema.as_ref()),
            strict: false,
        }
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call);
        let payload = serde_json::to_value(&frozen).map_err(|error| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen external invocation: {error}"),
            )
            .with_name(&self.entry.model_name)
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };

        let mcp_frozen: McpFrozenInvocation = match serde_json::from_value(frozen.payload.clone()) {
            Ok(mcp_frozen) => mcp_frozen,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen external invocation is invalid: {error}"),
                )
                .with_name(&self.entry.model_name)
            }
        };

        if mcp_frozen.version != MCP_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen external invocation version {} (expected {})",
                    mcp_frozen.version, MCP_FROZEN_VERSION
                ),
            )
            .with_name(&self.entry.model_name);
        }

        // The frozen server must be the one this tool executes through: a
        // payload naming another server is not this tool's call (fail-closed).
        if mcp_frozen.server != self.entry.server {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "frozen external invocation names server {:?}, but this tool executes through {:?}",
                    mcp_frozen.server, self.entry.server
                ),
            )
            .with_name(&self.entry.model_name);
        }

        // The descriptor the human approved must still be the live one: the
        // approval covers **that** descriptor, not a different one wearing
        // the same name. A changed descriptor needs a new approval.
        if mcp_frozen.descriptor != self.descriptor_receipt() {
            return ToolResult::permanent_error(
                &call.id,
                "external tool descriptor changed since approval; approve a new call",
            )
            .with_name(&self.entry.model_name);
        }

        self.execute_frozen(call, &mcp_frozen).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = self.entry.model_name.as_str();
        // The module generation is the tool's aliveness token: a tool whose
        // generation died must not run any more.
        if !self.connection.accepts_generation(self.generation) {
            return ToolResult::permanent_error(
                &call.id,
                "external tool belongs to an expired module generation",
            )
            .with_name(name);
        }
        let frozen = self.build_frozen(call);
        self.execute_frozen(call, &frozen).await
    }
}
