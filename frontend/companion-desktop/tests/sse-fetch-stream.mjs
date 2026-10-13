// SSE fetch 流式读取面 — 线格式解析 / 流生命周期 / 订阅链带令牌过门零回归
// (真实模块 src/lib/sse-stream.ts + src/lib/presence.ts + src/lib/runtime.ts)。
//
// 覆盖 (W2-② 残余 (d) 配对批):
//   ① 帧解析与 EventSource 同口径 (空行分发 / data 多行 / 注释 / 断行边界 /
//      CRLF / 未完帧丢弃 / 无 data 帧不发);
//   ② startSseFetchStream 生命周期 (建链 / 到帧 / 读尽 / 非 2xx / 取消);
//   ③ presence 订阅链带令牌走 fetch 流: 帧解析零回归 (重连/帧/断线/三态徽章/
//      SIM 纪律), 快照与事件双面都带 Bearer;
//   ④ companion 事件订阅链带令牌走 fetch 流: 具名帧路由 + 退避重连零回归。
//
// 已注册: tests/run-all.mjs SUITES (strip:true, 随 pnpm test 全量跑)。
// 独立运行: node --experimental-strip-types tests/sse-fetch-stream.mjs
import assert from 'node:assert/strict';

const {createSseFrameParser, startSseFetchStream} = await import('../src/lib/sse-stream.ts');
const presence = await import('../src/lib/presence.ts');
const {subscribeCompanionEvents} = await import('../src/lib/runtime.ts');

console.log('--- SSE fetch stream (real modules) ---');

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

// ---------------------------------------------------------------------------
// ① 线格式解析 — 与 EventSource 逐条同口径
// ---------------------------------------------------------------------------

check('帧解析: 具名帧 / 无名帧 / 多行 data / 注释忽略', () => {
  const parser = createSseFrameParser();
  const frames = parser.push(
    'event: presence_state\ndata: {"type":"presence_state"}\n\n' +
      'data: hello\n\n' +
      ': heartbeat comment\n' +
      'event: backend_ready\ndata: line-1\ndata: line-2\n\n',
  );
  assert.deepEqual(frames, [
    {event: 'presence_state', data: '{"type":"presence_state"}'},
    {event: 'message', data: 'hello'},
    {event: 'backend_ready', data: 'line-1\nline-2'},
  ]);
});

check('帧解析: 无 data 的帧不发 (含只有 event 行的帧) / 未知字段忽略 / 空 data 也发', () => {
  const parser = createSseFrameParser();
  const frames = parser.push('event: only_event\n\n' + 'id: 7\nretry: 100\nunknown: x\ndata: \n\n');
  assert.deepEqual(frames, [{event: 'message', data: ''}], 'EventSource 同口径: 有 data 行才发帧');
});

check('帧解析: 值只去一个前导空格 / 冒号后即值 / 无冒号行忽略', () => {
  const parser = createSseFrameParser();
  const frames = parser.push('data:  two spaces\ndata:a:b\nnocolon\n\n');
  assert.deepEqual(frames, [{event: 'message', data: ' two spaces\na:b'}]);
});

check('帧解析: 分块切割在行中 / \\r\\n 跨块不断行不重帧', () => {
  const parser = createSseFrameParser();
  assert.deepEqual(parser.push('event: pres'), []);
  assert.deepEqual(parser.push('ence_state\ndata: {"a":'), []);
  assert.deepEqual(parser.push('1}\r'), [], '尾部孤立 \\r 留待下块 (可能是 \\r\\n 的一半)');
  assert.deepEqual(parser.push('\n\r\n'), [{event: 'presence_state', data: '{"a":1}'}]);
});

check('帧解析: 流断时未完帧丢弃不猜 (finish 清缓冲)', () => {
  const parser = createSseFrameParser();
  assert.deepEqual(parser.push('event: presence_state\ndata: {"half"'), []);
  parser.finish();
  assert.deepEqual(parser.push('\n\n'), [], '断流后残帧不补发');
});

// ---------------------------------------------------------------------------
// ② startSseFetchStream 生命周期
// ---------------------------------------------------------------------------

const encoder = new TextEncoder();

function makeStream() {
  const controllers = [];
  const stream = new ReadableStream({
    start(controller) {
      controllers.push(controller);
    },
  });
  return {stream, controllers};
}

