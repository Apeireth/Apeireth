// 性格卡逻辑面（R2 性格锁定上屏）：四滑杆现值 + 锁定态 + 最近移动 + 「本周性格
// 周报」区块 + 锁定/解锁 CLI 指引。
//
// 合同出处（镜像消费，不改写）：frontend-data-contract.md §10.9 / §10.11——
//   A. 面板读面 `GET /v1/panel/traits`（gateway-api-contract.md §7f，与
//      `tool.trait_report` 同账同源：同一本 tuning-log.jsonl + 同一份
//      trait-locks.json）：values 四滑杆现值（生效值）+ locked 四杆锁定态 +
//      recent_moves 最近移动 + records 账面行 + skipped_records 坏行计数；
//   B. `tool.self_status` 调参/性格节：四杆值 + `tuning.locked`（四杆锁定态）+
//      `tuning.recent_moves`（≤4 条，新 → 旧：{param,label,at_epoch_ms,delta,reason}）；
//   C. `tool.trait_report` 帧（只读零审批）：近 7 天窗（两端含，未来时刻不计）、
//      sliders[]（value/locked/net_delta/move_count/locked_skipped/last_move）、
//      top_reasons、records_in_window、locked_skipped、skipped_records；
//   D. 锁定动作面：CLI `apeireth trait lock|unlock <SLIDER>`（内核 API
//      `set_trait_lock`）；锁定/解锁**写面**无 HTTP 面（面板只有只读面
//      `/v1/panel/traits`）——故锁定/解锁按钮只给 CLI 指引、不代跑（只读 + 指引纪律）。
//
// 数据 = 成长计既有账（tuning-log.jsonl，与「性格养成」学习日志同账，不新造账）；
// 周报区块是同一本账的 7 天窗**投影**，口径逐项镜像 `orchestration::trait_report`
// （moved 判据 / 锁定跳过留痕前缀 / top_reasons 排序 / last_move 取最新），
// 由 tests/trait-card.mjs 对照同批 fixture 钉死。权威读面 = `tool.trait_report`
// （对话内）/ `apeireth trait report`（CLI）；本卡是人类界面的同账投影。
//
// 锁定档（trait-locks.json）读面 = `GET /v1/panel/traits`（缺档 = 全解锁默认态、
// 坏档即帧）；该部署读面缺席（501/404）时锁定态显「—」+ 真因，绝不拿「未锁定」
// 冒充未知。

import {DASH} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 合同形状（§10.9；snake_case 字段名照收）
// ---------------------------------------------------------------------------

/** 四根特质滑杆（稳定 id，顺序 = `TunableParam::ALL`）。 */
export const TRAIT_SLIDERS = [
  {param: 'memory_fade', label: '遗忘衰减强度'},
  {param: 'curiosity_strength', label: '好奇心强度'},
  {param: 'tone_saturation', label: '语气情绪饱和度'},
  {param: 'consolidation_cadence', label: '整合节奏'},
] as const;

export type TraitParam = (typeof TRAIT_SLIDERS)[number]['param'];

export function traitSliderLabel(param: string): string {
  return TRAIT_SLIDERS.find((slider) => slider.param === param)?.label ?? param;
}

/** 锁定跳过留痕的 reason 前缀（`self_tuning::TRAIT_LOCKED_SKIP_PREFIX` 同源）。 */
export const TRAIT_LOCKED_SKIP_PREFIX = '已锁定, 跳过沉积';

/** 周报窗口天数（`TRAIT_REPORT_WINDOW_DAYS`）。 */
export const TRAIT_WINDOW_DAYS = 7;

/** 移动原因排行条数上限（`TRAIT_REPORT_TOP_REASONS`）。 */
export const TRAIT_TOP_REASONS = 5;

/** 「最近移动」条数上限（`TRAIT_RECENT_MOVES`）。 */
export const TRAIT_RECENT_MOVES = 4;

/** 四杆现值（缺源显式 null → 「—」，不编数）。 */
export interface TraitValuesLike {
  memory_fade?: number | null;
  curiosity_strength?: number | null;
  tone_saturation?: number | null;
  consolidation_cadence?: number | null;
}

/** 锁定态（`tuning.locked` / `trait-locks.json`；缺源 = null → 「—」）。 */
export interface TraitLocksLike {
  memory_fade?: boolean | null;
  curiosity_strength?: boolean | null;
  tone_saturation?: boolean | null;
  consolidation_cadence?: boolean | null;
}

/** 成长计既有账一行（`self_tuning::TuningRecord` / tuning-log.jsonl；IPC `read_tuning_log` 同形）。 */
export interface TuningLogEntryLike {
  seq: number;
  param: string;
  previous: number;
  next: number;
  reason: string;
  at_epoch_ms: number;
}

