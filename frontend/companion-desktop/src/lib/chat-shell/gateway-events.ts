// T0 聊天壳骨骼 — 网关事件总线（GET /v1/apeireth/events）归一化纯逻辑
//
// 契约: docs/gateway-api-contract.md §8/§8a；后端生产点
// crates/adapters/gateway/src/events.rs（approval_required/approval_resolved
// 帧 payload 逐字段核实: session/request/trace_id/approval_id/capability_id/
// tool_name/tool_call_id 与 session/trace_id/approval_id/decision/round）。
//
// 用途:
// ① 审批事件归一化 —— SSE approval_required 实时把「待签文书」推入对话
//    （00-PHILOSOPHY §3.2 动作收敛为卡片），并按 session 归属在会话列表
//    主页打「待签」标记；approval_resolved 清除。
// ② 回合级治理通报分流 —— 守卫通报卡（琥珀）只接真实信号：回合被治理面
//    拦停（评审驳回 / 权限否决）时在发生处呈现。canonical 总线当前没有
//    guard_* 事件类型（§8 事件清单），守卫的实时通报面是断点——此处只做
//    回合错误对象的归一化，不轮询、不伪造节奏。
//
// 本文件刻意零运行时依赖（不 import svelte / DOM），纯函数可被 Node 直接
// import 测试（tests/chat-shell.mjs），与 presence.ts 同一约定。

/** approval_required / approval_resolved 帧的归一化结果。 */
export interface ApprovalEventInfo {
  session: string | null;
  approvalId: string | null;
  toolName: string | null;
  /** 仅 approval_resolved 携带。 */
  decision: 'approve' | 'reject' | 'cancel' | null;
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value ? value : null;
}

/**
 * 归一化 approval_required / approval_resolved 的 SSE data payload。
 * 非对象 / 缺 session 与 approval_id 的帧返回 null（容忍前向兼容的字段增删，
 * 但两个主键都没有的帧无法路由，宁可丢弃也不猜）。
 */
export function parseApprovalEventPayload(payload: unknown): ApprovalEventInfo | null {
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) return null;
  const p = payload as Record<string, unknown>;
  const session = asString(p.session);
  const approvalId = asString(p.approval_id);
  if (!session && !approvalId) return null;
  const decisionRaw = asString(p.decision);
  const decision =
    decisionRaw === 'approve' || decisionRaw === 'reject' || decisionRaw === 'cancel'
      ? decisionRaw
      : null;
  return {
    session,
    approvalId,
    toolName: asString(p.tool_name),
    decision,
  };
}

/** 由 SSE 流推导「哪几个会话有他停下等签的文书」的账本（纯函数，可测）。 */
export function applyApprovalEventToPending(
  pending: ReadonlySet<string>,
  event: ApprovalEventInfo,
  kind: 'approval_required' | 'approval_resolved',
): Set<string> {
  const next = new Set(pending);
  if (kind === 'approval_required') {
    if (event.session) next.add(event.session);
  } else {
    // resolved：按 session 清除（该会话同一时刻只挂一张待签文书——
    // 契约 §6 resolve 后继续原回合，新审批会以新的 approval_required 再推）。
    if (event.session) next.delete(event.session);
  }
  return next;
}

// ============================================================
// 回合级治理通报分流（守卫通报卡的真实信号源）
// ============================================================

/**
 * 诚实断点（0 装，2026-10-13 逐字段核实 crates/adapters/gateway/src/
 * error_codes.rs 错误帧目录）：canonical 错误帧没有 guard 专属 code——
 * 守卫否决当前只经由审批流（approval_required → 待签文书卡）或
 * tool_failed（工具卡红边）上浮，没有独立的实时通报信号；events 总线
 * （§8）也没有 guard_* 事件类型。因此守卫通报卡只接一种核实过的真实
 * 信号：`review_rejected`（认知评审驳回，error_codes.rs:21）。
 * 未来 guard 否决获得专属错误码 / 总线事件后，在此扩展匹配分支即可，
 * 卡片组件（GuardNoticeCard.svelte）形状不变。
 */
export type GovernanceNoticeKind = 'review_rejected';

export interface GovernanceNotice {
  kind: GovernanceNoticeKind;
  /** 卡片标题（人话，写在发生处）。 */
  title: string;
}

/**
 * 把回合错误对象分流为守卫通报（琥珀卡）或普通错误（红 banner）。
 * 输入是 App 持有的原始 caught（可能是 Error / RuntimeError / 字符串）。
 * 只认后端错误帧里核实存在的 code；其余一律 null —— 普通错误走
 * ErrorSolutionBanner，不冒充守卫通报。
 */
export function classifyGovernanceNotice(caught: unknown): GovernanceNotice | null {
  if (caught === null || caught === undefined) return null;
  const obj = caught as Record<string, unknown>;
  const backendCode = typeof obj.backendCode === 'string' ? obj.backendCode : null;
  if (backendCode === 'review_rejected') {
    return {kind: 'review_rejected', title: '守卫通报：本轮回答被认知评审驳回'};
  }
  return null;
}
