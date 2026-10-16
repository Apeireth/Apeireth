// 随手记（Quick Capture）— 快捷助手窗纯采集面（C 轴入口面）。
// 一键记一条 → 未整理原料入记忆流；不走对话回合、不调模型（纯采集零 token）。
//
// 契约（C-memory-deepening「随手记」条 + docs/04-internal/backend-enhancement/quick-capture-delivery.md）：
//   1. 模式：输入 `#` 前缀 = 随手记模式（detectQuickMode 随输入即时切换，不加窗格）；
//      Enter/发送键在随手记模式下走采集，不进对话回合；
//   2. 入流同源：走 runtime.appendMemoryEpisode（契约 §5 POST /v1/memory/append），
//      不另开写入口。来源标注随文携带（后端 schema 仅收 content，随文序列化，
//      与深流 buildMemoryContent 同一先例口径）：
//      `[随手记 #N] <原文>（来源 capture）` —— 备注级标注，parseCaptureNote 可逆解析，
//      供后续整编（consolidation）吸收时还原原文，不吞标注语义；
//   3. 分级：capture = 未整理原料级——protection=off（不走 protect 通道，落流即未保护），
//      可被后续整编吸收；与教训（教训 #N）/ 里程碑同族分级呼应。#N = 账本自管连记
//      序号：发号在写流之前（正文随号落流），发号即持久、号只发不收（失败留空档，
//      不复用不改写）；
//   4. 回看：账本只读（recent 取最近 CAPTURE_RECENT_VIEW 条，新→旧）；账本超
//      CAPTURE_LEDGER_MAX 丢最旧，序号不动；
//   5. 零打扰：不发通知、不触发对话、不烧 token；空内容/非随手记输入显式拒绝
//      （{ok:false, reason} 帧，不猜不补、不静默吞）。
//
// 测试：tests/quick-capture.mjs（真实模块直测 + fetch 记录面）。

import {appendMemoryEpisode} from '../../lib/runtime.ts';
import type {ApeirethConfig} from '../../lib/types';

/** 随手记输入前缀（斜杠式入口：`#` 开头即随手记模式）。 */
export const CAPTURE_PREFIX = '#';

/** 来源标注（未整理原料级的来源面，随文携带进记忆流）。 */
export const CAPTURE_SOURCE = 'capture';

/** 本地回看账本存储键（读缓存；记忆流是真源）。 */
export const CAPTURE_LEDGER_KEY = 'apeireth:quick-capture:ledger';

/** 账本条数上限：超限丢最旧，序号不动。 */
export const CAPTURE_LEDGER_MAX = 50;

/** 回看展示条数（最近 N 条只读）。 */
export const CAPTURE_RECENT_VIEW = 5;

export type QuickMode = 'chat' | 'capture';

/** 一条随手记账本记录（分级标注与来源标注是一等字段，不藏不猜）。 */
export interface CaptureRecord {
  /** 连记序号（发号即占用，单调递增，只发不收）。 */
  readonly seq: number;
  /** 原文（未整理原料，不改写不截断）。 */
  readonly text: string;
  /** 采集时刻（epoch ms）。 */
  readonly at: number;
  /** 来源标注：capture（随手记采集，未整理原料级）。 */
  readonly source: typeof CAPTURE_SOURCE;
  /** 未整理原料级：protection=off，可被后续整编吸收。 */
  readonly protected: false;
}

/** 采集拒绝帧（显式拒绝原因，不猜不补）。 */
export type CaptureRejectReason = 'not-capture' | 'empty' | 'append-failed';

export type CaptureOutcome =
  | {ok: true; record: CaptureRecord; content: string}
  | {ok: false; reason: CaptureRejectReason; message: string};

/** 账本持久化面（注入可测；默认 localStorage，缺席时内存兜底）。 */
export interface CaptureStorage {
  read(): string | null;
  write(raw: string): void;
}

interface LedgerState {
  nextSeq: number;
  records: CaptureRecord[];
}

/** 模式判定：`#` 前缀（前导空白后）= 随手记，否则对话。 */
export function detectQuickMode(input: string): QuickMode {
  return input.trimStart().startsWith(CAPTURE_PREFIX) ? 'capture' : 'chat';
}

/**
 * 解析随手记输入：`#` 后原文（裁前后空白）。拒绝帧：
 * - not-capture：不带 `#` 前缀（入口名 = 入口权力，随手记缝只收随手记输入）；
 * - empty：`#` 后无内容（纯空白算空，0 装不猜不补）。
 */
export function parseCaptureInput(input: string): {ok: true; text: string} | {ok: false; reason: 'not-capture' | 'empty'} {
  const trimmed = input.trim();
  if (!trimmed.startsWith(CAPTURE_PREFIX)) return {ok: false, reason: 'not-capture'};
  const text = trimmed.slice(CAPTURE_PREFIX.length).trim();
  if (!text) return {ok: false, reason: 'empty'};
  return {ok: true, text};
}

/** 备注级标注序列化：`[随手记 #N] <原文>（来源 capture）`。 */
export function buildCaptureNote(seq: number, text: string): string {
  return `[随手记 #${seq}] ${text}（来源 ${CAPTURE_SOURCE}）`;
}

