// 预算面板纯逻辑件（无副作用）：回合/上下文预算旋钮的解析与生效值徽标、
// 多维配额真实可配面注册表、预算耗尽固定语义、会话消耗对生效上限的余量条。
//
// 语义同源纪律：旋钮解析与后端 CLI 同一条语义——回合预算两旋钮越界钳制
// 1..=64、未设/非法回默认；上下文字符预算正整数直通、越界/非法回默认（无
// 上界钳制）。生效值徽标沿用自述预算节的诚实语法（source=configured/constant
// + 口径注记）。配额注册表只登记读码确认的真实可配面——没有配置/env 接口的
// 维度一律「暂无接口」，不硬造旋钮。
//
// 本文件零运行时依赖（不 import svelte / DOM），纯函数可被 Node 直接 import
// 测试（tests/budget-knobs.mjs / tests/budget-meter.mjs）。

import {DASH, formatDuration} from './chat-shell/turn-telemetry.ts';
import type {SessionUsageTotals} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 常量（与后端解析路径同源取值）
// ---------------------------------------------------------------------------

/** 单回合轮数上限默认（runtime canonical `DEFAULT_MAX_ROUNDS`）。 */
export const DEFAULT_MAX_ROUNDS = 8;
/** 单轮工具调用上限默认（execute `MAX_TOOL_CALLS_PER_ROUND`）。 */
export const DEFAULT_MAX_TOOL_CALLS = 16;
/** 上下文注入总字符预算默认（runtime `DEFAULT_CONTEXT_BUDGET_CHARS`）。 */
export const DEFAULT_CONTEXT_BUDGET_CHARS = 24000;
/** 回合预算旋钮取值域下界（`MIN_TURN_ROUNDS` / `MIN_TOOL_CALL_LIMIT`）。 */
export const MIN_BUDGET_LIMIT = 1;
/** 回合预算旋钮取值域上界（`MAX_TURN_ROUNDS` / `MAX_TOOL_CALL_LIMIT`）。 */
export const MAX_BUDGET_LIMIT = 64;

// ---------------------------------------------------------------------------
// 生效值徽标（自述预算节同款诚实语法：source=configured/constant + 口径注记）
// ---------------------------------------------------------------------------

export type BudgetSource = 'configured' | 'constant';

/** 口径：读生效中的预算旋钮（env 可配；未设值时回落默认）。 */
export const BUDGET_SOURCE_CONFIGURED: BudgetSource = 'configured';
/** 口径：读编译期常量（旋钮未配置，取常量默认值）。 */
export const BUDGET_SOURCE_CONSTANT: BudgetSource = 'constant';
export const BUDGET_SOURCE_CONFIGURED_NOTE = '读自生效中的预算旋钮（env 可配；未设值时回落默认）';
export const BUDGET_SOURCE_CONSTANT_NOTE = '读编译期常量（旋钮未配置，取常量默认值）';

// ---------------------------------------------------------------------------
// 预算旋钮注册表（三枚真实旋钮：回合预算两枚 + 上下文字符预算一枚）
// ---------------------------------------------------------------------------

export type BudgetKnobKey = 'maxTurnRounds' | 'maxToolCalls' | 'contextBudgetChars';

export interface BudgetKnobSpec {
  key: BudgetKnobKey;
  label: string;
  /** 后端 env 旋钮名（即效写 config→env 注入的同源字段）。 */
  env: string;
  /** 非法/未配置时的默认值（实际生效值的 constant 回落）。 */
  fallback: number;
  min: number;
  /** 取值域上界；null = 无上界钳制。 */
  max: number | null;
  /**
   * 越界处置（与后端解析语义同源）：
   * 'clamp' = 钳到 [min,max]；'reject' = 越界回默认。
   */
  outOfRange: 'clamp' | 'reject';
  desc: string;
}

