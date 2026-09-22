// 治理卷宗纯逻辑单测 — 审批账本 / 守卫映射 / 审计轨迹
// 直接 import 真实实现 ../src/lib/governance/*.ts（Node ≥23.6 类型擦除；
// 三个模块零运行时依赖，DOM-free，与 presence.ts / chat-shell 同一约定）。
// 契约依据: docs/gateway-api-contract.md §6/§7/§8；00-PHILOSOPHY §4（卷宗=账）。
// 不依赖后端进程。
import assert from 'node:assert/strict';

import {
  approvalTimeMs,
  isApprovalExpired,
  mergeSessionInbox,
  applyApprovalEventToLedger,
  approvalFilterCounts,
  filterApprovalLedger,
  nextSseState,
  nextRetryDelay,
  SSE_INITIAL_RETRY_MS,
} from '../src/lib/governance/ledger.ts';
import {
  guardDecisionBadge,
  guardStageLabel,
  riskScorePercent,
  guardTally,
  parseDryRunArguments,
  validCapabilityId,
  dryRunResultRow,
} from '../src/lib/governance/guard.ts';
import {
  spanTreeSort,
  spanDepth,
  spanDurationMs,
  spanStatusBadge,
  traceRowFromSummary,
  traceFilterCounts,
  filterTraceRows,
  auditEventGroup,
  auditGroupChips,
  filterAuditItems,
} from '../src/lib/governance/trace.ts';

console.log('--- Starting Governance Ledger Logic Check ---');

const approval = (id, session, over = {}) => ({
  session,
  approval_id: id,
  request: `req-${id}`,
  trace_id: `trace-${id}`,
  capability_id: 'tool.repo',
  tool_name: 'tool.repo',
  governance_hook: 'pre_execution',
  governance_reason: '高危调用需主人批准',
  command_text: 'repo write',
  arguments_summary: '{}',
  created_at: '2026-10-14T02:00:00Z',
  expires_at: '2026-10-14T02:30:00Z',
  ...over,
});

// ---------------------------------------------------------------------------
// 1. approvalTimeMs — RFC 3339 字符串（线上形状）与 epoch 数字（契约示例）都解析
// ---------------------------------------------------------------------------
{
  assert.equal(approvalTimeMs('2026-10-14T02:00:00Z'), Date.parse('2026-10-14T02:00:00Z'));
  assert.equal(approvalTimeMs(1726992000000), 1726992000000);
  assert.equal(approvalTimeMs('not-a-date'), undefined);
  assert.equal(approvalTimeMs(null), undefined);
  assert.equal(approvalTimeMs(undefined), undefined);
}

// 2. mergeSessionInbox — 该会话旧条目被后端真值整体替换，其他会话不动，按时间倒序
{
  const s1 = approval('a1', 's1');
  const s2 = approval('a2', 's2', {created_at: '2026-10-14T04:00:00Z'});
  let ledger = mergeSessionInbox([], 's1', [s1], '会话一');
  ledger = mergeSessionInbox(ledger, 's2', [s2], '会话二');
  assert.equal(ledger.length, 2);
  assert.equal(ledger[0].approval.approval_id, 'a2'); // 较新的在前
  // s1 inbox 重拉为空 → s1 条目消失（自愈），s2 保留
  ledger = mergeSessionInbox(ledger, 's1', [], '会话一');
  assert.equal(ledger.length, 1);
  assert.equal(ledger[0].approval.approval_id, 'a2');
  assert.equal(ledger[0].sessionTitle, '会话二');
  assert.equal(typeof ledger[0].createdAtMs, 'number');
}

// 排序：较新的在前
{
  const older = approval('old', 's1', {created_at: '2026-10-14T01:00:00Z'});
  const newer = approval('new', 's1', {created_at: '2026-10-14T03:00:00Z'});
  const ledger = mergeSessionInbox([], 's1', [older, newer]);
  assert.deepEqual(ledger.map((e) => e.approval.approval_id), ['new', 'old']);
}

// 3. applyApprovalEventToLedger — resolved 按 approval_id 精确撤下；找不到按 session 撤
{
  const a = approval('a1', 's1');
  const b = approval('b1', 's2');
  const ledger = mergeSessionInbox(mergeSessionInbox([], 's1', [a]), 's2', [b]);

  const r1 = applyApprovalEventToLedger(ledger, {session: 's1', approvalId: 'a1'}, 'approval_resolved');
  assert.equal(r1.entries.length, 1);
  assert.equal(r1.entries[0].approval.approval_id, 'b1');
  assert.deepEqual(r1.dirtySessions, []);

  // approval_id 不在账本（帧与账本错位）→ 按 session 整组撤
  const r2 = applyApprovalEventToLedger(ledger, {session: 's2', approvalId: 'ghost'}, 'approval_resolved');
  assert.equal(r2.entries.length, 1);
  assert.equal(r2.entries[0].approval.approval_id, 'a1');

  // required 不拼假卡——只标记 dirty session 让调用方重拉
  const r3 = applyApprovalEventToLedger(ledger, {session: 's1', approvalId: 'a9'}, 'approval_required');
  assert.equal(r3.entries.length, 2);
  assert.deepEqual(r3.dirtySessions, ['s1']);
}

