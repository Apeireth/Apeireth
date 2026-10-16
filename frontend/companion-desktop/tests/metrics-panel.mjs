// 运行指标卡（D9 上屏：真实模块 src/lib/metrics.ts），并对 MetricsPanel.svelte /
// SettingsView.svelte 做源码级镜像校验。
// 契约（frontend-data-contract §10.1，schema_version: 1）：
//   1. 真值渲染：延迟 P50/P95/均值、token 流向（入/出窗口累计）、缓存命中
//      （token · 率）、成功/失败/待批计数。
//   2. 空态「—」语义：null / 分布空窗口显「—」，未知不用 0 冒充；计数位
//      真实 0 照显 0。
//   3. 缓存命中率上屏由消费端换算（0.0–1.0 → 百分比）。
//   4. 信封归一不合契约 = null（不编数）；取数失败 = 显式错误帧（真因）。
//   5. 刷新按钮 + 30s 自动刷新（同一条取数缝；卸载即停，不泄漏定时器）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  DASH,
} from '../src/lib/chat-shell/turn-telemetry.ts';
import {
  METRICS_AUTO_REFRESH_MS,
  METRICS_RETENTION,
  MetricsEnvelopeError,
  fetchPanelMetricsSnapshot,
  formatCacheHitRate,
  metricsPanelView,
  normalizeMetricsSnapshot,
  panelMetricsEndpoint,
  parseMetricsEnvelope,
  startMetricsAutoRefresh,
} from '../src/lib/metrics.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const metricsPanelSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'components', 'MetricsPanel.svelte'),
  'utf8',
);
const settingsSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'),
  'utf8',
);

console.log('--- metrics panel (D9) ---');

/** 合同形状的样例信封（与 panel_metrics.rs DTO 全表一致）。 */
function sampleSnapshot(overrides = {}) {
  return {
    schema_version: 1,
    window_turns: 50,
    retention: 'in_process_rolling',
    global: {
      turns: 3,
      succeeded: 2,
      failed: 1,
      pending_approval: 0,
      latency_ms: {count: 3, mean: 12.0, p50: 10, p95: 30, sum: 36},
      input_tokens: {count: 2, mean: 100.0, p50: 100, p95: 120, sum: 200},
      output_tokens: {count: 2, mean: 20.0, p50: 20, p95: 24, sum: 40},
      rounds: {count: 3, mean: 1.0, p50: 1, p95: 2, sum: 3},
      tool_calls: {count: 2, mean: 1.0, p50: 1, p95: 2, sum: 2},
      cache_hit_tokens: 125,
      cache_hit_rate: 0.625,
      cost_micros: null,
    },
    sessions: [],
    ...overrides,
  };
}

const cellOf = (view, key) => view.cells.find((cell) => cell.key === key);

// ---------------------------------------------------------------------------
// 1. 真值渲染：四格照合同真值上屏
// ---------------------------------------------------------------------------
{
  const snapshot = normalizeMetricsSnapshot(sampleSnapshot());
  assert.ok(snapshot, '合同形状的信封必须归一成功');
  const view = metricsPanelView(snapshot);
  assert.equal(cellOf(view, 'latency').value, '10 / 30 / 12', '延迟 P50 / P95 / 均值真值');
  assert.equal(cellOf(view, 'tokens').value, '200 / 40', 'token 流向 入 / 出（窗口累计）真值');
  assert.equal(cellOf(view, 'cache').value, '命中 125 · 62.5%', '缓存命中（token · 率）真值');
  assert.equal(cellOf(view, 'counts').value, '2 / 1 / 0', '成功 / 失败 / 待批计数真值');
  assert.ok(view.caption.includes('窗口 50 拍'), '口径注记带生效窗口');
  assert.ok(view.caption.includes('留存 进程内滚动（不落盘）'), '留存如实转述（不落盘）');
  assert.ok(view.caption.includes('样本 3 拍'), '样本数 = 窗口内收束拍');

  // 千分位展示与遥测条同口径；均值小数最多 1 位。
  const big = sampleSnapshot();
  big.global.input_tokens.sum = 12345;
  big.global.latency_ms.mean = 12.34;
  const bigView = metricsPanelView(normalizeMetricsSnapshot(big));
  assert.equal(cellOf(bigView, 'tokens').value, '12,345 / 40', '千分位只加组分隔');
  assert.equal(cellOf(bigView, 'latency').value, '10 / 30 / 12.3', '均值最多 1 位小数');
  console.log('  -> PASS: 指标卡真值渲染（延迟/token 流向/缓存命中/收束计数）');
}

