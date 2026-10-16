// 随手记回看逻辑面（V1-b/d 上屏）：工作台「随手记」卡 —— 最近 50 条账本 + 搜索过滤。
//
// 数据源口径（同源只读）：快捷窗回看已有 5 条 = `CaptureLedger.recent(CAPTURE_RECENT_VIEW)`
// （`src/features/quick/quick-capture.ts`，账本 localStorage 单键，CAPTURE_LEDGER_MAX=50）。
// 本卡消费同一本账本的 recent(CAPTURE_LEDGER_MAX) = 最近 50 条，只读展示；
// features/quick/** 为禁区（本文件只 import 不改写）。
//
// 记忆流是真源、本地账本只是回看缓存（quick-capture 契约注记）——本卡定位
// 「回看」，故以账本为准；账本空不冒充有记录。

import {
  CAPTURE_LEDGER_MAX,
  formatCaptureTime,
  type CaptureRecord,
} from '../features/quick/quick-capture.ts';

/** 本卡回看条数 = 账本上限（最近 50 条账本，任务口径）。 */
export const CAPTURE_REVIEW_LIMIT = CAPTURE_LEDGER_MAX;

export interface CaptureReviewRow {
  seq: number;
  text: string;
  /** 本地 HH:mm（与快捷窗回看同格式）。 */
  time: string;
  at: number;
}

function toRow(record: CaptureRecord): CaptureReviewRow {
  return {
    seq: record.seq,
    text: record.text,
    time: formatCaptureTime(record.at),
    at: record.at,
  };
}

/**
 * 回看列表投影：最近 N 条（新→旧，与账本 recent 同排序口径），只读副本。
 */
export function captureReviewRows(records: CaptureRecord[], limit: number = CAPTURE_REVIEW_LIMIT): CaptureReviewRow[] {
  return records
    .slice()
    .sort((a, b) => b.at - a.at || b.seq - a.seq)
    .slice(0, Math.max(0, limit))
    .map(toRow);
}

/**
 * 搜索过滤：空白 = 全量；命中序号（#3 / 3）或正文子串（大小写不敏感）即保留。
 * 过滤不改排序（新→旧），不改写行内容。
 */
export function filterCaptureRows(rows: CaptureReviewRow[], query: string): CaptureReviewRow[] {
  const q = query.trim();
  if (q === '') return rows;
  const needle = q.replace(/^#/, '').toLowerCase();
  return rows.filter((row) => {
    const seqHit = /^\d+$/.test(needle) && String(row.seq) === needle;
    const textHit = row.text.toLowerCase().includes(needle);
    return seqHit || textHit;
  });
}
