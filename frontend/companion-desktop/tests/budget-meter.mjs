// 预算「仪表面」（真实模块 src/lib/chat-shell/turn-telemetry.ts + src/lib/budget.ts），
// 并对 App.svelte / SettingsView.svelte 做源码级镜像校验。
// 契约：
//   1. 会话累计器：真值累加（token 入/出/缓存命中/耗时/工具调用/回合数），
//      未上报的位保持未知（不拿 0 顶数），真实 0 照记 0。
//   2. 消耗卡真值渲染：入/出、命中（数 · 率）、回合数、累计耗时；
//      无数据位诚实「—」。
//   3. 预算余量条：有生效上限的维度对上限求余量（超限钳到 0 不编负数）；
//      无上限维度显「—」；未测得的已耗不编数。
//   4. hover 详情补「预算余量」一行（有摘要才补，无摘要不加行）。
//   5. 源码镜像：App 记 running totals 并把会话累计值传给设置页仪表。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  DASH,
  accumulateTurnUsage,
  emptySessionTotals,
  formatSessionTotals,
  formatTurnTelemetry,
} from '../src/lib/chat-shell/turn-telemetry.ts';
import {
  budgetRemainingRows,
  budgetRemainingSummary,
  effectiveBudget,
} from '../src/lib/budget.ts';
import {DEFAULT_CAPABILITY_TOGGLES} from '../src/lib/types.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const appSrc = readFileSync(join(testsDir, '..', 'src', 'App.svelte'), 'utf8');
const settingsSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'),
  'utf8',
);

console.log('--- budget meter ---');

// ---------------------------------------------------------------------------
// 1. 会话累计器：真值累加、未上报不编数、真实 0 照记
// ---------------------------------------------------------------------------
let totals = emptySessionTotals();
{
  assert.deepEqual(
    totals,
    {
      turns: 0,
      promptTokens: null,
      completionTokens: null,
      cacheHitTokens: null,
      durationMs: null,
      toolCalls: 0,
      lastTurnToolCalls: null,
    },
    '空累计器：未测得位为 null（未知），计数位为真实 0',
  );

  totals = accumulateTurnUsage(totals, {
    usage: {promptTokens: 100, completionTokens: 20, cacheHitTokens: 60},
    durationMs: 820,
    toolCalls: 2,
  });
  totals = accumulateTurnUsage(totals, {
    usage: {promptTokens: 50, completionTokens: 5, cacheHitTokens: 40},
    durationMs: 400,
    toolCalls: 0,
  });
  totals = accumulateTurnUsage(totals, {usage: null, durationMs: 100, toolCalls: 1});

  assert.equal(totals.turns, 3, '回合数逐回合 +1');
  assert.equal(totals.promptTokens, 150, '输入 token 累计');
  assert.equal(totals.completionTokens, 25, '输出 token 累计');
  assert.equal(totals.cacheHitTokens, 100, '缓存命中累计');
  assert.equal(totals.durationMs, 1320, '累计耗时');
  assert.equal(totals.toolCalls, 3, '工具调用按事件计数累加');
  assert.equal(totals.lastTurnToolCalls, 1, '最近一回合计数留作单轮余量基数');

  // 未上报的位保持未知（不编 0）；真零工具调用照记 0。
  const sparse = accumulateTurnUsage(emptySessionTotals(), {durationMs: 100, toolCalls: 0});
  assert.equal(sparse.promptTokens, null, '从未上报输入 token → 未知（不编 0）');
  assert.equal(sparse.cacheHitTokens, null);
  assert.equal(sparse.toolCalls, 0, '真零工具调用照记 0');
  assert.equal(sparse.lastTurnToolCalls, 0);

  // 流里真报 0：真实 0 是值，照记。
  const zeros = accumulateTurnUsage(emptySessionTotals(), {
    usage: {promptTokens: 0, completionTokens: 0},
    durationMs: 0,
    toolCalls: 1,
  });
  assert.equal(zeros.promptTokens, 0);
  assert.equal(zeros.durationMs, 0);

  // 不可变更新：旧累计器不被就地改写。
  assert.equal(emptySessionTotals().turns, 0);
  console.log('  -> PASS: 会话累计器真值累加（未知保持未知、真实 0 照记）');
}

// ---------------------------------------------------------------------------
// 2. 消耗卡真值渲染（无数据位诚实「—」）
// ---------------------------------------------------------------------------
{
  const view = formatSessionTotals(totals);
  assert.equal(view.tokens, '入 150 / 出 25', '本会话 token（入/出）真值串');
  assert.equal(view.cache, '命中 100 · 67%', '提示缓存命中（数 · 率）= 累计命中/累计输入');
  assert.equal(view.turns, '3', '回合数真值');
  assert.equal(view.duration, '1.3s', '累计耗时真值');
  assert.equal(view.toolCalls, '3', '工具调用累计真值');

  const empty = formatSessionTotals(emptySessionTotals());
  assert.equal(empty.tokens, DASH, '未上报 → 「—」（不编 0）');
  assert.equal(empty.cache, DASH);
  assert.equal(empty.turns, '0', '回合数真实 0 照显');
  assert.equal(empty.duration, DASH, '未测得耗时 → 「—」');

  const hitOnly = formatSessionTotals(
    accumulateTurnUsage(emptySessionTotals(), {usage: {cacheHitTokens: 300}, durationMs: 10, toolCalls: 1}),
  );
  assert.equal(hitOnly.cache, `命中 300 · ${DASH}`, '输入未知 → 率位「—」（不做除法不编率）');
  assert.equal(hitOnly.tokens, DASH, '入/出两位都未知 → token 位整位「—」');
  console.log('  -> PASS: 消耗卡真值渲染（未测得位诚实「—」、真实 0 照显）');
}