/** 一次滑杆移动（`trait_report::TraitMove` 同语义）。 */
export interface TraitMoveLike {
  param: string;
  label: string;
  at_epoch_ms: number;
  delta: number;
  reason: string;
}

// ---------------------------------------------------------------------------
// 读面归一（GET /v1/panel/traits → 快照；坏行丢弃不编值，同账面扫描语义）
// ---------------------------------------------------------------------------

function traitNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function traitBoolean(value: unknown): boolean | null {
  return typeof value === 'boolean' ? value : null;
}

/** 四杆现值归一（缺键显式 null，不编数）。 */
function traitValuesOf(raw: unknown): TraitValuesLike {
  const rec = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  return {
    memory_fade: traitNumber(rec.memory_fade),
    curiosity_strength: traitNumber(rec.curiosity_strength),
    tone_saturation: traitNumber(rec.tone_saturation),
    consolidation_cadence: traitNumber(rec.consolidation_cadence),
  };
}

/** 四杆锁定态归一（缺键显式 null = 未知，不编「未锁定」）。 */
function traitLocksOf(raw: unknown): TraitLocksLike {
  const rec = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  return {
    memory_fade: traitBoolean(rec.memory_fade),
    curiosity_strength: traitBoolean(rec.curiosity_strength),
    tone_saturation: traitBoolean(rec.tone_saturation),
    consolidation_cadence: traitBoolean(rec.consolidation_cadence),
  };
}

/**
 * 性格卡取数缝的快照（`GET /v1/panel/traits` 一次快照读的投影；每块都可缺席）。
 * `moves` = 成长计既有账账面行（周报/最近移动同账投影的数据面；坏行丢弃）。
 */
export interface TraitSnapshot {
  values: TraitValuesLike | null;
  locks: TraitLocksLike | null;
  moves: TuningLogEntryLike[] | null;
  skippedRecords: number | null;
}

/**
 * 归一 `GET /v1/panel/traits` 响应帧 → 性格卡取数缝快照。
 * 载荷一个已知节都没有 = 读不了这帧（返回 null → 缺源显「—」），不把陌生载荷
 * 冒充空账。
 */
export function normalizeTraitSnapshot(raw: unknown): TraitSnapshot | null {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null;
  const rec = raw as Record<string, unknown>;
  const known = ['values', 'locked', 'recent_moves', 'records', 'skipped_records'];
  if (!known.some((key) => key in rec)) return null;
  const moves: TuningLogEntryLike[] = [];
  if (Array.isArray(rec.records)) {
    for (const item of rec.records) {
      const entry = normalizeTuningEntry(item);
      if (entry) moves.push(entry);
    }
  }
  return {
    values: traitValuesOf(rec.values),
    locks: traitLocksOf(rec.locked),
    moves,
    skippedRecords: traitNumber(rec.skipped_records),
  };
}

// ---------------------------------------------------------------------------
// 周报投影（`orchestration::trait_report` 同账同语义，纯函数）
// ---------------------------------------------------------------------------

const DAY_MS = 86_400_000;

/** 判定实际移动（零移动留痕不算移动；`moved()` 同判据）。 */
export function tuningEntryMoved(entry: TuningLogEntryLike): boolean {
  return Math.abs(entry.next - entry.previous) > 1e-9;
}

/** 锁定跳过留痕判定（reason 前缀同源）。 */
export function tuningEntryLockedSkip(entry: TuningLogEntryLike): boolean {
  return entry.reason.startsWith(TRAIT_LOCKED_SKIP_PREFIX);
}

/** 归一一行账（坏行读作不存在——与账面扫描语义一致，不静默编值）。 */
export function normalizeTuningEntry(raw: unknown): TuningLogEntryLike | null {
  if (!raw || typeof raw !== 'object') return null;
  const rec = raw as Record<string, unknown>;
  const {seq, param, previous, next, reason, at_epoch_ms: at} = rec;
  if (typeof seq !== 'number' || !Number.isFinite(seq)) return null;
  if (typeof param !== 'string' || param === '') return null;
  if (typeof previous !== 'number' || !Number.isFinite(previous)) return null;
  if (typeof next !== 'number' || !Number.isFinite(next)) return null;
  if (typeof reason !== 'string') return null;
  if (typeof at !== 'number' || !Number.isFinite(at)) return null;
  return {seq, param, previous, next, reason, at_epoch_ms: at};
}

export interface TraitSliderWeek {
  param: string;
  label: string;
  netDelta: number;
  moveCount: number;
  lockedSkipped: number;
  lastMove: TraitMoveLike | null;
}