const CAPTURE_NOTE_RE = /^\[随手记 #(\d+)\] ([\s\S]*)（来源 capture）$/;

/** 备注级标注解析（整编吸收缝）：还原 {seq, 原文}；非随手记内容返回 null。 */
export function parseCaptureNote(content: string): {seq: number; text: string} | null {
  const m = CAPTURE_NOTE_RE.exec(content);
  if (!m) return null;
  return {seq: Number(m[1]), text: m[2]};
}

/** 回看行时间（本地 HH:mm）。 */
export function formatCaptureTime(at: number): string {
  return new Date(at).toTimeString().slice(0, 5);
}

function defaultStorage(): CaptureStorage {
  try {
    const ls = globalThis.localStorage;
    if (ls) {
      ls.getItem(CAPTURE_LEDGER_KEY);
      return {
        read: () => ls.getItem(CAPTURE_LEDGER_KEY),
        write: (raw) => ls.setItem(CAPTURE_LEDGER_KEY, raw),
      };
    }
  } catch {
    // localStorage 不可读写（隐私模式/非浏览器）→ 内存兜底，如实只护当次会话。
  }
  let raw: string | null = null;
  return {
    read: () => raw,
    write: (next) => {
      raw = next;
    },
  };
}

function normalizeState(raw: string | null): LedgerState {
  if (!raw) return {nextSeq: 1, records: []};
  try {
    const parsed = JSON.parse(raw) as {nextSeq?: unknown; records?: unknown};
    const records: CaptureRecord[] = [];
    if (Array.isArray(parsed.records)) {
      for (const item of parsed.records) {
        const rec = item as Partial<CaptureRecord>;
        if (
          typeof rec.seq === 'number' &&
          Number.isInteger(rec.seq) &&
          rec.seq > 0 &&
          typeof rec.text === 'string' &&
          rec.text.trim() !== '' &&
          typeof rec.at === 'number' &&
          Number.isFinite(rec.at) &&
          rec.source === CAPTURE_SOURCE &&
          rec.protected === false
        ) {
          records.push({
            seq: rec.seq,
            text: rec.text,
            at: rec.at,
            source: CAPTURE_SOURCE,
            protected: false,
          });
        }
      }
    }
    records.sort((a, b) => a.seq - b.seq);
    const maxSeq = records.reduce((max, r) => Math.max(max, r.seq), 0);
    const carried = typeof parsed.nextSeq === 'number' && Number.isInteger(parsed.nextSeq) ? parsed.nextSeq : 1;
    return {nextSeq: Math.max(carried, maxSeq + 1), records};
  } catch {
    // 坏档：本地账本是读缓存，读不出就从空账本起（不冒充恢复了什么）；记忆流是真源。
    return {nextSeq: 1, records: []};
  }
}

/**
 * 随手记账本：发号（begin）→ 入账（commit）。发号在写流之前、即持久；
 * 入账只在写流成功后发生。回看只读。
 */
export class CaptureLedger {
  private state: LedgerState;
  private readonly storage: CaptureStorage;

  constructor(storage: CaptureStorage = defaultStorage()) {
    this.storage = storage;
    this.state = normalizeState(storage.read());
  }

  /**
   * 发号 + 组流：校验输入（拒帧见 parseCaptureInput）、占号（发号即持久）、
   * 生成备注级正文。号只发不收：写流失败留空档，不复用不改写。
   */
  begin(input: string, at: number): CaptureOutcome {
    const parsed = parseCaptureInput(input);
    if (!parsed.ok) {
      return {
        ok: false,
        reason: parsed.reason,
        message: parsed.reason === 'empty' ? '内容为空，没记下' : '随手记内容需以 # 开头',
      };
    }
    const seq = this.state.nextSeq;
    this.state = {nextSeq: seq + 1, records: this.state.records};
    this.persist();
    const record: CaptureRecord = {
      seq,
      text: parsed.text,
      at,
      source: CAPTURE_SOURCE,
      protected: false,
    };
    return {ok: true, record, content: buildCaptureNote(seq, parsed.text)};
  }

  /** 入账（写流成功后调用）：按序号插位、超限丢最旧、序号不动。 */
  commit(record: CaptureRecord): void {
    if (this.state.records.some((r) => r.seq === record.seq)) return;
    const records = [...this.state.records, record].sort((a, b) => a.seq - b.seq);
    this.state = {
      nextSeq: this.state.nextSeq,
      records: records.length > CAPTURE_LEDGER_MAX ? records.slice(records.length - CAPTURE_LEDGER_MAX) : records,
    };
    this.persist();
  }

  /** 回看：最近 n 条（新→旧），只读展示（返回副本，改它不动账本）。 */
  recent(n: number = CAPTURE_RECENT_VIEW): CaptureRecord[] {
    return this.state.records
      .slice()
      .sort((a, b) => b.at - a.at || b.seq - a.seq)
      .slice(0, Math.max(0, n));
  }

  /** 全账本副本（序号升序，测试/审计用）。 */
  records(): CaptureRecord[] {
    return this.state.records.slice();
  }

  /** 已发号数（含未入账的失败号，如实反映发号史）。 */
  issued(): number {
    return this.state.nextSeq - 1;
  }

  private persist(): void {
    this.storage.write(JSON.stringify({nextSeq: this.state.nextSeq, records: this.state.records}));
  }
}

/**
 * 一键记 → 记忆流（同源缝：runtime.appendMemoryEpisode）。纯采集：
 * 仅一次记忆追加请求，零对话回合、零模型调用、零通知。
 */
export async function captureToMemoryStream(
  config: ApeirethConfig,
  ledger: CaptureLedger,
  input: string,
  at: number = Date.now(),
): Promise<CaptureOutcome> {
  const begun = ledger.begin(input, at);
  if (!begun.ok) return begun;
  let ok = false;
  try {
    ok = await appendMemoryEpisode(config, begun.content);
  } catch (caught) {
    const detail = caught instanceof Error ? caught.message : String(caught);
    return {ok: false, reason: 'append-failed', message: `写入记忆流失败：${detail}`};
  }
  if (!ok) {
    return {ok: false, reason: 'append-failed', message: '写入记忆流失败：记忆追加缝未接受'};
  }
  ledger.commit(begun.record);
  return begun;
}