// 4. 过期判定与过滤 chips 计数
{
  const now = Date.parse('2026-10-14T02:10:00Z');
  const open = approval('open', 's1', {expires_at: '2026-10-14T02:30:00Z'});
  const expired = approval('dead', 's1', {expires_at: '2026-10-14T02:00:00Z'});
  const ledger = mergeSessionInbox([], 's1', [open, expired]);
  const [eOpen, eDead] = [
    ledger.find((e) => e.approval.approval_id === 'open'),
    ledger.find((e) => e.approval.approval_id === 'dead'),
  ];
  assert.equal(isApprovalExpired(eOpen, now), false);
  assert.equal(isApprovalExpired(eDead, now), true);
  assert.deepEqual(approvalFilterCounts(ledger, now), {all: 2, open: 1, expired: 1});
  assert.equal(filterApprovalLedger(ledger, 'open', now).length, 1);
  assert.equal(filterApprovalLedger(ledger, 'expired', now)[0].approval.approval_id, 'dead');
  assert.equal(filterApprovalLedger(ledger, 'all', now).length, 2);
}

// 5. SSE 连接态状态机 — 断线进 reconnecting 不装在线；open 回到 live
{
  assert.equal(nextSseState('connecting', 'open'), 'live');
  assert.equal(nextSseState('live', 'error'), 'reconnecting');
  assert.equal(nextSseState('reconnecting', 'open'), 'live');
  assert.equal(nextSseState('reconnecting', 'error'), 'reconnecting');
  assert.equal(nextSseState('connecting', 'unsupported'), 'unsupported');
  // 退避：×1.5 封顶 30s
  assert.equal(nextRetryDelay(SSE_INITIAL_RETRY_MS), 3000);
  assert.equal(nextRetryDelay(20000), 30000);
  assert.equal(nextRetryDelay(29000), 30000);
}

// ---------------------------------------------------------------------------
// 6. guard.ts — decision/stage/风险分映射
// ---------------------------------------------------------------------------
{
  assert.deepEqual(guardDecisionBadge('allow'), {variant: 'green', label: '放行'});
  assert.deepEqual(guardDecisionBadge('denied'), {variant: 'danger', label: '否决'});
  assert.deepEqual(guardDecisionBadge('approval_required'), {variant: 'amber', label: '待签'});
  assert.equal(guardDecisionBadge('').variant, 'dim');
  assert.equal(guardDecisionBadge('escalate').variant, 'blue'); // 未知决策如实显示原文
  assert.equal(guardDecisionBadge('escalate').label, 'escalate');

  assert.equal(guardStageLabel('fast_guard'), '快速守卫');
  assert.equal(guardStageLabel('chain_guard'), '链式守卫');
  assert.equal(guardStageLabel('decision_fusion'), '决策融合');
  assert.equal(guardStageLabel('other'), 'other');

  assert.equal(riskScorePercent(0.42), 42);
  assert.equal(riskScorePercent(1.7), 100); // 钳制
  assert.equal(riskScorePercent(-0.5), 0);
  assert.equal(riskScorePercent(NaN), 0);

  const tally = guardTally({
    enabled: true, fast_guard_active: true, chain_guard_active: true,
    active_chains: 2, total_evaluations: 10, total_allowed: 7,
    total_denied: 1, total_approval_required: 2, dataset_recording_enabled: false,
  });
  assert.deepEqual(tally, {evaluations: 10, allowed: 7, denied: 1, approvalRequired: 2});
}

// 7. dry-run 表单解析 — 空 = {}，非法 JSON 报错不吞
{
  assert.deepEqual(parseDryRunArguments(''), {ok: true, value: {}});
  assert.deepEqual(parseDryRunArguments('  {"path":"."} '), {ok: true, value: {path: '.'}});
  const bad = parseDryRunArguments('{oops');
  assert.equal(bad.ok, false);
  assert.match(bad.error, /JSON/);

  assert.equal(validCapabilityId('tool.repo'), true);
  assert.equal(validCapabilityId('  '), false);
  assert.equal(validCapabilityId('has space'), false);

  const row = dryRunResultRow({
    decision: 'deny', stage: 'chain_guard', risk_score: 0.91, reasons: ['r'], evidence: [],
  });
  assert.deepEqual(row.badge, {variant: 'danger', label: '否决'});
  assert.equal(row.stageLabel, '链式守卫');
  assert.equal(row.riskPct, 91);
}

