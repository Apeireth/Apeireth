// 运行指标面板（D9 上屏）纯逻辑件：`GET /v1/panel/metrics` 的合同类型
// （frontend-data-contract §10.1，schema_version: 1）、响应信封归一、
// 「—」语义展示映射、30 秒自动刷新控制器与取数面组装/解析。
//
// 合同纪律（照契约，不粉饰）：
//   1. 分布 `{count,mean,p50,p95,sum}` 空窗口（count: 0）时四个数值列显式
//      null；缓存命中/计价列为可选列（null = 未知）。上屏一律「未知 = —」，
//      未知不用 0 冒充；计数位（成功/失败/待批）是真实 0 时照显 0。
//   2. `cache_hit_rate` 合同给 0.0–1.0 原值，上屏百分比由本消费端换算。
//   3. 留存恒报 `in_process_rolling`（进程内滚动、不落盘），界面如实转述。
//   4. 信封不合契约（缺字段/类型不符/版本不认识）→ 显式报错，不降级编数。
//
// 本文件零运行时依赖（不 import svelte / DOM 专属 API），纯函数可被 Node
// 直接 import 测试（tests/metrics-panel.mjs）。取数面只组装请求与解析响应：
// 真正的 fetch 与鉴权头（tauri-bridge `gatewayAuthHeaders` 单源）由调用侧
// 注入，令牌面不在此处开第二源。
//
// 合同出处：`crates/adapters/gateway/src/panel_metrics.rs`（DTO 全表 +
// 路由）；`docs/02-guides/frontend-data-contract.md` §10.1。

import {DASH} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 合同类型（镜像 `panel_metrics.rs` 的信封 DTO，字段改名须同步契约文档）
// ---------------------------------------------------------------------------

/** 端点合同版本（与 Rust `METRICS_SCHEMA_VERSION` 同源）。 */
export const METRICS_SCHEMA_VERSION = 1;

/** 留存策略标识（与 Rust `METRICS_RETENTION` 同源：进程内滚动、不落盘）。 */
export const METRICS_RETENTION = 'in_process_rolling';

/** 分布 `{count,mean,p50,p95,sum}`；空窗口（count: 0）四数值列显式 null。 */
export interface MetricsDistribution {
  count: number;
  mean: number | null;
  p50: number | null;
  p95: number | null;
  sum: number | null;
}

/** 一层（全局或会话）的窗口聚合。 */
export interface MetricsAggregate {
  turns: number;
  succeeded: number;
  failed: number;
  pending_approval: number;
  latency_ms: MetricsDistribution;
  input_tokens: MetricsDistribution;
  output_tokens: MetricsDistribution;
  rounds: MetricsDistribution;
  tool_calls: MetricsDistribution;
  /** 缓存命中累计；null = 计量源无缓存列（未知 → 「—」）。 */
  cache_hit_tokens: number | null;
  /** 缓存命中率（0.0–1.0）；null = 命中未知或输入为 0。 */
  cache_hit_rate: number | null;
  /** 成本累计（微单位）；null = 未计价。 */
  cost_micros: number | null;
}

/** 会话层聚合条目。 */
export interface MetricsSessionAggregate {
  session_id: string;
  aggregate: MetricsAggregate;
}

/** `GET /v1/panel/metrics` 响应信封。 */
export interface PanelMetricsSnapshot {
  schema_version: number;
  window_turns: number;
  retention: string;
  global: MetricsAggregate;
  sessions: MetricsSessionAggregate[];
}

// ---------------------------------------------------------------------------
// 信封归一（不合契约 = null，绝不编数）
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asCount(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
}

function asOptionalNumber(value: unknown): number | null | undefined {
  if (value === null) return null;
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  return undefined;
}

function normalizeDistribution(raw: unknown): MetricsDistribution | null {
  if (!isRecord(raw)) return null;
  const count = asCount(raw.count);
  if (count === null) return null;
  const mean = asOptionalNumber(raw.mean);
  const p50 = asOptionalNumber(raw.p50);
  const p95 = asOptionalNumber(raw.p95);
  const sum = asOptionalNumber(raw.sum);
  if (mean === undefined || p50 === undefined || p95 === undefined || sum === undefined) return null;
  return {count, mean, p50, p95, sum};
}

