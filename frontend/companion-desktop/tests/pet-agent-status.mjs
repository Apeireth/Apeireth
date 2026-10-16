// 伴星智能体状态机（agent-status.ts，真实模块）——五态词汇与真实源表的插口证明。
//
// 契约要点：
//   1. 等签 ← approval_required（带工具名）；干活 ← approval_resolved；收工 ← turn_completed；
//   2. 无产点预留态（待审阅/出错）绝不制造；未知信号种类零操作；
//   3. 衰减即诚实：干活 90s / 收工 30s 回落空闲，等签永不自衰减；
//   4. 乱序信号（时间倒拨）零操作；pendingTool 只在等签态携带。
import assert from 'node:assert/strict';
import {
  createInitialAgentStatus,
  reduceAgentStatus,
  decayAgentStatus,
  isStatusDisplayable,
  WORKING_DECAY_MS,
  DONE_DECAY_MS,
} from '../src/lib/pet/agent-status.ts';

console.log('--- Starting Pet Agent Status Check ---');

// ---------------------------------------------------------------------------
// 1. 真实源表：三种有产点状态的映射
// ---------------------------------------------------------------------------
{
  let state = createInitialAgentStatus(1000);
  assert.equal(state.status, 'idle');

  state = reduceAgentStatus(state, {
    kind: 'approval_required',
    at: 2000,
    toolName: 'tool.filesystem',
  });
  assert.equal(state.status, 'awaiting_approval');
  assert.equal(state.pendingTool, 'tool.filesystem', '等签态必须携带工具名');
  assert.equal(state.since, 2000);

  state = reduceAgentStatus(state, {kind: 'approval_resolved', at: 5000, decision: 'approve'});
  assert.equal(state.status, 'working', '解析后继续原回合——回干活态');
  assert.equal(state.pendingTool, null, 'pendingTool 只在等签态携带');

  state = reduceAgentStatus(state, {kind: 'turn_completed', at: 9000});
  assert.equal(state.status, 'done');
  assert.equal(state.since, 9000);
  console.log('  [1] live-source mapping OK');
}

// ---------------------------------------------------------------------------
// 2. 0 装：预留态绝不制造、未知种类零操作
// ---------------------------------------------------------------------------
{
  const base = createInitialAgentStatus(1000);
  for (const kind of ['backend_ready', 'initiative', 'guard_notice', '', 'mystery_event']) {
    const next = reduceAgentStatus(base, {kind, at: 2000});
    assert.equal(next, base, `未知/非工作信号 ${kind || '(空)'} 必须零操作（同引用）`);
  }
  assert.equal(base.status, 'idle');
  assert.equal(isStatusDisplayable('awaiting_review'), false, '待审阅无产点，不亮灯');
  assert.equal(isStatusDisplayable('error'), false, '出错无产点，不亮灯');
  assert.equal(isStatusDisplayable('idle'), false);
  for (const live of ['working', 'awaiting_approval', 'done']) {
    assert.equal(isStatusDisplayable(live), true);
  }
  console.log('  [2] zero-fabrication of reserved states OK');
}

// ---------------------------------------------------------------------------
// 3. 衰减纪律：干活 90s / 收工 30s 回落，等签永不自衰减
// ---------------------------------------------------------------------------
{
  let state = reduceAgentStatus(createInitialAgentStatus(0), {kind: 'approval_resolved', at: 1000});
  assert.equal(decayAgentStatus(state, 1000 + WORKING_DECAY_MS - 1), state, '时限前不衰减');
  state = decayAgentStatus(state, 1000 + WORKING_DECAY_MS);
  assert.equal(state.status, 'idle', '干活 90s 无信号回落空闲');

  let done = reduceAgentStatus(createInitialAgentStatus(0), {kind: 'turn_completed', at: 1000});
  done = decayAgentStatus(done, 1000 + DONE_DECAY_MS - 1);
  assert.equal(done.status, 'done', '收工展示期内保持');
  done = decayAgentStatus(done, 1000 + DONE_DECAY_MS);
  assert.equal(done.status, 'idle', '收工 30s 回落空闲');

  const waiting = reduceAgentStatus(createInitialAgentStatus(0), {
    kind: 'approval_required',
    at: 1000,
    toolName: 'tool.shell',
  });
  const aged = decayAgentStatus(waiting, 1000 + WORKING_DECAY_MS * 100);
  assert.equal(aged.status, 'awaiting_approval', '等签永不自衰减——真实在等');
  assert.equal(aged.pendingTool, 'tool.shell');
  console.log('  [3] decay discipline OK');
}

// ---------------------------------------------------------------------------
// 4. 乱序信号零操作（时间倒拨不回拨状态史）
// ---------------------------------------------------------------------------
{
  let state = reduceAgentStatus(createInitialAgentStatus(1000), {kind: 'turn_completed', at: 5000});
  const back = reduceAgentStatus(state, {kind: 'approval_required', at: 4000, toolName: 'x'});
  assert.equal(back, state, '乱序信号必须零操作');
  assert.equal(state.status, 'done');
  const nonFinite = reduceAgentStatus(state, {kind: 'approval_required', at: Number.NaN});
  assert.equal(nonFinite, state, '非法时间戳必须零操作');
  console.log('  [4] out-of-order signal rejection OK');
}

console.log('--- Pet Agent Status Check PASSED ---');
