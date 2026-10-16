// 星语 · 语音面纯逻辑（faces.ts 星语，零依赖纯模块）
//
// 契约（I-starport-builtins.md「星语」）：
//   1. 状态机即承诺 —— idle → requesting → recording → stopped（+ error 态），
//      合法迁移表之外一律抛错（0 装：不猜不兜底）；未知状态/动作抛
//      invalid-argument，与「合法表内的非法迁移」分开记账；
//   2. 电平数学纯函数 —— rmsLevel 均方根（空输入抛错、非有限样本抛错）、
//      smoothLevel 指数平滑（factor ∈ [0,1] 校验）、peakHold 滑窗峰值
//      （窗口必须正整数）；
//   3. 录音账本 —— VoiceNote 增删列表，时长/大小必须正有限值，id 空/冲突
//      一律抛错；读出快照副本防篡改；
//   4. formatDuration 毫秒 → mm:ss（向下取整不虚报，负数/非有限抛错）。
//   5. 合成面（TTS，批次推进）—— 合成状态机 / 合成参数校验 / 语句账本，
//      与录音面共用 VoiceStoreError 码表（契约见文末「合成面」一节）。
//
// 测试：tests/starport-voice.mjs（真实模块直测）。

/** 语音面状态（状态机承诺面，五态封顶不自造第六态）。 */
export type VoiceState = 'idle' | 'requesting' | 'recording' | 'stopped' | 'error';

/** 状态机动作（真实事件归约面：申请/授权/拒绝/停止/复位/失败）。 */
export type VoiceAction = 'request' | 'grant' | 'deny' | 'stop' | 'reset' | 'fail';

export const VOICE_STATES: readonly VoiceState[] = [
  'idle',
  'requesting',
  'recording',
  'stopped',
  'error',
];

export const VOICE_ACTIONS: readonly VoiceAction[] = [
  'request',
  'grant',
  'deny',
  'stop',
  'reset',
  'fail',
];

const LEGAL_TRANSITIONS: Readonly<Record<VoiceState, Partial<Record<VoiceAction, VoiceState>>>> = {
  idle: {request: 'requesting'},
  requesting: {grant: 'recording', deny: 'error', fail: 'error'},
  recording: {stop: 'stopped', fail: 'error'},
  stopped: {reset: 'idle', request: 'requesting'},
  error: {reset: 'idle', request: 'requesting'},
};

export class VoiceStoreError extends Error {
  readonly code: 'illegal-transition' | 'invalid-argument' | 'empty-input' | 'duplicate-id' | 'missing';

  constructor(
    code: 'illegal-transition' | 'invalid-argument' | 'empty-input' | 'duplicate-id' | 'missing',
    message: string,
  ) {
    super(message);
    this.name = 'VoiceStoreError';
    this.code = code;
  }
}

/**
 * 状态迁移（纯函数）：合法表之外一律抛错。
 * - 未知状态 / 未知动作 → invalid-argument（调用方契约破坏）；
 * - 合法状态上的非法动作 → illegal-transition（迁移表即承诺）。
 */
export function transitionVoiceState(state: VoiceState, action: VoiceAction): VoiceState {
  if (!VOICE_STATES.includes(state)) {
    throw new VoiceStoreError('invalid-argument', `未知状态：${String(state)}`);
  }
  if (!VOICE_ACTIONS.includes(action)) {
    throw new VoiceStoreError('invalid-argument', `未知动作：${String(action)}`);
  }
  const table = LEGAL_TRANSITIONS[state];
  const next = table[action];
  if (!next) {
    throw new VoiceStoreError('illegal-transition', `非法迁移：${state} + ${action}`);
  }
  return next;
}

/** 均方根电平。空输入/非 Float32Array/含非有限样本一律抛错（0 装）。 */
export function rmsLevel(samples: Float32Array): number {
  if (!(samples instanceof Float32Array)) {
    throw new VoiceStoreError('invalid-argument', 'samples 必须是 Float32Array');
  }
  if (samples.length === 0) {
    throw new VoiceStoreError('empty-input', '空样本无电平可算');
  }
  let sum = 0;
  for (const x of samples) {
    if (!Number.isFinite(x)) {
      throw new VoiceStoreError('invalid-argument', '样本含非有限值');
    }
    sum += x * x;
  }
  return Math.sqrt(sum / samples.length);
}

/** 指数平滑电平：prev + (next - prev) * factor。factor ∈ [0,1]，值必须有限。 */
export function smoothLevel(prev: number, next: number, factor: number): number {
  if (!Number.isFinite(prev) || !Number.isFinite(next)) {
    throw new VoiceStoreError('invalid-argument', '电平值必须有限');
  }
  if (!Number.isFinite(factor) || factor < 0 || factor > 1) {
    throw new VoiceStoreError('invalid-argument', 'factor 必须在 [0,1]');
  }
  return prev + (next - prev) * factor;
}

