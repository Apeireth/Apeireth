// 深流 · 记忆压缩检索面纯逻辑（星港内置应用面，零依赖纯模块）
//
// 契约（I-starport-builtins.md「深流」，本切片三件套）：
//   1. 观察账本 —— 工具/事件观察的一等台账（id/at/kind/text/source），kind
//      是真实字段（decision/fix/security/note），非虚构分类；
//   2. 压缩视图 —— 按时间窗把观察折叠成「要点行」（保留 kind 标签与计数），
//      超预算丢最旧并如实报告丢弃数（0 装：不平滑、不隐藏丢弃）；
//      模型摘要由调用方注入文本，纯件不做假语义；
//   3. 召回查询 —— 词项命中计数排序（大小写不敏感、多词 OR、稳定排序），
//      空查询返回时间倒序；
//   4. 派发文本 —— buildMemoryContent 生成契约 §5 content 串（kind 前缀是
//      账本一等字段的真实序列化；后端 schema 仅收 content，故随文携带）；
//   5. 0 装 —— 空文本/非法 kind/非法时间/重复 id 一律抛错，不猜不修。
//
// 测试：tests/starport-memory-flow.mjs（真实模块直测）。

export type ObservationKind = 'decision' | 'fix' | 'security' | 'note';

export interface Observation {
  id: string;
  at: number;
  kind: ObservationKind;
  text: string;
  /** 来源标识（工具名/事件名等真实产点；可为空串=未标注）。 */
  source: string;
}

export class MemoryFlowError extends Error {
  readonly code:
    | 'invalid-id'
    | 'invalid-text'
    | 'invalid-kind'
    | 'invalid-time'
    | 'invalid-source'
    | 'duplicate-id'
    | 'missing'
    | 'invalid-budget';

  constructor(
    code:
      | 'invalid-id'
      | 'invalid-text'
      | 'invalid-kind'
      | 'invalid-time'
      | 'invalid-source'
      | 'duplicate-id'
      | 'missing'
      | 'invalid-budget',
    message: string,
  ) {
    super(message);
    this.name = 'MemoryFlowError';
    this.code = code;
  }
}

/** kind 全集与稳定展示序（压缩行计数按此序）。 */
export const OBSERVATION_KINDS: readonly ObservationKind[] = ['decision', 'fix', 'security', 'note'];

/** 压缩时间窗：一个自然日（毫秒）。 */
export const COMPRESS_WINDOW_MS = 86_400_000;

function normalizeKind(kind: unknown): ObservationKind {
  if (typeof kind === 'string' && (OBSERVATION_KINDS as readonly string[]).includes(kind)) {
    return kind as ObservationKind;
  }
  throw new MemoryFlowError('invalid-kind', `非法观察 kind：${String(kind)}`);
}

function normalizeText(text: unknown): string {
  if (typeof text !== 'string') {
    throw new MemoryFlowError('invalid-text', '观察文本必须是字符串');
  }
  if (!text.trim()) {
    throw new MemoryFlowError('invalid-text', '观察文本不能为空（纯空白算空）');
  }
  return text;
}

function normalizeAt(at: unknown): number {
  if (typeof at !== 'number' || !Number.isFinite(at)) {
    throw new MemoryFlowError('invalid-time', '观察时间必须是有限数值（毫秒）');
  }
  return at;
}

function normalizeId(id: unknown): string {
  if (typeof id !== 'string' || !id) {
    throw new MemoryFlowError('invalid-id', '观察 id 必须是非空字符串');
  }
  return id;
}

/** 追加观察（校验后返回新数组，输入不动）。 */
export function appendObservation(
  items: readonly Observation[],
  input: {
    id: string;
    at: number;
    kind: string;
    text: string;
    source?: string;
  },
): Observation[] {
  const id = normalizeId(input?.id);
  if (items.some((item) => item.id === id)) {
    throw new MemoryFlowError('duplicate-id', `观察 id 冲突：${id}`);
  }
  const source = input.source ?? '';
  if (typeof source !== 'string') {
    throw new MemoryFlowError('invalid-source', '来源标识必须是字符串');
  }
  const next: Observation = {
    id,
    at: normalizeAt(input.at),
    kind: normalizeKind(input.kind),
    text: normalizeText(input.text),
    source,
  };
  return [...items, next];
}

/** 移除观察（不存在抛 missing，0 装）。 */
export function removeObservation(items: readonly Observation[], id: string): Observation[] {
  if (!items.some((item) => item.id === id)) {
    throw new MemoryFlowError('missing', `观察不存在：${id}`);
  }
  return items.filter((item) => item.id !== id);
}

