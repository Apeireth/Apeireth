//! Turn-level prompt-injection warning — consumes the **output-side** scan marker.
//!
//! # The gap this closes
//!
//! `tool.fetch` scans the remote payload it brought back and writes a marker
//! into its own result ([`FETCH_PROMPT_INJECTION_FIELD`]). The governance
//! input-side hook (`PromptInjectionHook`) only ever sees capability
//! *arguments*, so **nothing** used to read that marker: the fact that a
//! remote page carried an injection attempt stopped at the tool boundary.
//!
//! This module is the reader. It runs at
//! [`HookPoint::AfterToolResult`] — the point where a tool result has just
//! become a transcript message and the **next** completion request is about to
//! be assembled — and turns the marker into a *visible* turn-level warning.
//!
//! # Boundaries
//!
//! - **Pure advice, never a block.** The directive is always
//!   [`ModuleOutcome::continue_`]; execution is never intercepted. Whether a
//!   detected injection should *block* is a product decision, and this crate
//!   does not make it.
//! - **The wording is fixed.** The model-facing text is
//!   [`FETCH_PROMPT_INJECTION_WARNING`] **verbatim** — the very same constant
//!   the fetch tool writes into its own result. A hostile body must not be
//!   able to talk its way past the notice that flags it, so the text is never
//!   composed from the payload. The payload-derived detail (which heuristic
//!   kinds matched) is kept in a *separate* structured field and never mixed
//!   into the warning sentence.
//! - **No second wording.** The field name and the warning text are re-used
//!   from `apeireth_tools_canonical::fetch`; this file defines no copy of its
//!   own.
//! - **No fabricated facts.** A result without the marker, with
//!   `detected: false`, or with a malformed marker produces *no* warning and
//!   no counter increment.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use parking_lot::Mutex;

use super::module::{
    BehaviorModule, HookPoint, Module, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use super::production::{BuildInstrumentation, ProductionModules};
use super::ModuleFamily;

/// The result field that carries the output-side prompt-injection verdict.
///
/// Re-used from the fetch tool so there is exactly one spelling of the marker
/// in the workspace.
pub use apeireth_tools_canonical::fetch::FETCH_PROMPT_INJECTION_FIELD;

/// The fixed, model-facing warning text.
///
/// Re-used from the fetch tool: the turn-level notice and the notice already
/// embedded in the tool result must read identically, otherwise a reader
/// comparing the two would have to guess which one is authoritative.
pub use apeireth_tools_canonical::fetch::FETCH_PROMPT_INJECTION_WARNING;

/// The guard module's stable id (the composition root de-duplicates by id).
pub const PROMPT_INJECTION_GUARD_MODULE_ID: &str = "guard.prompt_injection";

/// Maximum number of warning records kept for inspection.
///
/// The counter is the durable truth; the record list is a bounded window for
/// operators, so a long-running process cannot grow without limit.
pub const MAX_PROMPT_INJECTION_RECORDS: usize = 256;

/// One detected prompt injection, as reported at the turn layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptInjectionWarning {
    /// The tool call this warning answers.
    pub tool_call_id: String,
    /// The tool that produced the payload, when the provider echoed a name.
    pub tool: Option<String>,
    /// The fixed warning text (never composed from the payload).
    pub warning: String,
    /// Which heuristic kinds matched — a closed vocabulary, kept out of the
    /// warning sentence on purpose.
    pub signal_kinds: Vec<String>,
}

/// Read the output-side prompt-injection marker out of one tool result.
///
/// Returns `None` for every case that is not a confirmed detection:
///
/// - the result did not succeed (an error carries no scanned payload);
/// - the result carries no marker field at all;
/// - the marker says `detected: false`;
/// - the marker is malformed (wrong type, or `detected` is not a boolean).
///
/// The tool name falls back to the call name when the provider did not echo
/// one, matching the attribution order already used by the signal bridge.
pub fn detect_prompt_injection(
    tool_call: Option<&ToolCall>,
    tool_result: &ToolResult,
) -> Option<PromptInjectionWarning> {
    let ToolOutcome::Ok { value } = &tool_result.outcome else {
        return None;
    };
    let marker = value.get(FETCH_PROMPT_INJECTION_FIELD)?;
    if marker.get("detected").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }

    let tool = tool_result
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            tool_call
                .map(|call| call.name.as_str())
                .filter(|name| !name.trim().is_empty())
        })
        .map(str::to_owned);

    let signal_kinds = marker
        .get("signals")
        .and_then(serde_json::Value::as_array)
        .map(|signals| {
            signals
                .iter()
                .filter_map(|signal| signal.get("kind"))
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Some(PromptInjectionWarning {
        tool_call_id: tool_result.tool_call_id.clone(),
        tool,
        warning: FETCH_PROMPT_INJECTION_WARNING.to_owned(),
        signal_kinds,
    })
}

/// The turn-level warning channel.
///
/// One counter (the durable truth: *how many* injections were seen) plus a
/// bounded window of records (the inspectable detail). Both are low
/// cardinality: a record names the tool and the call, never the payload.
#[derive(Debug, Default)]
pub struct PromptInjectionReport {
    warnings: AtomicU64,
    records: Mutex<VecDeque<PromptInjectionWarning>>,
}

impl PromptInjectionReport {
    /// An empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one detection.
    pub fn record(&self, warning: PromptInjectionWarning) {
        self.warnings.fetch_add(1, Ordering::Relaxed);
        let mut records = self.records.lock();
        if records.len() == MAX_PROMPT_INJECTION_RECORDS {
            records.pop_front();
        }
        records.push_back(warning);
    }

    /// How many injections were reported.
    pub fn warnings(&self) -> u64 {
        self.warnings.load(Ordering::Relaxed)
    }

    /// The retained records, oldest first.
    pub fn records(&self) -> Vec<PromptInjectionWarning> {
        self.records.lock().iter().cloned().collect()
    }
}

/// The module that turns the output-side marker into a visible turn warning.
///
/// It observes [`HookPoint::AfterToolResult`] and does exactly two things
/// when a marker says the payload was hostile:
///
/// 1. it records the detection in the shared [`PromptInjectionReport`]
///    (the count an operator can read), and
/// 2. it returns a **prompt overlay** carrying the fixed warning text, which
///    the runtime delivers into the next provider request — so the warning is
///    visible to the model instead of being silently swallowed.
///
/// The directive is always `Continue`: the turn is never intercepted.
#[derive(Clone)]
pub struct PromptInjectionGuardModule {
    report: Arc<PromptInjectionReport>,
    manifest: ModuleManifest,
}

impl PromptInjectionGuardModule {
    /// Build the guard over a shared report.
    pub fn new(report: Arc<PromptInjectionReport>) -> Self {
        Self {
            report,
            manifest: ModuleManifest::new(
                PROMPT_INJECTION_GUARD_MODULE_ID,
                "Prompt Injection Guard",
            ),
        }
    }

    /// The shared report this module writes to.
    pub fn report(&self) -> &Arc<PromptInjectionReport> {
        &self.report
    }
}

impl std::fmt::Debug for PromptInjectionGuardModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptInjectionGuardModule")
            .field("manifest", &self.manifest)
            .field("warnings", &self.report.warnings())
            .finish()
    }
}