/** 一周性格周报（§10.9B 同语义投影；value/locked 属自述节真值，另由视图注入）。 */
export interface TraitWeekReport {
  windowDays: number;
  windowFromMs: number;
  windowToMs: number;
  generatedAtMs: number;
  sliders: TraitSliderWeek[];
  recordsInWindow: number;
  lockedSkipped: number;
  topReasons: Array<{reason: string; count: number}>;
}

function toMove(entry: TuningLogEntryLike): TraitMoveLike {
  return {
    param: entry.param,
    label: traitSliderLabel(entry.param),
    at_epoch_ms: entry.at_epoch_ms,
    delta: entry.next - entry.previous,
    reason: entry.reason,
  };
}

/**
 * 成长计既有账 → 7 天窗周报（`TraitReport::build` 同口径）：窗 `[now-7d, now]`
 * 两端含、未来时刻不计；net_delta = Σ(next-previous)；零移动留痕与锁定跳过不计
 * 移动（后者单列 locked_skipped）；top_reasons 只数实际移动，计数降序 + 同数
 * 字典序升序、5 条封顶。纯函数：同输入同输出。
 */
export function traitWeekFromTuningLog(
  entries: readonly TuningLogEntryLike[],
  nowMs: number,
): TraitWeekReport {
  const windowFromMs = nowMs - TRAIT_WINDOW_DAYS * DAY_MS;
  const inWindow = entries.filter(
    (entry) => entry.at_epoch_ms >= windowFromMs && entry.at_epoch_ms <= nowMs,
  );

  const reasonCounts = new Map<string, number>();
  let lockedSkippedTotal = 0;
  for (const entry of inWindow) {
    if (tuningEntryLockedSkip(entry)) {
      lockedSkippedTotal += 1;
      continue;
    }
    if (tuningEntryMoved(entry)) {
      reasonCounts.set(entry.reason, (reasonCounts.get(entry.reason) ?? 0) + 1);
    }
  }

  const sliders: TraitSliderWeek[] = TRAIT_SLIDERS.map(({param, label}) => {
    let netDelta = 0;
    let moveCount = 0;
    let lockedSkipped = 0;
    let lastMove: {entry: TuningLogEntryLike; move: TraitMoveLike} | null = null;
    for (const entry of inWindow) {
      if (entry.param !== param) continue;
      if (tuningEntryLockedSkip(entry)) {
        lockedSkipped += 1;
        continue;
      }
      if (tuningEntryMoved(entry)) {
        netDelta += entry.next - entry.previous;
        moveCount += 1;
        const candidate = {entry, move: toMove(entry)};
        const newer =
          lastMove === null ||
          candidate.entry.at_epoch_ms > lastMove.entry.at_epoch_ms ||
          (candidate.entry.at_epoch_ms === lastMove.entry.at_epoch_ms &&
            candidate.entry.seq > lastMove.entry.seq);
        if (newer) lastMove = candidate;
      }
    }
    return {
      param,
      label,
      netDelta,
      moveCount,
      lockedSkipped,
      lastMove: lastMove === null ? null : lastMove.move,
    };
  });

  const topReasons = [...reasonCounts.entries()]
    .map(([reason, count]) => ({reason, count}))
    .sort((a, b) => (b.count - a.count) || (a.reason < b.reason ? -1 : a.reason > b.reason ? 1 : 0))
    .slice(0, TRAIT_TOP_REASONS);

  return {
    windowDays: TRAIT_WINDOW_DAYS,
    windowFromMs,
    windowToMs: nowMs,
    generatedAtMs: nowMs,
    sliders,
    recordsInWindow: inWindow.length,
    lockedSkipped: lockedSkippedTotal,
    topReasons,
  };
}

/**
 * 「最近移动」（`recent_moves` 同口径）：最近 `limit` 条实际移动（新 → 旧；
 * 未来时刻不计、零移动留痕不计）。与周报 last_move 同一账本同一判据。
 */
export function recentTraitMoves(
  entries: readonly TuningLogEntryLike[],
  nowMs: number,
  limit: number = TRAIT_RECENT_MOVES,
): TraitMoveLike[] {
  return entries
    .filter((entry) => entry.at_epoch_ms <= nowMs && tuningEntryMoved(entry))
    .sort((a, b) => b.at_epoch_ms - a.at_epoch_ms || b.seq - a.seq)
    .slice(0, limit)
    .map(toMove);
}

// ---------------------------------------------------------------------------
// 锁定/解锁指引（CLI 显式动作；无 HTTP 管理面 → 只读 + 指引，按钮不代跑）
// ---------------------------------------------------------------------------