function normalizeAggregate(raw: unknown): MetricsAggregate | null {
  if (!isRecord(raw)) return null;
  const turns = asCount(raw.turns);
  const succeeded = asCount(raw.succeeded);
  const failed = asCount(raw.failed);
  const pendingApproval = asCount(raw.pending_approval);
  if (turns === null || succeeded === null || failed === null || pendingApproval === null) return null;
  const latencyMs = normalizeDistribution(raw.latency_ms);
  const inputTokens = normalizeDistribution(raw.input_tokens);
  const outputTokens = normalizeDistribution(raw.output_tokens);
  const rounds = normalizeDistribution(raw.rounds);
  const toolCalls = normalizeDistribution(raw.tool_calls);
  if (
    latencyMs === null ||
    inputTokens === null ||
    outputTokens === null ||
    rounds === null ||
    toolCalls === null
  ) {
    return null;
  }
  const cacheHitTokens = asOptionalNumber(raw.cache_hit_tokens);
  const cacheHitRate = asOptionalNumber(raw.cache_hit_rate);
  const costMicros = asOptionalNumber(raw.cost_micros);
  if (
    cacheHitTokens === undefined ||
    cacheHitRate === undefined ||
    costMicros === undefined
  ) {
    return null;
  }
  return {
    turns,
    succeeded,
    failed,
    pending_approval: pendingApproval,
    latency_ms: latencyMs,
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    rounds,
    tool_calls: toolCalls,
    cache_hit_tokens: cacheHitTokens,
    cache_hit_rate: cacheHitRate,
    cost_micros: costMicros,
  };
}

/**
 * 信封归一：形状合格 → 归一快照；缺字段/类型不符 → null（调用侧显式报错，
 * 不降级编数）。未知字段忽略（契约 §9.3 向后兼容）。
 */
export function normalizeMetricsSnapshot(raw: unknown): PanelMetricsSnapshot | null {
  if (!isRecord(raw)) return null;
  const schemaVersion = asCount(raw.schema_version);
  const windowTurns = asCount(raw.window_turns);
  if (schemaVersion === null || windowTurns === null || typeof raw.retention !== 'string') return null;
  const global = normalizeAggregate(raw.global);
  if (global === null) return null;
  const rawSessions = raw.sessions;
  if (!Array.isArray(rawSessions)) return null;
  const sessions: MetricsSessionAggregate[] = [];
  for (const entry of rawSessions) {
    if (!isRecord(entry) || typeof entry.session_id !== 'string') return null;
    const aggregate = normalizeAggregate(entry.aggregate);
    if (aggregate === null) return null;
    sessions.push({session_id: entry.session_id, aggregate});
  }
  return {
    schema_version: schemaVersion,
    window_turns: windowTurns,
    retention: raw.retention,
    global,
    sessions,
  };
}

/** 取数/解析失败的显式错误帧（带 HTTP 状态，真因不静默）。 */
export class MetricsEnvelopeError extends Error {
  /** HTTP 状态；信封形状问题时为 null。 */
  readonly status: number | null;

  constructor(message: string, status: number | null) {
    super(message);
    this.name = 'MetricsEnvelopeError';
    this.status = status;
  }
}

/** 取数面请求的最小响应形状（结构化注入，测试可换假传输）。 */
export interface MetricsHttpResponse {
  status: number;
  json(): Promise<unknown>;
}

/** 取数面依赖（调用侧注入鉴权头单源与 HTTP 传输）。 */
export interface MetricsFetchDeps {
  /** 网关鉴权头（tauri-bridge `gatewayAuthHeaders`，令牌面单源）。 */
  authHeaders: () => Promise<Record<string, string>>;
  /** HTTP 传输（调用侧包一层 `fetch`，避免 `this` 绑定问题）。 */
  fetchJson: (url: string, init: {headers: Record<string, string>}) => Promise<MetricsHttpResponse>;
}

