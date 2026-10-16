// 随手记（Quick Capture）— 纯采集入记忆流 / 零 token / 回看（真实模块直测）。
//
// 覆盖（C 轴入口面批，测试 ≥8）：
//   ① 快捷窗模式切换：`#` 前缀 = 随手记，去前缀回对话（detectQuickMode 全表）；
//   ② 随手记入记忆流：走 appendMemoryEpisode 同源缝（POST /v1/memory/append），
//      正文 = 原文 + 备注级标注（内容/来源标注/序号）；
//   ③ 未保护级：source=capture / protected=false，全程零 protect/unprotect/forget 通道；
//   ④ 零模型调用（纯采集零 token）：唯一请求面 = 记忆追加，零 chat/completions，
//      零通知；模块/接线层零模型面（源码镜像）；
//   ⑤ 回看列表：最近 N 条只读展示（新→旧、返回副本）；
//   ⑥ 空内容拒：`#` 后无内容显式拒绝（拒帧），不写流、不发号；
//   ⑦ 并发连记：多条并发采集序号连续（1,2,3…），条条入流；
//   ⑧ 账本：持久化 round-trip / 超限丢最旧序号不动 / 发号只发不收；
//   ⑨ 接线镜像：QuickWindowView 随手记分支直连采集缝、不进对话回合、不发通知。
//
// 已注册: tests/run-all.mjs SUITES (strip:true, 随 pnpm test 全量跑)。
// 独立运行: node --experimental-strip-types tests/quick-capture.mjs
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const qc = await import('../src/features/quick/quick-capture.ts');

console.log('--- quick capture (real modules) ---');

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

/** 注入式账本存储（node 无 localStorage 也能钉持久化语义）。 */
function fakeStorage(initial = null) {
  let raw = initial;
  return {
    read: () => raw,
    write: (next) => {
      raw = next;
    },
    dump: () => raw,
  };
}

/** fetch 记录面：只放行 /v1/memory/append（200），其余照记不误（零模型断言用）。 */
function installFetchRecorder() {
  const realFetch = globalThis.fetch;
  const calls = [];
  globalThis.fetch = async (input, init) => {
    calls.push({
      url: String(input),
      method: init?.method ?? 'GET',
      body: typeof init?.body === 'string' ? init.body : '',
    });
    return new Response('{"ok":true}', {status: 200, headers: {'content-type': 'application/json'}});
  };
  return {
    calls,
    restore() {
      globalThis.fetch = realFetch;
    },
  };
}

const cfg = {baseUrl: 'http://gateway.test', apiKey: '', model: 'm-x'};

// ---------------------------------------------------------------------------
// ① 快捷窗模式切换：`#` 前缀进随手记，去前缀回对话（三选一取最简 = 斜杠前缀）
// ---------------------------------------------------------------------------
check('模式切换: `#` 前缀 = 随手记 / 去前缀 = 对话（即时切换）', () => {
  assert.equal(qc.detectQuickMode('总结今天'), 'chat', '普通输入 = 对话');
  assert.equal(qc.detectQuickMode(''), 'chat', '空输入 = 对话（发送键禁用）');
  assert.equal(qc.detectQuickMode('# 明早买牛奶'), 'capture', '`#` 前缀 = 随手记');
  assert.equal(qc.detectQuickMode('#明早买牛奶'), 'capture', '无空格紧跟也算随手记');
  assert.equal(qc.detectQuickMode('  # 明早买牛奶'), 'capture', '前导空白不破坏前缀判定');
  assert.equal(qc.detectQuickMode('#'), 'capture', '裸 `#` = 随手记模式（内容为空 → 拒）');
  // 同一输入往返切换 = 前缀即模式，无需独立 tab/按钮
  assert.equal(qc.detectQuickMode('#' + '笔记'), 'capture');
  assert.equal(qc.detectQuickMode('#笔记'.slice(1)), 'chat', '删前缀即回对话模式');
});