/**
 * 滑窗峰值（尾窗）：result[i] = max(levels[max(0, i-window+1) .. i])。
 * window 必须正整数；电平含非有限值抛错；空输入返回空表。
 */
export function peakHold(levels: readonly number[] | Float32Array, window: number): number[] {
  if (!Number.isInteger(window) || window < 1) {
    throw new VoiceStoreError('invalid-argument', 'window 必须是正整数');
  }
  const out: number[] = [];
  for (let i = 0; i < levels.length; i += 1) {
    const current = levels[i];
    if (!Number.isFinite(current)) {
      throw new VoiceStoreError('invalid-argument', '电平含非有限值');
    }
    let peak = current;
    for (let j = i - window + 1; j < i; j += 1) {
      if (j < 0) continue;
      const candidate = levels[j];
      if (candidate > peak) peak = candidate;
    }
    out.push(peak);
  }
  return out;
}

/** 录音账本条目（时长/大小为正有限值，命名即承诺）。 */
export interface VoiceNote {
  id: string;
  at: number;
  durationMs: number;
  sizeBytes: number;
  label: string;
}

/** 录音账本：增删列表（插入序），读出快照副本防篡改。 */
export class VoiceLedger {
  private notes: VoiceNote[] = [];

  add(input: {
    id: string;
    at: number;
    durationMs: number;
    sizeBytes: number;
    label?: string;
  }): VoiceNote {
    const id = typeof input.id === 'string' ? input.id.trim() : '';
    if (!id) {
      throw new VoiceStoreError('invalid-argument', '录音条目必须带非空 id');
    }
    if (this.notes.some((note) => note.id === id)) {
      throw new VoiceStoreError('duplicate-id', `录音 id 冲突：${id}`);
    }
    if (!Number.isFinite(input.at)) {
      throw new VoiceStoreError('invalid-argument', 'at 必须有限');
    }
    if (!Number.isFinite(input.durationMs) || input.durationMs <= 0) {
      throw new VoiceStoreError('invalid-argument', '时长必须正有限值');
    }
    if (!Number.isFinite(input.sizeBytes) || input.sizeBytes <= 0) {
      throw new VoiceStoreError('invalid-argument', '大小必须正有限值');
    }
    const label = input.label === undefined ? '' : input.label;
    if (typeof label !== 'string') {
      throw new VoiceStoreError('invalid-argument', 'label 必须是字符串');
    }
    const note: VoiceNote = {
      id,
      at: input.at,
      durationMs: input.durationMs,
      sizeBytes: input.sizeBytes,
      label,
    };
    this.notes.push(note);
    return {...note};
  }

  remove(id: string): VoiceNote {
    const hit = this.notes.find((note) => note.id === id);
    if (!hit) {
      throw new VoiceStoreError('missing', `录音不存在：${id}`);
    }
    this.notes = this.notes.filter((note) => note.id !== id);
    return {...hit};
  }

  /** 插入序列表（快照副本；显示倒序由视图层自理）。 */
  list(): VoiceNote[] {
    return this.notes.map((note) => ({...note}));
  }
}

/** 毫秒 → mm:ss（向下取整不虚报；负数/非有限抛错）。 */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms)) {
    throw new VoiceStoreError('invalid-argument', '毫秒必须有限');
  }
  if (ms < 0) {
    throw new VoiceStoreError('invalid-argument', '时长不能为负');
  }
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

// ===========================================================================
// 合成面（TTS 真合成的纯件侧，批次推进）
//
// 契约：
//   1. 合成状态机即承诺 —— idle → speaking → done/error（四态封顶不自造
//      第五态）；迁移表之外一律抛错，未知状态/动作单列 invalid-argument
//      （与录音面同一口径）；
//   2. reset 语义 —— speaking + reset = 主动停止/取消（回 idle）；done/
//      error + reset = 复位（回 idle）；起读 speak 只从 idle/done/error；
//   3. 合成参数规范化 —— buildSpeechUtterance 文本去空白后非空（空抛
//      empty-input、非字符串抛 invalid-argument）、rate/pitch ∈ [0.1, 3]
//      （缺省 1.0，越界/非有限抛错），返回规范对象（0 装：不夹紧不猜）；
//   4. 语句账本 —— SpeechJob 增删列表，text 非空、at 有限、可选的
//      durationEstimateMs 必须正有限值；读出快照副本防篡改。
// ===========================================================================

/** 合成状态（四态封顶）。 */
export type SpeechState = 'idle' | 'speaking' | 'done' | 'error';