/** 账本视图：kind 过滤 + 文本过滤（大小写不敏感子串）+ 时间倒序。 */
export function listObservations(
  items: readonly Observation[],
  options: {kind?: string; text?: string} = {},
): Observation[] {
  const kind = options.kind === undefined ? null : normalizeKind(options.kind);
  const needle = options.text === undefined ? '' : options.text.toLowerCase();
  return items
    .filter((item) => (kind === null ? true : item.kind === kind))
    .filter((item) => (needle ? item.text.toLowerCase().includes(needle) : true))
    .slice()
    .sort((a, b) => b.at - a.at);
}

interface CompressBucket {
  key: number;
  items: Observation[];
}

/**
 * 压缩视图：按自然日时间窗折叠成要点行。
 * 行格式：`YYYY-MM-DD [decision×2 fix×1] 文本1；文本2…`（kind 计数按稳定序，
 * 文本按时间升序拼接并截断 80 字）。窗口数超 budget 时丢最旧窗口，
 * dropped 如实等于被丢弃窗口的观察条数（不吞、不平滑）。
 */
export function compressObservations(
  items: readonly Observation[],
  budget: number,
): {lines: string[]; dropped: number} {
  if (!Number.isInteger(budget) || budget <= 0) {
    throw new MemoryFlowError('invalid-budget', '压缩预算必须是正整数（行数）');
  }
  const buckets = new Map<number, Observation[]>();
  for (const item of items) {
    const key = Math.floor(item.at / COMPRESS_WINDOW_MS);
    const bucket = buckets.get(key);
    if (bucket) bucket.push(item);
    else buckets.set(key, [item]);
  }
  const ordered: CompressBucket[] = [...buckets.entries()]
    .map(([key, bucketItems]) => ({
      key,
      items: bucketItems.slice().sort((a, b) => a.at - b.at),
    }))
    .sort((a, b) => a.key - b.key);

  let dropped = 0;
  let kept = ordered;
  if (ordered.length > budget) {
    const doomed = ordered.slice(0, ordered.length - budget);
    dropped = doomed.reduce((sum, bucket) => sum + bucket.items.length, 0);
    kept = ordered.slice(ordered.length - budget);
  }

  const lines = kept.map((bucket) => {
    const date = new Date(bucket.key * COMPRESS_WINDOW_MS).toISOString().slice(0, 10);
    const counts = OBSERVATION_KINDS.map((kind) => {
      const n = bucket.items.filter((item) => item.kind === kind).length;
      return n > 0 ? `${kind}×${n}` : null;
    })
      .filter((part): part is string => part !== null)
      .join(' ');
    const joined = bucket.items.map((item) => item.text).join('；');
    const sample = joined.length > 80 ? `${joined.slice(0, 80)}…` : joined;
    return `${date} [${counts}] ${sample}`;
  });
  return {lines, dropped};
}

/**
 * 召回排序：词项命中计数降序 → 时间降序 → 原始次序升序（稳定）。
 * 词项 = 空白切分的非空词（大小写不敏感子串命中，每词每条最多计 1 次，
 * 多词 OR——任一词命中即参与计分）。空查询返回时间倒序。
 */
export function rankRecall(items: readonly Observation[], query: string): Observation[] {
  // 词项去重（每词每条最多计 1，重复查询词不加成）。
  const terms = [
    ...new Set(
      String(query)
        .toLowerCase()
        .split(/\s+/)
        .filter((term) => term.length > 0),
    ),
  ];
  const scored = items.map((item, index) => {
    const haystack = item.text.toLowerCase();
    const hits = terms.reduce(
      (sum, term) => sum + (haystack.includes(term) ? 1 : 0),
      0,
    );
    return {item, index, hits};
  });
  scored.sort((a, b) => {
    if (b.hits !== a.hits) return b.hits - a.hits;
    if (b.item.at !== a.item.at) return b.item.at - a.item.at;
    return a.index - b.index;
  });
  return scored.map((entry) => entry.item);
}

/**
 * 派发文本（契约 §5 content 串）：`[深流/<kind>] <text>（来源 <source>）`。
 * kind 前缀是账本一等字段的真实序列化（后端 schema 仅收 content，随文携带），
 * 非虚构分类。
 */
export function buildMemoryContent(observation: Observation): string {
  const kind = normalizeKind(observation.kind);
  const text = normalizeText(observation.text);
  const base = `[深流/${kind}] ${text}`;
  return observation.source ? `${base}（来源 ${observation.source}）` : base;
}
