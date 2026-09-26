// Presence 频道客户端 — presence_state 契约帧解析 / mode 推导 / 平滑插值 / 显影派生 /
// store 行为 纯逻辑单测。直接 import 真实实现 ../src/lib/presence.ts
// (Node ≥23.6 默认类型擦除; presence.ts 零运行时依赖、DOM/EventSource/rAF
// 全部惰性守卫, 可在 Node 下加载)。
// 契约依据: docs 契约 §8a (presence_state 具名 SSE 帧, 回合级 + 60s 心跳)。
// 不依赖后端。legacy 四事件 (emotion/initiative/dream/memory_recall) 已退役,
// 考古注释见 presence.ts 头注, 此套件不再覆盖。
import assert from 'node:assert/strict';

import {
  parsePresenceStateFrame,
  parsePresenceSnapshot,
  applyPresenceSnapshot,
  subscribePresence,
  derivePresenceMode,
  approachExponential,
  derivePresenceGlow,
  deriveEmberBreath,
  createPresenceStore,
  PRESENCE_GLOW_REST,
  EMBER_REST_BREATH,
  SMOOTHING_RATE_PER_SEC,
  RECENT_EVENTS_CAPACITY,
} from '../src/lib/presence.ts';

console.log('--- Starting presence_state Contract Check ---');

/** 契约 §8a 线上帧实例 (回合级) */
const TURN_FRAME_JSON =
  '{"type":"presence_state","at":1755820200000,"pad":{"p":0.2,"a":-0.1,"d":0.3},' +
  '"dominant":"serene","intensity":0.38,"stance":"attentive_presence",' +
  '"breath":{"period_secs":4,"amplitude":0.6},"significance":"turn",' +
  '"source":{"kind":"heuristic_v0","confidence":0.5}}';

