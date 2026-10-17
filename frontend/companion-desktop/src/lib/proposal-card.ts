// 提案卡逻辑面（提案账上屏）：提案账最近 N 条只读列表 + IM 卡风格呈现
// （每卡带「建议你亲自确认」行）。
//
// 数据源口径（诚实边界，零假旋钮）：提案账 = 记忆六历史流的 proposal 流
// （提案流/立场史，append-only；`crates/engine/memory/src/streams.rs`
// `proposal_stream`）。读面 = `GET /v1/panel/memory/streams?kind=proposal`
// （契约 frontend-data-contract.md §4.3：{kind, subject, count, entries[]}，
// entry = {id, subject_id, session_id, created_at(**unix 秒**), payload(自由
// JSON), source, tags}）。取数缝由 runtime.ts 的 `fetchProposalLedger` 一行
// 换源；该部署读面缺席（501/404）时如实上屏「暂无接口」空态，不硬造端点、
// 不摆假数据（同技能书架先例）。
//
// 只读纪律：提案账是 append-only 历史流，本卡零写面——不代决、不代批、不回写。
// 是否采纳由你定夺（IM 卡「建议你亲自确认」行即此口径）。

import {DASH} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 合同形状（§4.3；snake_case 字段名照收，不改写）
// ---------------------------------------------------------------------------

/** 提案账一行（HistoryEntry 投影；`created_at` = unix 秒，与契约同口径）。 */
export interface ProposalEntryLike {
  id: string;
  subject_id?: string;
  session_id?: string | null;
  /** unix 秒（§4.3 口径；上屏换算毫秒）。 */
  created_at: number;
  payload?: unknown;
  source?: string;
  tags?: string[];
}

/** 提案账扫描（读面一次快照读的归一结果）。 */
export interface ProposalLedger {
  entries: ProposalEntryLike[];
  /** 读面回显的主体 id（缺省 companion-main）。 */
  subject: string;
}

/** 最近 N 条上屏上限（可配）。 */
export const PROPOSAL_LEDGER_LIMIT = 8;

/** IM 卡「建议你亲自确认」行（逐字上屏）。 */
export const PROPOSAL_CONFIRM_LINE = '建议你亲自确认';

/** 提案账口径注。 */
export const PROPOSAL_NOTE =
  '提案账 = 记忆六历史流的 proposal 流（append-only，只读回显）；提案是否采纳由你定夺——本卡不代决、不代批、不回写';

/** 读面缺席时的诚实空态口径（不硬造端点，不摆假旋钮）。 */
export const PROPOSAL_NO_FACE =
  '提案账暂无接口（该部署的 proposal 流无 HTTP/IPC 读面）——如实上屏，不硬造端点、不摆假数据';

function asStr(v: unknown): string {
  return typeof v === 'string' ? v : '';
}

