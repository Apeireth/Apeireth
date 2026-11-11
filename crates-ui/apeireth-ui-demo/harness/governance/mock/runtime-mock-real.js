// `runtime` 替身的**已接线部分**（真实现）。
//
// ## 边界
//
// - 只有**治理卷宗面四个 Tab 真正调用到**的导出在这里给真实现；其余导出是
//   生成脚本产出的**显式报错桩**（见 `runtime-mock.js` 的生成头）。
// - 真实现**只喂固定 fixture** ⇒ 不碰 HTTP / SSE 契约（设计文档 §1.3 第 6 条）。
// - **变更调用不接线**：`resolveCanonicalApproval` / `revokeGrant` 保持为桩
//   （调用即抛错）⇒ 对照 harness 不覆盖「点签批 / 点撤销」之后的服务端状态。
//
// ## 用例驱动的数据行为
//
// 参考侧把当前用例写进 `window.__GOV_CASE__`（对照 harness 在**导航前**注入
// ⇒ 生产组件的 `onMount` 读得到）⇒ mock 按用例的 `data` 字段决定行为：
//
// | `data` | 行为 |
// |---|---|
// | `default` | 返回 fixture 的固定数据 |
// | `empty` | 返回空集合（空态分支） |
// | `error` | 抛错 / 返回 `{error}`（错误态分支；文案 = fixture 的 `error_text`） |
// | `loading` | **永不 resolve**（加载态分支：组件停在 loading） |

import fixture from "../fixtures/governance-cases.json";

/** 置「数据请求已发出」标志（对照 harness 据此判定画面已到数据态）。 */
function markDataRequested() {
  if (typeof window !== "undefined") window.__GOV_DATA_READY__ = true;
}

/** 当前用例（对照 harness 注入）。 */
function currentCase() {
  if (typeof window === "undefined") return null;
  return window.__GOV_CASE__ || null;
}

/** 当前用例的数据行为（缺省 `default`）。 */
function dataMode() {
  const item = currentCase();
  return (item && item.data) || "default";
}

/** 本次运行的基准时刻（毫秒；对照 harness 注入 ⇒ 两侧同一份）。 */
function baseMs() {
  if (typeof window !== "undefined" && typeof window.__GOV_BASE__ === "number") {
    return window.__GOV_BASE__;
  }
  return Date.now();
}

/** 基准 + 偏移 ⇒ RFC 3339 文本（与引擎侧的 `toISOString()` 同形）。 */
function offsetToIso(offsetMs) {
  if (!offsetMs) return undefined;
  return new Date(baseMs() + offsetMs).toISOString();
}

/** 基准 + 偏移 ⇒ epoch 毫秒。 */
function offsetToMs(offsetMs) {
  if (!offsetMs) return 0;
  return baseMs() + offsetMs;
}

/** 永不 resolve 的 promise（加载态分支）。 */
function neverResolve() {
  return new Promise(() => {});
}

/** 能力别名表（与生产同口径）。 */
const CAPABILITY_ALIASES = {
  "approvals.read": "permissions.approval.read",
  "approvals.resolve": "permissions.approval.resolve",
};

/** 取规范 id。 */
function canonicalCapabilityId(id) {
  return CAPABILITY_ALIASES[id] ?? id;
}

/** 跨组查找能力声明。 */
export function findCapability(manifest, id) {
  if (!manifest) return null;
  const want = canonicalCapabilityId(id);
  let aliasMatch = null;
  for (const group of manifest.capabilities || []) {
    for (const cap of group.capabilities || []) {
      if (cap.id === want || cap.id === id) return cap;
      if (cap.alias_of === want || cap.alias_of === id) aliasMatch = cap;
    }
  }
  return aliasMatch;
}

/** 该能力**此时此刻**是否可调用（与生产 `capabilityAvailable` 同口径）。 */
export function capabilityAvailable(manifest, id) {
  if (!manifest) return false;
  const cap = findCapability(manifest, id);
  if (!cap) return false;
  return cap.supported === true && (cap.available === undefined || cap.available === true);
}

/** 该能力不可用的 machine-readable 原因（与生产同口径）。 */
export function capabilityUnavailableReason(manifest, id) {
  if (!manifest) return null;
  const cap = findCapability(manifest, id);
  if (!cap) return null;
  if (cap.available === false) return cap.reason ?? null;
  return null;
}

