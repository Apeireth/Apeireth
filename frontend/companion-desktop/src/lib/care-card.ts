// 关怀卡逻辑面（V3-a 双引擎关怀上屏）：关怀规则列表（四类触发器/参数/最近触发/
// 下次触发）+ 打扰纪律（勿扰窗口/频率上限/专注抑制）**只读展示** + 心跳态两列
// （下次 poll / 最近命中）。
//
// 合同出处（镜像消费，不改写）：
//   - `tool.self_status` 关怀节（`crates/capabilities/tools/src/self_status.rs`
//     `CareStatus`）：rules / next_trigger_at_ms / last_care_at_ms / last_care_note /
//     heartbeat_next_poll_at_ms / heartbeat_last_hit_at_ms / reason —— 缺源显式
//     null + reason（0 装诚实）；心跳两列 null = 生产者未运行/未接线，绝不冒充在跑；
//   - `tool.care_schedule` `care_list` 帧（`crates/engine/organ/src/care/tool.rs`）：
//     rules[] {id, trigger{kind,…}, trigger_kind, material, enabled, created_at_ms} +
//     discipline{quiet_start_minutes, quiet_end_minutes, frequency_max,
//     frequency_window_hours, daily_max, focus_gap_minutes, catchup_minutes} + last_care；
//   - 触发器四类闭合词表与语义（`organ/care/rules.rs`：`CareTrigger::describe()`
//     文案同源；每规则「下次触发」按 `next_occurrence_ms` 同口径推算，UTC 公历
//     算法同 `emergence.rs`；交互锚点缺位时同口径退化——只用发起锚/创建锚）。
//
// 只读纪律（零装边界）：本文件零动作面——配置改写走 `tool.care_schedule`
// （care_add/care_remove = 改规则 = 写文件同族，每调用停人工审批，无自动放行档）；
// 面板写面可后续。指引只登记命令文本，界面不代跑。
//
// 取数缝（诚实边界）：关怀读面当前只在工具面自述（面板无 HTTP/IPC 读面，读码
// 实证——gateway 路由无 /v1/panel/care 族、桌面桥无 care 读命令）——卡片经注入缝
// 收 CareSnapshot；读面缺席 = 诚实空态（CARE_NO_FACE），不硬造端点、不摆假数据。
// 读面落地后一行换源（同技能书架先例）。

import {DASH} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 合同形状（镜像 Rust 侧 DTO，snake_case 字段名照收，不改写）
// ---------------------------------------------------------------------------

/** 触发器四类闭合词表（`CareTrigger` 稳定词，词外拒绝）。 */
export type CareTriggerKind = 'inactive' | 'daily_moment' | 'anniversary' | 'interval';

export const CARE_TRIGGER_KINDS: ReadonlyArray<CareTriggerKind> = [
  'inactive',
  'daily_moment',
  'anniversary',
  'interval',
];

/** 触发器中文名（展示用；词表外的词原样上屏，不改写不就近映射）。 */
export const CARE_TRIGGER_LABELS: Record<CareTriggerKind, string> = {
  inactive: '久未联系',
  daily_moment: '作息时刻',
  anniversary: '纪念日',
  interval: '间隔循环',
};

/** 触发器（`CareTrigger` 内联标签形状：{kind, hours?, minutes_of_day?, month?, day?}）。 */
export interface CareTriggerLike {
  kind: string;
  hours?: number;
  minutes_of_day?: number;
  month?: number;
  day?: number;
}

/** 规则行（`rule_json` + `care_rules.json` fired_at 的每规则最近发起）。 */
export interface CareRuleLike {
  id: string;
  trigger: CareTriggerLike;
  trigger_kind?: string;
  material?: string;
  enabled?: boolean;
  created_at_ms: number;
  /** 每规则最近发起时刻（`care_rules.json` `fired_at[rule_id]`）；缺源 = null。 */
  last_fired_at_ms?: number | null;
  /** 每规则下次触发真值；缺源 = null（由本卡按触发器语义推算，note 如实标注）。 */
  next_trigger_at_ms?: number | null;
}