// ---------------------------------------------------------------------------
// ② 随手记入记忆流（内容）+ 备注级标注（来源 capture + 连记序号）
// ---------------------------------------------------------------------------
{
  const recorder = installFetchRecorder();
  try {
    const ledger = new qc.CaptureLedger(fakeStorage());
    const outcome = await qc.captureToMemoryStream(cfg, ledger, '# 明早买牛奶', 1_000);
    assert.equal(outcome.ok, true, '采集成功');
    assert.equal(recorder.calls.length, 1, '恰好一次记忆追加（零多余请求）');
    const call = recorder.calls[0];
    assert.equal(call.method, 'POST');
    assert.ok(call.url.endsWith('/v1/memory/append'), `走 appendMemoryEpisode 同源缝（实际=${call.url}）`);
    const body = JSON.parse(call.body);
    assert.equal(body.role, 'user', '契约 §5 role=user');
    assert.equal(typeof body.session, 'string');
    assert.ok(body.session.length > 0, 'session 走面板记忆账本（非空）');
    assert.equal(body.content, '[随手记 #1] 明早买牛奶（来源 capture）', '内容 = 原文 + 备注级标注');
    assert.ok(body.content.includes('明早买牛奶'), '原文不改写');
    check('入记忆流: appendMemoryEpisode 同源缝收到原文（POST /v1/memory/append）', () => {});
  } finally {
    recorder.restore();
  }
}

check('来源标注: capture 来源标注 + 连记序号随文携带，parseCaptureNote 可逆（整编吸收缝）', () => {
  const note = qc.buildCaptureNote(7, '把门口的快递拆了');
  assert.equal(note, '[随手记 #7] 把门口的快递拆了（来源 capture）');
  assert.deepEqual(qc.parseCaptureNote(note), {seq: 7, text: '把门口的快递拆了'});
  // 原文自带尾注也不吞（贪心剥最后一个标注 = build 的逆操作）
  const tricky = qc.buildCaptureNote(3, '原文以（来源 capture）结尾也安全');
  assert.deepEqual(qc.parseCaptureNote(tricky), {seq: 3, text: '原文以（来源 capture）结尾也安全'});
  assert.equal(qc.parseCaptureNote('普通对话内容'), null, '非随手记内容不冒充');
  assert.equal(qc.parseCaptureNote('[深流/note] 观察（来源 tool）'), null, '他族标注不误认');
  assert.equal(qc.CAPTURE_SOURCE, 'capture', '来源标注口径唯一');
});

// ---------------------------------------------------------------------------
// ③ 未保护级：source=capture / protected=false，零 protect 通道
// ---------------------------------------------------------------------------
{
  const recorder = installFetchRecorder();
  try {
    const ledger = new qc.CaptureLedger(fakeStorage());
    const outcome = await qc.captureToMemoryStream(cfg, ledger, '# 未整理原料一条', 2_000);
    assert.equal(outcome.ok, true);
    assert.equal(outcome.record.source, 'capture', '来源标注 = capture（未整理原料语义）');
    assert.equal(outcome.record.protected, false, 'protection=off（可被后续整编吸收）');
    for (const call of recorder.calls) {
      assert.ok(
        !/protect|unprotect|forget/.test(call.url),
        `采集不走保护/遗忘通道（实际=${call.url}）`,
      );
    }
    check('未保护级: source=capture / protected=false，零 protect/unprotect/forget 通道', () => {});
  } finally {
    recorder.restore();
  }
}

// ---------------------------------------------------------------------------
// ④ 零模型调用（纯采集零 token）：唯一请求面 = 记忆追加
// ---------------------------------------------------------------------------
{
  const recorder = installFetchRecorder();
  try {
    const ledger = new qc.CaptureLedger(fakeStorage());
    await qc.captureToMemoryStream(cfg, ledger, '# 零 token 采集', 3_000);
    const urls = recorder.calls.map((c) => c.url);
    assert.deepEqual(
      urls.filter((u) => u.endsWith('/v1/memory/append')),
      urls,
      `采集唯一请求面 = 记忆追加（实际=${JSON.stringify(urls)}）`,
    );
    assert.ok(!urls.some((u) => u.includes('/chat/completions')), '零对话回合（零 token）');
    check('零模型调用: 纯采集唯一请求面 = 记忆追加，零对话回合零 token', () => {});
  } finally {
    recorder.restore();
  }
}

