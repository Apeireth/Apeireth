// 治理卷宗 — 审批收件箱账本纯逻辑（DOM-free，Node 可直测）
//
// 契约依据：docs/gateway-api-contract.md §6（`/v1/approvals?session=` 是会话级
// 端点——全局总账 = 会话清单 ⋈ 逐会话 inbox）、§8（SSE approval_required /
// approval_resolved）；00-PHILOSOPHY §3.2/§4（对话内待签文书是「当下」，
// 这里是「账」——全局待批的总账）。
//
// 诚实边界（0 装）：
// - 后端没有「已批/已拒历史」端点，本账本只承载 **当前待签**；已落笔的判断
//   去审计 tab 的留痕里看。UI 文案不许暗示这里有历史。
// - approval_required 帧只含路由主键（session/approval_id/tool_name），
//   完整文书字段（command_text 等）必须重拉该会话 inbox，不许用帧拼假数据。

import type {CanonicalPendingApproval} from '../runtime';

/** 账本条目 = 后端文书 + 派生展示字段（全部由真实字段推导，无虚构）。 */
export interface ApprovalLedgerEntry {
  approval: CanonicalPendingApproval;
  /** 会话标题（来自 /v1/panel/sessions；后端无标题时为 undefined，UI 显示短 id）。 */
  sessionTitle?: string;
  /** created_at 解析为 epoch ms；解析失败为 undefined（UI 隐藏时间行）。 */
  createdAtMs?: number;
  /** expires_at 解析为 epoch ms。 */
  expiresAtMs?: number;
}

/** kernel Timestamp 线上形状是 RFC 3339 字符串；容忍 epoch 数字（契约示例）。 */
export function approvalTimeMs(value: unknown): number | undefined {
  if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
  if (typeof value === 'string') {
    const t = Date.parse(value);
    return Number.isNaN(t) ? undefined : t;
  }
  return undefined;
}

export function isApprovalExpired(entry: ApprovalLedgerEntry, nowMs: number): boolean {
  return entry.expiresAtMs !== undefined && entry.expiresAtMs <= nowMs;
}

/**
 * 把某会话的 inbox 合入账本：该会话的旧条目整体被后端真值替换（自愈纪律——
 * 后端列表是唯一事实源，过期/已处理的条目以后端缺席为准消失），其他会话不动。
 */
export function mergeSessionInbox(
  ledger: ApprovalLedgerEntry[],
  sessionId: string,
  inbox: CanonicalPendingApproval[],
  sessionTitle?: string,
): ApprovalLedgerEntry[] {
  const rest = ledger.filter((e) => e.approval.session !== sessionId);
  const fresh = inbox.map((approval) => ({
    approval,
    sessionTitle,
    createdAtMs: approvalTimeMs(approval.created_at),
    expiresAtMs: approvalTimeMs(approval.expires_at),
  }));
  return [...rest, ...fresh].sort(
    (a, b) => (b.createdAtMs ?? 0) - (a.createdAtMs ?? 0),
  );
}

/**
 * SSE 事件落账（纯函数）：
 * - approval_resolved：按 approval_id 精确撤下（撤不下时按 session 整组撤，
 *   因为该会话同一时刻只挂一张文书——契约 §6 resolve 后继续原回合）。
 * - approval_required：账本无法凭帧补全字段，返回需要重拉的会话集合，
 *   由调用方发 HTTP 重拉（0 装：不拿帧里的 tool_name 拼半张卡）。
 */
export function applyApprovalEventToLedger(
  ledger: ApprovalLedgerEntry[],
  event: {session: string | null; approvalId: string | null},
  kind: 'approval_required' | 'approval_resolved',
): {entries: ApprovalLedgerEntry[]; dirtySessions: string[]} {
  if (kind === 'approval_required') {
    return {
      entries: ledger,
      dirtySessions: event.session ? [event.session] : [],
    };
  }
  let entries = ledger;
  if (event.approvalId && ledger.some((e) => e.approval.approval_id === event.approvalId)) {
    entries = ledger.filter((e) => e.approval.approval_id !== event.approvalId);
  } else if (event.session) {
    entries = ledger.filter((e) => e.approval.session !== event.session);
  }
  return {entries, dirtySessions: []};
}

// ---------------------------------------------------------------------------
// 过滤器 chips（带计数）——「全部 / 待签 / 已过期」
// ---------------------------------------------------------------------------

export type ApprovalFilter = 'all' | 'open' | 'expired';

export function approvalFilterCounts(
  ledger: ApprovalLedgerEntry[],
  nowMs: number,
): Record<ApprovalFilter, number> {
  let open = 0;
  let expired = 0;
  for (const e of ledger) {
    if (isApprovalExpired(e, nowMs)) expired++;
    else open++;
  }
  return {all: ledger.length, open, expired};
}

export function filterApprovalLedger(
  ledger: ApprovalLedgerEntry[],
  filter: ApprovalFilter,
  nowMs: number,
): ApprovalLedgerEntry[] {
  if (filter === 'all') return ledger;
  const wantExpired = filter === 'expired';
  return ledger.filter((e) => isApprovalExpired(e, nowMs) === wantExpired);
}

// ---------------------------------------------------------------------------
// SSE 连接态（真实连接态，不装）——状态机纯函数，EventSource 接线在视图层
// ---------------------------------------------------------------------------

/** connecting=首次建连中；live=已连接；reconnecting=断线退避中；unsupported=能力/环境无 SSE。 */
export type SseConnectionState = 'connecting' | 'live' | 'reconnecting' | 'unsupported';

export type SseSignal = 'open' | 'error' | 'giveup' | 'unsupported';

/** 状态迁移：error 不进 offline，进 reconnecting（订阅器带指数退避，永不放弃）。 */
export function nextSseState(_current: SseConnectionState, signal: SseSignal): SseConnectionState {
  switch (signal) {
    case 'open':
      return 'live';
    case 'error':
      return 'reconnecting';
    case 'unsupported':
      return 'unsupported';
    case 'giveup':
      return 'unsupported';
  }
}

export const SSE_STATE_LABEL: Record<SseConnectionState, string> = {
  connecting: '连接中',
  live: '实时已连接',
  reconnecting: '连接中断 · 重连中',
  unsupported: '实时不可用',
};

/** 退避序列（ms）：2s 起，×1.5，封顶 30s——与 runtime.subscribeCompanionEvents 同一约定。 */
export function nextRetryDelay(prevMs: number): number {
  return Math.min(Math.round(prevMs * 1.5), 30000);
}

export const SSE_INITIAL_RETRY_MS = 2000;
