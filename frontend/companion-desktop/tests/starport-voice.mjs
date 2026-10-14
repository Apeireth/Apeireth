// 星语语音面纯逻辑（voice-store.ts，真实模块）——状态机/电平数学/录音账本/合成面证明。
//
// 契约要点：
//   1. 状态机全迁移表：合法迁移逐一核对，合法表之外一律抛 illegal-transition
//      （未知状态/动作单列 invalid-argument）；
//   2. 电平数学手算核对：rmsLevel 均方根、smoothLevel 指数平滑、peakHold 尾窗峰值；
//   3. 录音账本 0 装：空 id/冲突/非正时长/非正大小/缺条目一律抛；
//   4. formatDuration 向下取整不虚报，负数/非有限抛错；
//   5. 合成状态机全表（四态封顶，reset=停止/复位语义）；
//   6. 合成参数校验矩阵 + 语句账本 0 装。
import assert from 'node:assert/strict';
import {
  transitionVoiceState,
  rmsLevel,
  smoothLevel,
  peakHold,
  VoiceLedger,
  VoiceStoreError,
  formatDuration,
  transitionSpeechState,
  buildSpeechUtterance,
  SpeechJobLedger,
  transitionTranscriptState,
  TranscriptLedger,
  VOICE_STATES,
  VOICE_ACTIONS,
  SPEECH_STATES,
  SPEECH_ACTIONS,
  TRANSCRIPT_STATES,
  TRANSCRIPT_ACTIONS,
} from '../src/lib/starport/apps/voice-store.ts';

console.log('--- Starting Starport Voice Check ---');

