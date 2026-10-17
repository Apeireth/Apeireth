// 性格卡（R2 性格锁定上屏，真实模块 src/lib/trait-card.ts）：
//   1. 锁定态显示：已锁定/未锁定/缺源「—」（不拿「未锁定」冒充未知）。
//   2. 周报区块：7 天窗（两端含、未来剔除）/ net_delta / move_count / 锁定跳过 /
//      top_reasons（降序 + 同数字典序、5 封顶）/ last_move —— 与
//      orchestration::trait_report 同账同语义（同批 fixture 对照钉死）。
//   3. 最近移动：新 → 旧、≤4 条、零移动留痕与未来不计。
//   4. 锁定/解锁按钮 = CLI 指引（trait 管理面无 HTTP 面，读码定：只读 + 指引）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  TRAIT_LOCKED_SKIP_PREFIX,
  TRAIT_LOCK_GUIDANCE,
  TRAIT_LOCK_NO_FACE,
  TRAIT_SLIDERS,
  TRAIT_TOP_REASONS,
  TRAIT_WEEK_NOTE,
  recentTraitMoves,
  traitCardView,
  traitLockCommand,
  traitSliderLabel,
  traitWeekFromTuningLog,
  tuningEntryMoved,
} from '../src/lib/trait-card.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const traitSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'trait-card.ts'), 'utf8');
const panelSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'components', 'TraitCard.svelte'),
  'utf8',
);
const workbenchSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'Workbench.svelte'),
  'utf8',
);

console.log('--- trait card (R2) ---');

function check(name, fn) {
  fn();
  console.log(`  ok - ${name}`);
}

const DAY_MS = 86_400_000;
const NOW = 70 * DAY_MS;

function record(seq, param, previous, next, reason, atEpochMs) {
  return {seq, param, previous, next, reason, at_epoch_ms: atEpochMs};
}

/** 同批 fixture（对照 trait_report.rs::weekly_report_seven_day_window_and_movement_amounts）。 */
function weekFixture() {
  return [
    record(1, 'memory_fade', 1.0, 1.25, '上调一档', NOW - DAY_MS),
    record(2, 'memory_fade', 1.25, 0.75, '下调半档', NOW - 3 * DAY_MS),
    record(3, 'tone_saturation', 1.0, 1.2, '过平淡', NOW - 2 * DAY_MS),
    record(4, 'tone_saturation', 1.2, 1.2, '已达参数上限: x', NOW - 2 * DAY_MS),
    record(5, 'memory_fade', 1.0, 1.0, `${TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 测试意图`, NOW - DAY_MS),
    record(6, 'curiosity_strength', 1.0, 1.1, '左端', NOW - 7 * DAY_MS),
    record(7, 'curiosity_strength', 1.1, 3.0, '窗外', NOW - 7 * DAY_MS - 1),
    record(8, 'consolidation_cadence', 1.0, 9.0, '未来', NOW + 1),
  ];
}

// ---------------------------------------------------------------------------
// 1. 周报区块：7 天窗 / 移动量 / 锁定跳过 / 最近移动（trait_report 同口径）
// ---------------------------------------------------------------------------
check('周报: 7 天窗两端含、窗外与未来剔除、net_delta/move_count/锁定跳过分列', () => {
  const report = traitWeekFromTuningLog(weekFixture(), NOW);
  assert.equal(report.windowDays, 7);
  assert.equal(report.windowFromMs, NOW - 7 * DAY_MS);
  assert.equal(report.windowToMs, NOW);
  assert.equal(report.recordsInWindow, 6, '窗内 6 条（含留痕）');
  assert.equal(report.lockedSkipped, 1);
  assert.equal(report.sliders.length, 4, '四杆一行不多不少');

  const byParam = Object.fromEntries(report.sliders.map((row) => [row.param, row]));
  const fade = byParam.memory_fade;
  assert.ok(Math.abs(fade.netDelta - -0.25) < 1e-9, '净移动量 = -0.25');
  assert.equal(fade.moveCount, 2);
  assert.equal(fade.lockedSkipped, 1);
  assert.equal(fade.lastMove.at_epoch_ms, NOW - DAY_MS, '最近移动 = 窗内最新一次');
  assert.equal(fade.lastMove.reason, '上调一档');
  assert.ok(Math.abs(fade.lastMove.delta - 0.25) < 1e-9);

  const tone = byParam.tone_saturation;
  assert.ok(Math.abs(tone.netDelta - 0.2) < 1e-9, '零移动留痕不计移动量');
  assert.equal(tone.moveCount, 1, '零移动留痕不计移动次数');

  const curiosity = byParam.curiosity_strength;
  assert.ok(Math.abs(curiosity.netDelta - 0.1) < 1e-9, '左端含、窗外剔');
  assert.equal(curiosity.moveCount, 1);

  const cadence = byParam.consolidation_cadence;
  assert.equal(cadence.moveCount, 0, '未来记录不计');
  assert.equal(cadence.netDelta, 0);
  assert.equal(cadence.lastMove, null);
});

