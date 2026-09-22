// 治理卷宗 — 审计 tab（执行轨迹 × 事件留痕）纯逻辑（DOM-free，Node 可直测）
//
// 契约依据：docs/gateway-api-contract.md §8——
//   GET /v1/panel/traces?limit=   → {traces: [TraceSummaryDto]}
//   GET /v1/panel/traces/:id      → {spans: [TraceSpanDto]}
//   GET /v1/panel/audit?limit=    → {events: [{ts, event, service, detail?}]}
// DTO 逐字段核实于 crates/adapters/gateway/src/panels.rs:43-103。
//
// spanTreeSort / spanDepth 吸收自 views/ActivityView.svelte（原本地函数），
// 治理卷宗与日志视图共用一份实现——复用而非另起炉灶。

import type {TraceSpanItem, TraceSummaryItem} from '../runtime';
import type {ActivityItem} from '../types';

// ---------------------------------------------------------------------------
// span 树（主从右栏的轨迹详情）
// ---------------------------------------------------------------------------

/** 按开始时刻排序（父子层级由 spanDepth 推导，不重组数组）。 */
export function spanTreeSort(spans: TraceSpanItem[]): TraceSpanItem[] {
  return [...spans].sort((a, b) => a.started_at - b.started_at);
}

/** 父链深度（带环防卫，上限 6 层缩进）。 */
export function spanDepth(spans: TraceSpanItem[], span: TraceSpanItem): number {
  let depth = 0;
  let cur = span.parent_span_id;
  const guard = new Set<string>();
  while (cur && !guard.has(cur)) {
    guard.add(cur);
    depth++;
    const parent = spans.find((s) => s.span_id === cur);
    cur = parent?.parent_span_id ?? null;
  }
  return Math.min(depth, 6);
}

/** span 时长（ms）；未收口（ended_at 缺省）返回 null——UI 显示「进行中」。 */
export function spanDurationMs(span: TraceSpanItem): number | null {
  if (span.ended_at === null || span.ended_at === undefined) return null;
  return Math.max(0, span.ended_at - span.started_at);
}

/** span kind → badge 语义（五色纪律：绿=正常收口、红=异常、蓝=信息、灰=其他）。 */
export function spanStatusBadge(status: string): {variant: 'green' | 'danger' | 'blue' | 'dim'; label: string} {
  const s = status.toLowerCase();
  if (s === 'ok') return {variant: 'green', label: 'ok'};
  if (s === 'error') return {variant: 'danger', label: 'error'};
  if (!s) return {variant: 'dim', label: '—'};
  return {variant: 'blue', label: status};
}

// ---------------------------------------------------------------------------
// 轨迹列表行（主从左栏）
// ---------------------------------------------------------------------------

export interface TraceRow {
  traceId: string;
  spanCount: number;
  startedAtMs: number;
  rootKind: string;
  rootActor: string;
  rootStatus: string;
  rootSummary: string;
  sessionId: string | null;
  hasError: boolean;
}

/** TraceSummaryDto → 左栏行；root_summary 缺省不编造，落「(无摘要)」由 UI 决定。 */
export function traceRowFromSummary(t: TraceSummaryItem): TraceRow {
  return {
    traceId: t.trace_id,
    spanCount: t.span_count,
    startedAtMs: t.started_at,
    rootKind: t.root_span?.kind ?? '',
    rootActor: t.root_span?.actor ?? '',
    rootStatus: t.root_span?.status ?? '',
    rootSummary: t.root_span?.summary ?? '',
    sessionId: t.root_span?.session_id ?? null,
    hasError: (t.root_span?.status ?? '').toLowerCase() === 'error',
  };
}

export type TraceFilter = 'all' | 'error';

export function traceFilterCounts(rows: TraceRow[]): Record<TraceFilter, number> {
  return {all: rows.length, error: rows.filter((r) => r.hasError).length};
}

export function filterTraceRows(rows: TraceRow[], filter: TraceFilter): TraceRow[] {
  if (filter === 'error') return rows.filter((r) => r.hasError);
  return rows;
}

// ---------------------------------------------------------------------------
// 事件留痕（/v1/panel/audit → ActivityItem，fetchAuditLogs 已归一化）
// ---------------------------------------------------------------------------

/**
 * audit 事件名分组（chips 带计数）：取 `chat.turn.completed` 的首段作组名。
 * 未知/无点号事件归 'runtime' 组由 fetchAuditLogs 兜底——此处只认真实前缀。
 */
export function auditEventGroup(eventName: string): string {
  const head = eventName.split('.')[0]?.trim();
  return head || 'other';
}

export interface AuditGroupChip {
  group: string;
  count: number;
}

/** 分组计数（按 count 倒序，'all' 由视图层另算）。 */
export function auditGroupChips(items: ActivityItem[]): AuditGroupChip[] {
  const counts = new Map<string, number>();
  for (const item of items) {
    const g = auditEventGroup(item.title);
    counts.set(g, (counts.get(g) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([group, count]) => ({group, count}))
    .sort((a, b) => b.count - a.count);
}

export function filterAuditItems(items: ActivityItem[], group: string | 'all'): ActivityItem[] {
  if (group === 'all') return items;
  return items.filter((i) => auditEventGroup(i.title) === group);
}