/** 打扰纪律配置（`care_list` 帧 discipline 节；显式 null = 关窗）。 */
export interface CareDisciplineLike {
  quiet_start_minutes?: number | null;
  quiet_end_minutes?: number | null;
  frequency_max?: number;
  frequency_window_hours?: number;
  daily_max?: number;
  focus_gap_minutes?: number;
  catchup_minutes?: number;
}

/** 关怀台账一行（`record_json`）。 */
export interface CareRecordLike {
  rule_id: string;
  at_ms: number;
  judge: string;
  spoke: boolean;
  note: string;
}

/** `tool.self_status` 关怀节（缺源显式 null + reason）。 */
export interface CareStatusLike {
  rules?: number | null;
  next_trigger_at_ms?: number | null;
  last_care_at_ms?: number | null;
  last_care_note?: string | null;
  heartbeat_next_poll_at_ms?: number | null;
  heartbeat_last_hit_at_ms?: number | null;
  reason?: string | null;
}

/** 关怀卡取数缝的快照（工具面两帧同源拼装；每块都可缺席）。 */
export interface CareSnapshot {
  status: CareStatusLike;
  rules?: CareRuleLike[] | null;
  discipline?: CareDisciplineLike | null;
  lastCare?: CareRecordLike | null;
}

// ---------------------------------------------------------------------------
// 诚实口径常量
// ---------------------------------------------------------------------------

/** 读面缺席时的诚实空态口径（不硬造端点，不摆假旋钮）。 */
export const CARE_NO_FACE =
  '关怀读面暂无接口（关怀数据只在工具面自述：tool.self_status 关怀节 / tool.care_schedule care_list）——如实上屏，不硬造端点、不摆假数据';

/** 只读纪律注（写面归口工具，面板零写面）。 */
export const CARE_READ_ONLY_NOTE =
  '本卡只读展示 + 指引：不代跑任何写面。配置改写走 tool.care_schedule（care_add/care_remove，每调用停人工审批）；apeireth CLI 暂无 care 子命令（读码实证），面板写面可后续。';

// ---------------------------------------------------------------------------
// 触发器语义（`CareTrigger::describe()` / `next_occurrence_ms` 同口径镜像）
// ---------------------------------------------------------------------------

const HOUR_MS = 3_600_000;
const DAY_MS = 86_400_000;

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** 分钟 of day（0..1439）→ `HH:mm`（与 `CareTrigger::format_hhmm` 同口径）。 */
export function formatMinutesOfDay(minutes: number): string {
  const clamped = Math.min(1439, Math.max(0, Math.floor(minutes)));
  return `${pad2(Math.floor(clamped / 60))}:${pad2(clamped % 60)}`;
}

/** epoch 毫秒 → 当天分钟 of day（UTC，`minutes_of_day_from_epoch_ms` 同口径）。 */
export function minutesOfDayFromEpochMs(atMs: number): number {
  const mins = Math.floor(((atMs % DAY_MS) + DAY_MS) % DAY_MS / 60_000);
  return Math.min(1439, Math.max(0, mins));
}

/** epoch 毫秒 → `YYYY-MM-DD HH:mm`（UTC 面板时间口径，与触发器 UTC 公历一致）。 */
export function formatUtcTime(atMs: number | null | undefined): string {
  if (typeof atMs !== 'number' || !Number.isFinite(atMs)) return DASH;
  const d = new Date(atMs);
  return (
    `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}-${pad2(d.getUTCDate())} ` +
    `${pad2(d.getUTCHours())}:${pad2(d.getUTCMinutes())}`
  );
}

/**
 * 触发器语义描述（`CareTrigger::describe()` 文案逐字同源）。
 * 词表外/参数缺位的触发器原样报真因，不编语义。
 */
export function careTriggerDescribe(trigger: CareTriggerLike): string {
  const kind = trigger?.kind ?? '';
  if (kind === 'inactive' && typeof trigger.hours === 'number') {
    return `久未联系 ${trigger.hours} 小时无交互`;
  }
  if (kind === 'daily_moment' && typeof trigger.minutes_of_day === 'number') {
    return `每日作息时刻 ${formatMinutesOfDay(trigger.minutes_of_day)}`;
  }
  if (
    kind === 'anniversary' &&
    typeof trigger.month === 'number' &&
    typeof trigger.day === 'number'
  ) {
    return `纪念日 ${pad2(trigger.month)}-${pad2(trigger.day)}`;
  }
  if (kind === 'interval' && typeof trigger.hours === 'number') {
    return `间隔循环 每 ${trigger.hours} 小时`;
  }
  return `触发器参数缺位（kind=${kind === '' ? DASH : kind}）`;
}