// ---------------------------------------------------------------------------
// 3. 预算余量条：有上限求余量 / 无上限显「—」
// ---------------------------------------------------------------------------
{
  const budget = effectiveBudget({...DEFAULT_CAPABILITY_TOGGLES, maxToolCalls: 16});
  const rows = budgetRemainingRows(totals, budget);
  const byKey = Object.fromEntries(rows.map((row) => [row.key, row]));

  // 有上限维度：单轮工具调用对生效上限求余量（最近一回合计数）。
  const tool = byKey.toolCalls;
  assert.equal(tool.cap, '16 · configured', '有配置上限：生效值 + 口径');
  assert.equal(tool.used, '1', '已耗 = 最近一回合实测');
  assert.equal(tool.remaining, '15', '余量 = 上限 − 已耗');
  assert.ok(tool.ratio !== null && tool.ratio > 0, '有上限且已耗已知 → 给占比（不编百分比的例外位）');

  // 无上限维度显「—」（token/缓存/回合数/耗时无上限接口）。
  for (const key of ['tokenIn', 'tokenOut', 'cacheHit', 'turns', 'duration']) {
    assert.equal(byKey[key].cap, DASH, `${key} 无上限 → 上限「—」`);
    assert.equal(byKey[key].remaining, DASH, `${key} 无上限 → 余量「—」`);
    assert.equal(byKey[key].ratio, null, `${key} 不编百分比`);
  }

  // 回合轮数：生效上限在（constant 口径如实显示），前端未测得轮数 → 已耗/余量「—」。
  const rounds = byKey.turnRounds;
  assert.equal(rounds.cap, '8 · constant', '未配置轮数上限 = 常量默认口径');
  assert.equal(rounds.used, DASH, '未测得不编数');
  assert.equal(rounds.remaining, DASH);

  // 超限（最近回合工具数 > 上限）→ 余量钳到 0，不编负数。
  const over = budgetRemainingRows(
    accumulateTurnUsage(emptySessionTotals(), {usage: null, durationMs: 1, toolCalls: 99}),
    effectiveBudget({maxToolCalls: 16}),
  );
  assert.equal(over.find((row) => row.key === 'toolCalls').remaining, '0', '超限余量 = 0（不编负数）');

  // 余量摘要一行：只报有生效上限的维度（hover 补行取数）。
  const summary = budgetRemainingSummary(rows);
  assert.ok(summary.includes('单轮工具调用 余 15（上限 16 · configured）'), '摘要含单轮工具余量');
  assert.ok(summary.includes('回合轮数 余 —'), '摘要如实带未测得位');
  console.log('  -> PASS: 余量条有上限求余量、无上限「—」、超限钳 0、未测得不编数');
}

// ---------------------------------------------------------------------------
// 4. hover 详情补「预算余量」一行（有摘要才补）
// ---------------------------------------------------------------------------
{
  const withBudget = formatTurnTelemetry({
    usage: {promptTokens: 10},
    budgetRemaining: '单轮工具调用 余 15（上限 16 · configured）',
  });
  assert.ok(withBudget.hover.includes('预算余量 单轮工具调用 余 15'), 'hover 补预算余量一行');
  const without = formatTurnTelemetry({usage: {promptTokens: 10}});
  assert.ok(!without.hover.includes('预算余量'), '无摘要不加行（既有四项 hover 不变）');
  console.log('  -> PASS: hover 预算余量补行（有摘要才补）');
}

// ---------------------------------------------------------------------------
// 5. 源码镜像：App 记 running totals + 设置页仪表取真值（不手搓数字）
// ---------------------------------------------------------------------------
{
  assert.ok(appSrc.includes('accumulateTurnUsage('), 'App 记 running totals（会话累计器）');
  assert.ok(appSrc.includes('sessionUsageByConv'), '按会话分账（切换会话不串账）');
  assert.ok(appSrc.includes('turnToolCalls += 1'), '工具调用按事件计数（真值）');
  assert.ok(appSrc.includes('sessionUsage={sessionUsage}'), '设置页仪表取会话累计值');
  assert.ok(appSrc.includes('effectiveBudget(config.capabilities)'), '余量对生效上限求（配置同源）');
  assert.ok(appSrc.includes('budgetRemainingSummary('), 'hover 余量摘要同源取数');
  assert.ok(appSrc.includes('budgetRemaining: budgetSummary'), 'hover 补行经纯函数接线');

  assert.ok(settingsSrc.includes('formatSessionTotals(sessionUsage ?? emptySessionTotals())'), '消耗卡显示串由纯函数产出');
  assert.ok(settingsSrc.includes('budgetRemainingRows(sessionUsage ?? emptySessionTotals(), budgetBadges)'), '余量条由纯函数产出');
  assert.ok(/class="meter-grid"/.test(settingsSrc), '会话消耗卡挂仪表格');
  for (const slot of ['tokens', 'cache', 'turns', 'duration']) {
    assert.ok(settingsSrc.includes(`{sessionTotalsView.${slot}}`), `消耗卡渲染消耗位 ${slot}`);
  }
  assert.ok(settingsSrc.includes('余 {row.remaining}'), '余量条渲染余量位');
  console.log('  -> PASS: 源码镜像——App 累计器接线 + 仪表纯函数取数');
}

console.log('budget meter: all assertions passed');