#[async_trait::async_trait]
impl BehaviorModule for PromptInjectionGuardModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::AfterToolResult {
            return Ok(ModuleOutcome::continue_());
        }
        let Some(result) = ctx.tool_result() else {
            return Ok(ModuleOutcome::continue_());
        };
        let Some(warning) = detect_prompt_injection(ctx.tool_call(), result) else {
            return Ok(ModuleOutcome::continue_());
        };

        self.report.record(warning.clone());

        // The overlay text is the fixed constant: the warning can never be
        // talked out of existence by the payload it is about.
        Ok(ModuleOutcome::continue_().with_system_overlay(warning.warning))
    }
}

/// 回合层提示注入告警面的**装配接线** (2026-11-05): fetch 的**输出侧**扫描会把
/// `prompt_injection` 标记写进工具结果 ⇒ ⇒ 但治理面的 `PromptInjectionHook`
/// 只扫**入参** ⇒ ⇒ 此前**无人读**这个标记。
/// ⇒ ⇒ 这里装一个只读告警模块: 在 `AfterToolResult` 读标记 ⇒ ⇒ 回合层报一条
/// **固定措辞**的可见告警 (恒不阻断, 显式化不静默)。
///
/// ⚠️ 构造条件 = fetch 工具开着 (fetch 关 = 根本没有这个输出面 ⇒ ⇒ 零注册,
/// 零告警)。故本函数**只由抓取面在场时**调用; 调用点留在装配根 (那里才知道
/// fetch 工具在不在), 实现与说明留在本文件 (装配根只留一行接线)。
///
/// 返回交回的是**注册进去的那个模块所写的同一本账** (同一 `Arc`) ⇒ ⇒ 宿主
/// 读到的计数与告警模块写进去的计数逐条同源。
pub(super) fn install(
    modules: &mut Vec<Arc<dyn Module>>,
    instrument: &BuildInstrumentation,
) -> Arc<PromptInjectionReport> {
    instrument.constructed(ModuleFamily::PromptInjectionGuard);
    let report = Arc::new(PromptInjectionReport::new());
    modules.push(Arc::new(PromptInjectionGuardModule::new(Arc::clone(&report))) as Arc<dyn Module>);
    report
}

impl ProductionModules {
    /// 回合层提示注入告警账句柄 (fetch 面开才 `Some`)。
    ///
    /// `None` = fetch 工具没注册 = 没有任何输出侧 `prompt_injection` 标记
    /// ⇒ 零告警 (是「没有这个面」, 不是「有面但没命中」)。宿主读它即可知道
    /// 本进程见过几次注入命中, 不用自己再解析 transcript。
    #[must_use]
    pub fn prompt_injection_report(&self) -> Option<&Arc<PromptInjectionReport>> {
        self.prompt_injection_report.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok_result(value: serde_json::Value) -> ToolResult {
        ToolResult::ok("call_1", value).with_name("fetch")
    }

    #[test]
    fn a_detected_marker_is_read_out_of_the_result_value() {
        let result = ok_result(json!({
            "url": "https://example.test/",
            FETCH_PROMPT_INJECTION_FIELD: {
                "detected": true,
                "signals": [{"kind": "ignore_previous_instructions", "start": 0, "end": 4}],
            },
        }));

        let warning = detect_prompt_injection(None, &result).expect("marker must be read");
        assert_eq!(warning.warning, FETCH_PROMPT_INJECTION_WARNING);
        assert_eq!(warning.tool.as_deref(), Some("fetch"));
        assert_eq!(warning.signal_kinds, ["ignore_previous_instructions"]);
    }

    #[test]
    fn a_result_without_the_marker_is_not_a_detection() {
        let result = ok_result(json!({"url": "https://example.test/"}));
        assert!(detect_prompt_injection(None, &result).is_none());
    }

    #[test]
    fn a_negative_marker_is_not_a_detection() {
        let result = ok_result(json!({
            FETCH_PROMPT_INJECTION_FIELD: {"detected": false, "signals": []},
        }));
        assert!(detect_prompt_injection(None, &result).is_none());
    }

    #[test]
    fn a_failed_result_carries_no_detection() {
        let result = ToolResult::permanent_error("call_1", "boom");
        assert!(detect_prompt_injection(None, &result).is_none());
    }
}