/**
 * 每规则「下次触发」推算（`CareTrigger::next_occurrence_ms` 同口径；交互锚点
 * 缺位时同口径退化 = 只用最近发起锚/创建锚）。UTC 公历算法与 `emergence.rs` 一致。
 * 参数缺位的触发器返回 null（不编时刻）。
 */
export function careNextTriggerAtMs(rule: CareRuleLike, nowMs: number): number | null {
  const trigger = rule?.trigger;
  if (!trigger || typeof trigger.kind !== 'string') return null;
  const anchor = typeof rule.created_at_ms === 'number' ? rule.created_at_ms : nowMs;
  const lastFired = typeof rule.last_fired_at_ms === 'number' ? rule.last_fired_at_ms : null;
  switch (trigger.kind) {
    case 'inactive': {
      if (typeof trigger.hours !== 'number') return null;
      const reference = Math.max(lastFired ?? anchor, anchor);
      return reference + trigger.hours * HOUR_MS;
    }
    case 'interval': {
      if (typeof trigger.hours !== 'number') return null;
      return (lastFired ?? anchor) + trigger.hours * HOUR_MS;
    }
    case 'daily_moment': {
      if (typeof trigger.minutes_of_day !== 'number') return null;
      const todayStart = nowMs - minutesOfDayFromEpochMs(nowMs) * 60_000;
      const target = todayStart + trigger.minutes_of_day * 60_000;
      return target >= nowMs ? target : target + DAY_MS;
    }
    case 'anniversary': {
      if (typeof trigger.month !== 'number' || typeof trigger.day !== 'number') return null;
      const now = new Date(nowMs);
      const year = now.getUTCFullYear();
      const thisYear = Date.UTC(year, trigger.month - 1, trigger.day);
      const passed =
        now.getUTCMonth() + 1 > trigger.month ||
        (now.getUTCMonth() + 1 === trigger.month && now.getUTCDate() > trigger.day);
      return passed ? Date.UTC(year + 1, trigger.month - 1, trigger.day) : thisYear;
    }
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------
// 卡片视图映射（纯函数；null → 「—」，未知不用 0 冒充）
// ---------------------------------------------------------------------------

/** 两列/规则行之外的通用行（值位「—」= 缺源，note 如实写真因）。 */
export interface CareRowView {
  key: string;
  label: string;
  value: string;
  note: string;
}

export interface CareRuleRowView {
  id: string;
  /** 稳定词（原文上屏，词表外不改写）。 */
  kind: string;
  kindLabel: string;
  trigger: string;
  material: string;
  enabled: string;
  lastFired: string;
  lastFiredNote: string;
  nextTrigger: string;
  nextTriggerNote: string;
}

export interface CareWriteGuidanceRow {
  key: string;
  label: string;
  command: string;
  desc: string;
}

/** 写面指引（命令文本可复制；界面不代跑）。 */
export const CARE_WRITE_GUIDANCE: ReadonlyArray<CareWriteGuidanceRow> = [
  {
    key: 'add',
    label: '增规则（写面，停人工审批）',
    command:
      'tool.care_schedule {"action":"care_add","id":"<规则id>","trigger":{"kind":"inactive|daily_moment|anniversary|interval", …},"material":"<素材>"}',
    desc: '触发器四类闭合词表：inactive/interval 带 hours；daily_moment 带 minutes_of_day（0..1439）；anniversary 带 month+day。改规则 = 写文件同族，每调用停人工审批。',
  },
  {
    key: 'remove',
    label: '删规则（写面，停人工审批）',
    command: 'tool.care_schedule {"action":"care_remove","id":"<规则id>"}',
    desc: '停用不删档（enabled=false 留档）；删除走本动作，同样停人工审批。',
  },
  {
    key: 'list',
    label: '列规则（工具面只读）',
    command: 'tool.care_schedule {"action":"care_list"}',
    desc: '列规则 + 打扰纪律 + 最近关怀台账；随双面能力同档（本能力无免审动作，宁严勿松）。',
  },
];

export interface CareCardView {
  /** false = 快照缺席（工具面之外无读面，如实空态）。 */
  available: boolean;
  ruleRows: CareRuleRowView[];
  disciplineRows: CareRowView[];
  /** 恒两行：下次 poll / 最近命中（心跳态两列）。 */
  heartbeatRows: CareRowView[];
  /** 规则数 / 下次触发 / 最近关怀（self_status 关怀节自述）。 */
  summaryRows: CareRowView[];
  note: string;
  writeGuidance: ReadonlyArray<CareWriteGuidanceRow>;
}

function dashRow(key: string, label: string, note: string): CareRowView {
  return {key, label, value: DASH, note};
}

/** 单条规则行投影（最近触发/下次触发缺源显「—」+ 真因，不编时刻）。 */
export function careRuleRow(rule: CareRuleLike, nowMs: number): CareRuleRowView {
  const kind = typeof rule.trigger?.kind === 'string' ? rule.trigger.kind : '';
  const lastFired = typeof rule.last_fired_at_ms === 'number' ? rule.last_fired_at_ms : null;
  const providedNext = typeof rule.next_trigger_at_ms === 'number' ? rule.next_trigger_at_ms : null;
  const computedNext = providedNext === null ? careNextTriggerAtMs(rule, nowMs) : null;
  return {
    id: rule.id,
    kind,
    kindLabel: CARE_TRIGGER_LABELS[kind as CareTriggerKind] ?? (kind === '' ? DASH : kind),
    trigger: careTriggerDescribe(rule.trigger ?? {kind}),
    material: rule.material ? rule.material : DASH,
    enabled: rule.enabled === false ? '停用（留档）' : rule.enabled === true ? '启用' : DASH,
    lastFired: formatUtcTime(lastFired),
    lastFiredNote: lastFired === null ? '尚无发起记录（缺源如实）' : '',
    nextTrigger: formatUtcTime(providedNext ?? computedNext),
    nextTriggerNote:
      providedNext !== null
        ? '真值（读面给定）'
        : computedNext !== null
          ? '语义推算（next_occurrence_ms 同口径；交互锚点缺位同口径退化）'
          : '触发器参数缺位，不编时刻',
  };
}

/**
 * 关怀卡视图映射。`snapshot` 为 null = 读面缺席 → 诚实空态（全「—」+ 真因）；
 * 有快照按真值渲染，逐缺位显「—」，reason 原样带出。
 */
export function careCardView(snapshot: CareSnapshot | null, nowMs: number = Date.now()): CareCardView {
  if (snapshot === null) {
    return {
      available: false,
      ruleRows: [],
      disciplineRows: [
        dashRow('quiet', '勿扰窗口', CARE_NO_FACE),
        dashRow('frequency', '频率上限', CARE_NO_FACE),
        dashRow('focus', '专注抑制', CARE_NO_FACE),
        dashRow('catchup', '作息补发窗口', CARE_NO_FACE),
      ],
      heartbeatRows: [
        dashRow('heartbeat_next_poll', '心跳 · 下次 poll', CARE_NO_FACE),
        dashRow('heartbeat_last_hit', '心跳 · 最近命中', CARE_NO_FACE),
      ],
      summaryRows: [
        dashRow('rules', '关怀规则数', CARE_NO_FACE),
        dashRow('next_trigger', '下次触发', CARE_NO_FACE),
        dashRow('last_care', '最近关怀', CARE_NO_FACE),
      ],
      note: CARE_NO_FACE,
      writeGuidance: CARE_WRITE_GUIDANCE,
    };
  }

  const status = snapshot.status ?? {};
  const discipline = snapshot.discipline ?? null;
  const reason = typeof status.reason === 'string' && status.reason !== '' ? status.reason : '';

  const ruleRows = (snapshot.rules ?? []).map((rule) => careRuleRow(rule, nowMs));

  const disciplineRows: CareRowView[] = [
    (() => {
      const start = discipline?.quiet_start_minutes;
      const end = discipline?.quiet_end_minutes;
      if (start === null || end === null) {
        return {
          key: 'quiet',
          label: '勿扰窗口',
          value: '未开勿扰窗口（显式关窗）',
          note: '显式 null = 关窗（缺省默认窗 23:00-08:00 由后端配置面持有）',
        };
      }
      if (typeof start === 'number' && typeof end === 'number') {
        return {
          key: 'quiet',
          label: '勿扰窗口',
          value: `${formatMinutesOfDay(start)}-${formatMinutesOfDay(end)}`,
          note: start > end ? '跨午夜（起 > 止）' : '',
        };
      }
      return dashRow('quiet', '勿扰窗口', '打扰纪律配置缺位（如实「—」）');
    })(),
    typeof discipline?.frequency_window_hours === 'number' &&
      typeof discipline?.frequency_max === 'number'
      ? {
          key: 'frequency',
          label: '频率上限',
          value: `每 ${discipline.frequency_window_hours} 小时 ≤ ${discipline.frequency_max} 次${
            typeof discipline.daily_max === 'number' ? ` · 每日 ≤ ${discipline.daily_max} 次` : ''
          }`,
          note: '滚动窗口档与日档同时生效，满了拦下不丢',
        }
      : dashRow('frequency', '频率上限', '打扰纪律配置缺位（如实「—」）'),
    typeof discipline?.focus_gap_minutes === 'number'
      ? {
          key: 'focus',
          label: '专注抑制',
          value: `连续交互 ${discipline.focus_gap_minutes} 分钟内不插话`,
          note: '',
        }
      : dashRow('focus', '专注抑制', '打扰纪律配置缺位（如实「—」）'),
    typeof discipline?.catchup_minutes === 'number'
      ? {
          key: 'catchup',
          label: '作息补发窗口',
          value: `过点 ${discipline.catchup_minutes} 分钟内补发`,
          note: '超窗当日跳过',
        }
      : dashRow('catchup', '作息补发窗口', '打扰纪律配置缺位（如实「—」）'),
  ];

  const heartbeatRows: CareRowView[] = [
    typeof status.heartbeat_next_poll_at_ms === 'number'
      ? {
          key: 'heartbeat_next_poll',
          label: '心跳 · 下次 poll',
          value: formatUtcTime(status.heartbeat_next_poll_at_ms),
          note: '',
        }
      : dashRow('heartbeat_next_poll', '心跳 · 下次 poll', '心跳生产者未运行/未接线（如实，不冒充在跑）'),
    typeof status.heartbeat_last_hit_at_ms === 'number'
      ? {
          key: 'heartbeat_last_hit',
          label: '心跳 · 最近命中',
          value: formatUtcTime(status.heartbeat_last_hit_at_ms),
          note: '',
        }
      : dashRow('heartbeat_last_hit', '心跳 · 最近命中', '尚无心跳命中'),
  ];

  const summaryRows: CareRowView[] = [
    typeof status.rules === 'number'
      ? {key: 'rules', label: '关怀规则数', value: String(status.rules), note: ''}
      : dashRow('rules', '关怀规则数', '探测口缺位（null 如实，不编 0）'),
    typeof status.next_trigger_at_ms === 'number'
      ? {
          key: 'next_trigger',
          label: '下次触发',
          value: formatUtcTime(status.next_trigger_at_ms),
          note: '无启用规则时后端如实给 null',
        }
      : dashRow('next_trigger', '下次触发', '无启用规则或探测口缺位（null 如实）'),
    (() => {
      const lastCareAt = typeof status.last_care_at_ms === 'number' ? status.last_care_at_ms : null;
      const note =
        typeof status.last_care_note === 'string' && status.last_care_note !== ''
          ? status.last_care_note
          : lastCareAt === null
            ? '尚无关怀记录'
            : '';
      return lastCareAt === null
        ? dashRow('last_care', '最近关怀', note)
        : {key: 'last_care', label: '最近关怀', value: formatUtcTime(lastCareAt), note};
    })(),
  ];

  return {
    available: true,
    ruleRows,
    disciplineRows,
    heartbeatRows,
    summaryRows,
    note: reason,
    writeGuidance: CARE_WRITE_GUIDANCE,
  };
}
