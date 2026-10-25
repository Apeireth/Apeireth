//! wasm32 侧的装配（把治理卷宗面的四个 Tab 挂进真实 DOM）。
//!
//! ## 调度
//!
//! 写入 ⇒ 一次**微任务** flush（设计文档 §4.2 规则 1）⇒ 同一轮事件处理里的
//! 多次写合并为一次 DOM 写入 ⇒ 界面在同一帧响应。

use std::cell::RefCell;
use std::rc::Rc;

use apeireth_ui_core::{HostScheduler, Runtime, Signal, UiComponent};
use apeireth_ui_dom::{Renderer, VNode, WebDom};
use apeireth_ui_kit::governance_tab::{
    format_rfc3339_ms, merge_session_inbox, ActivityItem, ApprovalFilter, ApprovalLedgerEntry,
    ApprovalsTab, ApprovalsTabProps, AuditSource, AuditTab, AuditTabProps, CapabilityManifest,
    GrantView, GrantsTab, GrantsTabProps, GuardEvent, GuardStatus, GuardTab, GuardTabProps,
    PendingApproval, TraceFilter, TraceRow, TraceSpanItem,
};
use wasm_bindgen::prelude::*;
use web_sys::Element;

use crate::fixture_generated::{
    audit_events, cases_approvals, cases_audit, cases_grants, cases_guard, error_text, grants,
    guard_events, guard_status, inbox, manifests, sessions, spans, traces, FixtureCase,
};

/// 微任务调度器：把 flush 排进浏览器的微任务队列。
struct MicrotaskScheduler;

impl HostScheduler for MicrotaskScheduler {
    fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
        let callback = Closure::once_into_js(move || {
            flush();
            JsValue::UNDEFINED
        });
        queue_microtask(&callback);
    }
}

#[wasm_bindgen]
extern "C" {
    /// 浏览器微任务队列（`queueMicrotask`）。
    #[wasm_bindgen(js_name = queueMicrotask)]
    fn queue_microtask(callback: &JsValue);
}

/// 应用状态（响应式部分；**可共享** ⇒ effect 闭包能捕获它）。
#[derive(Clone)]
struct GovState {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前卷宗面（`approvals` / `grants` / `audit` / `guard`）。
    tab: String,
    /// 基准时刻（毫秒；对照 harness 注入 ⇒ 两侧同一份）。
    base_ms: i64,
    /// 当前用例下标。
    case_index: Signal<usize>,
    /// 审批过滤档。
    filter: Signal<ApprovalFilter>,
    /// 审计数据源。
    source: Signal<AuditSource>,
    /// 轨迹过滤档。
    trace_filter: Signal<TraceFilter>,
    /// 留痕分组。
    audit_group: Signal<String>,
    /// 已展开的事件 id。
    expanded_ids: Signal<Vec<String>>,
    /// 正在确认撤销的授权 id。
    confirming_id: Signal<Option<String>>,
    /// 选中的轨迹 id。
    selected_trace_id: Signal<Option<String>>,
    /// 选中的留痕 id。
    selected_audit_id: Signal<Option<String>>,
}

impl GovState {
    /// 建一份状态。
    fn new(tab: String, case_index: usize, base_ms: i64) -> Self {
        let runtime = Runtime::new();
        let case_index = runtime.signal(case_index);
        let filter = runtime.signal(ApprovalFilter::All);
        let source = runtime.signal(AuditSource::Traces);
        let trace_filter = runtime.signal(TraceFilter::All);
        let audit_group = runtime.signal(String::from("all"));
        let expanded_ids = runtime.signal(Vec::new());
        let confirming_id = runtime.signal(None);
        let selected_trace_id = runtime.signal(None);
        let selected_audit_id = runtime.signal(None);
        Self {
            runtime,
            tab,
            base_ms,
            case_index,
            filter,
            source,
            trace_filter,
            audit_group,
            expanded_ids,
            confirming_id,
            selected_trace_id,
            selected_audit_id,
        }
    }
}

/// 应用（wasm 侧的单例）。
struct GovApp {
    /// 应用状态。
    state: GovState,
}

thread_local! {
    /// wasm 侧的单例应用。
    static APP: RefCell<Option<GovApp>> = const { RefCell::new(None) };
}