export interface TraitGuidanceRow {
  key: string;
  label: string;
  command: string;
  desc: string;
}

/** 单杆锁定/解锁命令（按钮 = 复制指引，不执行）。 */
export function traitLockCommand(action: 'lock' | 'unlock', param: string): string {
  return `apeireth trait ${action} ${param}`;
}

export const TRAIT_LOCK_GUIDANCE: ReadonlyArray<TraitGuidanceRow> = [
  {
    key: 'lock',
    label: '锁定滑杆（显式动作）',
    command: 'apeireth trait lock <SLIDER>',
    desc: 'SLIDER ∈ memory_fade / curiosity_strength / tone_saturation / consolidation_cadence。锁定后成长沉积跳过该杆并留审计痕（reason 前缀「已锁定, 跳过沉积」）。',
  },
  {
    key: 'unlock',
    label: '解锁滑杆（显式动作）',
    command: 'apeireth trait unlock <SLIDER>',
    desc: '解锁是显式动作，不隐式回弹；缺档 = 全解锁默认态，坏档拒绝即帧。',
  },
  {
    key: 'report',
    label: '看周报（只读，零审批）',
    command: 'apeireth trait report',
    desc: '性格周报官方读面（近 7 天四杆移动量/原因 top/锁定态），与本卡周报区块同一本账。',
  },
];

// ---------------------------------------------------------------------------
// 诚实口径常量
// ---------------------------------------------------------------------------

/** 读面缺席（501/404）时锁定态的诚实空态口径（不拿「未锁定」冒充未知）。 */
export const TRAIT_LOCK_NO_FACE =
  '锁定态暂无接口（该部署无 GET /v1/panel/traits）——锁定态如实显「—」，不拿「未锁定」冒充未知；锁定/解锁走 CLI apeireth trait lock|unlock <SLIDER>（面板按钮只给指引）';

export const TRAIT_WEEK_NOTE =
  '周报 = 成长计既有账（tuning-log.jsonl）近 7 天窗投影（与 tool.trait_report 同账同语义：两端含、未来时刻不计、零移动留痕不计移动）；读面 = GET /v1/panel/traits（同账同源），权威读面 = tool.trait_report / apeireth trait report';

export const TRAIT_VALUES_NOTE =
  '现值 = 面板读面真值（GET /v1/panel/traits：TuningParam 生效值 = 进程内自动调整 > env 旋钮 > 基线，与 tool.self_status 调参节 / tool.trait_report 同账）';

// ---------------------------------------------------------------------------
// 卡片视图映射（纯函数；null → 「—」，未知不用 0 冒充）
// ---------------------------------------------------------------------------

/** 带真因注记的两列行。 */
export interface TraitRowView {
  key: string;
  label: string;
  value: string;
  note: string;
}

export interface TraitSliderRowView {
  param: string;
  label: string;
  value: string;
  /** '已锁定' / '未锁定' / '—'（缺源不冒充）。 */
  locked: string;
  lockedNote: string;
  lastMove: string;
  lastMoveNote: string;
}

export interface TraitWeekRowView {
  param: string;
  label: string;
  netDelta: string;
  moveCount: string;
  lockedSkipped: string;
  lastMove: string;
}

export interface TraitMoveRowView {
  param: string;
  label: string;
  delta: string;
  reason: string;
  at: string;
}

export interface TraitWeekView {
  title: string;
  window: string;
  rows: TraitWeekRowView[];
  topReasons: Array<{reason: string; count: string}>;
  totals: TraitRowView[];
  note: string;
}

export interface TraitCardView {
  sliderRows: TraitSliderRowView[];
  recentMoveRows: TraitMoveRowView[];
  week: TraitWeekView;
  lockGuidance: ReadonlyArray<TraitGuidanceRow>;
  note: string;
}

export interface TraitCardInput {
  values?: TraitValuesLike | null;
  locks?: TraitLocksLike | null;
  /** 成长计既有账（桌面 IPC readTuningLog 真源）；null = 读不到账（如实，不编数）。 */
  moves?: readonly TuningLogEntryLike[] | null;
  /** 账面坏行跳过计数（`skipped_records`）；读面缺席 = null → 「—」。 */
  skippedRecords?: number | null;
  nowMs?: number;
}

/** 数值 → 展示串（最多 3 位小数、去尾零；null → 「—」）。 */
export function formatTraitNumber(value: number | null | undefined, signed = false): string {
  if (typeof value !== 'number' || !Number.isFinite(value)) return DASH;
  const text = String(Math.round(value * 1000) / 1000);
  return signed && value > 0 ? `+${text}` : text;
}

