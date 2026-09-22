// 底部状态条纯逻辑单测 — 四指标推导 / 三态色 / 0 装口径
// 直接 import 真实实现 ../src/lib/statusbar.ts（Node ≥23.6 类型擦除，DOM-free）。
// 契约依据: 00-PHILOSOPHY §2（余光投影）/ gap-plan §4.3 P1 / 01-DESIGN-SYSTEM §2.4 §5.4。
import assert from 'node:assert/strict';

import {
  sseSourceOf,
  sseIndicator,
  turnIndicator,
  guardIndicator,
  memoryIndicator,
} from '../src/lib/statusbar.ts';

// ---- sseSourceOf：四个真值组合 → 四来源 ----
assert.equal(sseSourceOf({supported: false, connected: false, simulated: false}), 'unsupported');
assert.equal(sseSourceOf({supported: false, connected: true, simulated: false}), 'unsupported',
  '能力缺席优先于连接态（不假装在线）');
assert.equal(sseSourceOf({supported: true, connected: true, simulated: false}), 'live');
assert.equal(sseSourceOf({supported: true, connected: false, simulated: false}), 'retrying',
  '断连 30s 内 = 重连中');
assert.equal(sseSourceOf({supported: true, connected: false, simulated: true}), 'lost',
  '断连超 SIM_AFTER_MS = 真实来源缺失');
assert.equal(sseSourceOf({supported: true, connected: true, simulated: true}), 'live',
  '连接恢复即使 sim 标志未清也判在线（连接态优先）');
assert.equal(sseSourceOf({supported: null, connected: false, simulated: false}), 'retrying',
  '清单未知（离线）+ 未连接 = 重连中，不谎报「不可用」');
assert.equal(sseSourceOf({supported: null, connected: false, simulated: true}), 'lost',
  '清单未知 + 断连超时 = 断连 SIM');

// ---- sseIndicator：三态色与 SIM 标注 ----
assert.equal(sseIndicator('live').tone, 'quiet', '已连接 = 安静');
assert.equal(sseIndicator('live').clickable, true, '已连接也可点（手动重连入口）');
assert.equal(sseIndicator('retrying').tone, 'warn', '重连中 = 黄');
assert.equal(sseIndicator('lost').tone, 'danger', '断连 = 红');
assert.equal(sseIndicator('lost').sim, true, '断连带 SIM 纪律标注');
assert.equal(sseIndicator('retrying').sim, undefined, '重连中尚未 SIM');
assert.equal(sseIndicator('unsupported').tone, 'quiet', '能力缺席不告警（0 装 = 安静标注）');
assert.equal(sseIndicator('unsupported').clickable, false, '能力缺席不可点');
assert.ok(!sseIndicator('live').text.includes('SIM'), '在线不含 SIM 字样');

// ---- turnIndicator ----
assert.equal(turnIndicator(true).clickable, true, '进行中可点 = 打断');
assert.equal(turnIndicator(true).tone, 'quiet', '进行中是常态不挣色');
assert.ok(turnIndicator(true).title.includes('不再听他说完'), '打断语义诚实');
assert.equal(turnIndicator(false).clickable, false, '空闲不可点');
assert.equal(turnIndicator(false).tone, 'quiet');

// ---- guardIndicator ----
assert.equal(guardIndicator({supported: false, count: null, limit: 50, hasNew: false}).text,
  '守卫 不可用');
assert.equal(guardIndicator({supported: true, count: null, limit: 50, hasNew: false}).tone,
  'warn', '拉取失败 = 黄（计数缺失而非零）');
assert.ok(guardIndicator({supported: true, count: null, limit: 50, hasNew: false}).title.includes('缺失而非为零'));
assert.equal(guardIndicator({supported: true, count: 3, limit: 50, hasNew: false}).text, '守卫 3');
assert.equal(guardIndicator({supported: true, count: 3, limit: 50, hasNew: false}).tone, 'quiet');
assert.equal(guardIndicator({supported: true, count: 3, limit: 50, hasNew: true}).tone, 'warn',
  '未读新事件 = 黄');
assert.equal(guardIndicator({supported: true, count: 50, limit: 50, hasNew: false}).text, '守卫 50+',
  '满窗显示 N+（诚实口径：端点不供总数）');
assert.ok(guardIndicator({supported: true, count: 50, limit: 50, hasNew: false}).title.includes('窗口'));

// ---- memoryIndicator ----
assert.equal(memoryIndicator({supported: false, count: null, limit: 100}).text, '记忆 不可用');
assert.equal(memoryIndicator({supported: false, count: null, limit: 100}).clickable, false);
assert.equal(memoryIndicator({supported: true, count: null, limit: 100}).tone, 'warn');
assert.equal(memoryIndicator({supported: true, count: 0, limit: 100}).text, '记忆 0',
  '真空 = 0（拉取成功返回空），与读取失败区分');
assert.equal(memoryIndicator({supported: true, count: 0, limit: 100}).tone, 'quiet');
assert.equal(memoryIndicator({supported: true, count: 100, limit: 100}).text, '记忆 100+');
assert.equal(memoryIndicator({supported: true, count: 42, limit: 100}).tone, 'quiet',
  '计数大小不是异常，永远不挣色');
assert.ok(memoryIndicator({supported: true, count: 42, limit: 100}).title.includes('不供总数'));

console.log('statusbar indicators: all assertions passed');