/// 读全局（对照 harness 在**导航前**注入）。
fn read_global(name: &str) -> Option<JsValue> {
    let window = web_sys::window()?;
    js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name)).ok()
}

/// 读全局字符串。
fn read_global_string(name: &str) -> Option<String> {
    read_global(name).and_then(|value| value.as_string())
}

/// 读全局数字。
fn read_global_number(name: &str) -> Option<f64> {
    read_global(name).and_then(|value| value.as_f64())
}

/// 当前用例清单。
fn cases_for(tab: &str) -> Vec<FixtureCase> {
    match tab {
        "approvals" => cases_approvals(),
        "grants" => cases_grants(),
        "audit" => cases_audit(),
        "guard" => cases_guard(),
        _ => Vec::new(),
    }
}

/// 能力清单（`None` ⇒ 清单未到）。
fn manifest_for(case: &FixtureCase) -> Option<CapabilityManifest> {
    let key = case.manifest.as_ref()?;
    manifests()
        .into_iter()
        .find(|(name, _)| name == key)
        .map(|(_, manifest)| manifest)
}

/// 基准 + 偏移 ⇒ RFC 3339 文本（`0` 偏移 ⇒ 无时刻）。
fn offset_iso(base_ms: i64, offset_ms: i64) -> Option<String> {
    if offset_ms == 0 {
        return None;
    }
    Some(format_rfc3339_ms(base_ms + offset_ms))
}

/// 审批账本（按生产的「会话清单 ⋈ 逐会话 inbox」合账口径）。
fn ledger_for(case: &FixtureCase, base_ms: i64) -> Vec<ApprovalLedgerEntry> {
    if case.data != "default" {
        return Vec::new();
    }
    let mut ledger: Vec<ApprovalLedgerEntry> = Vec::new();
    for session in sessions() {
        let items: Vec<PendingApproval> = inbox(&session.id)
            .iter()
            .map(|item| PendingApproval {
                approval_id: item.approval_id.clone(),
                session: item.session.clone(),
                tool_name: item.tool_name.clone(),
                governance_reason: item.governance_reason.clone(),
                command_text: item.command_text.clone(),
                arguments_summary: item.arguments_summary.clone(),
                created_at: offset_iso(base_ms, item.created_offset_ms),
                expires_at: offset_iso(base_ms, item.expires_offset_ms),
            })
            .collect();
        ledger = merge_session_inbox(&ledger, &session.id, &items, session.title.as_deref());
    }
    ledger
}

/// 授权列表（固定数据）。
fn grants_for(case: &FixtureCase, base_ms: i64) -> Vec<GrantView> {
    if case.data != "default" {
        return Vec::new();
    }
    grants()
        .iter()
        .map(|item| GrantView {
            id: item.id.clone(),
            name: item.name.clone(),
            activated_at_ms: if item.activated_offset_ms == 0 {
                0
            } else {
                base_ms + item.activated_offset_ms
            },
        })
        .collect()
}

/// 轨迹行（固定数据；根 span ⇒ 行）。
fn trace_rows_for(case: &FixtureCase, base_ms: i64) -> Vec<TraceRow> {
    if case.data != "default" {
        return Vec::new();
    }
    traces()
        .iter()
        .map(|item| {
            let root = &item.root;
            TraceRow {
                trace_id: item.trace_id.clone(),
                span_count: item.span_count,
                started_at_ms: base_ms + item.started_offset_ms,
                root_kind: root.kind.clone(),
                root_actor: root.actor.clone(),
                root_status: root.status.clone(),
                root_summary: root.summary.clone().unwrap_or_default(),
                session_id: root.session_id.clone(),
                has_error: root.status.to_lowercase() == "error",
            }
        })
        .collect()
}

/// span 树（固定数据）。
fn spans_for(case: &FixtureCase, trace_id: &str, base_ms: i64) -> Vec<TraceSpanItem> {
    if case.data != "default" {
        return Vec::new();
    }
    spans(trace_id)
        .iter()
        .map(|item| TraceSpanItem {
            span_id: item.span_id.clone(),
            parent_span_id: item.parent_span_id.clone(),
            kind: item.kind.clone(),
            actor: item.actor.clone(),
            status: item.status.clone(),
            summary: item.summary.clone(),
            started_at: (base_ms + item.started_offset_ms) as f64,
            ended_at: item.ended_offset_ms.map(|value| (base_ms + value) as f64),
        })
        .collect()
}

