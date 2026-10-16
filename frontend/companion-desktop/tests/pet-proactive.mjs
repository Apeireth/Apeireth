// 主动涌现引擎（pet-proactive.ts，真实模块）——防唠叨纪律的机器证明。
//
// 契约要点（00-PHILOSOPHY §5 initiative 约定化）：
//   1. 只消费真实信号，绝不生成内容：审批待签出通报，回合结束不出台词；
//   2. 待签 >90s 抬头一瞥（瞬时 2.5s），5 分钟内不重复抬头；
//   3. 他开口受每日预算硬约束（≤3 次/天，与后端预算器同数），超支一个字不说；
//   4. 通报限频（45s）+ 同文去重（120s）+ 气泡上限 2，防唠叨；
//   5. initiative 帧解析是契约空间接线预留，v0 后端无生产者。
import assert from 'node:assert/strict';
import {
  createInitialProactiveState,
  reduceProactive,
  evaluateGlance,
  isGlanceActive,
  parseInitiativeFrame,
  GLANCE_DELAY_MS,
  GLANCE_REPEAT_MS,
  GLANCE_HOLD_MS,
  SPEECH_DAILY_LIMIT,
  NOTICE_MIN_INTERVAL_MS,
} from '../src/lib/pet/proactive.ts';

console.log('--- Starting Pet Proactive Engine Check ---');

const T0 = 1_700_000_000_000;

// ---------------------------------------------------------------------------
// 1. 审批待签 → 通报气泡（人话文案）；resolved → 静默收势（不响不闹）
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  s = reduceProactive(s, {
    type: 'signal',
    signal: {kind: 'approval_required', at: T0, toolName: 'file_write'},
  });
  assert.equal(s.pendingApprovals, 1);
  assert.equal(s.bubbles.length, 1);
  assert.equal(s.bubbles[0].kind, 'notice');
  assert.equal(s.bubbles[0].text, '有文书等你签：file_write');

  s = reduceProactive(s, {
    type: 'signal',
    signal: {kind: 'approval_resolved', at: T0 + 3000},
  });
  assert.equal(s.pendingApprovals, 0);
  assert.equal(s.bubbles.length, 0, 'resolved 静默收势：旧通报一并收走');
  console.log('  -> PASS: 待签出通报、解决静默收势');
}

// ---------------------------------------------------------------------------
// 2. 通报限频 + 同文去重：连环审批不连环响（防唠叨）
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'approval_required', at: T0, toolName: 'a'}});
  s = reduceProactive(s, {
    type: 'signal',
    signal: {kind: 'approval_required', at: T0 + 5000, toolName: 'b'},
  });
  assert.equal(s.bubbles.length, 1, '45s 内第二条通报必须静默');
  s = reduceProactive(s, {
    type: 'signal',
    signal: {kind: 'approval_required', at: T0 + NOTICE_MIN_INTERVAL_MS + 1000, toolName: 'a'},
  });
  assert.equal(
    s.bubbles.filter((b) => b.text === '有文书等你签：a').length,
    0,
    '同文 120s 去重窗口内不重复播报',
  );
  console.log('  -> PASS: 通报限频 45s + 同文去重 120s 生效');
}

// ---------------------------------------------------------------------------
// 3. 抬头一瞥：>90s 抬头一次，2.5s 后收，5 分钟内不重复
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'approval_required', at: T0, toolName: 'x'}});
  s = evaluateGlance(s, T0 + GLANCE_DELAY_MS - 1);
  assert.equal(isGlanceActive(s, T0 + GLANCE_DELAY_MS - 1), false, '未到 90s 不抬头');

  s = evaluateGlance(s, T0 + GLANCE_DELAY_MS);
  assert.equal(isGlanceActive(s, T0 + GLANCE_DELAY_MS + 100), true, '到点抬头一瞥');
  assert.equal(
    isGlanceActive(s, T0 + GLANCE_DELAY_MS + GLANCE_HOLD_MS + 1),
    false,
    '2.5s 后收势（瞬时动作，不是常驻状态）',
  );

  s = evaluateGlance(s, T0 + GLANCE_DELAY_MS + GLANCE_REPEAT_MS - 1);
  assert.equal(isGlanceActive(s, T0 + GLANCE_DELAY_MS + GLANCE_REPEAT_MS), false, '5 分钟内不重复抬头');
  s = evaluateGlance(s, T0 + GLANCE_DELAY_MS + GLANCE_REPEAT_MS);
  assert.equal(isGlanceActive(s, T0 + GLANCE_DELAY_MS + GLANCE_REPEAT_MS + 1), true, '5 分钟后再次抬头');
  console.log('  -> PASS: 抬头一瞥 90s 门槛 / 2.5s 瞬时 / 5 分钟限频');
}