// ---------------------------------------------------------------------------
// 2. 空态「—」语义：未知不用 0 冒充；计数位真实 0 照显 0
// ---------------------------------------------------------------------------
{
  // 未取到快照：整卡「—」。
  const empty = metricsPanelView(null);
  for (const cell of empty.cells) {
    assert.equal(cell.value, DASH, `${cell.key} 无快照 → 「—」`);
  }
  assert.ok(empty.caption.includes(`窗口 ${DASH}`), '口径注记同步「—」');

  // 取到空窗口快照：分布 count:0 → 四数值列 null → 「—」；计数位真实 0 照显。
  const blank = sampleSnapshot();
  blank.global = {
    turns: 0,
    succeeded: 0,
    failed: 0,
    pending_approval: 0,
    latency_ms: {count: 0, mean: null, p50: null, p95: null, sum: null},
    input_tokens: {count: 0, mean: null, p50: null, p95: null, sum: null},
    output_tokens: {count: 0, mean: null, p50: null, p95: null, sum: null},
    rounds: {count: 0, mean: null, p50: null, p95: null, sum: null},
    tool_calls: {count: 0, mean: null, p50: null, p95: null, sum: null},
    cache_hit_tokens: null,
    cache_hit_rate: null,
    cost_micros: null,
  };
  const view = metricsPanelView(normalizeMetricsSnapshot(blank));
  assert.equal(cellOf(view, 'latency').value, DASH, '空窗口延迟 → 「—」（不拿 0 冒充有数据的 0）');
  assert.equal(cellOf(view, 'tokens').value, DASH, '空窗口 token → 「—」');
  assert.equal(cellOf(view, 'cache').value, DASH, '缓存列未知 → 「—」');
  assert.equal(cellOf(view, 'counts').value, '0 / 0 / 0', '计数位真实 0 照显 0');

  // 部分未知：命中数在、率未知 → 率位「—」（不做除法不编率）。
  const hitOnly = sampleSnapshot();
  hitOnly.global.cache_hit_tokens = 300;
  hitOnly.global.cache_hit_rate = null;
  const hitView = metricsPanelView(normalizeMetricsSnapshot(hitOnly));
  assert.equal(cellOf(hitView, 'cache').value, `命中 300 · ${DASH}`, '率未知显「—」');
  console.log('  -> PASS: 空态「—」语义（未知不冒充 0、真实 0 照显）');
}

// ---------------------------------------------------------------------------
// 3. 缓存命中率上屏换算（合同 0.0–1.0 → 百分比，消费端换算）
// ---------------------------------------------------------------------------
{
  assert.equal(formatCacheHitRate(0.625), '62.5%', '合同原值换算百分比');
  assert.equal(formatCacheHitRate(0.6666), '66.7%', '四舍五入到 0.1%');
  assert.equal(formatCacheHitRate(0), '0%', '真实 0 率照显');
  assert.equal(formatCacheHitRate(1), '100%', '满率照显');
  assert.equal(formatCacheHitRate(null), DASH, 'null → 「—」（不编百分比）');
  assert.equal(formatCacheHitRate(Number.NaN), DASH, '非有限值 → 「—」');
  console.log('  -> PASS: 缓存命中率换算（null 不编百分比）');
}