check('周报: top_reasons 只数实际移动、计数降序 + 同数字典序、5 封顶', () => {
  const records = [];
  for (let index = 0; index < 7; index += 1) {
    records.push(record(index, 'memory_fade', 1.0, 1.1, 'reason-b', NOW - DAY_MS));
  }
  for (let index = 7; index < 10; index += 1) {
    records.push(record(index, 'memory_fade', 1.1, 1.2, 'reason-a', NOW - DAY_MS));
  }
  records.push(record(20, 'memory_fade', 1.2, 1.2, 'reason-c', NOW - DAY_MS));
  records.push(record(21, 'memory_fade', 1.0, 1.0, `${TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): x`, NOW - DAY_MS));
  ['reason-d', 'reason-e', 'reason-f', 'reason-g'].forEach((reason, index) => {
    records.push(record(30 + index, 'tone_saturation', 1.0, 1.1, reason, NOW - DAY_MS));
  });

  const report = traitWeekFromTuningLog(records, NOW);
  assert.equal(report.topReasons.length, TRAIT_TOP_REASONS, 'top 5 封顶');
  assert.deepEqual(report.topReasons[0], {reason: 'reason-b', count: 7});
  assert.deepEqual(report.topReasons[1], {reason: 'reason-a', count: 3});
  assert.deepEqual(
    report.topReasons.slice(2).map((row) => row.reason),
    ['reason-d', 'reason-e', 'reason-f'],
    '同计数按字典序升序（确定性）',
  );
  assert.ok(
    !report.topReasons.some((row) => row.reason === 'reason-c' || row.reason.startsWith(TRAIT_LOCKED_SKIP_PREFIX)),
    '零移动留痕与锁定跳过不得混进移动原因',
  );
});

check('最近移动: 新 → 旧、上限 4、零移动留痕与未来不计', () => {
  const records = [
    record(1, 'memory_fade', 1.0, 1.1, '一', NOW - 5 * DAY_MS),
    record(2, 'tone_saturation', 1.0, 0.9, '二', NOW - 3 * DAY_MS),
    record(3, 'memory_fade', 1.1, 1.2, '三', NOW - DAY_MS),
    record(4, 'memory_fade', 1.0, 1.0, `${TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): x`, NOW - 1),
    record(5, 'curiosity_strength', 1.0, 5.0, '未来', NOW + 10),
  ];
  const moves = recentTraitMoves(records, NOW);
  assert.deepEqual(moves.map((move) => move.reason), ['三', '二', '一'], '新 → 旧，跳过/未来不计');
  assert.equal(moves[0].label, '遗忘衰减强度');
  assert.equal(recentTraitMoves(records, NOW, 1)[0].reason, '三');
  assert.equal(tuningEntryMoved({seq: 0, param: 'memory_fade', previous: 1, next: 1, reason: 'x', at_epoch_ms: 0}), false);
});

check('周报区块: 视图渲染「本周性格周报」+ 数值串（不编数）', () => {
  const view = traitCardView({
    values: {memory_fade: 1.0, curiosity_strength: 1.0, tone_saturation: 1.0, consolidation_cadence: 1},
    locks: null,
    moves: weekFixture(),
    nowMs: NOW,
  });
  assert.equal(view.week.title, '本周性格周报');
  assert.ok(view.week.window.includes('UTC，两端含'));
  const fadeRow = view.week.rows.find((row) => row.param === 'memory_fade');
  assert.equal(fadeRow.netDelta, '-0.25');
  assert.equal(fadeRow.moveCount, '2');
  assert.equal(fadeRow.lockedSkipped, '1');
  assert.deepEqual(view.week.topReasons.map((row) => row.reason), [
    '上调一档',
    '下调半档',
    '左端',
    '过平淡',
  ], '同计数按字典序升序（上<下<左<过）');
  const totals = Object.fromEntries(view.week.totals.map((row) => [row.key, row]));
  assert.equal(totals.records_in_window.value, '6');
  assert.equal(totals.locked_skipped.value, '1');
  assert.equal(totals.skipped_records.value, '—', '坏行计数无读面 → 「—」（不编 0）');
  assert.ok(view.week.note === TRAIT_WEEK_NOTE, '周报口径注在案（同账同语义）');
});

// ---------------------------------------------------------------------------
// 2. 锁定态显示
// ---------------------------------------------------------------------------
check('锁定态: 已锁定/未锁定如实显示；缺源「—」+ 真因（不冒充未锁定）', () => {
  const lockedView = traitCardView({
    values: {memory_fade: 1.25, curiosity_strength: 1, tone_saturation: 1, consolidation_cadence: 1},
    locks: {memory_fade: true, curiosity_strength: false, tone_saturation: null, consolidation_cadence: null},
    moves: [],
    nowMs: NOW,
  });
  const byParam = Object.fromEntries(lockedView.sliderRows.map((row) => [row.param, row]));
  assert.equal(byParam.memory_fade.locked, '已锁定');
  assert.equal(byParam.curiosity_strength.locked, '未锁定');
  assert.equal(byParam.tone_saturation.locked, '—', '缺源 = 「—」（不冒充）');
  assert.ok(byParam.tone_saturation.lockedNote.includes('暂无面板读面'));
  assert.ok(TRAIT_LOCK_NO_FACE.includes('不拿「未锁定」冒充未知'));

  const noFace = traitCardView({values: null, locks: null, moves: [], nowMs: NOW});
  for (const row of noFace.sliderRows) {
    assert.equal(row.locked, '—');
    assert.equal(row.value, '—', '现值缺源 = 「—」（不编数）');
  }
  assert.equal(noFace.sliderRows.length, 4, '四滑杆恒四行');
});