// ---------------------------------------------------------------------------
// 8. trace.ts — span 树 / 时长 / 轨迹行
// ---------------------------------------------------------------------------
{
  const spans = [
    {span_id: 'c', trace_id: 't', parent_span_id: 'b', kind: 'tool', actor: 'tool.repo', status: 'ok', summary: null, attributes: null, started_at: 300, ended_at: 350, session_id: 's'},
    {span_id: 'a', trace_id: 't', parent_span_id: null, kind: 'turn', actor: 'runtime', status: 'ok', summary: 'root', attributes: null, started_at: 100, ended_at: 400, session_id: 's'},
    {span_id: 'b', trace_id: 't', parent_span_id: 'a', kind: 'provider', actor: 'p', status: 'error', summary: null, attributes: null, started_at: 200, ended_at: null, session_id: 's'},
  ];
  assert.deepEqual(spanTreeSort(spans).map((s) => s.span_id), ['a', 'b', 'c']);
  assert.equal(spanDepth(spans, spans[0]), 2); // c → b → a
  assert.equal(spanDepth(spans, spans[1]), 0);
  assert.equal(spanDurationMs(spans[0]), 50);
  assert.equal(spanDurationMs(spans[2]), null); // 未收口 → null（进行中）
  // 环防卫：a.parent = c 成环时不死循环
  const cyclic = [
    {span_id: 'a', trace_id: 't', parent_span_id: 'b', kind: '', actor: '', status: 'ok', summary: null, attributes: null, started_at: 1, ended_at: null, session_id: null},
    {span_id: 'b', trace_id: 't', parent_span_id: 'a', kind: '', actor: '', status: 'ok', summary: null, attributes: null, started_at: 2, ended_at: null, session_id: null},
  ];
  assert.equal(spanDepth(cyclic, cyclic[0]) <= 6, true);

  assert.deepEqual(spanStatusBadge('ok'), {variant: 'green', label: 'ok'});
  assert.equal(spanStatusBadge('error').variant, 'danger');
  assert.equal(spanStatusBadge('').variant, 'dim');
}

// 9. traceRowFromSummary / 过滤器
{
  const row = traceRowFromSummary({
    trace_id: 't1',
    span_count: 3,
    started_at: 1234,
    root_span: {span_id: 'r', trace_id: 't1', parent_span_id: null, kind: 'turn', actor: 'runtime', status: 'error', summary: '一轮对话', attributes: null, started_at: 1234, ended_at: 1300, session_id: 's1'},
  });
  assert.equal(row.traceId, 't1');
  assert.equal(row.spanCount, 3);
  assert.equal(row.hasError, true);
  assert.equal(row.sessionId, 's1');

  const ok = traceRowFromSummary({
    trace_id: 't2', span_count: 1, started_at: 5,
    root_span: {span_id: 'r', trace_id: 't2', parent_span_id: null, kind: 'turn', actor: 'runtime', status: 'ok', summary: null, attributes: null, started_at: 5, ended_at: null, session_id: null},
  });
  assert.deepEqual(traceFilterCounts([row, ok]), {all: 2, error: 1});
  assert.deepEqual(filterTraceRows([row, ok], 'error').map((r) => r.traceId), ['t1']);
  assert.equal(filterTraceRows([row, ok], 'all').length, 2);
}

// 10. audit 分组 chips — 按事件名首段分组计数，过滤精确
{
  const item = (title) => ({id: title, timestamp: 1, category: 'runtime', title, summary: '', source: 'audit', severity: 'info'});
  const items = [item('chat.turn.completed'), item('chat.turn.started'), item('approval.resolved'), item('memory.append')];
  assert.equal(auditEventGroup('chat.turn.completed'), 'chat');
  assert.equal(auditEventGroup('plain'), 'plain');
  assert.equal(auditEventGroup(''), 'other');
  const chips = auditGroupChips(items);
  assert.deepEqual(chips[0], {group: 'chat', count: 2});
  assert.equal(chips.length, 3);
  assert.equal(filterAuditItems(items, 'chat').length, 2);
  assert.equal(filterAuditItems(items, 'all').length, 4);
}

console.log('--- Governance Ledger Logic Check PASSED ---');