function makeFrame(overrides = {}) {
  return {
    type: 'presence_state',
    at: 1755820200000,
    pad: {p: 0.2, a: -0.1, d: 0.3},
    dominant: 'serene',
    intensity: 0.38,
    stance: 'attentive_presence',
    breath: {period_secs: 4, amplitude: 0.6},
    significance: 'turn',
    source: {kind: 'heuristic_v0', confidence: 0.5},
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// 1. parsePresenceStateFrame — 契约形状 / 严格主键 / 防御钳制 / 前向兼容
// ---------------------------------------------------------------------------

// ① 契约 §8a 实例原样解析
{
  const f = parsePresenceStateFrame(TURN_FRAME_JSON);
  assert.ok(f, '契约实例必须解析成功');
  assert.equal(f.type, 'presence_state');
  assert.equal(f.at, 1755820200000);
  assert.deepEqual(f.pad, {p: 0.2, a: -0.1, d: 0.3});
  assert.equal(f.dominant, 'serene');
  assert.equal(f.intensity, 0.38);
  assert.equal(f.stance, 'attentive_presence');
  assert.deepEqual(f.breath, {period_secs: 4, amplitude: 0.6});
  assert.equal(f.significance, 'turn');
  assert.deepEqual(f.source, {kind: 'heuristic_v0', confidence: 0.5});
}

// ② 坏 JSON / 非对象 / 错 type → null (宁可丢弃不猜)
{
  assert.equal(parsePresenceStateFrame('{"type":"presence_state",broken'), null);
  assert.equal(parsePresenceStateFrame(''), null);
  assert.equal(parsePresenceStateFrame('[1,2,3]'), null);
  assert.equal(parsePresenceStateFrame('null'), null);
  assert.equal(parsePresenceStateFrame('42'), null);
  assert.equal(parsePresenceStateFrame('{"type":"emotion","at":1}'), null, 'legacy 帧不再是本频道契约');
  assert.equal(parsePresenceStateFrame('{"foo":1}'), null);
}

// ③ 主键缺失/不合法 → null (at / stance / significance)
{
  assert.equal(parsePresenceStateFrame('{"type":"presence_state"}'), null, '缺 at');
  assert.equal(parsePresenceStateFrame('{"type":"presence_state","at":"2026-08-21"}'), null, 'at 必须 epoch 毫秒数值');
  assert.equal(
    parsePresenceStateFrame('{"type":"presence_state","at":1,"significance":"turn"}'),
    null,
    '缺 stance',
  );
  assert.equal(
    parsePresenceStateFrame('{"type":"presence_state","at":1,"stance":"curious_glance","significance":"turn"}'),
    null,
    '未知 stance 丢弃 (不猜)',
  );
  assert.equal(
    parsePresenceStateFrame('{"type":"presence_state","at":1,"stance":"attentive_presence","significance":"epoch"}'),
    null,
    '未知 significance 丢弃',
  );
}

// ④ 数值防御钳制 + 缺省回落
{
  const f = parsePresenceStateFrame(
    '{"type":"presence_state","at":1,"pad":{"p":9,"a":-9,"d":"x"},' +
      '"intensity":7,"stance":"deep_coding_focus","breath":{"period_secs":0,"amplitude":3},' +
      '"significance":"heartbeat","source":{"confidence":-2}}',
  );
  assert.ok(f);
  assert.deepEqual(f.pad, {p: 1, a: -1, d: 0}, 'pad 钳到 [-1,1], 非数回 0');
  assert.equal(f.intensity, 1, 'intensity 钳到 [0,1]');
  assert.equal(f.breath.period_secs, 1, 'period_secs 下限 1s');
  assert.equal(f.breath.amplitude, 1, 'amplitude 钳到 [0,1]');
  assert.equal(f.source.kind, 'unknown', '缺 source.kind 显式 unknown');
  assert.equal(f.source.confidence, 0, 'confidence 钳到 [0,1]');
  assert.equal(f.dominant, '', '缺 dominant 回空串');
}

// ⑤ 前向兼容: 未知附加字段容忍; 可选子对象整体缺失也有缺省
{
  const f = parsePresenceStateFrame(
    '{"type":"presence_state","at":1,"stance":"dreaming_consolidation","significance":"ritual","future_field":{"x":1}}',
  );
  assert.ok(f, '未知附加字段不影响解析');
  assert.equal(f.significance, 'ritual');
  assert.deepEqual(f.pad, {p: 0, a: 0, d: 0});
  assert.equal(f.breath.period_secs, 4.0, 'breath 缺失回落契约基线 4s');
}

console.log('✓ parsePresenceStateFrame: 契约实例 / 坏帧丢弃 / 钳制缺省 / 前向兼容');

// ---------------------------------------------------------------------------
// 2. derivePresenceMode — speaking > thinking > quiet (本地真实信号)
// ---------------------------------------------------------------------------

assert.equal(derivePresenceMode({speaking: true, chatActive: true}), 'speaking');
assert.equal(derivePresenceMode({speaking: true, chatActive: false}), 'speaking');
assert.equal(derivePresenceMode({speaking: false, chatActive: true}), 'thinking');
assert.equal(derivePresenceMode({speaking: false, chatActive: false}), 'quiet');

console.log('✓ derivePresenceMode: speaking 优先 / chatActive → thinking / quiet 回落');

// ---------------------------------------------------------------------------
// 3. approachExponential — rate 2.4/s 指数趋近 (motion.stateSmoothing)
// ---------------------------------------------------------------------------

// 经过 1/rate 秒, 残差 = 1/e
{
  const dtMs = 1000 / SMOOTHING_RATE_PER_SEC;
  const next = approachExponential(0, 1, dtMs);
  assert.ok(Math.abs(next - (1 - 1 / Math.E)) < 1e-9, `1/rate 步进应为 1-1/e, 实得 ${next}`);
}
// dt<=0 不动; 显式 rate 参数生效
assert.equal(approachExponential(0.5, 1, 0), 0.5);
assert.equal(approachExponential(0.5, 1, -16), 0.5);
{
  const next = approachExponential(0, 1, 1000, 1); // rate=1/s, 1s → 1-1/e
  assert.ok(Math.abs(next - (1 - 1 / Math.E)) < 1e-9);
}
// 长期收敛: 60fps 跑 4s ≈ 9.6 个时间常数
{
  let v = 0;
  for (let i = 0; i < 240; i++) v = approachExponential(v, 0.8, 16.7);
  assert.ok(Math.abs(v - 0.8) < 1e-3, '240 帧 × 16.7ms 后应基本收敛到目标');
}

console.log('✓ approachExponential: rate 语义 / 零 dt 不动 / 长期收敛');

// ---------------------------------------------------------------------------
// 4. derivePresenceGlow — T1 光晕钩子 (null 静息 / mode 项 / clamp)
// ---------------------------------------------------------------------------

assert.equal(derivePresenceGlow(null), PRESENCE_GLOW_REST, '无真实数据 → 静息微光 0.14');

function makeCurrent(overrides = {}) {
  return {
    p: 0,
    a: 0,
    d: 0,
    intensity: 0,
    stance: 'attentive_presence',
    mode: 'quiet',
    sourceKind: 'heuristic_v0',
    sourceConfidence: 0.5,
    at: 1,
    ...overrides,
  };
}

{
  const quiet = derivePresenceGlow(makeCurrent({mode: 'quiet'}));
  const thinking = derivePresenceGlow(makeCurrent({mode: 'thinking'}));
  const speaking = derivePresenceGlow(makeCurrent({mode: 'speaking'}));
  assert.ok(speaking > thinking && thinking > quiet, 'speaking > thinking > quiet');
  assert.ok(quiet > PRESENCE_GLOW_REST, '有帧时高于静息 (bright 项有下限贡献)');
}
// p/intensity 正向抬高; 极端值 clamp 到 [0.08, 0.65]
{
  const dim = derivePresenceGlow(makeCurrent({p: -1, intensity: 0}));
  const bright = derivePresenceGlow(makeCurrent({p: 1, intensity: 1, mode: 'speaking'}));
  assert.ok(bright > dim, 'bright/intensity 正向抬高');
  assert.ok(bright <= 0.65, '上限 0.65');
  assert.ok(dim >= 0.08, '下限 0.08');
}

console.log('✓ derivePresenceGlow: 静息回落 / mode 项 / clamp 范围');

// ---------------------------------------------------------------------------
// 5. deriveEmberBreath — T0 余烬点呼吸参数回落
// ---------------------------------------------------------------------------

assert.deepEqual(deriveEmberBreath(null), EMBER_REST_BREATH, '无帧 → 契约基线 4s/0.65');
assert.deepEqual(deriveEmberBreath({period_secs: 6, amplitude: 0.4}), {periodSecs: 6, amplitude: 0.4});
assert.deepEqual(
  deriveEmberBreath({period_secs: 0.2, amplitude: 5}),
  {periodSecs: 1, amplitude: 1},
  '周期下限 1s / 振幅钳到 [0,1]',
);

console.log('✓ deriveEmberBreath: null 回落 / 透传 / 钳制');

// ---------------------------------------------------------------------------
// 6. createPresenceStore — 显影分级 / 去重 / SIM 纪律 / subscribe 契约
// ---------------------------------------------------------------------------

// 注入假时钟, 避免测试依赖真实时间
let fakeNow = 1_000_000;
const store = createPresenceStore(() => fakeNow);

// 初始: 无真实数据 → current/breath null, 不 simulated
assert.equal(store.get().current, null);
assert.equal(store.get().breath, null);
assert.equal(store.get().simulated, false);
assert.equal(store.get().connected, false);

// turn 帧驱动 current + breath (首帧直接落位; Node 无 rAF, 插值循环惰性跳过)
{
  store.ingest(makeFrame());
  const s = store.get();
  assert.deepEqual({p: s.current.p, a: s.current.a, d: s.current.d}, {p: 0.2, a: -0.1, d: 0.3});
  assert.equal(s.current.intensity, 0.38);
  assert.equal(s.current.stance, 'attentive_presence');
  assert.equal(s.current.mode, 'quiet');
  assert.equal(s.current.sourceKind, 'heuristic_v0');
  assert.equal(s.current.sourceConfidence, 0.5);
  assert.equal(s.current.at, 1755820200000);
  assert.deepEqual(s.breath, {period_secs: 4, amplitude: 0.6});
  assert.equal(s.lastSignificance, 'turn');
  assert.equal(s.connected, true, '真实事件到达 = 频道有活性');
}

// 显影分级纪律: 心跳帧只动余烬点, 不动 current.pad/stance/intensity
{
  const before = store.get().current;
  store.ingest(
    makeFrame({
      at: 1755820260000,
      pad: {p: -0.9, a: 0.9, d: -0.8}, // 心跳携带的 pad 不得显影
      intensity: 0.99,
      stance: 'deep_coding_focus',
      breath: {period_secs: 5, amplitude: 0.8},
      significance: 'heartbeat',
    }),
  );
  const s = store.get();
  assert.deepEqual({p: s.current.p, a: s.current.a, d: s.current.d}, {p: 0.2, a: -0.1, d: 0.3}, '心跳不动 PAD');
  assert.equal(s.current.intensity, 0.38, '心跳不动 intensity');
  assert.equal(s.current.stance, 'attentive_presence', '心跳不动 stance');
  assert.equal(s.current.at, 1755820200000, '心跳不动帧时刻');
  assert.deepEqual(s.breath, {period_secs: 5, amplitude: 0.8}, '心跳只动 breath 通道');
  assert.equal(s.lastSignificance, 'heartbeat');
  assert.equal(s.recentEvents.length, 2, '心跳仍入考古缓冲');
}

// 去重: 同 (type|at|significance) 帧到达两次只记一次 (双订阅挂接防御)
{
  const before = store.get().recentEvents.length;
  const dup = makeFrame({at: 1755820300000, significance: 'heartbeat'});
  store.ingest(dup);
  store.ingest(dup);
  assert.equal(store.get().recentEvents.length, before + 1, '同帧去重');
  // 不同 at 的同 significance 帧不去重
  store.ingest(makeFrame({at: 1755820360000, significance: 'heartbeat'}));
  assert.equal(store.get().recentEvents.length, before + 2);
}

// 环形缓冲容量 50, 最新在前
{
  store.reset();
  for (let i = 0; i < RECENT_EVENTS_CAPACITY + 10; i++) {
    store.ingest(makeFrame({at: 1755830000000 + i * 1000, significance: 'heartbeat'}));
  }
  const evts = store.get().recentEvents;
  assert.equal(evts.length, RECENT_EVENTS_CAPACITY);
  assert.equal(evts[0].event.at, 1755830000000 + (RECENT_EVENTS_CAPACITY + 9) * 1000, '最新在前');
}

// SIM 纪律: 断连 >30s → simulated=true, current 回落本机中性默认 (不编造情绪)
{
  store.reset();
  store.ingest(makeFrame());
  store.setSimulated(true);
  const s = store.get();
  assert.equal(s.simulated, true);
  assert.deepEqual({p: s.current.p, a: s.current.a, d: s.current.d}, {p: 0, a: 0, d: 0});
  assert.equal(s.current.sourceKind, 'local_fallback', 'simulated 显式标注本机回落');
  assert.equal(s.breath, null, 'simulated 时 breath 不保留 (不伪造呼吸)');
  // 连通恢复 → simulated 清除, 真实 (虽陈旧的) 数据恢复
  store.setConnected(true);
  const r = store.get();
  assert.equal(r.simulated, false);
  assert.equal(r.connected, true);
  assert.notEqual(r.current, null, '有过真实帧时重连后恢复真实值而非 null');
  assert.equal(r.current.sourceKind, 'heuristic_v0');
}

// setSpeaking / setChatActive 驱动 mode (speaking pulse 源 = 本地流式)
store.setChatActive(true);
assert.equal(store.get().current.mode, 'thinking');
store.setSpeaking(true);
assert.equal(store.get().current.mode, 'speaking');
store.setSpeaking(false);
store.setChatActive(false);
assert.equal(store.get().current.mode, 'quiet');

// subscribe 契约: 立即回放 + 变更推送 + 退订
{
  store.reset();
  const seen = [];
  const unsubscribe = store.subscribe((state) => seen.push(state));
  assert.equal(seen.length, 1, 'subscribe 立即回放当前快照');
  assert.equal(seen[0].current, null);
  store.ingest(makeFrame());
  assert.equal(seen.length, 2, 'ingest 推送新快照');
  assert.equal(seen[1].current.p, 0.2);
  unsubscribe();
  store.ingest(makeFrame({at: 1755820400000}));
  assert.equal(seen.length, 2, '退订后不再推送');
}

console.log('✓ presenceStore: 显影分级 (心跳只动余烬点) / 去重 / SIM 纪律 / subscribe 契约');

// ---------------------------------------------------------------------------
// 快照口 (GET /v1/apeireth/presence): 快照即读 / 失败即帧 / 重连先取快照
// ---------------------------------------------------------------------------

const SNAPSHOT_JSON = JSON.stringify({
  type: 'presence_snapshot',
  frame: makeFrame(),
  failure: null,
});

// ① 快照解析: 帧走与实时帧同一套校验; failure 显式携带, 不猜不补
{
  const s = parsePresenceSnapshot(SNAPSHOT_JSON);
  assert.ok(s, '合法快照必须解析成功');
  assert.equal(s.frame.type, 'presence_state');
  assert.equal(s.frame.at, 1755820200000);
  assert.equal(s.failure, null, '无失败不伪造失败帧');

  const withFailure = parsePresenceSnapshot(
    JSON.stringify({type: 'presence_snapshot', frame: null, failure: 'producer crashed'}),
  );
  assert.equal(withFailure.frame, null);
  assert.equal(withFailure.failure, 'producer crashed', '失败即帧: 显式事实原样携带');
}

// ② 坏形状 → null / 非法帧丢弃不猜 (宁可丢弃不猜)
{
  assert.equal(parsePresenceSnapshot('{"type":"presence_snapshot",broken'), null);
  assert.equal(parsePresenceSnapshot('{"type":"other"}'), null);
  assert.equal(parsePresenceSnapshot('[]'), null);
  assert.equal(parsePresenceSnapshot('42'), null);
  const badFrame = parsePresenceSnapshot(
    JSON.stringify({type: 'presence_snapshot', frame: {type: 'presence_state'}, failure: null}),
  );
  assert.equal(badFrame.frame, null, '帧缺主键宁可丢弃不猜');
}

// ③ 快照即读: applyPresenceSnapshot 立即落位, 无需等下一条实时帧
{
  const local = createPresenceStore(() => 42);
  applyPresenceSnapshot(local, parsePresenceSnapshot(SNAPSHOT_JSON));
  const s = local.get();
  assert.notEqual(s.current, null, '快照即读立即落位');
  assert.equal(s.current.p, 0.2);
  assert.equal(s.connected, true, '快照到达 = 频道有活性');
  // 只有失败帧: 不编造任何数据 (降级表现交给 SIM 纪律)
  const empty = createPresenceStore(() => 42);
  applyPresenceSnapshot(empty, {frame: null, failure: 'gap'});
  assert.equal(empty.get().current, null, '失败帧不伪造数据');
}

// ④ 重连先取快照: 首连与每次重连都先取快照落位, 断线期事件不回放
{
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const realEventSource = globalThis.EventSource;
  const timers = [];
  globalThis.setTimeout = (fn, ms) => {
    const timer = {fn, ms, cleared: false};
    timers.push(timer);
    return timer;
  };
  globalThis.clearTimeout = (timer) => {
    if (timer && typeof timer === 'object') timer.cleared = true;
  };

  class FakeEventSource {
    constructor(url) {
      this.url = url;
      this.listeners = new Map();
      this.closed = false;
      FakeEventSource.instances.push(this);
    }
    addEventListener(name, fn) {
      this.listeners.set(name, fn);
    }
    close() {
      this.closed = true;
    }
  }
  FakeEventSource.instances = [];
  globalThis.EventSource = FakeEventSource;

  const settle = () => new Promise((resolve) => realSetTimeout(resolve, 0));
  const local = createPresenceStore(() => 7);
  let snapshotCalls = 0;
  const cancel = subscribePresence('http://gateway.test/', {
    store: local,
    fetchSnapshot: async () => {
      snapshotCalls += 1;
      return SNAPSHOT_JSON;
    },
  });

  await settle();
  assert.equal(snapshotCalls, 1, '首次连接先取快照');
  assert.equal(FakeEventSource.instances.length, 1);
  assert.notEqual(local.get().current, null, '快照即读: 不等实时帧即落位');
  assert.equal(local.get().current.p, 0.2);

  // 断线 → 指数退避重连: 重连同样先取快照
  FakeEventSource.instances[0].onerror();
  assert.equal(FakeEventSource.instances[0].closed, true);
  const retry = timers.find((t) => !t.cleared && t.ms === 2000);
  assert.ok(retry, '指数退避重连计时器已挂');
  retry.fn();
  await settle();
  assert.equal(snapshotCalls, 2, '重连再取快照');
  assert.equal(FakeEventSource.instances.length, 2, '重连重建连接');
  assert.notEqual(local.get().current, null, '重连后立即读到当前帧');

  cancel();
  globalThis.setTimeout = realSetTimeout;
  globalThis.clearTimeout = realClearTimeout;
  if (realEventSource === undefined) {
    delete globalThis.EventSource;
  } else {
    globalThis.EventSource = realEventSource;
  }
}

console.log('✓ 快照口: 快照即读 / 失败即帧 / 重连先取快照 (GET /v1/apeireth/presence)');

console.log('--- presence_state Contract Check PASSED ---');