// ---------------------------------------------------------------------------
// 3. 空账 / 账本缺席：0 与「—」的分界（不拿 0 冒充、不拿「—」冒充空账）
// ---------------------------------------------------------------------------
check('空账 = 如实 0 移动（不编数）；账本读不到 = 「—」（不拿 0 冒充空账的 0）', () => {
  const empty = traitCardView({values: null, locks: null, moves: [], nowMs: NOW});
  for (const row of empty.week.rows) {
    assert.equal(row.netDelta, '0');
    assert.equal(row.moveCount, '0');
    assert.equal(row.lockedSkipped, '0');
    assert.equal(row.lastMove, '—');
  }
  assert.equal(empty.week.totals.find((row) => row.key === 'records_in_window').value, '0');

  const unreadable = traitCardView({values: null, locks: null, moves: null, nowMs: NOW});
  for (const row of unreadable.week.rows) {
    assert.equal(row.netDelta, '—', '账本缺席不拿 0 冒充');
    assert.equal(row.moveCount, '—');
  }
  assert.ok(unreadable.week.note.includes('如实「—」'));
  assert.equal(unreadable.recentMoveRows.length, 0);
});

// ---------------------------------------------------------------------------
// 4. 锁定/解锁 = CLI 指引（只读 + 指引纪律，按钮不代跑）
// ---------------------------------------------------------------------------
check('CLI 指引: lock/unlock/report 命令逐字 + 按钮只给指引零代跑', () => {
  assert.equal(traitLockCommand('lock', 'memory_fade'), 'apeireth trait lock memory_fade');
  assert.equal(traitLockCommand('unlock', 'tone_saturation'), 'apeireth trait unlock tone_saturation');
  assert.deepEqual(
    TRAIT_LOCK_GUIDANCE.map((row) => row.command),
    ['apeireth trait lock <SLIDER>', 'apeireth trait unlock <SLIDER>', 'apeireth trait report'],
  );
  assert.ok(panelSrc.includes('traitLockCommand('), '锁定/解锁按钮走 CLI 指引纯函数');
  assert.ok(!panelSrc.includes('fetch('), '性格卡零 fetch');
  assert.ok(!panelSrc.includes('invoke('), '性格卡零 IPC 写面（readTuningLog 取数缝在外）');
  assert.ok(!/\b(POST|PATCH|DELETE|PUT)\b/.test(panelSrc), '性格卡零写动词');
  assert.ok(panelSrc.includes('界面不代跑'), '不代跑口径上屏');
  assert.ok(!traitSrc.includes('fetch('), '性格逻辑件零网络面');
});

// ---------------------------------------------------------------------------
// 5. 源码镜像：滑杆名同源 + 周报区块 + 工作台接线 + 现值同源注
// ---------------------------------------------------------------------------
check('源码镜像: 四滑杆名与 self_tuning::TunableParam::label 逐字同源', () => {
  assert.deepEqual(
    TRAIT_SLIDERS.map((slider) => [slider.param, slider.label]),
    [
      ['memory_fade', '遗忘衰减强度'],
      ['curiosity_strength', '好奇心强度'],
      ['tone_saturation', '语气情绪饱和度'],
      ['consolidation_cadence', '整合节奏'],
    ],
  );
  assert.equal(traitSliderLabel('unknown-param'), 'unknown-param', '词表外原样上屏');
  assert.ok(TRAIT_LOCKED_SKIP_PREFIX === '已锁定, 跳过沉积', '锁定跳过前缀与 self_tuning 同源');
});

check('源码镜像: 周报区块/锁定指引/工作台接线/取数缝', () => {
  assert.ok(panelSrc.includes('view.week.title'), '周报区块经视图投影上屏');
  assert.ok(panelSrc.includes('view.lockGuidance'), '锁定指引区在案');
  assert.ok(panelSrc.includes('只读纪律'), '只读纪律脚注在案');
  assert.ok(workbenchSrc.includes('<TraitCard'), '工作台挂性格卡');
  assert.ok(workbenchSrc.includes('readTuningLog()'), '最近移动取数缝 ← 桌面 IPC 真源');
  assert.ok(workbenchSrc.includes('capabilityEnvFromConfig'), '现值 = 下发旋钮值（APEIRETH_TUNE_* 同源）');
  assert.ok(traitSrc.includes('tool.trait_report'), '权威读面口径在案（不冒充工具读面）');
});

console.log('trait-card: all checks passed');