export function formatUtcMinute(atMs: number | null | undefined): string {
  if (typeof atMs !== 'number' || !Number.isFinite(atMs)) return DASH;
  const d = new Date(atMs);
  const pad = (n: number): string => (n < 10 ? `0${n}` : String(n));
  return (
    `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ` +
    `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`
  );
}

/**
 * 性格卡视图映射：四滑杆（现值/锁定态/最近移动）+ 最近移动列表 + 本周性格周报。
 * 账本缺席（moves === null）时周报区块如实显「—」（不拿 0 冒充空账的 0）；
 * 空账（moves === []）按空周报如实报 0（`empty_ledger_reports_zeros` 同口径）。
 */
export function traitCardView(input: TraitCardInput): TraitCardView {
  const nowMs = input.nowMs ?? Date.now();
  const values = input.values ?? null;
  const locks = input.locks ?? null;
  const moves = input.moves ?? null;

  const week: TraitWeekReport | null =
    moves === null ? null : traitWeekFromTuningLog(moves, nowMs);
  const recent: TraitMoveLike[] = moves === null ? [] : recentTraitMoves(moves, nowMs);

  const sliderRows: TraitSliderRowView[] = TRAIT_SLIDERS.map(({param, label}) => {
    const value = values === null ? null : (values[param] ?? null);
    const locked = locks === null ? null : (locks[param] ?? null);
    const latest = recent.find((move) => move.param === param) ?? null;
    return {
      param,
      label,
      value: formatTraitNumber(value),
      locked: locked === null ? DASH : locked ? '已锁定' : '未锁定',
      lockedNote: locked === null ? TRAIT_LOCK_NO_FACE : '',
      lastMove: latest === null ? DASH : `${formatUtcMinute(latest.at_epoch_ms)} ${formatTraitNumber(latest.delta, true)}`,
      lastMoveNote:
        latest === null
          ? moves === null
            ? '账本读不到（如实「—」）'
            : '账内暂无实际移动（零移动留痕不计）'
          : latest.reason,
    };
  });

  const recentMoveRows: TraitMoveRowView[] = recent.map((move) => ({
    param: move.param,
    label: move.label,
    delta: formatTraitNumber(move.delta, true),
    reason: move.reason,
    at: formatUtcMinute(move.at_epoch_ms),
  }));

  const weekRows: TraitWeekRowView[] = (week?.sliders ?? TRAIT_SLIDERS.map(({param, label}) => ({
    param,
    label,
    netDelta: 0,
    moveCount: 0,
    lockedSkipped: 0,
    lastMove: null,
  }))).map((row) => ({
    param: row.param,
    label: row.label,
    netDelta: week === null ? DASH : formatTraitNumber(row.netDelta, true),
    moveCount: week === null ? DASH : String(row.moveCount),
    lockedSkipped: week === null ? DASH : String(row.lockedSkipped),
    lastMove: week === null ? DASH : row.lastMove === null ? DASH : formatUtcMinute(row.lastMove.at_epoch_ms),
  }));

  const weekView: TraitWeekView = {
    title: '本周性格周报',
    window:
      week === null
        ? DASH
        : `${formatUtcMinute(week.windowFromMs)} ~ ${formatUtcMinute(week.windowToMs)}（UTC，两端含）`,
    rows: weekRows,
    topReasons: (week?.topReasons ?? []).map((row) => ({
      reason: row.reason,
      count: week === null ? DASH : String(row.count),
    })),
    totals: [
      {
        key: 'records_in_window',
        label: '窗内账面记录',
        value: week === null ? DASH : String(week.recordsInWindow),
        note: week === null ? '账本读不到（如实「—」，不编数）' : '含零移动留痕',
      },
      {
        key: 'locked_skipped',
        label: '锁定跳过留痕',
        value: week === null ? DASH : String(week.lockedSkipped),
        note: '',
      },
      {
        key: 'skipped_records',
        label: '账面坏行跳过',
        value: typeof input.skippedRecords === 'number' ? String(input.skippedRecords) : DASH,
        note: typeof input.skippedRecords === 'number' ? '' : '坏行计数无读面（如实「—」，不静默吞）',
      },
    ],
    note: week === null ? '成长计账本读不到（非桌面版/读取失败）——如实「—」，不拿 0 冒充空账的 0' : TRAIT_WEEK_NOTE,
  };

  return {
    sliderRows,
    recentMoveRows,
    week: weekView,
    lockGuidance: TRAIT_LOCK_GUIDANCE,
    note: TRAIT_VALUES_NOTE,
  };
}
