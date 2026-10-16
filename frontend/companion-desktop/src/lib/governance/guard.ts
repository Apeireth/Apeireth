// 治理卷宗 — 守卫（Behavior-Chain Guard）展示纯逻辑（DOM-free，Node 可直测）
//
// 契约依据：`crates/engine/guard/src/introspection.rs`（GuardStatusDto /
// GuardEventDto / GuardDryRunRequest|Response 逐字段核实）、
// `decision.rs`（GuardStage serde snake_case：fast_guard / chain_guard /
// decision_fusion）；端点 `/v1/safety/guard/status|events|evaluate`。
// 能力门控：safety.guard.status.read / safety.guard.events.read /
// safety.guard.evaluate（网关 capabilities §9 已核实）。
//
// badge 五色语义（全局唯一一套，gap-plan §4.4）：绿=正常 / 蓝=信息 /
// 黄=待完善（此处=需要你签）/ 红=危险 / 灰=禁用。

import type {GuardDryRunResponse, GuardEvent, GuardStatus} from '../types';

export type GuardBadgeVariant = 'green' | 'blue' | 'amber' | 'danger' | 'dim';

/** guard decision 字符串 → badge 语义（后端 decision 是自由字符串，逐值核实映射）。 */
export function guardDecisionBadge(decision: string): {variant: GuardBadgeVariant; label: string} {
  const d = decision.toLowerCase();
  if (d === 'allow' || d === 'allowed') return {variant: 'green', label: '放行'};
  if (d === 'deny' || d === 'denied' || d === 'block' || d === 'blocked')
    return {variant: 'danger', label: '否决'};
  if (d === 'approval_required' || d === 'require_approval' || d === 'approval')
    return {variant: 'amber', label: '待签'};
  if (d === 'warn' || d === 'warning') return {variant: 'amber', label: '预警'};
  if (!d) return {variant: 'dim', label: '未知'};
  return {variant: 'blue', label: decision};
}

/** GuardStage snake_case → 中文名（守卫三段管线）。 */
export function guardStageLabel(stage: string): string {
  switch (stage) {
    case 'fast_guard':
      return '快速守卫';
    case 'chain_guard':
      return '链式守卫';
    case 'decision_fusion':
      return '决策融合';
    default:
      return stage;
  }
}

/** 风险分 0..1 → 0..100 整数百分比；越界钳制（后端是 f64，防御性钳制）。 */
export function riskScorePercent(score: number): number {
  if (!Number.isFinite(score)) return 0;
  return Math.round(Math.min(1, Math.max(0, score)) * 100);
}

/** 守卫总账条（status 卡）的四枚计数。 */
export function guardTally(status: GuardStatus): {
  evaluations: number;
  allowed: number;
  denied: number;
  approvalRequired: number;
} {
  return {
    evaluations: status.total_evaluations,
    allowed: status.total_allowed,
    denied: status.total_denied,
    approvalRequired: status.total_approval_required,
  };
}

/** 守卫事件 → 列表行（时间倒序由后端保证；这里只做展示字段推导）。 */
export function guardEventRow(ev: GuardEvent): {
  id: string;
  badge: {variant: GuardBadgeVariant; label: string};
  stageLabel: string;
  riskPct: number;
} {
  return {
    id: `${ev.trace_id}-${ev.round}-${ev.timestamp_ms}`,
    badge: guardDecisionBadge(ev.decision),
    stageLabel: guardStageLabel(ev.stage),
    riskPct: riskScorePercent(ev.risk_score),
  };
}

// ---------------------------------------------------------------------------
// dry-run 试算表单（POST /v1/safety/guard/evaluate）
// ---------------------------------------------------------------------------

/** arguments 文本框解析：必须是 JSON 对象/数组/字面量；空 = {}（试算不带参）。 */
export function parseDryRunArguments(text: string): {ok: true; value: unknown} | {ok: false; error: string} {
  const trimmed = text.trim();
  if (!trimmed) return {ok: true, value: {}};
  try {
    return {ok: true, value: JSON.parse(trimmed) as unknown};
  } catch (e) {
    return {ok: false, error: `参数不是合法 JSON：${e instanceof Error ? e.message : String(e)}`};
  }
}

/** capability_id 校验：非空、无空白（能力 id 形如 tool.repo / memory.read）。 */
export function validCapabilityId(id: string): boolean {
  return id.trim().length > 0 && !/\s/.test(id.trim());
}

/** dry-run 结果 → 展示行（与 guardEventRow 同一映射纪律）。 */
export function dryRunResultRow(res: GuardDryRunResponse): {
  badge: {variant: GuardBadgeVariant; label: string};
  stageLabel: string;
  riskPct: number;
} {
  return {
    badge: guardDecisionBadge(res.decision),
    stageLabel: guardStageLabel(res.stage),
    riskPct: riskScorePercent(res.risk_score),
  };
}
