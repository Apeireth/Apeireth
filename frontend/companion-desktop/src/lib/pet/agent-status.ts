// 伴星 · 智能体状态机（内置应用面 I 批 1 轨 B，纯逻辑零依赖）
//
// 契约（I-starport-builtins.md）：
//   1. 五态词汇是承诺面：干活 / 等签 / 待审阅 / 收工 / 出错（+ 空闲基态）；
//   2. 真实源表（0 装：无产点的状态不制造）：
//      - 等签 awaiting_approval  ← approval_required 网关帧（真实，含工具名）
//      - 干活 working           ← approval_resolved 网关帧（契约 §6：解析后
//                                  继续原回合）；回合内活动信号后续接入
//      - 收工 done              ← turn_completed 信号（桌宠信号词汇既有）
//      - 待审阅 awaiting_review ← 预留：回合生命周期总线尚无此产点（诚实断点）
//      - 出错 error             ← 预留：canonical 总线无回合失败帧给桌宠
//                                  （gateway-events.ts §守卫分流同源断点）
//   3. 衰减即诚实：干活/收工无信号即回落空闲（90s/30s），等签不衰减——
//      他停下等签是真的在等，直到 approval_resolved 真实到来；
//   4. 未知信号种类一律无操作（不猜不改写状态）。
//
// 测试：tests/pet-agent-status.mjs（真实模块直测）。

export type AgentStatus =
  | 'idle'
  | 'working'
  | 'awaiting_approval'
  | 'awaiting_review'
  | 'done'
  | 'error';

export interface AgentStatusState {
  status: AgentStatus;
  /** 当前状态确立时刻（毫秒）。 */
  since: number;
  /** 等签文书的工具名（仅 awaiting_approval 携带，其余态恒 null）。 */
  pendingTool: string | null;
}

export interface AgentStatusSignal {
  kind: string;
  at: number;
  toolName?: string | null;
  decision?: 'approve' | 'reject' | 'cancel' | null;
}

/** 干活态无信号回落空闲的时限。 */
export const WORKING_DECAY_MS = 90_000;
/** 收工态展示后回落空闲的时限。 */
export const DONE_DECAY_MS = 30_000;

export function createInitialAgentStatus(now = 0): AgentStatusState {
  return {status: 'idle', since: now, pendingTool: null};
}

/**
 * 归约一个真实信号。未知种类返回原状态（同引用，0 装：不猜）。
 * 乱序信号（at 早于 since）同样无操作——不回拨状态史。
 */
export function reduceAgentStatus(
  state: AgentStatusState,
  signal: AgentStatusSignal,
): AgentStatusState {
  if (!Number.isFinite(signal.at) || signal.at < state.since) return state;
  switch (signal.kind) {
    case 'approval_required':
      return {
        status: 'awaiting_approval',
        since: signal.at,
        pendingTool: typeof signal.toolName === 'string' && signal.toolName ? signal.toolName : null,
      };
    case 'approval_resolved':
      // 契约 §6：resolve 后继续原回合——回到干活态。
      return {status: 'working', since: signal.at, pendingTool: null};
    case 'turn_completed':
      return {status: 'done', since: signal.at, pendingTool: null};
    default:
      // backend_ready / initiative 等：真实但不是工作状态信号；预留态
      // （awaiting_review / error）无产点，一律不制造。
      return state;
  }
}

/** 时间推进的衰减拍（桌宠 1s 记账拍调用；纯函数）。 */
export function decayAgentStatus(state: AgentStatusState, now: number): AgentStatusState {
  const age = now - state.since;
  if (state.status === 'working' && age >= WORKING_DECAY_MS) {
    return {status: 'idle', since: now, pendingTool: null};
  }
  if (state.status === 'done' && age >= DONE_DECAY_MS) {
    return {status: 'idle', since: now, pendingTool: null};
  }
  return state;
}

/** 展示层只外显有真实源的状态（预留态不亮灯）。 */
export function isStatusDisplayable(status: AgentStatus): boolean {
  return status === 'working' || status === 'awaiting_approval' || status === 'done';
}
