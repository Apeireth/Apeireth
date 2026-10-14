// 桌宠常驻投影 — 主动涌现引擎（防唠叨纪律的执行者）
//
// 范式: docs/design/00-PHILOSOPHY.md §5 initiative 约定化 —— 他主动开口不是
// 随机事件，而是节奏化的来与离开；主动开口限量（默认 ≤3 次/天），频率写入
// 契约，宁少勿多——陪伴者的话越少，每句越重。
//
// 本引擎只消费**真实信号**（网关事件总线的生命周期事件 + initiative 契约空间
// 帧），绝不生成内容、绝不模拟节奏：
//   - approval_required → 通报气泡「有文书等你签」+ 待签 >90s 抬头看一眼
//     （k3 steering playbook: 不弹通知，抬头即是提醒）；
//   - approval_resolved → 静默收势（不响不闹，防唠叨）；
//   - initiative（契约空间）→ 他开口的气泡，受每日开口预算硬约束；
//   - turn_completed / backend_ready → 不出气泡（回合级事件不配拥有台词）。
//
// 本文件零运行时依赖（不 import svelte / DOM / 计时器），纯 reducer + 时钟
// 注入，Node 可直接 import 测试（tests/pet-proactive.mjs）。

// ============================================================
// 信号（真实来源；kind 与 gateway 契约 §8/§8a 对齐）
// ============================================================

export type ProactiveSignalKind =
  | 'approval_required'
  | 'approval_resolved'
  | 'turn_completed'
  | 'backend_ready'
  /** 契约空间：他从 dreaming_consolidation 苏醒主动开口。v0 后端无生产者。 */
  | 'initiative';

export interface ProactiveSignal {
  kind: ProactiveSignalKind;
  /** epoch ms */
  at: number;
  /** approval_required 携带（工具名，人话展示） */
  toolName?: string | null;
  /** initiative 携带（他要说的话，契约空间） */
  text?: string | null;
}

// ============================================================
// 纪律参数（数值即契约，改数即改口径）
// ============================================================

/** 待签文书久候 → 抬头看一眼的门槛（ms）。k3 playbook: 90s 未处理抬头。 */
export const GLANCE_DELAY_MS = 90_000;

/** 待签持续未处理时重复抬头的最小间隔（ms）——抬头也是话语，限量。 */
export const GLANCE_REPEAT_MS = 300_000;

/** 抬头动作持续时长（ms），过后回到原姿态。 */
export const GLANCE_HOLD_MS = 2_500;

/** 他主动开口的每日预算（次/天）——与后端 presence 预算器同数（≤3 次/天）。 */
export const SPEECH_DAILY_LIMIT = 3;

/** 通报气泡的最小间隔（ms）：审批连环来也不连环响。 */
export const NOTICE_MIN_INTERVAL_MS = 45_000;

/** 同文气泡去重窗口（ms）。 */
export const NOTICE_DEDUPE_WINDOW_MS = 120_000;

/** 同时在场的气泡上限（最旧先退）。 */
export const MAX_ALIVE_BUBBLES = 2;

/** 通报气泡驻留（ms）。 */
export const NOTICE_TTL_MS = 6_000;

/** 他开口的气泡驻留（ms）——话比通报重，多留一会。 */
export const SPEECH_TTL_MS = 12_000;

// ============================================================
// 状态与动作（纯 reducer）
// ============================================================

export interface ProactiveBubble {
  id: string;
  /** notice = 事实通报；speech = 他开口（受每日预算） */
  kind: 'notice' | 'speech';
  text: string;
  at: number;
  /** 到期时刻（epoch ms）；tick 时清理 */
  expiresAt: number;
}

export interface ProactiveState {
  bubbles: ProactiveBubble[];
  /** 未处理的待签文书数（approval_required +1 / resolved −1，不为负） */
  pendingApprovals: number;
  /** 最早一张待签文书的挂起时刻（0 = 无待签） */
  pendingSinceAt: number;
  /** 最近一次抬头时刻（0 = 从未） */
  lastGlanceAt: number;
  /** 抬头动作到期时刻（>now 即正在抬头） */
  glanceUntilAt: number;
  /** 最近一次通报气泡时刻（限频用） */
  lastNoticeAt: number;
  /** 同文去重账本：文本 → 上次出现时刻 */
  noticeSeenAt: Record<string, number>;
  /** 开口预算：自然日序号 + 当日已开口次数 */
  speechDay: number;
  speechUsed: number;
  /** 事件 id 自增（气泡 id 用） */
  seq: number;
}

export type ProactiveAction =
  | {type: 'signal'; signal: ProactiveSignal}
  | {type: 'tick'; at: number}
  | {type: 'dismiss'; id: string}
  | {type: 'reset'; at: number};

/** 自然日序号（本地时区）——每日开口预算的账期。 */
export function dayIndexOf(at: number): number {
  return Math.floor(at / 86_400_000);
}

export function createInitialProactiveState(at: number = 0): ProactiveState {
  return {
    bubbles: [],
    pendingApprovals: 0,
    pendingSinceAt: 0,
    lastGlanceAt: 0,
    glanceUntilAt: 0,
    lastNoticeAt: 0,
    noticeSeenAt: {},
    speechDay: dayIndexOf(at),
    speechUsed: 0,
    seq: 0,
  };
}

export function reduceProactive(state: ProactiveState, action: ProactiveAction): ProactiveState {
  switch (action.type) {
    case 'signal':
      return applySignal(state, action.signal);
    case 'tick':
      return expireBubbles(state, action.at);
    case 'dismiss':
      return {
        ...state,
        bubbles: state.bubbles.filter((bubble) => bubble.id !== action.id),
      };
    case 'reset':
      return createInitialProactiveState(action.at);
  }
}