/** 指标端点 URL（`baseUrl` 由调用侧按 runtime `normalizeBaseUrl` 归一后传入）。 */
export function panelMetricsEndpoint(baseUrl: string): string {
  return `${baseUrl.replace(/\/+$/, '')}/v1/panel/metrics`;
}

/**
 * 解析一次端点响应：非 200 / 信封不合契约 → 抛 [`MetricsEnvelopeError`]
 * （错误即帧，真因上屏）；合格 → 归一快照。版本不认识的信封也显式拒绝，
 * 不拿旧形状硬套新字段。
 */
export function parseMetricsEnvelope(status: number, body: unknown): PanelMetricsSnapshot {
  if (status !== 200) {
    throw new MetricsEnvelopeError(`指标端点取数失败：HTTP ${status}`, status);
  }
  const snapshot = normalizeMetricsSnapshot(body);
  if (snapshot === null) {
    throw new MetricsEnvelopeError(
      '指标信封不合契约（字段缺失或类型不符），未上屏不编数',
      null,
    );
  }
  if (snapshot.schema_version !== METRICS_SCHEMA_VERSION) {
    throw new MetricsEnvelopeError(
      `指标信封 schema_version=${snapshot.schema_version} 不受支持（本界面认 ${METRICS_SCHEMA_VERSION}）`,
      null,
    );
  }
  return snapshot;
}

/**
 * 取数面（一次刷新）：组装契约 URL + 鉴权头，解析响应。
 * 失败抛 [`MetricsEnvelopeError`] / 传输错误，由调用侧展示为错误帧。
 */
export async function fetchPanelMetricsSnapshot(
  config: {baseUrl: string},
  deps: MetricsFetchDeps,
): Promise<PanelMetricsSnapshot> {
  const response = await deps.fetchJson(panelMetricsEndpoint(config.baseUrl), {
    headers: await deps.authHeaders(),
  });
  const body = await response.json().catch(() => null);
  return parseMetricsEnvelope(response.status, body);
}

// ---------------------------------------------------------------------------
// 展示映射（「—」语义：null/未加载 → 「—」，真实 0 照显 0）
// ---------------------------------------------------------------------------

