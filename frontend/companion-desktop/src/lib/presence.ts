// Apeireth 桌面伙伴 — Presence 频道客户端 (presence_state 契约 v0)
//
// 契约: docs/gateway-api-contract.md §8a / docs/design/00-PHILOSOPHY.md §10。
// presence_state 是具名 SSE 帧 (event: presence_state) —— EventSource 的
// onmessage 只收无名帧, 必须 addEventListener('presence_state', …) 才收得到。
//
// 显影分级纪律 (00-PHILOSOPHY §10):
//   - significance=heartbeat (60s 心跳) 只动余烬点 → 只更新 breath 通道;
//   - significance=turn (回合级) 才动光环/姿态 → 更新 current 全量;
//   - significance=ritual 是契约空间, 后端 v0 永不产出 (无诚实触发器) ——
//     若未来出现, 本 store 按 turn 级全量接纳并记录 lastSignificance,
//     仪式表现由消费方读取后自行决定。
//
// 频率纪律: presence_state 是低频事件 (回合级 + 每 60s 心跳), 消费端不得
// 自己发明高频动画源; 两帧之间的中间态用指数趋近插值 (rate 2.4/s,
// motion.stateSmoothing), 插值是真实数据的中间态, 不算模拟, 无需 SIM 标注.
//
// SIM 纪律 (docs/design/01-DESIGN-SYSTEM.md §5.4): 频道断连持续 >30s 时
// store 标记 simulated=true, current 回落本机中性默认值 (PAD 0/0/0),
// 绝不编造情绪.
//
// 考古注记 (2026-09-22 改订): legacy 四事件 (emotion / initiative / dream /
// memory_recall) 的生产点已归入 legacy/donor/apeireth-companion/, canonical
// 总线上不存在; 旧类型定义随本次改订删除, 考古见 git 历史与
// docs/design/01-DESIGN-SYSTEM.md §3.1 v2 更正段。星尘卡 (memory_recall) 的
// 蛰伏断点注释保留在 App.svelte 对话流归并处。
//
// 本文件刻意零运行时依赖 (不 import svelte/store): store 手写 Svelte store
// 契约 (subscribe 返回 unsubscribe), 纯函数与 store 均可被 Node 直接 import
// 测试 (tests/presence-state.mjs), DOM/EventSource/rAF 全部惰性守卫.

import {PRESENCE_HEURISTIC_V0_GAIN} from './scene/tokens.ts';

// ============================================================
// presence_state 帧类型 — 字段名与契约 §8a 逐字一致
// ============================================================

export interface PresencePad {
  p: number;
  a: number;
  d: number;
}

/** 四姿态 (ember_hud_driver.rs EmberCognitiveStance, 线上值为小写下划线串) */
export type PresenceStance =
  | 'deep_coding_focus'
  | 'attentive_presence'
  | 'dreaming_consolidation'
  | 'empathetic_care';

/** 显影分级三档; ritual 为契约空间, 后端 v0 无生产者 */
export type PresenceSignificance = 'heartbeat' | 'turn' | 'ritual';

export interface PresenceBreath {
  period_secs: number;
  amplitude: number;
}

export interface PresenceSource {
  /** v0 恒为 heuristic_v0 —— 诚实标注, 不包装成精确情绪 */
  kind: string;
  confidence: number;
}

/** presence_state 帧 (契约 §8a 形状) */
export interface PresenceStateFrame {
  type: 'presence_state';
  /** epoch 毫秒 */
  at: number;
  pad: PresencePad;
  dominant: string;
  intensity: number;
  stance: PresenceStance;
  breath: PresenceBreath;
  significance: PresenceSignificance;
  source: PresenceSource;
}

const KNOWN_STANCES: ReadonlySet<string> = new Set([
  'deep_coding_focus',
  'attentive_presence',
  'dreaming_consolidation',
  'empathetic_care',
]);

const KNOWN_SIGNIFICANCE: ReadonlySet<string> = new Set(['heartbeat', 'turn', 'ritual']);