// ---------------------------------------------------------------------------
// ⑤ 回看列表：最近 N 条只读展示（新→旧、返回副本）
// ---------------------------------------------------------------------------
check('回看列表: 最近 N 条只读展示（新→旧 / 返回副本 / 默认 5 条）', () => {
  const ledger = new qc.CaptureLedger(fakeStorage());
  for (let i = 1; i <= 7; i += 1) {
    const begun = ledger.begin(`# 第${i}条`, i * 1_000);
    assert.equal(begun.ok, true);
    ledger.commit(begun.record);
  }
  const recent = ledger.recent();
  assert.equal(recent.length, qc.CAPTURE_RECENT_VIEW, '默认展示最近 5 条');
  assert.deepEqual(recent.map((r) => r.seq), [7, 6, 5, 4, 3], '新→旧只读展示');
  assert.deepEqual(ledger.recent(2).map((r) => r.seq), [7, 6], 'N 可调');
  recent.length = 0;
  assert.equal(ledger.recent().length, qc.CAPTURE_RECENT_VIEW, '返回副本：改它不动账本');
});

// ---------------------------------------------------------------------------
// ⑥ 空内容拒：显式拒帧，不写流、不发号
// ---------------------------------------------------------------------------
{
  const recorder = installFetchRecorder();
  try {
    const ledger = new qc.CaptureLedger(fakeStorage());
    const empty = await qc.captureToMemoryStream(cfg, ledger, '#', 4_000);
    assert.equal(empty.ok, false);
    assert.equal(empty.reason, 'empty', '裸 # = 空内容拒');
    assert.equal(empty.message, '内容为空，没记下');
    const blank = await qc.captureToMemoryStream(cfg, ledger, '#   ', 4_000);
    assert.equal(blank.reason, 'empty', '纯空白算空（0 装不猜不补）');
    const notCapture = await qc.captureToMemoryStream(cfg, ledger, '普通提问', 4_000);
    assert.equal(notCapture.reason, 'not-capture', '非随手记输入不入采集缝');
    assert.equal(recorder.calls.length, 0, '拒绝零副作用：不写流');
    assert.equal(ledger.issued(), 0, '拒绝不发号');
    assert.deepEqual(ledger.records(), [], '拒绝不入账本');
    check('空内容拒: 拒帧显式（empty/not-capture），零副作用零发号', () => {});
  } finally {
    recorder.restore();
  }
}

// ---------------------------------------------------------------------------
// ⑦ 并发连记：多条序号连续，条条入流
// ---------------------------------------------------------------------------
{
  const recorder = installFetchRecorder();
  try {
    const ledger = new qc.CaptureLedger(fakeStorage());
    const outcomes = await Promise.all([
      qc.captureToMemoryStream(cfg, ledger, '# 并发一', 5_000),
      qc.captureToMemoryStream(cfg, ledger, '# 并发二', 5_001),
      qc.captureToMemoryStream(cfg, ledger, '# 并发三', 5_002),
    ]);
    assert.deepEqual(outcomes.map((o) => o.ok), [true, true, true], '并发连记条条成功');
    assert.deepEqual(outcomes.map((o) => o.record.seq), [1, 2, 3], '序号连续（发号在先，正文随号落流）');
    assert.equal(recorder.calls.length, 3, '三条全部入记忆流');
    const contents = recorder.calls.map((c) => JSON.parse(c.body).content);
    assert.deepEqual(contents, [
      '[随手记 #1] 并发一（来源 capture）',
      '[随手记 #2] 并发二（来源 capture）',
      '[随手记 #3] 并发三（来源 capture）',
    ], '正文随号落流，号不串位');
    assert.deepEqual(ledger.records().map((r) => r.seq), [1, 2, 3], '账本序号连续');
    check('并发连记: 三条并发序号连续（1,2,3）且条条入流', () => {});
  } finally {
    recorder.restore();
  }
}