/**
 * 错误文案（与生产 `friendlyErrorMessage` 在「普通 Error」这一支上同口径）。
 *
 * ⚠️ 生产的 `HttpError` 分支在对照里**不会发生**（mock 抛的是普通 `Error`）。
 */
export function friendlyErrorMessage(caught) {
  if (caught instanceof Error) return caught.message;
  if (typeof caught === "string") return caught;
  if (caught === null) return "未知错误 (null)";
  if (caught === undefined) return "未知错误 (undefined)";
  if (typeof caught === "object") {
    for (const key of ["message", "error", "detail", "description"]) {
      const value = caught[key];
      if (typeof value === "string" && value.trim()) return value;
      if (value instanceof Error && value.message) return value.message;
      if (value && typeof value === "object") {
        const nested = value.message;
        if (typeof nested === "string" && nested.trim()) return nested;
      }
    }
    try {
      const json = JSON.stringify(caught);
      if (json && json !== "{}") return json;
    } catch {
      /* 循环引用 ⇒ 落到形状描述 */
    }
    const name = caught.constructor?.name;
    return `未知错误对象${name && name !== "Object" ? ` (${name})` : ""}`;
  }
  return String(caught);
}

/** 会话清单（固定 fixture）。 */
export function fetchBackendSessions() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  // ⚠️ **同步抛出**（不是 rejected promise）：生产的审批面两个 fetcher 都自带
  // `.catch(...)` ⇒ 异步拒绝会被吞掉 ⇒ 错误态**不可达**；同步抛出才落得到
  // 生产组件自己的 `catch` 分支（对照要覆盖的就是那条分支）。
  if (mode === "error") throw new Error(fixture.error_text);
  if (mode === "empty") return Promise.resolve([]);
  return Promise.resolve(fixture.sessions.map((session) => ({
    id: session.id,
    title: session.title || undefined,
    started_at: offsetToMs(-600000),
    last_active_at: offsetToMs(-60000),
    episode_count: 1,
  })));
}

/** 逐会话待签 inbox（固定 fixture）。 */
export function fetchCanonicalApprovals(_config, sessionId) {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  // ⚠️ **同步抛出**（不是 rejected promise）：生产的审批面两个 fetcher 都自带
  // `.catch(...)` ⇒ 异步拒绝会被吞掉 ⇒ 错误态**不可达**；同步抛出才落得到
  // 生产组件自己的 `catch` 分支（对照要覆盖的就是那条分支）。
  if (mode === "error") throw new Error(fixture.error_text);
  if (mode === "empty") return Promise.resolve([]);
  const inbox = fixture.inbox[sessionId] || [];
  return Promise.resolve(inbox.map((item) => {
    const approval = {
      session: item.session,
      approval_id: item.approval_id,
      request: item.request,
      trace_id: item.trace_id,
      capability_id: item.capability_id,
      tool_name: item.tool_name,
      governance_hook: item.governance_hook,
      governance_reason: item.governance_reason,
    };
    if (item.command_text) approval.command_text = item.command_text;
    if (item.arguments_summary) approval.arguments_summary = item.arguments_summary;
    const created = offsetToIso(item.created_offset_ms);
    if (created) approval.created_at = created;
    const expires = offsetToIso(item.expires_offset_ms);
    if (expires) approval.expires_at = expires;
    return approval;
  }));;
}

/** 授权列表（固定 fixture）。 */
export function fetchGrants() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  // ⚠️ **同步抛出**（不是 rejected promise）：生产的审批面两个 fetcher 都自带
  // `.catch(...)` ⇒ 异步拒绝会被吞掉 ⇒ 错误态**不可达**；同步抛出才落得到
  // 生产组件自己的 `catch` 分支（对照要覆盖的就是那条分支）。
  if (mode === "error") throw new Error(fixture.error_text);
  if (mode === "empty") return Promise.resolve([]);
  return Promise.resolve(fixture.grants.map((grant) => ({
    id: grant.id,
    name: grant.name,
    tools: [],
    paths: [],
    expiry: "",
    op_budget: null,
    used_ops: 0,
    spend_budget: null,
    spend_used: 0,
    activated_at_ms: offsetToMs(grant.activated_offset_ms),
    created_at_ms: offsetToMs(grant.activated_offset_ms),
    active: true,
    expired: false,
  })));;
}

