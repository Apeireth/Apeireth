// 对话回合遥测纯逻辑（聊天头状态行四项：本轮 token（输入/输出）· 提示缓存命中 ·
// 回合耗时 · 模型名）。
//
// 数据源 = provider 流式回应随包回传的 usage 字段（各协议字段名不同，这里统一
// 归一到 TurnUsage）。诚实占位纪律：没有真实数据的位一律「—」，绝不编数——连 0
// 都不虚构（流里真报了 0 才显示 0）。
//
// 命中率口径：命中率 = 命中 token / 输入 token；输入为 0 或缺 → 率显示「—」
// （不做除法也不编一个 0%）。

/** 无数据占位（诚实「—」，与真实的 0 严格区分）。 */
export const DASH = '—';

/** 一个回合的用量快照（缺省字段 = 该位未知 → 显示「—」）。 */
export interface TurnUsage {
  /** 输入 token（字段兼容 prompt_tokens / input_tokens）。 */
  promptTokens?: number;
  /** 输出 token（字段兼容 completion_tokens / output_tokens）。 */
  completionTokens?: number;
  /** 本轮总 token（total_tokens；缺 = 未知，不做加法虚构）。 */
  totalTokens?: number;
  /**
   * 提示缓存命中的输入 token 数（字段兼容 prompt_cache_hit_tokens /
   * prompt_tokens_details.cached_tokens / cache_read_input_tokens）。
   */
  cacheHitTokens?: number;
  /** 流块随包回传的模型标识（展示用；缺省由上层回落配置里的模型名）。 */
  model?: string;
}

/** 遥测条四项显示串 + hover 详情串。 */
export interface TurnTelemetryView {
  /** 本轮 token 位：「入 1,234 / 出 567」；两位都未知=「—」。 */
  tokens: string;
  /** 提示缓存命中位：「命中 890 · 72%」；命中未知=「—」，率未知率位=「—」。 */
  cache: string;
  /** 回合耗时位：「820ms」/「3.2s」/「1m05s」；未知=「—」。 */
  duration: string;
  /** 模型位：模型名；未知=「—」。 */
  model: string;
  /** hover 详情整串（title 属性）：总 token / 命中（含率与省下口径）/耗时 / 模型。 */
  hover: string;
}

export interface TurnTelemetryInput {
  usage?: TurnUsage | null;
  durationMs?: number | null;
  model?: string | null;
}

function num(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function str(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

/** 千分位展示（不做单位换算，只加组分隔）。 */
function fmtInt(value: number | undefined): string {
  return value === undefined ? DASH : String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/** 命中率串：命中/输入；输入为 0 或缺 → 「—」。 */
function cacheRate(hit: number | undefined, input: number | undefined): string {
  if (hit === undefined || input === undefined || input <= 0) return DASH;
  return `${Math.round((hit / input) * 100)}%`;
}

/** 回合耗时展示串；未知/非法=「—」，真实 0=「0ms」。 */
export function formatDuration(ms: number | null | undefined): string {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms < 0) return DASH;
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const seconds = ms / 1000;
  if (ms < 60_000) return seconds < 10 ? `${seconds.toFixed(1)}s` : `${Math.round(seconds)}s`;
  const minutes = Math.floor(ms / 60_000);
  const rest = Math.round((ms % 60_000) / 1000);
  return `${minutes}m${String(rest).padStart(2, '0')}s`;
}

/**
 * 从一个流式 JSON 块里提取用量快照：usage 对象可能在块顶层、块的 message
 * 事件体里，或两者的 usage 子对象里（协议差异）；字段名按兼容表归一。
 * 本块没有任何遥测信息 → null（不上报，不编数）。
 */
export function parseUsageChunk(json: unknown): TurnUsage | null {
  if (!isRecord(json)) return null;
  const sources: Record<string, unknown>[] = [json];
  if (isRecord(json.usage)) sources.push(json.usage);
  if (isRecord(json.message)) {
    sources.push(json.message);
    if (isRecord(json.message.usage)) sources.push(json.message.usage);
  }
  let merged: TurnUsage | null = null;
  for (const source of sources) merged = mergeUsage(merged, usageFromRecord(source));
  return merged;
}

function usageFromRecord(record: Record<string, unknown>): TurnUsage {
  const details = isRecord(record.prompt_tokens_details) ? record.prompt_tokens_details : undefined;
  return {
    promptTokens: num(record.prompt_tokens ?? record.input_tokens),
    completionTokens: num(record.completion_tokens ?? record.output_tokens),
    totalTokens: num(record.total_tokens),
    cacheHitTokens: num(
      record.prompt_cache_hit_tokens ??
        record.cache_read_input_tokens ??
        (details ? num(details.cached_tokens) : undefined),
    ),
    model: str(record.model),
  };
}

/**
 * 用量合并：逐字段后到非空者胜（null/undefined = 空不覆盖；真实 0 是值，照胜）。
 * 产物只带真有值的字段（稀疏快照）；两侧全空 → null。
 */
export function mergeUsage(
  prev: TurnUsage | null | undefined,
  next: TurnUsage | null | undefined,
): TurnUsage | null {
  const merged: TurnUsage = {
    promptTokens: next?.promptTokens ?? prev?.promptTokens,
    completionTokens: next?.completionTokens ?? prev?.completionTokens,
    totalTokens: next?.totalTokens ?? prev?.totalTokens,
    cacheHitTokens: next?.cacheHitTokens ?? prev?.cacheHitTokens,
    model: next?.model ?? prev?.model,
  };
  for (const key of Object.keys(merged) as Array<keyof TurnUsage>) {
    if (merged[key] === undefined) delete merged[key];
  }
  return Object.keys(merged).length > 0 ? merged : null;
}

/**
 * 遥测条展示：四项显示串 + hover 详情串。未知位诚实「—」；命中率只在输入
 * token 真实存在且 > 0 时给出，否则率位「—」。
 */
export function formatTurnTelemetry(input: TurnTelemetryInput): TurnTelemetryView {
  const usage = input.usage ?? null;
  const prompt = fmtInt(usage?.promptTokens);
  const completion = fmtInt(usage?.completionTokens);
  const tokens = prompt === DASH && completion === DASH ? DASH : `入 ${prompt} / 出 ${completion}`;

  const hit = usage?.cacheHitTokens;
  const rate = cacheRate(hit, usage?.promptTokens);
  const cache = hit === undefined ? DASH : `命中 ${fmtInt(hit)} · ${rate}`;

  const duration = formatDuration(input.durationMs);
  const model = str(input.model) ?? str(usage?.model) ?? DASH;

  const hover = [
    `本轮总 token ${fmtInt(usage?.totalTokens)}`,
    hit === undefined
      ? `提示缓存命中 ${DASH}`
      : `提示缓存命中 ${fmtInt(hit)} token（命中率 ${rate}，即省下 ${fmtInt(hit)} 个输入 token 的重复处理）`,
    `回合耗时 ${duration}`,
    `模型 ${model}`,
  ].join(' · ');

  return {tokens, cache, duration, model, hover};
}
