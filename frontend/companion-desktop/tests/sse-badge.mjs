// SSE 徽章三态显示单测（真实模块 src/lib/statusbar.ts）。
// 病灶（内测实测）：状态栏常显「SSE 断连」。查证结论是真断线（订阅链冻在旧
// 端点卡死），且旧文案用「重连中/断连」半诊断词——徽章按连接事实改三态：
// 已连接 / 连接中 / 未连接（能力缺席另计「不可用」），点击语义与接线一致
// （点击 = 重建事件订阅，见 shell-ia.mjs 的接线镜像）。
import assert from 'node:assert/strict';

import {sseIndicator, sseSourceOf} from '../src/lib/statusbar.ts';

// ---- 三态文案：如实显示，不带半诊断词 ----
assert.equal(
  sseIndicator(sseSourceOf({supported: true, connected: true, simulated: false})).text,
  'SSE 已连接',
);
assert.equal(
  sseIndicator(sseSourceOf({supported: true, connected: false, simulated: false})).text,
  'SSE 连接中',
  '未连上但仍在建连/退避 = 连接中',
);
assert.equal(
  sseIndicator(sseSourceOf({supported: true, connected: false, simulated: true})).text,
  'SSE 未连接',
  '断连超 30s（SIM）= 未连接，不再写「断连」这种半诊断词',
);
assert.equal(sseIndicator('unsupported').text, 'SSE 不可用', '能力缺席另计（不是连接态）');

// ---- 三态互不混淆 ----
const texts = ['live', 'retrying', 'lost'].map((s) => sseIndicator(s).text);
assert.equal(new Set(texts).size, 3, '三态三词，互不串台');
assert.ok(!texts.join('|').includes('重连中'), '旧文案「重连中」已退役');
assert.ok(!texts.join('|').includes('断连'), '旧文案「断连」已退役');
assert.ok(sseIndicator('lost').sim, '未连接带 SIM 纪律标注（真实来源缺失显式标）');
assert.equal(sseIndicator('retrying').sim, undefined, '连接中尚未 SIM');
assert.ok(!sseIndicator('live').text.includes('SIM'), '在线不含 SIM 字样');

// ---- 点击承诺必须与接线一致：点击 = 立即重连（重建事件订阅）----
for (const state of ['live', 'retrying', 'lost']) {
  const ind = sseIndicator(state);
  assert.equal(ind.clickable, true, `${state} 可点（手动重连入口）`);
  assert.ok(
    ind.title.includes('点击立即重连（重建事件订阅）'),
    `${state} 的 title 承诺与 onSseClick 实际动作一致`,
  );
}
assert.equal(sseIndicator('unsupported').clickable, false, '能力缺席不可点（点了也没得连）');

console.log('sse badge three-state: all assertions passed');