function asFiniteNumber(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

/**
 * 解析一条 presence_state 帧的 data JSON。严格但不脆弱:
 * 主键 (type/at/stance/significance) 缺失或不合法 → null (宁可丢弃不猜);
 * 数值字段做防御性钳制; 未知附加字段容忍 (前向兼容)。
 */
export function parsePresenceStateFrame(data: string): PresenceStateFrame | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(data);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return null;
  const f = parsed as Record<string, unknown>;
  if (f.type !== 'presence_state') return null;
  const at = asFiniteNumber(f.at);
  if (at === null) return null;
  const stanceRaw = typeof f.stance === 'string' ? f.stance : null;
  if (!stanceRaw || !KNOWN_STANCES.has(stanceRaw)) return null;
  const sigRaw = typeof f.significance === 'string' ? f.significance : null;
  if (!sigRaw || !KNOWN_SIGNIFICANCE.has(sigRaw)) return null;

  const padRaw = (f.pad ?? {}) as Record<string, unknown>;
  const breathRaw = (f.breath ?? {}) as Record<string, unknown>;
  const sourceRaw = (f.source ?? {}) as Record<string, unknown>;

  return {
    type: 'presence_state',
    at,
    pad: {
      p: clampPadValue(asFiniteNumber(padRaw.p) ?? 0),
      a: clampPadValue(asFiniteNumber(padRaw.a) ?? 0),
      d: clampPadValue(asFiniteNumber(padRaw.d) ?? 0),
    },
    dominant: typeof f.dominant === 'string' ? f.dominant : '',
    intensity: clamp01(asFiniteNumber(f.intensity) ?? 0),
    stance: stanceRaw as PresenceStance,
    breath: {
      // 节律周期缺省回落契约基线 4.0s; 幅度钳到 [0,1]
      period_secs: Math.max(1, asFiniteNumber(breathRaw.period_secs) ?? 4.0),
      amplitude: clamp01(asFiniteNumber(breathRaw.amplitude) ?? 0),
    },
    significance: sigRaw as PresenceSignificance,
    source: {
      kind: typeof sourceRaw.kind === 'string' ? sourceRaw.kind : 'unknown',
      confidence: clamp01(asFiniteNumber(sourceRaw.confidence) ?? 0),
    },
  };
}

// ============================================================
// 纯逻辑: mode 推导 / 平滑插值 / 显影派生 (无 DOM, 可单测)
// ============================================================

export type PresenceMode = 'quiet' | 'thinking' | 'speaking';

export interface PresenceModeInput {
  /** 对话流进行中 (调用方 setSpeaking 告知 —— 本地真实信号) */
  speaking: boolean;
  /** 对话请求进行中 (已发送未流完, 调用方 setChatActive 告知) */
  chatActive: boolean;
}

/**
 * mode 推导: speaking = 对话流进行中; thinking = 对话请求中; 否则 quiet。
 * (v0 契约无 initiative 事件, 旧「spoke 后 5s 内视为 thinking」窗口随
 * legacy 事件一并退役 —— speaking pulse 触发源就是本地流式进行中这一真信号。)
 */
export function derivePresenceMode(input: PresenceModeInput): PresenceMode {
  if (input.speaking) return 'speaking';
  if (input.chatActive) return 'thinking';
  return 'quiet';
}

/** 平滑插值速率 (motion.stateSmoothing ✅ 2.4/s; 契约频率纪律: 不要逐帧跳变) */
export const SMOOTHING_RATE_PER_SEC = 2.4;

/** 指数趋近一步: dtMs 后从 current 向 target 靠近 (速率 ratePerSec, /s) */
export function approachExponential(
  current: number,
  target: number,
  dtMs: number,
  ratePerSec: number = SMOOTHING_RATE_PER_SEC,
): number {
  if (dtMs <= 0) return current;
  return target + (current - target) * Math.exp((-ratePerSec * dtMs) / 1000);
}