/** 执行轨迹列表（固定 fixture）。 */
export function fetchTraceList() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  if (mode === "error") return { error: fixture.error_text };
  if (mode === "empty") return Promise.resolve([]);
  return Promise.resolve(fixture.traces.map((trace) => ({
    trace_id: trace.trace_id,
    span_count: trace.span_count,
    started_at: offsetToMs(trace.started_offset_ms),
    root_span: spanItem(trace.root_span),
  })));;
}

/** 轨迹详情（固定 fixture）。 */
export function fetchTraceDetail(_config, traceId) {
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  if (mode === "error") return { error: fixture.error_text };
  const spans = fixture.spans[traceId] || [];
  return Promise.resolve(spans.map((span) => spanItem(span)));
}

/** span 条目（fixture 的偏移形态 ⇒ 运行期形态）。 */
function spanItem(span) {
  return {
    span_id: span.span_id,
    trace_id: span.trace_id,
    parent_span_id: span.parent_span_id,
    kind: span.kind,
    actor: span.actor,
    status: span.status,
    summary: span.summary || null,
    attributes: span.attributes,
    started_at: offsetToMs(span.started_offset_ms),
    ended_at: span.ended_offset_ms ? offsetToMs(span.ended_offset_ms) : null,
    session_id: span.session_id,
  };
}

/** 事件留痕（固定 fixture）。 */
export function fetchAuditLogs() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  // ⚠️ **同步抛出**（不是 rejected promise）：生产的审批面两个 fetcher 都自带
  // `.catch(...)` ⇒ 异步拒绝会被吞掉 ⇒ 错误态**不可达**；同步抛出才落得到
  // 生产组件自己的 `catch` 分支（对照要覆盖的就是那条分支）。
  if (mode === "error") throw new Error(fixture.error_text);
  if (mode === "empty") return Promise.resolve([]);
  return Promise.resolve(fixture.audit_events.map((event, index) => {
    const ts = offsetToMs(event.ts_offset_ms);
    const raw = { ts, event: event.event, service: event.service, detail: event.detail };
    return {
      id: `${event.event || "audit"}-${ts || index}`,
      timestamp: ts,
      category: "runtime",
      title: event.event || "系统事件",
      summary: event.detail || event.event || "系统操作留痕",
      source: "audit",
      severity: "info",
      detail: JSON.stringify(raw, null, 2),
      raw,
    };
  }));;
}

/** 守卫状态（固定 fixture）。 */
export function fetchGuardStatus() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  if (mode === "error") return { error: fixture.error_text };
  const status = fixture.guard_status;
  return Promise.resolve({
    enabled: status.enabled,
    fast_guard_active: status.fast_guard_active,
    chain_guard_active: status.chain_guard_active,
    intent_guard_active: status.intent_guard_active,
    cross_turn_monitoring_active: status.cross_turn_monitoring_active,
    active_chains: status.active_chains,
    total_evaluations: status.total_evaluations,
    total_allowed: status.total_allowed,
    total_denied: status.total_denied,
    total_approval_required: status.total_approval_required,
    dataset_recording_enabled: status.dataset_recording_enabled,
    ml_classifier_available: status.ml_classifier_available,
    ml_model_version: status.ml_model_version,
  });;
}

/** 守卫事件（固定 fixture）。 */
export function fetchGuardEvents() {
  markDataRequested();
  const mode = dataMode();
  if (mode === "loading") return neverResolve();
  if (mode === "error") return { error: fixture.error_text };
  if (mode === "empty") return Promise.resolve([]);
  return Promise.resolve(fixture.guard_events.map((event) => ({
    timestamp_ms: offsetToMs(event.timestamp_offset_ms),
    session_id: event.session_id,
    trace_id: event.trace_id,
    round: event.round,
    capability_id: event.capability_id,
    stage: event.stage,
    decision: event.decision,
    risk_score: event.risk_score,
    reasons: event.reasons,
    evidence: event.evidence,
  })));;
}

/** 试算（固定 fixture）。 */
export async function evaluateGuard() {
  const mode = dataMode();
  if (mode === "error") return { error: fixture.error_text };
  return {
    decision: fixture.dry_run.decision,
    stage: fixture.dry_run.stage,
    risk_score: fixture.dry_run.risk_score,
    reasons: fixture.dry_run.reasons,
    evidence: fixture.dry_run.evidence,
  };
}