/** 千分位展示（与遥测条/预算表同口径）。 */
function fmtCount(value: number | null): string {
  return value === null ? DASH : String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/** 均值展示（最多 1 位小数；整数不拖 .0）。 */
function fmtMean(value: number | null): string {
  if (value === null) return DASH;
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

/**
 * 缓存命中率上屏换算（合同给 0.0–1.0，百分比由消费端换算）：
 * 0.625 → '62.5%'；null/非有限 → '—'（不编百分比）。
 */
export function formatCacheHitRate(rate: number | null): string {
  if (rate === null || !Number.isFinite(rate)) return DASH;
  return `${Math.round(rate * 1000) / 10}%`;
}

/** 留存策略上屏（已知值给口径注记；未知值原样透出，不粉饰）。 */
export const METRICS_RETENTION_LABELS: Record<string, string> = {
  [METRICS_RETENTION]: '进程内滚动（不落盘）',
};

function retentionLabel(retention: string): string {
  return METRICS_RETENTION_LABELS[retention] ?? retention;
}

/** 运行指标卡的一格。 */
export interface MetricsCellView {
  key: 'latency' | 'tokens' | 'cache' | 'counts';
  label: string;
  /** 展示串；无数据位诚实「—」。 */
  value: string;
}

/** 运行指标卡视图（格 + 口径注记行）。 */
export interface MetricsPanelView {
  cells: MetricsCellView[];
  /** 口径注记：窗口 / 留存 / 样本（如实陈述）。 */
  caption: string;
}

const EMPTY_VIEW_CELLS: ReadonlyArray<MetricsCellView> = [
  {key: 'latency', label: '回合延迟 P50 / P95 / 均值（ms）', value: DASH},
  {key: 'tokens', label: 'Token 流向 入 / 出（窗口累计）', value: DASH},
  {key: 'cache', label: '缓存命中（token · 率）', value: DASH},
  {key: 'counts', label: '收束计数 成功 / 失败 / 待批', value: DASH},
];

/**
 * 指标卡视图映射：null（未取到快照）整卡「—」；有快照按合同真值渲染，
 * 分布空窗口/可选列 null 逐位「—」，计数位真实 0 照显 0。
 */
export function metricsPanelView(snapshot: PanelMetricsSnapshot | null): MetricsPanelView {
  if (snapshot === null) {
    return {
      cells: EMPTY_VIEW_CELLS.map((cell) => ({...cell})),
      caption: `窗口 ${DASH} · 留存 ${DASH} · 样本 ${DASH}`,
    };
  }
  const global = snapshot.global;
  const latency = global.latency_ms;
  const latencyAllNull = latency.p50 === null && latency.p95 === null && latency.mean === null;
  const input = global.input_tokens.sum;
  const output = global.output_tokens.sum;
  const cacheHit = global.cache_hit_tokens;
  const cacheRate = formatCacheHitRate(global.cache_hit_rate);
  const cacheEmpty = cacheHit === null && cacheRate === DASH;
  return {
    cells: [
      {
        key: 'latency',
        label: '回合延迟 P50 / P95 / 均值（ms）',
        value: latencyAllNull
          ? DASH
          : `${fmtCount(latency.p50)} / ${fmtCount(latency.p95)} / ${fmtMean(latency.mean)}`,
      },
      {
        key: 'tokens',
        label: 'Token 流向 入 / 出（窗口累计）',
        value: input === null && output === null ? DASH : `${fmtCount(input)} / ${fmtCount(output)}`,
      },
      {
        key: 'cache',
        label: '缓存命中（token · 率）',
        value: cacheEmpty ? DASH : `命中 ${fmtCount(cacheHit)} · ${cacheRate}`,
      },
      {
        key: 'counts',
        label: '收束计数 成功 / 失败 / 待批',
        value: `${global.succeeded} / ${global.failed} / ${global.pending_approval}`,
      },
    ],
    caption: `窗口 ${snapshot.window_turns} 拍 · 留存 ${retentionLabel(snapshot.retention)} · 样本 ${global.turns} 拍`,
  };
}

// ---------------------------------------------------------------------------
// 自动刷新控制器（30s；可注入定时器宿主 = Node 可测，不碰 DOM）
// ---------------------------------------------------------------------------

/** 自动刷新间隔（毫秒）。 */
export const METRICS_AUTO_REFRESH_MS = 30_000;

/** 定时器宿主（浏览器/Node 的 setInterval 对；测试注入假宿主）。 */
export interface MetricsTimerHost {
  setInterval(handler: () => void, ms: number): unknown;
  clearInterval(handle: unknown): void;
}

const globalTimerHost: MetricsTimerHost = {
  setInterval: (handler, ms) => globalThis.setInterval(handler, ms),
  clearInterval: (handle) =>
    globalThis.clearInterval(handle as Parameters<typeof globalThis.clearInterval>[0]),
};

/** 自动刷新句柄。 */
export interface MetricsAutoRefresh {
  /** 停止自动刷新（幂等；组件卸载即清理，不泄漏定时器）。 */
  stop(): void;
}

/**
 * 启动自动刷新：每 `intervalMs`（默认 [`METRICS_AUTO_REFRESH_MS`]）触发一次
 * `tick`；返回句柄由调用侧在卸载/重建时 `stop()`。
 */
export function startMetricsAutoRefresh(
  tick: () => void,
  timers: MetricsTimerHost = globalTimerHost,
  intervalMs: number = METRICS_AUTO_REFRESH_MS,
): MetricsAutoRefresh {
  const handle = timers.setInterval(tick, intervalMs);
  let stopped = false;
  return {
    stop(): void {
      if (stopped) return;
      stopped = true;
      timers.clearInterval(handle);
    },
  };
}