// ---------------------------------------------------------------------------
// 4. 信封归一：不合契约 = null（不编数）；未知字段忽略（§9.3 向后兼容）
// ---------------------------------------------------------------------------
{
  assert.equal(normalizeMetricsSnapshot(null), null, '非对象 → null');
  assert.equal(normalizeMetricsSnapshot('<html>not json</html>'), null, '非 JSON 形状 → null');
  const missing = sampleSnapshot();
  delete missing.global.latency_ms;
  assert.equal(normalizeMetricsSnapshot(missing), null, '缺分布 → null');
  const wrongType = sampleSnapshot();
  wrongType.global.succeeded = '2';
  assert.equal(normalizeMetricsSnapshot(wrongType), null, '计数类型不符 → null');
  const wrongDist = sampleSnapshot();
  wrongDist.global.latency_ms.mean = '12';
  assert.equal(normalizeMetricsSnapshot(wrongDist), null, '分布数值类型不符 → null');
  const missingSessions = sampleSnapshot();
  delete missingSessions.sessions;
  assert.equal(normalizeMetricsSnapshot(missingSessions), null, '缺 sessions → null');

  // 后端加字段不改名：未知字段忽略，归一照常。
  const extra = sampleSnapshot();
  extra.global.future_column = 42;
  extra.future_envelope_field = 'ok';
  assert.ok(normalizeMetricsSnapshot(extra), '未知字段忽略（向后兼容）');
  console.log('  -> PASS: 信封归一严格守约（不合契约不编数、未知字段忽略）');
}

// ---------------------------------------------------------------------------
// 5. 取数面解析：错误即帧（真因上屏），版本不认识显式拒绝
// ---------------------------------------------------------------------------
{
  const ok = parseMetricsEnvelope(200, sampleSnapshot());
  assert.equal(ok.global.turns, 3, '200 + 合同信封 → 归一快照');

  assert.throws(
    () => parseMetricsEnvelope(401, {error: 'auth_missing_key'}),
    (error) =>
      error instanceof MetricsEnvelopeError &&
      error.status === 401 &&
      error.message.includes('HTTP 401'),
    '401 显式报 HTTP 状态（不静默降级成空态）',
  );
  assert.throws(
    () => parseMetricsEnvelope(200, {nope: true}),
    (error) => error instanceof MetricsEnvelopeError && error.status === null,
    '形状不合契约 → 显式错误帧',
  );
  assert.throws(
    () => parseMetricsEnvelope(200, sampleSnapshot({schema_version: 2})),
    (error) => error instanceof MetricsEnvelopeError && error.message.includes('不受支持'),
    '不认识的 schema_version 显式拒绝（不拿旧形状硬套）',
  );
  console.log('  -> PASS: 取数面错误即帧（HTTP 状态/形状/版本真因上屏）');
}

// ---------------------------------------------------------------------------
// 6. 取数组装：契约 URL + 令牌面鉴权头单源
// ---------------------------------------------------------------------------
{
  assert.equal(
    panelMetricsEndpoint('http://127.0.0.1:8080'),
    'http://127.0.0.1:8080/v1/panel/metrics',
    '契约路径 /v1/panel/metrics',
  );
  assert.equal(
    panelMetricsEndpoint('http://127.0.0.1:8080/'),
    'http://127.0.0.1:8080/v1/panel/metrics',
    '尾斜杠不双拼',
  );

  const calls = [];
  const deps = {
    authHeaders: async () => ({Authorization: 'Bearer test-token'}),
    fetchJson: async (url, init) => {
      calls.push({url, init});
      return {status: 200, json: async () => sampleSnapshot()};
    },
  };
  const snapshot = await fetchPanelMetricsSnapshot({baseUrl: 'http://127.0.0.1:8080'}, deps);
  assert.equal(snapshot.global.turns, 3, '取数面返回归一快照');
  assert.equal(calls.length, 1, '一次刷新一次取数');
  assert.equal(calls[0].url, 'http://127.0.0.1:8080/v1/panel/metrics', '请求契约 URL');
  assert.equal(calls[0].init.headers.Authorization, 'Bearer test-token', '鉴权头走注入的令牌面单源');

  const failing = {
    authHeaders: async () => ({}),
    fetchJson: async () => ({status: 500, json: async () => ({error: 'boom'})}),
  };
  await assert.rejects(
    () => fetchPanelMetricsSnapshot({baseUrl: 'http://127.0.0.1:8080'}, failing),
    (error) => error instanceof MetricsEnvelopeError && error.status === 500,
    '5xx 显式失败（不吞错不编数）',
  );
  console.log('  -> PASS: 取数组装走契约 URL + 令牌面鉴权头（失败显式）');
}