/** 合成动作：speak=起读 / finish=自然读完 / fail=引擎失败 / reset=停止或复位。 */
export type SpeechAction = 'speak' | 'finish' | 'fail' | 'reset';

export const SPEECH_STATES: readonly SpeechState[] = ['idle', 'speaking', 'done', 'error'];

export const SPEECH_ACTIONS: readonly SpeechAction[] = ['speak', 'finish', 'fail', 'reset'];

const SPEECH_TRANSITIONS: Readonly<Record<SpeechState, Partial<Record<SpeechAction, SpeechState>>>> = {
  idle: {speak: 'speaking'},
  speaking: {finish: 'done', fail: 'error', reset: 'idle'},
  done: {speak: 'speaking', reset: 'idle'},
  error: {speak: 'speaking', reset: 'idle'},
};

/** 合成参数规范对象（文本已去空白；rate/pitch 已过区间校验）。 */
export interface SpeechUtteranceParams {
  text: string;
  rate: number;
  pitch: number;
}

/** 合成语句账本条目（时长估算可选，给了就必须正有限值）。 */
export interface SpeechJob {
  id: string;
  text: string;
  at: number;
  durationEstimateMs?: number;
}

/** 合成状态迁移（纯函数）：口径与 transitionVoiceState 完全一致。 */
export function transitionSpeechState(state: SpeechState, action: SpeechAction): SpeechState {
  if (!SPEECH_STATES.includes(state)) {
    throw new VoiceStoreError('invalid-argument', `未知状态：${String(state)}`);
  }
  if (!SPEECH_ACTIONS.includes(action)) {
    throw new VoiceStoreError('invalid-argument', `未知动作：${String(action)}`);
  }
  const table = SPEECH_TRANSITIONS[state];
  const next = table[action];
  if (!next) {
    throw new VoiceStoreError('illegal-transition', `非法迁移：${state} + ${action}`);
  }
  return next;
}

/** 语速/音高合法区间（系统语音引擎口径）。 */
export const SPEECH_RATE_RANGE: readonly [number, number] = [0.1, 3];

/** 合成参数规范化与校验（0 装：越界抛错不夹紧）。 */
export function buildSpeechUtterance(
  text: string,
  options: {rate?: number; pitch?: number} = {},
): SpeechUtteranceParams {
  if (typeof text !== 'string') {
    throw new VoiceStoreError('invalid-argument', 'text 必须是字符串');
  }
  const trimmed = text.trim();
  if (!trimmed) {
    throw new VoiceStoreError('empty-input', '朗读文本不能为空');
  }
  const check = (value: number | undefined, fallback: number, field: string): number => {
    if (value === undefined) return fallback;
    if (typeof value !== 'number' || !Number.isFinite(value)) {
      throw new VoiceStoreError('invalid-argument', `${field} 必须是有限数值`);
    }
    const [low, high] = SPEECH_RATE_RANGE;
    if (value < low || value > high) {
      throw new VoiceStoreError('invalid-argument', `${field} 必须在 [${low}, ${high}]`);
    }
    return value;
  };
  return {
    text: trimmed,
    rate: check(options.rate, 1, 'rate'),
    pitch: check(options.pitch, 1, 'pitch'),
  };
}

/** 合成语句账本：增删列表（插入序），读出快照副本防篡改。 */
export class SpeechJobLedger {
  private jobs: SpeechJob[] = [];

  add(input: {
    id: string;
    text: string;
    at: number;
    durationEstimateMs?: number;
  }): SpeechJob {
    const id = typeof input.id === 'string' ? input.id.trim() : '';
    if (!id) {
      throw new VoiceStoreError('invalid-argument', '语句条目必须带非空 id');
    }
    if (this.jobs.some((job) => job.id === id)) {
      throw new VoiceStoreError('duplicate-id', `语句 id 冲突：${id}`);
    }
    if (typeof input.text !== 'string' || !input.text.trim()) {
      throw new VoiceStoreError('empty-input', '语句文本不能为空');
    }
    if (!Number.isFinite(input.at)) {
      throw new VoiceStoreError('invalid-argument', 'at 必须有限');
    }
    if (
      input.durationEstimateMs !== undefined &&
      (!Number.isFinite(input.durationEstimateMs) || input.durationEstimateMs <= 0)
    ) {
      throw new VoiceStoreError('invalid-argument', '时长估算必须是正有限值');
    }
    const job: SpeechJob = {
      id,
      text: input.text,
      at: input.at,
      ...(input.durationEstimateMs === undefined
        ? {}
        : {durationEstimateMs: input.durationEstimateMs}),
    };
    this.jobs.push(job);
    return {...job};
  }