// ---------------------------------------------------------------------------
// ⑧ 账本：持久化 round-trip / 超限丢最旧序号不动 / 发号只发不收
// ---------------------------------------------------------------------------
check('账本: 持久化 round-trip / 超限丢最旧序号不动 / 发号即持久不复用', () => {
  const storage = fakeStorage();
  const ledgerA = new qc.CaptureLedger(storage);
  for (let i = 1; i <= 3; i += 1) {
    const begun = ledgerA.begin(`# 记${i}`, i * 100);
    ledgerA.commit(begun.record);
  }
  const ledgerB = new qc.CaptureLedger(storage);
  assert.deepEqual(ledgerB.records().map((r) => r.seq), [1, 2, 3], '跨实例 round-trip');
  assert.equal(ledgerB.begin('# 记4', 400).record.seq, 4, '发号自账本续（连续）');

  // 发号即持久、只发不收：begin 成功但从未 commit 的号不复用
  const storage2 = fakeStorage();
  const ledgerC = new qc.CaptureLedger(storage2);
  const failed = ledgerC.begin('# 写流会失败的号', 500);
  assert.equal(failed.ok, true);
  const ledgerD = new qc.CaptureLedger(storage2);
  assert.equal(ledgerD.begin('# 后续记', 600).record.seq, 2, '失败号留空档不复用（如实发号史）');

  // 超限丢最旧，序号不动
  const storage3 = fakeStorage();
  const ledgerE = new qc.CaptureLedger(storage3);
  const total = qc.CAPTURE_LEDGER_MAX + 3;
  for (let i = 1; i <= total; i += 1) {
    const begun = ledgerE.begin(`# 超限${i}`, i * 10);
    ledgerE.commit(begun.record);
  }
  const kept = ledgerE.records();
  assert.equal(kept.length, qc.CAPTURE_LEDGER_MAX, '超限丢最旧');
  assert.equal(kept[0].seq, 4, '序号不动（只丢记录不改号）');
  assert.equal(kept[kept.length - 1].seq, total, '最新保尾');
});

// ---------------------------------------------------------------------------
// ⑨ 接线镜像：快捷窗随手记分支直连采集缝，零对话回合、零通知、零模型面
// ---------------------------------------------------------------------------
{
  const quickSrc = readFileSync(join(here, '..', 'src', 'features', 'quick', 'QuickWindowView.svelte'), 'utf8');
  const captureSrc = readFileSync(join(here, '..', 'src', 'features', 'quick', 'quick-capture.ts'), 'utf8');

  // 快捷窗接线：模式随输入切换 + 采集缝直连 + 落地反馈 + 回看列表
  assert.ok(quickSrc.includes('detectQuickMode(query)'), '模式随输入即时切换（# 前缀）');
  assert.ok(quickSrc.includes('captureToMemoryStream(config, captureLedger, query)'), '一键记直连采集缝（appendMemoryEpisode 同源）');
  assert.ok(quickSrc.includes('已记下 ✓'), '落地反馈 = 已记下 ✓');
  assert.ok(quickSrc.includes('capture-recent'), '快捷窗内最近随手记列表（回看）');
  assert.ok(quickSrc.includes('formatCaptureTime'), '回看行带采集时刻');

  // 零打扰：随手记分支不进对话回合、不调模型、不发通知
  const region = /async function handleCapture[\s\S]*?\n  \}/.exec(quickSrc);
  assert.ok(region, 'QuickWindowView 必须有 handleCapture');
  assert.ok(!region[0].includes('runtime.run'), '采集分支不进对话回合');
  assert.ok(!region[0].includes('submit('), '采集分支不落对话提交');
  assert.ok(!/\bnotification\b/i.test(quickSrc), '快捷窗零通知（零打扰纪律）');
  assert.ok(!quickSrc.includes('notify'), '快捷窗零通知（零打扰纪律）');

  // 纯采集面零模型痕迹（源码镜像）
  for (const banned of ['chat/completions', 'createAgentRuntime', 'streamChat', 'chatOnce', 'runtime.run', 'notification']) {
    assert.ok(!captureSrc.includes(banned), `采集模块零模型面（禁止 ${banned}）`);
  }
  assert.ok(captureSrc.includes('appendMemoryEpisode'), '入流走 appendMemoryEpisode 同源缝');

  // 既有快捷窗生命周期契约零回归（关闭销毁缝不受本批影响）
  assert.ok(quickSrc.includes('close_quick_window'), '关闭缝零回归');
  check('接线镜像: 模式切换/采集缝/反馈/回看在案 + 零对话零通知零模型面', () => {});
}

console.log(`--- ${checks} checks ---`);