/// 事件留痕（固定数据；`detail` = 原始记录的缩进 JSON ⇒ 与脚本侧同口径）。
fn audit_items_for(case: &FixtureCase, base_ms: i64) -> Vec<ActivityItem> {
    if case.data != "default" {
        return Vec::new();
    }
    audit_events()
        .iter()
        .map(|item| {
            let timestamp = base_ms + item.ts_offset_ms;
            let raw = apeireth_ui_kit::json::JsonValue::Object(vec![
                (
                    String::from("ts"),
                    apeireth_ui_kit::json::JsonValue::Number(timestamp as f64),
                ),
                (
                    String::from("event"),
                    apeireth_ui_kit::json::JsonValue::String(item.event.clone()),
                ),
                (
                    String::from("service"),
                    apeireth_ui_kit::json::JsonValue::String(item.service.clone()),
                ),
                (
                    String::from("detail"),
                    match &item.detail {
                        Some(text) => apeireth_ui_kit::json::JsonValue::String(text.clone()),
                        None => apeireth_ui_kit::json::JsonValue::Null,
                    },
                ),
            ]);
            let detail = raw.to_string_pretty();
            ActivityItem {
                id: format!("{}-{}", item.event, timestamp),
                timestamp,
                title: item.event.clone(),
                summary: item.detail.clone().unwrap_or_else(|| item.event.clone()),
                source: String::from("audit"),
                detail: Some(detail),
            }
        })
        .collect()
}

/// 守卫状态（固定数据）。
fn guard_status_for(case: &FixtureCase) -> Option<GuardStatus> {
    if case.data == "error" {
        return None;
    }
    let item = guard_status();
    Some(GuardStatus {
        enabled: item.enabled,
        fast_guard_active: item.fast_guard_active,
        chain_guard_active: item.chain_guard_active,
        intent_guard_active: item.intent_guard_active,
        cross_turn_monitoring_active: item.cross_turn_monitoring_active,
        ml_classifier_available: item.ml_classifier_available,
        ml_model_version: item.ml_model_version.clone(),
        total_evaluations: item.total_evaluations,
        total_allowed: item.total_allowed,
        total_denied: item.total_denied,
        total_approval_required: item.total_approval_required,
    })
}

/// 守卫事件（固定数据）。
fn guard_events_for(case: &FixtureCase, base_ms: i64) -> Vec<GuardEvent> {
    if case.data != "default" {
        return Vec::new();
    }
    guard_events()
        .iter()
        .map(|item| GuardEvent {
            timestamp_ms: base_ms + item.timestamp_offset_ms,
            session_id: item.session_id.clone(),
            trace_id: item.trace_id.clone(),
            round: item.round,
            capability_id: item.capability_id.clone(),
            stage: item.stage.clone(),
            decision: item.decision.clone(),
            risk_score: item.risk_score,
            reasons: item.reasons.clone(),
            evidence: item.evidence.clone(),
        })
        .collect()
}

/// 渲染当前状态（按卷宗面分派）。
fn render(state: &GovState) -> VNode {
    let cases = cases_for(&state.tab);
    let index = state.case_index.get();
    let Some(case) = cases.get(index) else {
        return VNode::fragment(Vec::new());
    };
    match state.tab.as_str() {
        "approvals" => render_approvals(state, case),
        "grants" => render_grants(state, case),
        "audit" => render_audit(state, case),
        "guard" => render_guard(state, case),
        _ => VNode::fragment(Vec::new()),
    }
}

/// 审批收件箱。
fn render_approvals(state: &GovState, case: &FixtureCase) -> VNode {
    let filter = state.filter.clone();
    let on_filter = Some(Rc::new({
        let filter = state.filter.clone();
        move |value: ApprovalFilter| filter.set(value)
    }) as Rc<dyn Fn(ApprovalFilter)>);
    let props = ApprovalsTabProps {
        capabilities: manifest_for(case),
        ledger: ledger_for(case, state.base_ms),
        loading: case.data == "loading",
        error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        filter,
        sse_state: apeireth_ui_kit::governance_tab::SseConnectionState::Unsupported,
        now_ms: state.base_ms,
        resolving_id: None,
        resolve_note: String::new(),
        resolve_error: String::new(),
        on_open_chat: None,
        on_go_audit: None,
        on_resolve: None,
        on_filter,
    };
    ApprovalsTab::render(&props)
}