export const BUDGET_KNOB_SPECS: Record<BudgetKnobKey, BudgetKnobSpec> = {
  maxTurnRounds: {
    key: 'maxTurnRounds',
    label: '轮数上限（每回合）',
    env: 'APEIRETH_MAX_TURN_ROUNDS',
    fallback: DEFAULT_MAX_ROUNDS,
    min: MIN_BUDGET_LIMIT,
    max: MAX_BUDGET_LIMIT,
    outOfRange: 'clamp',
    desc: '单回合逻辑轮数上限（含模块重试占位）。轮预算耗尽收束为 turn_not_converged 错误帧，带真实数字。',
  },
  maxToolCalls: {
    key: 'maxToolCalls',
    label: '单轮工具上限',
    env: 'APEIRETH_MAX_TOOL_CALLS',
    fallback: DEFAULT_MAX_TOOL_CALLS,
    min: MIN_BUDGET_LIMIT,
    max: MAX_BUDGET_LIMIT,
    outOfRange: 'clamp',
    desc: '单轮最多派发多少个工具调用；超出部分截断并补合成结果（不整轮失败）。',
  },
  contextBudgetChars: {
    key: 'contextBudgetChars',
    label: '上下文字符预算',
    env: 'APEIRETH_CONTEXT_BUDGET_CHARS',
    fallback: DEFAULT_CONTEXT_BUDGET_CHARS,
    min: 1,
    max: null,
    outOfRange: 'reject',
    desc: '组装期注入上下文块（记忆注入 / 器官产物 / 教训等）的总字符预算；核心块永不截断，超出按长尾贪心截断。',
  },
};

// ---------------------------------------------------------------------------
// 旋钮输入解析（数字步进器失焦/回车提交语义，与后端 CLI 同源）
// ---------------------------------------------------------------------------

export interface BudgetKnobResolution {
  /** 归一后写入配置的值；null = 未配置（回默认落点，source=constant）。 */
  value: number | null;
  /** 实际生效值（徽标显示）。 */
  effective: number;
  /** ok = 直通；clamped = 越界钳制；fallback = 非法/越界回默认。 */
  status: 'ok' | 'clamped' | 'fallback';
  /** 人话反馈（钳制/回默认时非空，空 = 无需提示）。 */
  feedback: string;
}

/** 整数字面量（镜像后端整数解析：非整数文本一律非法）。 */
const INT_RE = /^[+-]?\d+$/;