function asFiniteNumber(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

/**
 * 归一一行提案账条目：形状不符的行直接丢（不编默认值冒充真值）。
 * `created_at` 必须是有限数（unix 秒）；payload 自由 JSON 原样带走。
 */
export function normalizeProposalEntry(raw: unknown): ProposalEntryLike | null {
  if (!raw || typeof raw !== 'object') return null;
  const rec = raw as Record<string, unknown>;
  const id = asStr(rec.id);
  const createdAt = asFiniteNumber(rec.created_at);
  if (id === '' || createdAt === null) return null;
  const tags = Array.isArray(rec.tags) ? rec.tags.filter((t): t is string => typeof t === 'string') : [];
  return {
    id,
    subject_id: asStr(rec.subject_id),
    session_id: typeof rec.session_id === 'string' ? rec.session_id : null,
    created_at: createdAt,
    payload: rec.payload,
    source: asStr(rec.source),
    tags,
  };
}

/**
 * 归一提案账扫描：读面信封 `{entries: [...]}` / `{kind, subject, count, entries}`
 * 或裸数组皆可；`subject` 原样带出（缺省空串 = 读面未回显，不编主体）。
 */
export function normalizeProposalLedger(raw: unknown): ProposalLedger {
  const rec = (raw && typeof raw === 'object' ? raw : {}) as {entries?: unknown; subject?: unknown};
  const list = Array.isArray(raw) ? raw : Array.isArray(rec.entries) ? rec.entries : [];
  const entries: ProposalEntryLike[] = [];
  for (const item of list) {
    const entry = normalizeProposalEntry(item);
    if (entry) entries.push(entry);
  }
  return {entries, subject: asStr(rec.subject)};
}

// ---------------------------------------------------------------------------
// IM 卡视图映射（纯函数；null → 「—」，未知不用 0 冒充）
// ---------------------------------------------------------------------------

/** 正文摘录上限（有界摘录；全文留在账面 payload，不在此截改真值）。 */
export const PROPOSAL_EXCERPT_LIMIT = 240;

/** payload 自由 JSON → 有界 JSON 摘录（超限截断并显式「…」）。 */
export function proposalJsonExcerpt(payload: unknown, limit: number = PROPOSAL_EXCERPT_LIMIT): string {
  let text: string;
  try {
    text = JSON.stringify(payload ?? null) ?? String(payload);
  } catch {
    text = String(payload);
  }
  return text.length > limit ? `${text.slice(0, limit)}…` : text;
}

function firstStringField(payload: unknown, keys: readonly string[]): string {
  if (!payload || typeof payload !== 'object') return '';
  const rec = payload as Record<string, unknown>;
  for (const key of keys) {
    const value = rec[key];
    if (typeof value === 'string' && value !== '') return value;
  }
  return '';
}

/** 提案标题（约定字段名取值；取不到如实「—」，不编标题）。 */
export function proposalTitle(payload: unknown): string {
  const title = firstStringField(payload, ['title', 'summary', 'subject', 'name']);
  return title === '' ? DASH : title;
}

/** 提案正文（约定字段名取值；取不到回 JSON 摘录，不编正文）。 */
export function proposalBody(payload: unknown): string {
  const body = firstStringField(payload, ['content', 'text', 'body', 'detail', 'reason', 'proposal']);
  return body === '' ? proposalJsonExcerpt(payload) : body;
}

export interface ProposalCardRowView {
  id: string;
  title: string;
  body: string;
  /** `YYYY-MM-DD HH:mm`（UTC；created_at unix 秒 → 毫秒换算）。 */
  at: string;
  source: string;
  tags: string;
  subject: string;
  /** 恒「建议你亲自确认」（IM 卡行，逐字）。 */
  confirmLine: string;
}

export interface ProposalCardView {
  /** false = 读面缺席（诚实空态）。 */
  available: boolean;
  rows: ProposalCardRowView[];
  note: string;
  confirmLine: string;
}

function formatProposalTime(unixSeconds: number | null): string {
  if (unixSeconds === null) return DASH;
  const d = new Date(unixSeconds * 1000);
  const pad = (n: number): string => (n < 10 ? `0${n}` : String(n));
  return (
    `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ` +
    `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`
  );
}

/**
 * 提案卡视图映射：最近 N 条（新 → 旧，同刻按 id 字典序定序可复核）IM 卡行。
 * `ledger` 为 null = 读面缺席 → 诚实空态（零假行）。
 */
export function proposalCardView(
  ledger: ProposalLedger | null,
  limit: number = PROPOSAL_LEDGER_LIMIT,
): ProposalCardView {
  if (ledger === null) {
    return {available: false, rows: [], note: PROPOSAL_NO_FACE, confirmLine: PROPOSAL_CONFIRM_LINE};
  }
  const rows = [...ledger.entries]
    .sort((a, b) => b.created_at - a.created_at || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .slice(0, Math.max(0, limit))
    .map((entry) => ({
      id: entry.id,
      title: proposalTitle(entry.payload),
      body: proposalBody(entry.payload),
      at: formatProposalTime(entry.created_at),
      source: entry.source === '' || entry.source === undefined ? DASH : entry.source,
      tags: entry.tags && entry.tags.length > 0 ? entry.tags.join(' / ') : DASH,
      subject: entry.subject_id === '' || entry.subject_id === undefined ? DASH : entry.subject_id,
      confirmLine: PROPOSAL_CONFIRM_LINE,
    }));
  return {available: true, rows, note: PROPOSAL_NOTE, confirmLine: PROPOSAL_CONFIRM_LINE};
}