{
  // fetch 面拒绝 = 断线如实回报 (不静默降级, 重连决策权在调用方)。
  const realFetch = globalThis.fetch;
  const events = [];
  globalThis.fetch = async () => {
    throw new Error('net down');
  };
  try {
    startSseFetchStream('http://gateway.test/v1/apeireth/events', {Authorization: 'Bearer tok'}, {
      onOpen: () => events.push('open'),
      onFrame: (frame) => events.push(['frame', frame.event, frame.data]),
      onEnd: () => events.push('end'),
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.deepEqual(events, ['end'], 'fetch 拒绝不建链、如实报断线');
  } finally {
    globalThis.fetch = realFetch;
  }
}

{
  // 建链 / 到帧 / 读尽 → onEnd; 非 2xx → 直接 onEnd 不 onOpen。
  const realFetch = globalThis.fetch;
  const {stream, controllers} = makeStream();
  const events = [];
  globalThis.fetch = async (url, init) => {
    events.push(['request', String(url), init.headers]);
    return {ok: true, body: stream};
  };
  try {
    const close = startSseFetchStream('http://gateway.test/v1/apeireth/events', {Authorization: 'Bearer tok-x'}, {
      onOpen: () => events.push('open'),
      onFrame: (frame) => events.push(['frame', frame.event, frame.data]),
      onEnd: () => events.push('end'),
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.deepEqual(events[0], [
      'request',
      'http://gateway.test/v1/apeireth/events',
      {Authorization: 'Bearer tok-x', Accept: 'text/event-stream'},
    ]);
    assert.ok(events.includes('open'), '响应过门即建链');
    controllers[0].enqueue(encoder.encode('event: ping\ndata: 1\n\n'));
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.ok(events.some((e) => Array.isArray(e) && e[0] === 'frame' && e[1] === 'ping' && e[2] === '1'));
    assert.ok(!events.includes('end'), '流未尽不报断线');
    controllers[0].close();
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.ok(events.includes('end'), '流读尽 = 断线如实回报 (重连决策权在调用方)');
    close();
  } finally {
    globalThis.fetch = realFetch;
  }

  {
    const {stream: deadStream} = makeStream();
    const bad = [];
    globalThis.fetch = async () => ({ok: false, status: 401, body: deadStream});
    try {
      startSseFetchStream('http://gateway.test/v1/apeireth/events', {}, {
        onOpen: () => bad.push('open'),
        onFrame: () => bad.push('frame'),
        onEnd: () => bad.push('end'),
      });
      await new Promise((resolve) => setTimeout(resolve, 0));
      assert.deepEqual(bad, ['end'], '非 2xx (含 401 过门失败) 不建链、只报断线');
    } finally {
      globalThis.fetch = realFetch;
    }
  }

  {
    const {stream: s2, controllers: c2} = makeStream();
    const quiet = [];
    globalThis.fetch = async () => ({ok: true, body: s2});
    try {
      const close = startSseFetchStream('http://gateway.test/v1/apeireth/events', {}, {
        onOpen: () => quiet.push('open'),
        onFrame: (frame) => quiet.push(['frame', frame.data]),
        onEnd: () => quiet.push('end'),
      });
      await new Promise((resolve) => setTimeout(resolve, 0));
      close();
      c2[0].enqueue(encoder.encode('data: late\n\n'));
      c2[0].close();
      await new Promise((resolve) => setTimeout(resolve, 0));
      assert.ok(!quiet.some((e) => Array.isArray(e)), '取消后到帧不再回调');
      assert.ok(!quiet.includes('end'), '取消后不报断线 (取消不是断线)');
    } finally {
      globalThis.fetch = realFetch;
    }
  }
  check('流生命周期: 建链/到帧/读尽=断线/非 2xx 只断线/取消静默', () => {});
}

// ---------------------------------------------------------------------------
// ③ presence 订阅链带令牌走 fetch 流 (重连/帧/断线/三态徽章/SIM 零回归)
// ---------------------------------------------------------------------------

const TURN_FRAME_OBJ = {
  type: 'presence_state',
  at: 1755820200000,
  pad: {p: 0.2, a: -0.1, d: 0.3},
  dominant: 'serene',
  intensity: 0.38,
  stance: 'attentive_presence',
  breath: {period_secs: 4, amplitude: 0.6},
  significance: 'turn',
  source: {kind: 'heuristic_v0', confidence: 0.5},
};
const TURN_FRAME = JSON.stringify(TURN_FRAME_OBJ);

/** 后续实时帧 (store 去重键 = type|at|significance, 换 at 才算新帧)。 */
function frameAt(at, p, significance = 'turn', breath = {period_secs: 4, amplitude: 0.6}) {
  return JSON.stringify({...TURN_FRAME_OBJ, at, pad: {p, a: 0, d: 0}, significance, breath});
}

{
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const realWindow = globalThis.window;
  const realFetch = globalThis.fetch;
  const timers = [];
  globalThis.setTimeout = (fn, ms) => {
    const timer = {fn, ms, cleared: false};
    timers.push(timer);
    return timer;
  };
  globalThis.clearTimeout = (timer) => {
    if (timer && typeof timer === 'object') timer.cleared = true;
  };
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd) => (cmd === 'get_gateway_token' ? 'tok-sse-1' : null),
    },
  };

  const requests = [];
  const streams = [];
  globalThis.fetch = async (url, init) => {
    const target = String(url);
    requests.push({url: target, headers: init?.headers});
    if (target.endsWith('/v1/apeireth/presence')) {
      return {
        ok: true,
        text: async () =>
          JSON.stringify({
            type: 'presence_snapshot',
            frame: JSON.parse(TURN_FRAME),
            failure: null,
          }),
      };
    }
    const made = makeStream();
    streams.push(made);
    return {ok: true, body: made.stream};
  };

  try {
    const local = presence.createPresenceStore(() => 7);
    const cancel = presence.subscribePresence('http://gateway.test/', {store: local});
    const settle = () => new Promise((resolve) => realSetTimeout(resolve, 0));
    await settle();

    // 快照 + 事件双面都带 Bearer (令牌门全 API 面化后 401 根因面)。
    assert.equal(requests.length, 2, '快照先取 + 事件建流');
    for (const req of requests) {
      assert.equal(req.headers.Authorization, 'Bearer tok-sse-1', `${req.url} 必须带 Bearer`);
    }
    assert.ok(requests.some((r) => r.url.endsWith('/v1/apeireth/presence')));
    assert.ok(requests.some((r) => r.url.endsWith('/v1/apeireth/events')));
    assert.notEqual(local.get().current, null, '快照即读落位');
    assert.equal(local.get().connected, true, '建链 = 三态徽章「已连接」真值');

    // 到帧: presence_state 具名帧按既有帧解析语义落位 (新 at = 新帧, 不被去重)。
    assert.equal(local.get().recentEvents.length, 1, '快照帧已入账');
    streams[0].controllers[0].enqueue(encoder.encode(`event: presence_state\ndata: ${frameAt(1755820260000, 0.9)}\n\n`));
    await settle();
    assert.equal(local.get().recentEvents.length, 2, '帧解析零回归: 实时帧入账');
    assert.equal(local.get().lastSignificance, 'turn');
    streams[0].controllers[0].enqueue(
      encoder.encode(`event: presence_state\ndata: ${frameAt(1755820320000, 0, 'heartbeat', {period_secs: 5, amplitude: 0.8})}\n\n`),
    );
    await settle();
    assert.deepEqual(local.get().breath, {period_secs: 5, amplitude: 0.8}, '显影分级零回归: 心跳只动余烬点通道');

    // 断线: 读尽 = 断线 → 徽章「连接中」+ 退避重连计时 + SIM 降级计时。
    streams[0].controllers[0].close();
    await settle();
    assert.equal(local.get().connected, false, '断线如实回报 (不装在线)');
    const retry = timers.find((t) => !t.cleared && t.ms === 2000);
    assert.ok(retry, '指数退避重连计时器已挂 (2s 起)');
    const sim = timers.find((t) => !t.cleared && t.ms === 30000);
    assert.ok(sim, 'SIM 降级计时器已挂 (断连 >30s 纪律)');
    sim.fn();
    assert.equal(local.get().simulated, true, '三态徽章「未连接」+ SIM 标注零回归');

    // 重连: 再取快照 + 重建流, 帧继续落位。
    retry.fn();
    await settle();
    assert.equal(requests.length, 4, '重连再取快照 + 重建事件流');
    assert.equal(local.get().connected, true, '重连建链徽章回真');
    const beforeRelay = local.get().recentEvents.length;
    streams[1].controllers[0].enqueue(encoder.encode(`event: presence_state\ndata: ${frameAt(1755820380000, 0.5)}\n\n`));
    await settle();
    assert.equal(local.get().recentEvents.length, beforeRelay + 1, '重连后帧继续入账 (逐帧 +1, 不重不漏)');

    cancel();
    streams[1].controllers[0].close();
    await settle();
    assert.equal(local.get().connected, false, '取消订阅如实收尾');
    assert.equal(requests.length, 4, '取消后不再重连');
  } finally {
    globalThis.fetch = realFetch;
    globalThis.setTimeout = realSetTimeout;
    globalThis.clearTimeout = realClearTimeout;
    if (realWindow === undefined) delete globalThis.window;
    else globalThis.window = realWindow;
  }
  check('presence 带令牌 fetch 流: Bearer 双面 / 帧解析 / 断线退避 / SIM / 重连零回归', () => {});
}

// ---------------------------------------------------------------------------
// ④ companion 事件订阅链带令牌走 fetch 流 (具名帧路由 + 退避重连)
// ---------------------------------------------------------------------------

{
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const realWindow = globalThis.window;
  const realFetch = globalThis.fetch;
  const timers = [];
  globalThis.setTimeout = (fn, ms) => {
    const timer = {fn, ms, cleared: false};
    timers.push(timer);
    return timer;
  };
  globalThis.clearTimeout = (timer) => {
    if (timer && typeof timer === 'object') timer.cleared = true;
  };
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd) => (cmd === 'get_gateway_token' ? 'tok-sse-2' : null),
    },
  };

  const requests = [];
  const streams = [];
  globalThis.fetch = async (url, init) => {
    requests.push({url: String(url), headers: init?.headers});
    const made = makeStream();
    streams.push(made);
    return {ok: true, body: made.stream};
  };

  try {
    const seen = [];
    const cancel = subscribeCompanionEvents(
      {baseUrl: 'http://gateway.test', model: 'm-x', apiKey: 'sk-stale'},
      (event) => seen.push(event),
    );
    const settle = () => new Promise((resolve) => realSetTimeout(resolve, 0));
    await settle();

    assert.equal(requests[0].headers.Authorization, 'Bearer tok-sse-2', '事件频道带 Bearer 过门');
    assert.ok(!JSON.stringify(requests[0].headers).includes('sk-stale'), '陈旧凭据不进头');

    streams[0].controllers[0].enqueue(
      encoder.encode(
        'event: backend_ready\ndata: {}\n\n' +
          'event: approval_required\ndata: {"tool_name":"shell"}\n\n' +
          'event: approval_resolved\ndata: {"approval_id":"a1"}\n\n' +
          'data: [他说] 无名帧不进本通道\n\n',
      ),
    );
    await settle();
    assert.deepEqual(
      seen.map((e) => e.kind),
      ['backend_ready', 'approval_required', 'approval_resolved'],
      '具名帧路由零回归 (无名帧不进 companion 通道)',
    );
    assert.equal(seen[0].text, '后端已就绪');
    assert.equal(seen[1].text, '[需要批准] shell');
    assert.equal(seen[2].text, '[审批已处理]');
    assert.equal(seen[1].payload.tool_name, 'shell');

    streams[0].controllers[0].close();
    await settle();
    const retry = timers.find((t) => !t.cleared && t.ms === 2000);
    assert.ok(retry, '断线按 2s 起退避续排');
    retry.fn();
    await settle();
    assert.equal(requests.length, 2, '重连重建事件流');

    cancel();
  } finally {
    globalThis.fetch = realFetch;
    globalThis.setTimeout = realSetTimeout;
    globalThis.clearTimeout = realClearTimeout;
    if (realWindow === undefined) delete globalThis.window;
    else globalThis.window = realWindow;
  }
  check('companion 事件带令牌 fetch 流: 具名帧路由 + 退避重连零回归', () => {});
}

console.log(`--- ${checks} checks ---`);