  remove(id: string): SpeechJob {
    const hit = this.jobs.find((job) => job.id === id);
    if (!hit) {
      throw new VoiceStoreError('missing', `语句不存在：${id}`);
    }
    this.jobs = this.jobs.filter((job) => job.id !== id);
    return {...hit};
  }

  /** 插入序列表（快照副本）。 */
  list(): SpeechJob[] {
    return this.jobs.map((job) => ({...job}));
  }
}

// ===========================================================================
// 转写面（批次推进）—— 转写状态机 / 结果账本
//
// 契约：
//   1. 转写状态机 —— idle → recognizing → done/error（+ reset 回路），合法
//      迁移表之外一律抛错（0 装）；未知状态/动作抛 invalid-argument；
//   2. 结果账本 —— TranscriptRecord 增删列表，id 非空、text 非空（去空白）、
//      at 有限、confidence 可选但给了必须是 [0,1] 有限值；快照副本防篡改。
// ===========================================================================

/** 转写状态（四态封顶）。 */
export type TranscriptState = 'idle' | 'recognizing' | 'done' | 'error';

/** 转写动作：start=起听 / finish=识别定稿 / fail=引擎失败 / reset=取消复位。 */
export type TranscriptAction = 'start' | 'finish' | 'fail' | 'reset';

export const TRANSCRIPT_STATES: readonly TranscriptState[] = [
  'idle',
  'recognizing',
  'done',
  'error',
];

export const TRANSCRIPT_ACTIONS: readonly TranscriptAction[] = ['start', 'finish', 'fail', 'reset'];

const TRANSCRIPT_TRANSITIONS: Readonly<
  Record<TranscriptState, Partial<Record<TranscriptAction, TranscriptState>>>
> = {
  idle: {start: 'recognizing'},
  recognizing: {finish: 'done', fail: 'error', reset: 'idle'},
  done: {start: 'recognizing', reset: 'idle'},
  error: {start: 'recognizing', reset: 'idle'},
};

/** 转写结果账本条目（confidence 可选：引擎给了才记，不自造）。 */
export interface TranscriptRecord {
  id: string;
  at: number;
  text: string;
  confidence?: number;
}

/** 转写状态迁移（纯函数）：口径与 transitionSpeechState 完全一致。 */
export function transitionTranscriptState(
  state: TranscriptState,
  action: TranscriptAction,
): TranscriptState {
  if (!TRANSCRIPT_STATES.includes(state)) {
    throw new VoiceStoreError('invalid-argument', `未知状态：${String(state)}`);
  }
  if (!TRANSCRIPT_ACTIONS.includes(action)) {
    throw new VoiceStoreError('invalid-argument', `未知动作：${String(action)}`);
  }
  const next = TRANSCRIPT_TRANSITIONS[state][action];
  if (!next) {
    throw new VoiceStoreError('illegal-transition', `非法迁移：${state} + ${action}`);
  }
  return next;
}

/** 转写结果账本：增删列表（插入序），快照副本防篡改。 */
export class TranscriptLedger {
  private records: TranscriptRecord[] = [];

  add(input: {id: string; at: number; text: string; confidence?: number}): TranscriptRecord {
    const id = typeof input.id === 'string' ? input.id.trim() : '';
    if (!id) {
      throw new VoiceStoreError('invalid-argument', '转写条目必须带非空 id');
    }
    if (this.records.some((record) => record.id === id)) {
      throw new VoiceStoreError('duplicate-id', `转写 id 冲突：${id}`);
    }
    if (typeof input.text !== 'string' || !input.text.trim()) {
      throw new VoiceStoreError('empty-input', '转写文本不能为空');
    }
    if (!Number.isFinite(input.at)) {
      throw new VoiceStoreError('invalid-argument', 'at 必须有限');
    }
    if (
      input.confidence !== undefined &&
      (!Number.isFinite(input.confidence) || input.confidence < 0 || input.confidence > 1)
    ) {
      throw new VoiceStoreError('invalid-argument', 'confidence 必须是 [0,1] 有限值');
    }
    const record: TranscriptRecord = {
      id,
      at: input.at,
      text: input.text.trim(),
      ...(input.confidence === undefined ? {} : {confidence: input.confidence}),
    };
    this.records.push(record);
    return {...record};
  }

  remove(id: string): TranscriptRecord {
    const hit = this.records.find((record) => record.id === id);
    if (!hit) {
      throw new VoiceStoreError('missing', `转写不存在：${id}`);
    }
    this.records = this.records.filter((record) => record.id !== id);
    return {...hit};
  }

  /** 插入序列表（快照副本）。 */
  list(): TranscriptRecord[] {
    return this.records.map((record) => ({...record}));
  }
}
