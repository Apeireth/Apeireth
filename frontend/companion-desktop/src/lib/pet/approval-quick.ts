// 伴星 · 面板代审批（内置应用面 I 批 1 轨 B，纯逻辑零依赖）
//
// 契约（I-starport-builtins.md「面板代审批」）：
//   1. 帧即真源 —— 待签记录只来自真实 approval_required 网关帧（session/
//      approval_id/toolName 逐字段取自帧，不猜不补）；
//   2. 权威核对 —— 落决策前必须以权威待签队列按 approval_id 精确匹配，
//      匹配不到一律判「已不在待签队列」（他人/别处已处理的文书不双批）；
//   3. 双向同步 —— approval_resolved 帧到达即清面板（与主窗审批收件箱
//      同吃一条事件总线，谁先答谁生效，先答为准）；
//   4. 决策三态与契约一致：approve / reject / cancel（不自造第四种）；
//   5. 0 装 —— 解析不出主键的帧返回 null 丢弃（宁丢不猜）。
//
// 测试：tests/pet-approval-quick.mjs（真实模块直测）。

import type {CanonicalPendingApproval} from '../runtime';

export type ApprovalDecision = 'approve' | 'reject' | 'cancel';

/** 桌宠面板的待签记录（全部字段来自真实帧）。 */
export interface ApprovalQuickRecord {
  session: string;
  approvalId: string;
  toolName: string | null;
  at: number;
}

export type ApprovalQuickState =
  | {phase: 'none'}
  | {phase: 'asking'; record: ApprovalQuickRecord}
  | {phase: 'resolving'; record: ApprovalQuickRecord}
  | {phase: 'stale'; record: ApprovalQuickRecord};

function asString(value: unknown): string | null {
  return typeof value === 'string' && value ? value : null;
}

/**
 * 从 approval_required 帧 payload 解析待签记录。
 * 缺 session 与 approval_id 双主键的帧返回 null（宁丢不猜）。
 */
export function parseApprovalQuickRecord(payload: unknown, at: number): ApprovalQuickRecord | null {
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) return null;
  const p = payload as Record<string, unknown>;
  const session = asString(p.session);
  const approvalId = asString(p.approval_id);
  if (!session && !approvalId) return null;
  return {
    session: session ?? '',
    approvalId: approvalId ?? '',
    toolName: asString(p.tool_name),
    at,
  };
}

/**
 * 权威待签队列按 approval_id 精确匹配（0 装：不按 session 模糊匹配）。
 * 找不到返回 null —— 调用方必须判 stale，不得落决策。
 */
export function selectPendingApproval(
  pending: readonly CanonicalPendingApproval[],
  record: ApprovalQuickRecord,
): CanonicalPendingApproval | null {
  if (!record.approvalId) return null;
  for (const item of pending) {
    if (item.approval_id === record.approvalId) return item;
  }
  return null;
}

/** 面板状态机（纯函数）：帧驱动推进，未知输入零操作。 */
export function reduceApprovalQuick(
  state: ApprovalQuickState,
  event:
    | {type: 'frame'; record: ApprovalQuickRecord}
    | {type: 'resolved'}
    | {type: 'begin-resolve'}
    | {type: 'stale'},
): ApprovalQuickState {
  switch (event.type) {
    case 'frame':
      // 新待签帧覆盖旧记录（同一时刻只挂一张文书，与契约 §6 同源）。
      return {phase: 'asking', record: event.record};
    case 'resolved':
      return {phase: 'none'};
    case 'begin-resolve':
      return state.phase === 'asking' ? {phase: 'resolving', record: state.record} : state;
    case 'stale':
      return state.phase === 'resolving' || state.phase === 'asking'
        ? {phase: 'stale', record: state.record}
        : state;
    default:
      return state;
  }
}