// ---------------------------------------------------------------------------
// 1. 状态机：全迁移表（合法/非法/未知输入）
// ---------------------------------------------------------------------------
{
  const legal = {
    idle: {request: 'requesting'},
    requesting: {grant: 'recording', deny: 'error', fail: 'error'},
    recording: {stop: 'stopped', fail: 'error'},
    stopped: {reset: 'idle', request: 'requesting'},
    error: {reset: 'idle', request: 'requesting'},
  };
  assert.equal(VOICE_STATES.length, 5, '五态封顶');
  assert.equal(VOICE_ACTIONS.length, 6);
  for (const state of VOICE_STATES) {
    for (const action of VOICE_ACTIONS) {
      const expected = legal[state][action];
      if (expected) {
        assert.equal(transitionVoiceState(state, action), expected, `合法迁移 ${state}+${action}`);
      } else {
        assert.throws(
          () => transitionVoiceState(state, action),
          (error) =>
            error instanceof VoiceStoreError && error.code === 'illegal-transition',
          `非法迁移必须拒绝：${state}+${action}`,
        );
      }
    }
  }
  assert.throws(
    () => transitionVoiceState('bogus', 'request'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '未知状态单列 invalid-argument',
  );
  assert.throws(
    () => transitionVoiceState('idle', 'explode'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '未知动作单列 invalid-argument',
  );
  console.log('  [1] state machine full table OK');
}

// ---------------------------------------------------------------------------
// 2. rmsLevel：均方根手算核对 + 0 装
// ---------------------------------------------------------------------------
{
  // 容差 1e-6：样本存 f32 有量化误差（0.6/0.8 非精确表示），按 f32 量级核对。
  assert.ok(Math.abs(rmsLevel(new Float32Array([3, 4])) - Math.sqrt(12.5)) < 1e-6);
  assert.equal(rmsLevel(new Float32Array([0, 0, 0])), 0);
  assert.equal(rmsLevel(new Float32Array([0.5])), 0.5);
  assert.ok(Math.abs(rmsLevel(new Float32Array([-0.6, 0.8])) - Math.sqrt(0.5)) < 1e-6);
  assert.throws(
    () => rmsLevel(new Float32Array([])),
    (error) => error instanceof VoiceStoreError && error.code === 'empty-input',
    '空输入抛错',
  );
  assert.throws(
    () => rmsLevel(new Float32Array([1, Number.NaN])),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '非有限样本抛错',
  );
  assert.throws(
    () => rmsLevel([0.1, 0.2]),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '非 Float32Array 抛错',
  );
  console.log('  [2] rmsLevel OK');
}

// ---------------------------------------------------------------------------
// 3. smoothLevel / peakHold：平滑与窗口峰值手算核对
// ---------------------------------------------------------------------------
{
  const near = (actual, expected, why) =>
    assert.ok(Math.abs(actual - expected) < 1e-12, `${why}: ${actual} ≈ ${expected}`);
  near(smoothLevel(0.2, 0.8, 0.5), 0.5, '半程平滑');
  assert.equal(smoothLevel(0.2, 0.8, 0), 0.2, 'factor 0 恒等 prev');
  near(smoothLevel(0.2, 0.8, 1), 0.8, 'factor 1 到 next');
  near(smoothLevel(0.1, 0.9, 0.25), 0.3, '四分之一程');
  for (const factor of [1.5, -0.1, Number.NaN]) {
    assert.throws(
      () => smoothLevel(0.1, 0.2, factor),
      (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
      `factor 越界必须抛：${factor}`,
    );
  }
  assert.throws(
    () => smoothLevel(Number.POSITIVE_INFINITY, 0.2, 0.5),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );

  assert.deepEqual(peakHold([4, 1, 3, 2], 2), [4, 4, 3, 3]);
  assert.deepEqual(peakHold([4, 1, 3, 2], 3), [4, 4, 4, 3]);
  assert.deepEqual(peakHold([2, 5, 1], 1), [2, 5, 1], '窗 1 恒等');
  assert.deepEqual(peakHold([2, 5, 1], 10), [2, 5, 5], '窗超长退化为前缀最大');
  assert.deepEqual(peakHold([], 3), [], '空输入空表');
  // f32 输入用 2 的幂分数（精确表示），避免量化误差混入比对。
  assert.deepEqual(peakHold(new Float32Array([0.25, 0.75, 0.5]), 2), [0.25, 0.75, 0.75]);
  for (const window of [0, -1, 2.5, Number.NaN]) {
    assert.throws(
      () => peakHold([1], window),
      (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
      `window 非法必须抛：${window}`,
    );
  }
  assert.throws(
    () => peakHold([1, Number.POSITIVE_INFINITY], 2),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  console.log('  [3] smoothLevel / peakHold OK');
}

// ---------------------------------------------------------------------------
// 4. 录音账本：增删列表 + 0 装校验
// ---------------------------------------------------------------------------
{
  const ledger = new VoiceLedger();
  const note = ledger.add({id: 'vn-1', at: 1000, durationMs: 1500, sizeBytes: 4096, label: '第一条'});
  assert.equal(note.label, '第一条');
  const listed = ledger.list();
  assert.equal(listed.length, 1);
  listed[0].label = '篡改';
  assert.equal(ledger.list()[0].label, '第一条', '读出快照副本防篡改');

  ledger.add({id: 'vn-2', at: 2000, durationMs: 500, sizeBytes: 128});
  assert.deepEqual(
    ledger.list().map((n) => n.id),
    ['vn-1', 'vn-2'],
    '插入序列表',
  );
  const removed = ledger.remove('vn-1');
  assert.equal(removed.id, 'vn-1');
  assert.deepEqual(
    ledger.list().map((n) => n.id),
    ['vn-2'],
  );

  const base = {id: 'vn-x', at: 1, durationMs: 10, sizeBytes: 10};
  assert.throws(
    () => ledger.add({...base, id: ''}),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  ledger.add(base); // 先占 id（下面测冲突）。
  assert.throws(
    () => ledger.add(base),
    (error) => error instanceof VoiceStoreError && error.code === 'duplicate-id',
    'id 冲突必须抛',
  );
  for (const bad of [
    {...base, id: 'vn-y', durationMs: 0},
    {...base, id: 'vn-y', durationMs: -5},
    {...base, id: 'vn-y', durationMs: Number.NaN},
    {...base, id: 'vn-y', sizeBytes: 0},
    {...base, id: 'vn-y', sizeBytes: -1},
    {...base, id: 'vn-y', sizeBytes: Number.POSITIVE_INFINITY},
    {...base, id: 'vn-y', at: Number.NaN},
    {...base, id: 'vn-y', label: 42},
  ]) {
    assert.throws(
      () => ledger.add(bad),
      (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
      '非法字段必须抛',
    );
  }
  assert.throws(
    () => ledger.remove('ghost'),
    (error) => error instanceof VoiceStoreError && error.code === 'missing',
    '删除缺条目必须抛',
  );
  console.log('  [4] voice ledger OK');
}

// ---------------------------------------------------------------------------
// 5. formatDuration：向下取整不虚报
// ---------------------------------------------------------------------------
{
  assert.equal(formatDuration(0), '00:00');
  assert.equal(formatDuration(999), '00:00', '向下取整不虚报');
  assert.equal(formatDuration(1000), '00:01');
  assert.equal(formatDuration(59999), '00:59');
  assert.equal(formatDuration(61000), '01:01');
  assert.equal(formatDuration(3600000), '60:00');
  assert.equal(formatDuration(732000), '12:12');
  assert.throws(
    () => formatDuration(-1),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  assert.throws(
    () => formatDuration(Number.NaN),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  console.log('  [5] formatDuration OK');
}

// ---------------------------------------------------------------------------
// 6. 合成状态机：四态全迁移表（合法/非法/未知输入）
// ---------------------------------------------------------------------------
{
  const legal = {
    idle: {speak: 'speaking'},
    speaking: {finish: 'done', fail: 'error', reset: 'idle'},
    done: {speak: 'speaking', reset: 'idle'},
    error: {speak: 'speaking', reset: 'idle'},
  };
  assert.equal(SPEECH_STATES.length, 4, '四态封顶');
  assert.equal(SPEECH_ACTIONS.length, 4);
  for (const state of SPEECH_STATES) {
    for (const action of SPEECH_ACTIONS) {
      const expected = legal[state][action];
      if (expected) {
        assert.equal(transitionSpeechState(state, action), expected, `合法迁移 ${state}+${action}`);
      } else {
        assert.throws(
          () => transitionSpeechState(state, action),
          (error) =>
            error instanceof VoiceStoreError && error.code === 'illegal-transition',
          `非法迁移必须拒绝：${state}+${action}`,
        );
      }
    }
  }
  assert.throws(
    () => transitionSpeechState('bogus', 'speak'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '未知状态单列 invalid-argument',
  );
  assert.throws(
    () => transitionSpeechState('idle', 'explode'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '未知动作单列 invalid-argument',
  );
  // reset 语义钉死：speaking+reset=主动停止（回 idle），done/error+reset=复位。
  assert.equal(transitionSpeechState('speaking', 'reset'), 'idle');
  assert.equal(transitionSpeechState('done', 'reset'), 'idle');
  assert.equal(transitionSpeechState('error', 'reset'), 'idle');
  console.log('  [6] speech state machine full table OK');
}

// ---------------------------------------------------------------------------
// 7. 合成参数校验矩阵 + 语句账本 0 装
// ---------------------------------------------------------------------------
{
  // 缺省规范（rate/pitch = 1.0）与文本去空白。
  assert.deepEqual(buildSpeechUtterance('hi'), {text: 'hi', rate: 1, pitch: 1});
  assert.deepEqual(buildSpeechUtterance('  你好  '), {text: '你好', rate: 1, pitch: 1});
  // 显式边界值放行（[0.1, 3] 闭区间）。
  assert.deepEqual(
    buildSpeechUtterance('x', {rate: 0.1, pitch: 3}),
    {text: 'x', rate: 0.1, pitch: 3},
    '区间端点放行',
  );
  assert.deepEqual(
    buildSpeechUtterance('x', {rate: undefined, pitch: undefined}),
    {text: 'x', rate: 1, pitch: 1},
    '显式 undefined = 缺省（声明默认值）',
  );

  // 空文本。
  for (const bad of ['', '   ', '\t\n']) {
    assert.throws(
      () => buildSpeechUtterance(bad),
      (error) => error instanceof VoiceStoreError && error.code === 'empty-input',
      `空文本必须抛：${JSON.stringify(bad)}`,
    );
  }
  // 非字符串文本。
  assert.throws(
    () => buildSpeechUtterance(42),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  // rate/pitch 越界/非法矩阵。
  for (const value of [0.09, 3.1, -1, Number.NaN, Number.POSITIVE_INFINITY, 'x']) {
    assert.throws(
      () => buildSpeechUtterance('x', {rate: value}),
      (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
      `rate 非法必须抛：${String(value)}`,
    );
    assert.throws(
      () => buildSpeechUtterance('x', {pitch: value}),
      (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
      `pitch 非法必须抛：${String(value)}`,
    );
  }

  // 语句账本：增删列表 + 快照防篡改 + 0 装。
  const jobs = new SpeechJobLedger();
  const first = jobs.add({id: 'sj-1', text: '第一句', at: 1000, durationEstimateMs: 800});
  assert.equal(first.durationEstimateMs, 800);
  assert.equal(first.text, '第一句');
  const listed = jobs.list();
  listed[0].text = '篡改';
  assert.equal(jobs.list()[0].text, '第一句', '读出快照副本防篡改');

  jobs.add({id: 'sj-2', text: '第二句', at: 2000});
  assert.deepEqual(
    jobs.list().map((job) => job.id),
    ['sj-1', 'sj-2'],
    '插入序列表',
  );
  assert.equal(jobs.list()[1].durationEstimateMs, undefined, '可选字段缺省不自造');
  const removed = jobs.remove('sj-1');
  assert.equal(removed.id, 'sj-1');
  assert.deepEqual(
    jobs.list().map((job) => job.id),
    ['sj-2'],
  );

  const base = {id: 'sj-x', text: 't', at: 1};
  assert.throws(
    () => jobs.add({...base, id: ''}),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
    '空 id 必须抛',
  );
  jobs.add(base);
  assert.throws(
    () => jobs.add(base),
    (error) => error instanceof VoiceStoreError && error.code === 'duplicate-id',
    'id 冲突必须抛',
  );
  for (const bad of [
    {...base, id: 'sj-y', text: ''},
    {...base, id: 'sj-y', text: '   '},
    {...base, id: 'sj-y', at: Number.NaN},
    {...base, id: 'sj-y', durationEstimateMs: 0},
    {...base, id: 'sj-y', durationEstimateMs: -3},
    {...base, id: 'sj-y', durationEstimateMs: Number.POSITIVE_INFINITY},
    {...base, id: 'sj-y', text: 42},
  ]) {
    assert.throws(
      () => jobs.add(bad),
      (error) => error instanceof VoiceStoreError,
      '非法字段必须抛',
    );
  }
  assert.throws(
    () => jobs.remove('ghost'),
    (error) => error instanceof VoiceStoreError && error.code === 'missing',
    '删除缺条目必须抛',
  );
  console.log('  [7] speech params matrix + job ledger OK');
}

// ---------------------------------------------------------------------------
// 8. 转写状态机全表（4 状态 × 4 动作逐一核对合法/非法 + 未知输入）
// ---------------------------------------------------------------------------
{
  const legal = {
    idle: {start: 'recognizing'},
    recognizing: {finish: 'done', fail: 'error', reset: 'idle'},
    done: {start: 'recognizing', reset: 'idle'},
    error: {start: 'recognizing', reset: 'idle'},
  };
  for (const state of TRANSCRIPT_STATES) {
    for (const action of TRANSCRIPT_ACTIONS) {
      const expected = legal[state][action];
      if (expected) {
        assert.equal(
          transitionTranscriptState(state, action),
          expected,
          `合法迁移：${state} + ${action}`,
        );
      } else {
        assert.throws(
          () => transitionTranscriptState(state, action),
          (error) => error instanceof VoiceStoreError && error.code === 'illegal-transition',
          `非法迁移必须抛：${state} + ${action}`,
        );
      }
    }
  }
  assert.throws(
    () => transitionTranscriptState('ghost', 'start'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  assert.throws(
    () => transitionTranscriptState('idle', 'ghost'),
    (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument',
  );
  console.log('  [8] transcript state machine full table OK');
}

// ---------------------------------------------------------------------------
// 9. 转写结果账本（增删列表 + 0 装校验矩阵 + 快照防篡改）
// ---------------------------------------------------------------------------
{
  const ledger = new TranscriptLedger();
  const first = ledger.add({id: 'tr-1', at: 100, text: '你好', confidence: 0.9});
  assert.equal(first.text, '你好');
  assert.equal(first.confidence, 0.9);
  ledger.add({id: 'tr-2', at: 200, text: '世界'});
  assert.deepEqual(ledger.list().map((record) => record.id), ['tr-1', 'tr-2'], '插入序');
  const snapshot = ledger.list();
  snapshot[0].text = '篡改';
  assert.equal(ledger.list()[0].text, '你好', '快照副本防篡改');

  assert.throws(() => ledger.add({id: '', at: 1, text: 'x'}), (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument', '空 id 必须拒');
  assert.throws(() => ledger.add({id: 'tr-1', at: 1, text: 'x'}), (error) => error instanceof VoiceStoreError && error.code === 'duplicate-id', 'id 冲突必须拒');
  assert.throws(() => ledger.add({id: 'tr-3', at: 1, text: '   '}), (error) => error instanceof VoiceStoreError && error.code === 'empty-input', '空文本必须拒');
  assert.throws(() => ledger.add({id: 'tr-3', at: Number.NaN, text: 'x'}), (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument', '非有限时间必须拒');
  assert.throws(() => ledger.add({id: 'tr-3', at: 1, text: 'x', confidence: 1.5}), (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument', '置信度越界必须拒');
  assert.throws(() => ledger.add({id: 'tr-3', at: 1, text: 'x', confidence: Number.NaN}), (error) => error instanceof VoiceStoreError && error.code === 'invalid-argument', '非有限置信度必须拒');
  const noConfidence = ledger.add({id: 'tr-3', at: 3, text: 'x'});
  assert.equal(noConfidence.confidence, undefined, '置信度不自造');
  assert.throws(() => ledger.remove('ghost'), (error) => error instanceof VoiceStoreError && error.code === 'missing', '删缺条目必须拒');
  const removed = ledger.remove('tr-1');
  assert.equal(removed.id, 'tr-1');
  assert.deepEqual(ledger.list().map((record) => record.id), ['tr-2', 'tr-3']);
  console.log('  [9] transcript ledger OK');
}

console.log('--- Starport Voice Check PASSED ---');