/// 授权管理。
fn render_grants(state: &GovState, case: &FixtureCase) -> VNode {
    let confirming = state.confirming_id.clone();
    let on_confirm = Some(Rc::new({
        let confirming = state.confirming_id.clone();
        move |id: &str| confirming.set(Some(id.to_owned()))
    }) as Rc<dyn Fn(&str)>);
    let on_cancel = Some(Rc::new({
        let confirming = state.confirming_id.clone();
        move |_id: &str| confirming.set(None)
    }) as Rc<dyn Fn(&str)>);
    let props = GrantsTabProps {
        capabilities: manifest_for(case),
        grants: grants_for(case, state.base_ms),
        loading: case.data == "loading",
        error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        confirming_id: confirming.get(),
        revoking_id: None,
        revoke_error: String::new(),
        now_ms: state.base_ms,
        on_confirm,
        on_revoke: None,
        on_cancel,
    };
    GrantsTab::render(&props)
}

/// 审计。
fn render_audit(state: &GovState, case: &FixtureCase) -> VNode {
    let source = state.source.clone();
    let on_source = Some(Rc::new({
        let source = state.source.clone();
        move |value: AuditSource| source.set(value)
    }) as Rc<dyn Fn(AuditSource)>);
    let trace_filter = state.trace_filter.clone();
    let on_trace_filter = Some(Rc::new({
        let trace_filter = state.trace_filter.clone();
        move |value: TraceFilter| trace_filter.set(value)
    }) as Rc<dyn Fn(TraceFilter)>);
    let audit_group = state.audit_group.clone();
    let on_audit_group = Some(Rc::new({
        let audit_group = state.audit_group.clone();
        move |value: &str| audit_group.set(value.to_owned())
    }) as Rc<dyn Fn(&str)>);
    let selected_trace_id = state.selected_trace_id.clone();
    let on_select_trace = Some(Rc::new({
        let selected = state.selected_trace_id.clone();
        move |id: &str| selected.set(Some(id.to_owned()))
    }) as Rc<dyn Fn(&str)>);
    let selected_audit_id = state.selected_audit_id.clone();
    let on_select_audit = Some(Rc::new({
        let selected = state.selected_audit_id.clone();
        move |id: &str| selected.set(Some(id.to_owned()))
    }) as Rc<dyn Fn(&str)>);
    let current_trace_id = selected_trace_id.get().unwrap_or_default();
    let props = AuditTabProps {
        capabilities: manifest_for(case),
        source,
        trace_rows: trace_rows_for(case, state.base_ms),
        trace_error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        trace_filter,
        selected_trace_id: selected_trace_id.get(),
        trace_spans: spans_for(case, &current_trace_id, state.base_ms),
        trace_detail_loading: false,
        trace_detail_error: String::new(),
        audit_items: audit_items_for(case, state.base_ms),
        audit_error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        audit_group,
        selected_audit_id: selected_audit_id.get(),
        loading: case.data == "loading",
        now_ms: state.base_ms,
        on_source,
        on_trace_filter,
        on_audit_group,
        on_select_trace,
        on_select_audit,
    };
    AuditTab::render(&props)
}

/// 守卫。
fn render_guard(state: &GovState, case: &FixtureCase) -> VNode {
    let expanded = state.expanded_ids.clone();
    let on_toggle_event = Some(Rc::new({
        let expanded = state.expanded_ids.clone();
        move |id: &str| {
            let mut ids = expanded.get();
            if let Some(index) = ids.iter().position(|item| item == id) {
                ids.remove(index);
            } else {
                ids.push(id.to_owned());
            }
            expanded.set(ids);
        }
    }) as Rc<dyn Fn(&str)>);
    let props = GuardTabProps {
        capabilities: manifest_for(case),
        status: guard_status_for(case),
        status_error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        events: guard_events_for(case, state.base_ms),
        events_error: if case.data == "error" {
            error_text()
        } else {
            String::new()
        },
        loading: case.data == "loading",
        expanded_ids: expanded,
        dry_result: None,
        dry_error: String::new(),
        now_ms: state.base_ms,
        on_toggle_event,
    };
    GuardTab::render(&props)
}