function clampPadValue(v: unknown): number {
  const n = typeof v === 'number' && Number.isFinite(v) ? v : 0;
  return Math.min(1, Math.max(-1, n));
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

/** T1 光晕钩子 --presence-glow 的派生 (规范 §5.3: 光晕随 bright 呼吸)。
 *  无真实数据 → 静息微光 0.14 (金线本身不消失, 消失的只是呼吸);
 *  有数据 → 模式基项 (本地真信号) + §4.1 亮度归一 + intensity 项,
 *  PAD/intensity 先经 heuristic_v0 保守增益 (tokens.ts PRESENCE_HEURISTIC_V0_GAIN)。 */
export const PRESENCE_GLOW_REST = 0.14;

export function derivePresenceGlow(current: PresenceCurrent | null): number {
  if (!current) return PRESENCE_GLOW_REST;
  const modeTerm = current.mode === 'speaking' ? 0.18 : current.mode === 'thinking' ? 0.08 : 0;
  const brightNorm = clamp01((current.p * PRESENCE_HEURISTIC_V0_GAIN + 1) / 2);
  const glow = 0.12 + modeTerm + 0.3 * brightNorm + 0.15 * clamp01(current.intensity) * PRESENCE_HEURISTIC_V0_GAIN;
  return Math.min(0.65, Math.max(0.08, glow));
}

/** T0 余烬点呼吸参数; 无真实帧时回落契约基线 (4s / 0.65 ≈ 原静息呼吸) */
export interface EmberBreath {
  periodSecs: number;
  amplitude: number;
}

export const EMBER_REST_BREATH: EmberBreath = {periodSecs: 4.0, amplitude: 0.65};

export function deriveEmberBreath(breath: PresenceBreath | null): EmberBreath {
  if (!breath) return EMBER_REST_BREATH;
  return {periodSecs: Math.max(1, breath.period_secs), amplitude: clamp01(breath.amplitude)};
}

// ============================================================
// presenceStore — Svelte store 契约 (subscribe → unsubscribe)
// ============================================================

/** recentEvents 环形缓冲容量 (最近帧考古/状态面用) */
export const RECENT_EVENTS_CAPACITY = 50;

const PAD_EPSILON = 0.001;

export interface PresenceCurrent {
  p: number;
  a: number;
  d: number;
  /** ∈ [0,1], 回合级帧携带 */
  intensity: number;
  stance: PresenceStance;
  /** 本地真实信号推导 (speaking/thinking/quiet) */
  mode: PresenceMode;
  /** 来源与置信度随帧携带 (v0 恒 heuristic_v0 / 0.5), 供状态面如实展示 */
  sourceKind: string;
  sourceConfidence: number;
  /** 帧时刻 epoch ms */
  at: number;
}

export interface PresenceEventRecord {
  event: PresenceStateFrame;
  /** 本地接收时刻 (ms epoch) */
  receivedAt: number;
}

export interface PresenceState {
  /** turn/ritual 帧驱动 + rAF 平滑插值; 无真实数据时为 null; simulated 时为本机中性默认 */
  current: PresenceCurrent | null;
  /** 余烬点呼吸通道: 每帧都更新 (含 heartbeat —— 显影分级: 心跳只动余烬点) */
  breath: PresenceBreath | null;
  /** 最近一帧的显影分级 (ritual 到达时消费方据此跑仪式; v0 恒非 ritual) */
  lastSignificance: PresenceSignificance | null;
  /** SIM 标注 (设计规范 §5.4): 断连 >30s 无真实来源时为 true */
  simulated: boolean;
  /** 频道当前是否连通 (任一路真实事件到达也会置 true) */
  connected: boolean;
  /** 最近帧环形缓冲, 最新在前, 容量 50 */
  recentEvents: PresenceEventRecord[];
}

export interface PresenceStore {
  subscribe(run: (state: PresenceState) => void): () => void;
  /** 非 Svelte 消费者/测试用快照 */
  get(): PresenceState;
  /** 喂入一帧已解析的 presence_state */
  ingest(frame: PresenceStateFrame): void;
  /** 调用方告知对话流开始/结束 (mode: speaking) */
  setSpeaking(speaking: boolean): void;
  /** 调用方告知对话请求开始/结束 (mode: thinking) */
  setChatActive(active: boolean): void;
  /** 订阅层回报连通状态; 连通会清除 simulated */
  setConnected(connected: boolean): void;
  setSimulated(simulated: boolean): void;
  /** 清空全部状态 (测试用) */
  reset(): void;
}

export function createPresenceStore(nowFn: () => number = () => Date.now()): PresenceStore {
  const subscribers = new Set<(state: PresenceState) => void>();

  let targetPad: PresencePad | null = null;
  let displayedPad: PresencePad | null = null;
  let targetIntensity = 0;
  let displayedIntensity = 0;
  let stance: PresenceStance = 'attentive_presence';
  let sourceKind = 'heuristic_v0';
  let sourceConfidence = 0;
  let frameAt = 0;
  let breath: PresenceBreath | null = null;
  let lastSignificance: PresenceSignificance | null = null;
  let speaking = false;
  let chatActive = false;
  let simulated = false;
  let connected = false;
  let recentEvents: PresenceEventRecord[] = [];
  let lastEventKey = '';

  let rafId: number | null = null;
  let lastFrameAt: number | undefined;
  let visibilityHooked = false;

  function currentMode(): PresenceMode {
    return derivePresenceMode({speaking, chatActive});
  }

  function snapshot(): PresenceState {
    let current: PresenceCurrent | null = null;
    if (simulated) {
      // SIM 纪律: 本机中性默认值, 不编造情绪; sourceKind 显式标注本机回落
      current = {
        p: 0,
        a: 0,
        d: 0,
        intensity: 0,
        stance: 'attentive_presence',
        mode: currentMode(),
        sourceKind: 'local_fallback',
        sourceConfidence: 0,
        at: 0,
      };
    } else if (displayedPad) {
      current = {
        p: displayedPad.p,
        a: displayedPad.a,
        d: displayedPad.d,
        intensity: displayedIntensity,
        stance,
        mode: currentMode(),
        sourceKind,
        sourceConfidence,
        at: frameAt,
      };
    }
    return {
      current,
      breath: simulated ? null : breath,
      lastSignificance,
      simulated,
      connected,
      recentEvents,
    };
  }

  function notify(): void {
    const state = snapshot();
    for (const run of subscribers) run(state);
  }

  function cancelLoop(): void {
    if (rafId !== null && typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(rafId);
    }
    rafId = null;
    lastFrameAt = undefined;
  }

  function hookVisibility(): void {
    if (visibilityHooked || typeof document === 'undefined') return;
    visibilityHooked = true;
    document.addEventListener('visibilitychange', () => {
      // hidden 时停插值循环 (省电); 连接本身不断线 (见 subscribePresence)
      if (document.hidden) {
        cancelLoop();
      } else {
        ensureLoop();
      }
    });
  }

  function converged(): boolean {
    if (!targetPad || !displayedPad) return true;
    return (
      Math.abs(displayedPad.p - targetPad.p) < PAD_EPSILON &&
      Math.abs(displayedPad.a - targetPad.a) < PAD_EPSILON &&
      Math.abs(displayedPad.d - targetPad.d) < PAD_EPSILON &&
      Math.abs(displayedIntensity - targetIntensity) < PAD_EPSILON
    );
  }

  function ensureLoop(): void {
    if (typeof requestAnimationFrame !== 'function') return;
    if (typeof document !== 'undefined' && document.hidden) return;
    if (rafId !== null || !targetPad || !displayedPad) return;
    if (converged()) return;
    hookVisibility();
    rafId = requestAnimationFrame(tick);
  }

  function tick(frameTime: number): void {
    rafId = null;
    if (!targetPad || !displayedPad) return;
    const dt = lastFrameAt === undefined ? 16 : Math.max(0, frameTime - lastFrameAt);
    lastFrameAt = frameTime;
    displayedPad = {
      p: approachExponential(displayedPad.p, targetPad.p, dt),
      a: approachExponential(displayedPad.a, targetPad.a, dt),
      d: approachExponential(displayedPad.d, targetPad.d, dt),
    };
    displayedIntensity = approachExponential(displayedIntensity, targetIntensity, dt);
    if (converged()) {
      displayedPad = {...targetPad};
      displayedIntensity = targetIntensity;
      lastFrameAt = undefined;
      notify();
      return;
    }
    notify();
    ensureLoop();
  }

  function dedupKeyFor(frame: PresenceStateFrame): string {
    // 双订阅挂接下同一帧会到达两次; 后端 at 为帧时刻 epoch 毫秒, 同帧 key 相同.
    return `${frame.type}|${frame.at}|${frame.significance}`;
  }

  function ingest(frame: PresenceStateFrame): void {
    const key = dedupKeyFor(frame);
    if (key === lastEventKey) return;
    lastEventKey = key;

    // 真实事件到达 = 频道有活性
    connected = true;
    simulated = false;

    recentEvents = [{event: frame, receivedAt: nowFn()}, ...recentEvents].slice(0, RECENT_EVENTS_CAPACITY);
    lastSignificance = frame.significance;

    // 显影分级: 每帧都更新余烬点呼吸通道
    breath = {
      period_secs: Math.max(1, frame.breath.period_secs),
      amplitude: clamp01(frame.breath.amplitude),
    };

    if (frame.significance === 'heartbeat') {
      // 心跳帧只动余烬点 —— 光环/姿态保持上一回合级状态, 不重渲场景
      notify();
      return;
    }

    // turn / ritual (契约空间, v0 无生产者): 全量更新光环与姿态目标
    targetPad = {
      p: clampPadValue(frame.pad.p),
      a: clampPadValue(frame.pad.a),
      d: clampPadValue(frame.pad.d),
    };
    targetIntensity = clamp01(frame.intensity);
    stance = frame.stance;
    sourceKind = frame.source.kind;
    sourceConfidence = clamp01(frame.source.confidence);
    frameAt = frame.at;
    if (!displayedPad) {
      // 首条直接落位, 不做无中生有的滑行
      displayedPad = {...targetPad};
      displayedIntensity = targetIntensity;
    }
    ensureLoop();
    notify();
  }

  return {
    subscribe(run) {
      subscribers.add(run);
      run(snapshot());
      return () => {
        subscribers.delete(run);
      };
    },
    get: snapshot,
    ingest,
    setSpeaking(value) {
      if (speaking === value) return;
      speaking = value;
      notify();
    },
    setChatActive(value) {
      if (chatActive === value) return;
      chatActive = value;
      notify();
    },
    setConnected(value) {
      const nextSimulated = value ? false : simulated;
      if (connected === value && simulated === nextSimulated) return;
      connected = value;
      simulated = nextSimulated;
      notify();
    },
    setSimulated(value) {
      if (simulated === value) return;
      simulated = value;
      notify();
    },
    reset() {
      cancelLoop();
      targetPad = null;
      displayedPad = null;
      targetIntensity = 0;
      displayedIntensity = 0;
      stance = 'attentive_presence';
      sourceKind = 'heuristic_v0';
      sourceConfidence = 0;
      frameAt = 0;
      breath = null;
      lastSignificance = null;
      speaking = false;
      chatActive = false;
      simulated = false;
      connected = false;
      recentEvents = [];
      lastEventKey = '';
      notify();
    },
  };
}

/** 全局单例 — 场景层组件消费 `$presenceStore.current` */
export const presenceStore: PresenceStore = createPresenceStore();

// ============================================================
// subscribePresence — EventSource 订阅 + 指数退避重连
// ============================================================

/** 断连持续超过该时长即按 SIM 纪律标记 simulated (设计规范 §5.4) */
export const SIM_AFTER_MS = 30000;

const RETRY_BASE_MS = 2000;
const RETRY_MAX_MS = 30000;

export interface SubscribePresenceOptions {
  /** 测试/特殊场景注入自定义 store; 默认全局 presenceStore */
  store?: PresenceStore;
}

/**
 * 订阅 GET /v1/apeireth/events 上的 presence_state 具名帧并驱动 presenceStore。
 * - 具名帧必须 addEventListener('presence_state', …) —— onmessage 收不到;
 * - 自动重连: 出错即关闭并自建指数退避 (2s ×1.5, 封顶 30s, 连通后复位),
 *   取代 EventSource 内置的固定间隔重试;
 * - 页面 hidden 不断线 — 只停插值 rAF, 不关闭连接;
 * - 断连持续 >30s → store.setSimulated(true) (SIM 纪律);
 * - 低频契约 (回合级 + 60s 心跳): 本订阅不发明任何高频动画源。
 * 注意: 服务端 broadcast 容量 256、落后即断连 — 重连后不假设能补到断线期事件.
 * 返回取消订阅函数.
 */
export function subscribePresence(baseUrl: string, options: SubscribePresenceOptions = {}): () => void {
  const store = options.store ?? presenceStore;
  const url = `${baseUrl.replace(/\/+$/, '')}/v1/apeireth/events`;
  // Caller must capability-gate (activity.sse).

  let active = true;
  let source: EventSource | null = null;
  let retryDelay = RETRY_BASE_MS;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;
  let simTimer: ReturnType<typeof setTimeout> | null = null;

  function clearTimers(): void {
    if (retryTimer !== null) {
      clearTimeout(retryTimer);
      retryTimer = null;
    }
    if (simTimer !== null) {
      clearTimeout(simTimer);
      simTimer = null;
    }
  }

  function armSimTimer(): void {
    if (simTimer !== null) return;
    simTimer = setTimeout(() => {
      simTimer = null;
      if (active) store.setSimulated(true);
    }, SIM_AFTER_MS);
  }

  function connect(): void {
    if (!active) return;
    const es = new EventSource(url);
    source = es;

    es.onopen = () => {
      retryDelay = RETRY_BASE_MS;
      if (simTimer !== null) {
        clearTimeout(simTimer);
        simTimer = null;
      }
      store.setConnected(true);
    };

    es.addEventListener('presence_state', (msg) => {
      const data = typeof (msg as MessageEvent).data === 'string' ? (msg as MessageEvent).data : '';
      if (!data) return;
      const frame = parsePresenceStateFrame(data);
      if (frame) store.ingest(frame);
    });

    es.onerror = () => {
      if (source === es) source = null;
      es.close();
      if (!active) return;
      store.setConnected(false);
      armSimTimer();
      retryTimer = setTimeout(connect, retryDelay);
      retryDelay = Math.min(retryDelay * 1.5, RETRY_MAX_MS);
    };
  }

  connect();

  return () => {
    active = false;
    clearTimers();
    source?.close();
    source = null;
    store.setConnected(false);
  };
}