// ---------------------------------------------------------------------------
// 7. 自动刷新控制器：30s 一拍，卸载即停（幂等，不泄漏定时器）
// ---------------------------------------------------------------------------
{
  assert.equal(METRICS_AUTO_REFRESH_MS, 30_000, '自动刷新间隔 = 30s');

  const timers = {active: new Map(), cleared: [], nextId: 1};
  const host = {
    setInterval(handler, ms) {
      const id = timers.nextId++;
      timers.active.set(id, {handler, ms});
      return id;
    },
    clearInterval(handle) {
      timers.cleared.push(handle);
      timers.active.delete(handle);
    },
  };
  let ticks = 0;
  const auto = startMetricsAutoRefresh(() => {
    ticks += 1;
  }, host);
  assert.equal(timers.active.size, 1, '启动即挂一个定时器');
  const [entry] = [...timers.active.values()];
  assert.equal(entry.ms, 30_000, '间隔钉 30s');
  entry.handler();
  entry.handler();
  assert.equal(ticks, 2, '到点触发 tick（手动模拟）');

  auto.stop();
  assert.equal(timers.active.size, 0, 'stop 后不再持有定时器');
  assert.equal(timers.cleared.length, 1, 'stop 真清理');
  auto.stop();
  assert.equal(timers.cleared.length, 1, 'stop 幂等（不重复清理）');
  console.log('  -> PASS: 自动刷新 30s 一拍、卸载即停（幂等）');
}

// ---------------------------------------------------------------------------
// 8. 源码镜像：面板接线（刷新按钮/自动刷新/令牌面/错误帧）+ 设置页预算区上屏
// ---------------------------------------------------------------------------
{
  const budgetStart = settingsSrc.indexOf("{:else if activeSection === 'budget'}");
  const budgetEnd = settingsSrc.indexOf("{:else if activeSection === 'runtime'}");
  const budgetSection = settingsSrc.slice(budgetStart, budgetEnd);
  assert.ok(budgetStart >= 0 && budgetEnd > budgetStart, '预算分区模板锚点在案');
  assert.ok(budgetSection.includes('<MetricsPanel {config} />'), '设置页预算区挂运行指标卡');
  assert.ok(metricsPanelSrc.includes('fetchPanelMetricsSnapshot('), '面板走同一取数缝');
  assert.ok(metricsPanelSrc.includes('gatewayAuthHeaders()'), '每个指标请求带令牌面鉴权头（单源）');
  assert.ok(metricsPanelSrc.includes('metricsPanelView(snapshot)'), '展示映射走纯函数（不手搓数字）');
  assert.ok(metricsPanelSrc.includes('startMetricsAutoRefresh('), '自动刷新接线');
  assert.ok(metricsPanelSrc.includes('return () => auto.stop();'), '卸载/重挂清理定时器');
  assert.ok(metricsPanelSrc.includes('onclick={() => void refresh()}'), '手动刷新按钮走同一条取数缝');
  assert.ok(metricsPanelSrc.includes('自动刷新 {METRICS_AUTO_REFRESH_MS / 1000}s'), '自动刷新口径上屏');
  assert.ok(metricsPanelSrc.includes('describeCaught(error)'), '取数失败报真因（错误即帧）');
  assert.ok(metricsPanelSrc.includes('disabled={loading}'), '刷新中禁用防连点');
  for (const forbidden of ['onSave', 'handleSaveSettings', '保存设置']) {
    assert.ok(!metricsPanelSrc.includes(forbidden), `指标卡只读（无保存按钮纪律：不许出现 ${forbidden}）`);
  }
  console.log('  -> PASS: 源码镜像——面板接线 + 预算区上屏 + 只读纪律');
}

console.log('metrics panel: all assertions passed');