function parseIntegerField(raw: string | number | null | undefined): number | null {
  const text =
    typeof raw === 'number'
      ? Number.isFinite(raw) && Number.isInteger(raw)
        ? String(raw)
        : ''
      : (raw ?? '').trim();
  if (!INT_RE.test(text)) return null;
  const parsed = Number(text);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

/**
 * 解析一次旋钮提交：越界按 spec 处置（钳制或回默认），非法一律回默认。
 * 与后端解析逐条对齐：回合预算两旋钮 = 钳到 1..=64、非法回默认；
 * 上下文字符预算 = 正整数直通、越界/非法回默认。
 */
export function resolveBudgetLimitInput(
  raw: string | number | null | undefined,
  spec: BudgetKnobSpec,
): BudgetKnobResolution {
  const parsed = parseIntegerField(raw);
  if (parsed === null) {
    return {
      value: null,
      effective: spec.fallback,
      status: 'fallback',
      feedback: `非法值：已回默认 ${spec.fallback}`,
    };
  }
  const inRange = parsed >= spec.min && (spec.max === null || parsed <= spec.max);
  if (inRange) {
    return {value: parsed, effective: parsed, status: 'ok', feedback: ''};
  }
  if (spec.outOfRange === 'reject') {
    const domain = spec.max === null ? `${spec.min} 起` : `${spec.min}..=${spec.max}`;
    return {
      value: null,
      effective: spec.fallback,
      status: 'fallback',
      feedback: `越界（${domain}）：已回默认 ${spec.fallback}`,
    };
  }
  const clamped = Math.min(spec.max ?? parsed, Math.max(spec.min, parsed));
  return {
    value: clamped,
    effective: clamped,
    status: 'clamped',
    feedback: `越界：已钳制到 ${clamped}（${spec.min}..=${spec.max}）`,
  };
}

// ---------------------------------------------------------------------------
// 旋钮旁实际生效值徽标
// ---------------------------------------------------------------------------

export interface BudgetBadge {
  /** 实际生效值。 */
  effective: number;
  /** 取值口径（自述预算节同款语法）。 */
  source: BudgetSource;
  /** 口径注记。 */
  note: string;
  /** 徽标文案：「生效 8 · constant」。 */
  label: string;
}

function badge(effective: number, source: BudgetSource, note: string): BudgetBadge {
  return {effective, source, note, label: `生效 ${effective} · ${source}`};
}

/**
 * 实际生效值徽标：已配置值经同一解析口径取生效值（越界钳制语义保留），
 * 未配置（或配置值非法）= 回落编译期常量默认值，口径如实报 constant。
 */
export function budgetKnobBadge(
  configured: number | null | undefined,
  spec: BudgetKnobSpec,
): BudgetBadge {
  if (configured === null || configured === undefined) {
    return badge(spec.fallback, BUDGET_SOURCE_CONSTANT, BUDGET_SOURCE_CONSTANT_NOTE);
  }
  const resolution = resolveBudgetLimitInput(configured, spec);
  if (resolution.status === 'fallback' || resolution.value === null) {
    return badge(spec.fallback, BUDGET_SOURCE_CONSTANT, BUDGET_SOURCE_CONSTANT_NOTE);
  }
  return badge(resolution.effective, BUDGET_SOURCE_CONFIGURED, BUDGET_SOURCE_CONFIGURED_NOTE);
}

/** 三枚旋钮的生效值徽标集（仪表与旋钮面共用同一份口径）。 */
export interface EffectiveBudget {
  maxTurnRounds: BudgetBadge;
  maxToolCalls: BudgetBadge;
  contextBudgetChars: BudgetBadge;
}

export function effectiveBudget(
  knobs:
    | {
        maxTurnRounds?: number | null;
        maxToolCalls?: number | null;
        contextBudgetChars?: number | null;
      }
    | null
    | undefined,
): EffectiveBudget {
  return {
    maxTurnRounds: budgetKnobBadge(knobs?.maxTurnRounds ?? null, BUDGET_KNOB_SPECS.maxTurnRounds),
    maxToolCalls: budgetKnobBadge(knobs?.maxToolCalls ?? null, BUDGET_KNOB_SPECS.maxToolCalls),
    contextBudgetChars: budgetKnobBadge(
      knobs?.contextBudgetChars ?? null,
      BUDGET_KNOB_SPECS.contextBudgetChars,
    ),
  };
}

// ---------------------------------------------------------------------------
// 多维配额真实可配面注册表（读码结论；没接口的维度如实「暂无接口」）
// ---------------------------------------------------------------------------

export type QuotaDimensionKey = 'tokens' | 'steps' | 'cost' | 'depth';

export interface QuotaDimensionRow {
  key: QuotaDimensionKey;
  label: string;
  /** 真实可配面：none = 暂无接口（本批读码结论四维全部如此）。 */
  interface: 'none';
  /** 诚实原因（为什么不给旋钮）。 */
  reason: string;
  /** 读码出处（工程诚实：指到真实类型/注记）。 */
  evidence: string;
}

/** 「暂无接口」徽标文案（无接口维度统一用词，禁硬造假旋钮）。 */
export const QUOTA_DIMENSION_STATUS_LABEL = '暂无接口';

export const QUOTA_DIMENSION_KEYS: ReadonlyArray<QuotaDimensionKey> = [
  'tokens',
  'steps',
  'cost',
  'depth',
];

export const QUOTA_DIMENSIONS: ReadonlyArray<QuotaDimensionRow> = [
  {
    key: 'tokens',
    label: 'Token 上限',
    interface: 'none',
    reason:
      '配额类型带该上限字段，但没有配置/env 接线（仅调度器单测构造，生产无调用点）——暂无接口。',
    evidence: 'cognitive_quota_scheduler.rs :: CognitiveQuota.max_tokens',
  },
  {
    key: 'steps',
    label: '步数上限',
    interface: 'none',
    reason:
      '同上：步数上限字段无配置/env 接线。单轮工具调用上限属回合预算旋钮（见上），不是本维配额——暂无接口。',
    evidence: 'cognitive_quota_scheduler.rs :: CognitiveQuota.max_tool_steps',
  },
  {
    key: 'cost',
    label: '花费上限',
    interface: 'none',
    reason: '同上：花费上限字段无配置/env 接线——暂无接口。',
    evidence: 'cognitive_quota_scheduler.rs :: CognitiveQuota.max_cost_micros',
  },
  {
    key: 'depth',
    label: '深度上限',
    interface: 'none',
    reason:
      '深度维度未实施：不跟踪调用深度，该上限字段已按「删除而非留空」移除——暂无接口。',
    evidence: 'cognitive_quota_scheduler.rs 模块诚实注记（无递归深度）',
  },
];

// ---------------------------------------------------------------------------
// 预算耗尽行为（后端无可配置语义 → 不出选择器，只出固定语义说明）
// ---------------------------------------------------------------------------

export interface BudgetExhaustionSemantics {
  /** 后端没有可配置的耗尽行为旋钮 → 设置面不出选择器。 */
  configurable: false;
  /** 固定语义（读码结论），只做说明不做控件。 */
  behaviors: ReadonlyArray<string>;
}

export const BUDGET_EXHAUSTION: BudgetExhaustionSemantics = {
  configurable: false,
  behaviors: [
    '轮预算耗尽：回合收束为 turn_not_converged 错误帧，带真实数字（limit / rounds / pending_tool）。',
    '同轮待批：审批冻结即收束为待批准态，不烧剩余轮预算；同回合重复提议同一待批项直接收束。',
    '单轮工具调用超限：超出的调用被截断并补合成结果，不整轮失败。',
    '多维配额耗尽中断：调度器定义了耗尽中断信号，但调度器未接入生产运行时，无可选行为。',
  ],
};

// ---------------------------------------------------------------------------
// 预算余量条（对生效上限求余量；无上限维度显「—」）
// ---------------------------------------------------------------------------

export interface BudgetRemainingRow {
  key: string;
  /** 维度名。 */
  label: string;
  /** 作用域诚实标注（上限管到哪一口径）。 */
  scope: string;
  /** 生效上限（「16 · constant」；无上限 = 「—」）。 */
  cap: string;
  /** 已耗（「—」= 未测得）。 */
  used: string;
  /** 余量（「—」= 无上限或已耗未测得）。 */
  remaining: string;
  /** 用量占比 0..1（有上限且已耗已知才给；否则 null，不编百分比）。 */
  ratio: number | null;
}

/** 千分位展示（与遥测条同口径，只加组分隔）。 */
function fmtCount(value: number | null | undefined): string {
  return value === undefined || value === null ? DASH : String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

export function budgetRemainingRows(
  totals: SessionUsageTotals,
  budget: EffectiveBudget,
): BudgetRemainingRow[] {
  const noCap = (key: string, label: string, scope: string, used: string): BudgetRemainingRow => ({
    key,
    label,
    scope,
    cap: DASH,
    used,
    remaining: DASH,
    ratio: null,
  });

  // 单轮工具调用：对单轮生效上限求余量（已耗 = 最近一回合计数，实测可得）。
  const toolCap = budget.maxToolCalls;
  const toolUsed = totals.lastTurnToolCalls;
  const toolRemaining = toolUsed === null ? null : Math.max(0, toolCap.effective - toolUsed);

  return [
    noCap('tokenIn', 'Token（入）', '会话累计', fmtCount(totals.promptTokens)),
    noCap('tokenOut', 'Token（出）', '会话累计', fmtCount(totals.completionTokens)),
    noCap('cacheHit', '提示缓存命中', '会话累计', fmtCount(totals.cacheHitTokens)),
    noCap('turns', '回合数', '会话累计', String(totals.turns)),
    noCap('duration', '累计耗时', '会话累计', formatDuration(totals.durationMs)),
    {
      key: 'toolCalls',
      label: '单轮工具调用',
      scope: '最近一回合',
      cap: `${toolCap.effective} · ${toolCap.source}`,
      used: toolUsed === null ? DASH : String(toolUsed),
      remaining: toolRemaining === null ? DASH : String(toolRemaining),
      ratio: toolUsed === null ? null : Math.min(1, toolUsed / toolCap.effective),
    },
    {
      key: 'turnRounds',
      label: '回合轮数',
      scope: '每回合（前端未测得轮数）',
      cap: `${budget.maxTurnRounds.effective} · ${budget.maxTurnRounds.source}`,
      used: DASH,
      remaining: DASH,
      ratio: null,
    },
  ];
}

/** 余量摘要一行（聊天头 hover 详情补行用）：只报有生效上限的维度。 */
export function budgetRemainingSummary(rows: ReadonlyArray<BudgetRemainingRow>): string {
  const capped = rows.filter((row) => row.cap !== DASH);
  if (capped.length === 0) return `无生效上限维度（${DASH}）`;
  return capped.map((row) => `${row.label} 余 ${row.remaining}（上限 ${row.cap}）`).join(' · ');
}