// ---------------------------------------------------------------------------
// 4. 他开口（initiative 契约空间）：每日预算硬约束，超支一个字不说
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  for (let i = 0; i < SPEECH_DAILY_LIMIT; i++) {
    s = reduceProactive(s, {
      type: 'signal',
      signal: {kind: 'initiative', at: T0 + i * 60_000, text: `话 ${i}`},
    });
  }
  assert.equal(s.speechUsed, SPEECH_DAILY_LIMIT);
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'initiative', at: T0 + 300_000, text: '超支的话'}});
  assert.equal(s.speechUsed, SPEECH_DAILY_LIMIT, '预算用尽后不再计数');
  assert.ok(!s.bubbles.some((b) => b.text === '超支的话'), '预算用尽后一个字也不说');

  // 跨天预算重置
  s = reduceProactive(s, {
    type: 'signal',
    signal: {kind: 'initiative', at: T0 + 86_400_000 + 1000, text: '新的一天'},
  });
  assert.equal(s.speechUsed, 1, '新自然日预算重置');
  assert.ok(s.bubbles.some((b) => b.text === '新的一天'));
  console.log('  -> PASS: 开口每日预算 ≤3 硬约束 + 跨天重置');
}

// ---------------------------------------------------------------------------
// 5. 回合结束/启动不配拥有台词；backend_ready 收走陈旧气泡
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'approval_required', at: T0, toolName: 'y'}});
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'turn_completed', at: T0 + 1000}});
  assert.equal(s.bubbles.length, 1, 'turn_completed 不出气泡也不扰动既有气泡');
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'backend_ready', at: T0 + 2000}});
  assert.equal(s.bubbles.length, 0, 'backend_ready = 新起点，陈旧气泡收走');
  assert.equal(s.pendingApprovals, 0);
  console.log('  -> PASS: 回合级/启动级事件不出台词');
}

// ---------------------------------------------------------------------------
// 6. 气泡到期清理 + 手动关闭
// ---------------------------------------------------------------------------
{
  let s = createInitialProactiveState(T0);
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'approval_required', at: T0, toolName: 'z'}});
  const id = s.bubbles[0].id;
  s = reduceProactive(s, {type: 'tick', at: T0 + 30});
  assert.equal(s.bubbles.length, 1, '未到期不清理');
  s = reduceProactive(s, {type: 'tick', at: T0 + 60_000});
  assert.equal(s.bubbles.length, 0, '到期即清理');
  s = reduceProactive(s, {type: 'signal', signal: {kind: 'approval_required', at: T0 + 200_000, toolName: 'w'}});
  s = reduceProactive(s, {type: 'dismiss', id: s.bubbles[0].id});
  assert.equal(s.bubbles.length, 0, '手动关闭生效');
  assert.ok(id.startsWith('notice-'));
  console.log('  -> PASS: 气泡到期与手动关闭');
}

// ---------------------------------------------------------------------------
// 7. initiative 帧解析（契约空间，v0 无生产者）：严格解析不猜
// ---------------------------------------------------------------------------
{
  assert.equal(parseInitiativeFrame('not json'), null);
  assert.equal(parseInitiativeFrame('{"type":"other","at":1,"text":"x"}'), null);
  assert.equal(parseInitiativeFrame('{"type":"initiative","at":1,"text":"  "}'), null);
  assert.equal(parseInitiativeFrame('{"type":"initiative","at":"1","text":"x"}'), null);
  const ok = parseInitiativeFrame('{"type":"initiative","at":42,"text":"我醒了"}');
  assert.deepEqual(ok, {kind: 'initiative', at: 42, text: '我醒了'});
  console.log('  -> PASS: initiative 帧严格解析（契约空间接线预留）');
}

console.log('--- All Pet Proactive Engine Checks PASSED! ---');