function nextSeq(state: ProactiveState): number {
  return state.seq + 1;
}

function expireBubbles(state: ProactiveState, at: number): ProactiveState {
  const alive = state.bubbles.filter((bubble) => bubble.expiresAt > at);
  if (alive.length === state.bubbles.length) return state;
  return {...state, bubbles: alive};
}

function applySignal(state: ProactiveState, signal: ProactiveSignal): ProactiveState {
  // 任何信号到达都先清过期气泡：气泡陈述的是当下事实，过期即撤。
  const fresh = expireBubbles(state, signal.at);
  switch (signal.kind) {
    case 'approval_required': {
      const pending = fresh.pendingApprovals + 1;
      const pendingSinceAt = fresh.pendingApprovals === 0 ? signal.at : fresh.pendingSinceAt;
      const tool = (signal.toolName ?? '').trim() || '工具';
      const text = `有文书等你签：${tool}`;
      const base: ProactiveState = {
        ...fresh,
        pendingApprovals: pending,
        pendingSinceAt,
      };
      // 通报限频 + 同文去重；超限静默丢弃（防唠叨：宁可不说）
      if (signal.at - fresh.lastNoticeAt < NOTICE_MIN_INTERVAL_MS) return base;
      const seenAt = fresh.noticeSeenAt[text] ?? 0;
      if (signal.at - seenAt < NOTICE_DEDUPE_WINDOW_MS) return base;
      const seq = nextSeq(base);
      const bubble: ProactiveBubble = {
        id: `notice-${seq}`,
        kind: 'notice',
        text,
        at: signal.at,
        expiresAt: signal.at + NOTICE_TTL_MS,
      };
      const bubbles = [bubble, ...base.bubbles].slice(0, MAX_ALIVE_BUBBLES);
      return {
        ...base,
        bubbles,
        seq,
        lastNoticeAt: signal.at,
        noticeSeenAt: {...fresh.noticeSeenAt, [text]: signal.at},
      };
    }
    case 'approval_resolved': {
      const pending = Math.max(0, fresh.pendingApprovals - 1);
      // 静默收势：不响不闹（防唠叨纪律）。待签清零时「有文书等你签」的
      // 事实已不成立，通报一并撤下——气泡不许陈述过期事实。
      const bubbles =
        pending === 0 ? fresh.bubbles.filter((bubble) => bubble.kind !== 'notice') : fresh.bubbles;
      return {
        ...fresh,
        bubbles,
        pendingApprovals: pending,
        pendingSinceAt: pending === 0 ? 0 : fresh.pendingSinceAt,
      };
    }
    case 'initiative': {
      // 契约空间：v0 后端无生产者；将来有帧时按每日开口预算硬约束。
      const text = (signal.text ?? '').trim();
      if (!text) return fresh;
      const day = dayIndexOf(signal.at);
      const used = day === fresh.speechDay ? fresh.speechUsed : 0;
      if (used >= SPEECH_DAILY_LIMIT) return fresh; // 预算用尽：一个字也不说
      const seq = nextSeq(fresh);
      const bubble: ProactiveBubble = {
        id: `speech-${seq}`,
        kind: 'speech',
        text,
        at: signal.at,
        expiresAt: signal.at + SPEECH_TTL_MS,
      };
      return {
        ...fresh,
        bubbles: [bubble, ...fresh.bubbles].slice(0, MAX_ALIVE_BUBBLES),
        seq,
        speechDay: day,
        speechUsed: used + 1,
      };
    }
    case 'turn_completed':
    case 'backend_ready':
      // 不出气泡：回合级/启动级事件不配拥有台词。backend_ready 视为新起点，
      // 陈旧气泡一并收走（干净的在场比多一句播报诚实）。
      return signal.kind === 'backend_ready'
        ? {...createInitialProactiveState(signal.at), seq: fresh.seq}
        : fresh;
  }
}

// ============================================================
// 抬头看一眼（glance）—— 待签文书久候的非语言提醒
// ============================================================

/**
 * 待签文书 ≥1 张且已挂起 ≥90s 时抬头一次，此后每 5 分钟至多再抬一次；
 * 抬头是瞬时动作（2.5s），不是常驻状态。审批清零即永不抬头。
 */
export function evaluateGlance(state: ProactiveState, at: number): ProactiveState {
  if (state.pendingApprovals <= 0 || state.pendingSinceAt === 0) {
    return state;
  }
  const waited = at - state.pendingSinceAt;
  if (waited < GLANCE_DELAY_MS) return state;
  if (state.lastGlanceAt !== 0 && at - state.lastGlanceAt < GLANCE_REPEAT_MS) return state;
  return {
    ...state,
    lastGlanceAt: at,
    glanceUntilAt: at + GLANCE_HOLD_MS,
  };
}

/** 此刻是否处于抬头动作中（形象层消费）。 */
export function isGlanceActive(state: ProactiveState, at: number): boolean {
  return at < state.glanceUntilAt;
}

// ============================================================
// initiative 帧解析（契约空间，v0 后端无生产者）
// ============================================================

/**
 * 解析 initiative 具名帧（gateway 契约 §8a 预留的独立事件类型）。
 * 形状：{type:'initiative', at:number, text:string}。严格解析：主键缺失或
 * 文本为空 → null（不猜）。当前总线上没有 initiative 生产者——此函数是
 * 接线预留，不谎报现役能力。
 */
export function parseInitiativeFrame(data: string): ProactiveSignal | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(data);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return null;
  const f = parsed as Record<string, unknown>;
  if (f.type !== 'initiative') return null;
  const at = typeof f.at === 'number' && Number.isFinite(f.at) ? f.at : null;
  const text = typeof f.text === 'string' ? f.text.trim() : '';
  if (at === null || !text) return null;
  return {kind: 'initiative', at, text};
}