/// 挂载当前卷宗面。
fn mount(container: &Element, tab: String, case_index: usize, base_ms: i64) -> Result<(), String> {
    // ⭐ **装上「浏览器时刻文本」**（宿主能力 ⇒ 与生产**逐字同源**）：
    // ⇒ 生产的时刻显示走浏览器 `Intl`（本地时区 + ICU）⇒
    // ⇒ 不装 ⇒ 引擎用**固定 UTC** ⇒ ⇒ 非 UTC 时区下**差几小时**。
    apeireth_ui_time::install();
    let dom = WebDom::new().map_err(|error| error.to_string())?;
    // 对照容器（**两侧同盒** ⇒ 逐像素对照的取景框一致；参考侧的容器在 Svelte
    // 包装里，class 名逐字相同）。
    let stage = document()
        .create_element("div")
        .map_err(|error| format!("创建对照容器失败：{error:?}"))?;
    let _ = stage.set_attribute("class", "governance-stage");
    container
        .append_child(&stage)
        .map_err(|error| format!("挂载对照容器失败：{error:?}"))?;
    let root: web_sys::Node = stage.into();
    let mut renderer = Renderer::new(dom, root);

    let state = GovState::new(tab, case_index, base_ms);
    state
        .runtime
        .set_host_scheduler(Rc::new(MicrotaskScheduler));

    renderer
        .mount(render(&state))
        .map_err(|error| error.to_string())?;

    let renderer = Rc::new(RefCell::new(renderer));
    let effect_renderer = renderer.clone();
    let effect_state = state.clone();
    state.runtime.effect(move || {
        let tree = render(&effect_state);
        let mut guard = effect_renderer.borrow_mut();
        let _ = guard.patch(tree);
        drop(guard);
        write_textarea_value();
    });

    // 试算表单的初值（`bind:value` 的 property 写回在引擎侧未接线 ⇒ 宿主代写）。
    // ⚠️ 挂载后**立刻**写一次（effect 的首跑不保证发生）⇒ 画面与参考侧一致。
    write_textarea_value();

    APP.with(|slot| {
        // 渲染器由 **effect 闭包**持有（`Rc` 共享 ⇒ 渲染器随闭包存活）⇒ 本结构
        // 只留状态。
        *slot.borrow_mut() = Some(GovApp { state });
    });
    Ok(())
}

/// 当前文档。
fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .unwrap_or_else(|| web_sys::Document::new().unwrap())
}

/// 写试算表单的初值（`bind:value` 的 property 写回在引擎侧未接线 ⇒ 宿主代写）。
fn write_textarea_value() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(node)) = document.query_selector("textarea.dry-input") else {
        return;
    };
    if let Ok(area) = node.dyn_into::<web_sys::HtmlTextAreaElement>() {
        let _ = area.set_value("{}");
    }
}

/// 引擎是否已就绪（挂载完成）。
#[wasm_bindgen]
pub fn is_ready() -> bool {
    APP.with(|slot| slot.borrow().is_some())
}

/// 当前卷宗面的用例数。
#[wasm_bindgen]
pub fn case_count() -> u32 {
    APP.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|app| cases_for(&app.state.tab).len() as u32)
            .unwrap_or(0)
    })
}

/// 切到第 `index` 组用例（越界 ⇒ 不切）。
#[wasm_bindgen]
pub fn select_case(index: u32) {
    APP.with(|slot| {
        let slot = slot.borrow();
        let Some(app) = slot.as_ref() else {
            return;
        };
        let count = cases_for(&app.state.tab).len();
        if (index as usize) < count {
            app.state.case_index.set(index as usize);
        }
    });
}

/// 启动（模块装载时跑一次）。
///
/// 应用形态由 `window.__GOV_TAB__` 决定（**必须在 wasm 初始化之前**设定）。
#[wasm_bindgen]
pub fn start() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(container) = document.get_element_by_id("app") else {
        return;
    };
    let tab = read_global_string("__GOV_TAB__").unwrap_or_default();
    let case_index = read_global_number("__GOV_CASE_INDEX__").unwrap_or(0.0) as usize;
    let base_ms = read_global_number("__GOV_BASE__").unwrap_or_else(js_sys::Date::now) as i64;
    let _ = mount(&container, tab, case_index, base_ms);
}
