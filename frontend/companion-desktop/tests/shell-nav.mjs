// 侧栏收纳（IA）导航目标纯逻辑单测（真实模块 src/lib/shell-nav.ts）。
// 病灶（内测实测）：左栏九件太杂。收敛为 对话/工具/状态/设置 四件；被移页面
// 换家不删功能——历史撤除、记忆/日记入工作台、治理入设置 › 安全与治理、
// 日志并入状态；旧深链 ?drawer= 与命令面板 nav.* 同走一张重定向表。
import assert from 'node:assert/strict';

import {
  NAV_COMMAND_TARGETS,
  RAIL_ITEMS,
  SETTINGS_SECURITY_SECTION,
  WORKBENCH_CARDS,
  resolveLegacyDrawer,
} from '../src/lib/shell-nav.ts';

// ---- 侧栏常驻四件 ----
assert.deepEqual([...RAIL_ITEMS], ['chat', 'tools', 'status', 'settings'], '左栏只留四件');
assert.equal(RAIL_ITEMS.includes('history'), false, '历史入口撤除（会话列表即历史）');
assert.equal(RAIL_ITEMS.includes('memory'), false, '记忆移出侧栏');
assert.equal(RAIL_ITEMS.includes('diary'), false, '日记移出侧栏');
assert.equal(RAIL_ITEMS.includes('governance'), false, '治理移出侧栏');
assert.equal(RAIL_ITEMS.includes('logs'), false, '日志移出侧栏');

// ---- 旧深链重定向：被移页面跳新位置 ----
assert.deepEqual(resolveLegacyDrawer('history'), {kind: 'chat'}, '历史深链 → 对话列表（历史的新家）');
assert.deepEqual(resolveLegacyDrawer('memory'), {kind: 'workbench', section: 'memory'}, '记忆深链 → 工作台记忆卷宗');
assert.deepEqual(resolveLegacyDrawer('diary'), {kind: 'workbench', section: 'diary'}, '日记深链 → 工作台他的日记');
assert.deepEqual(
  resolveLegacyDrawer('governance'),
  {kind: 'drawer', id: 'settings', section: SETTINGS_SECURITY_SECTION},
  '治理深链 → 设置 › 安全与治理',
);
assert.deepEqual(resolveLegacyDrawer('logs'), {kind: 'drawer', id: 'status'}, '日志深链 → 状态');
// 未搬动的入口保持原位
assert.deepEqual(resolveLegacyDrawer('tools'), {kind: 'drawer', id: 'tools'});
assert.deepEqual(resolveLegacyDrawer('status'), {kind: 'drawer', id: 'status'});
assert.deepEqual(resolveLegacyDrawer('settings'), {kind: 'drawer', id: 'settings'});
assert.equal(resolveLegacyDrawer('no-such-page'), null, '认不出的深链不猜（调用方落默认视图）');
assert.equal(resolveLegacyDrawer(null), null);

// ---- 命令面板 nav.* 同步：与深链同一张表 ----
assert.equal('nav.history' in NAV_COMMAND_TARGETS, false, 'nav.history 已随历史入口撤除');
assert.deepEqual(NAV_COMMAND_TARGETS['nav.chat'], {kind: 'chat'});
assert.deepEqual(NAV_COMMAND_TARGETS['nav.tools'], {kind: 'drawer', id: 'tools'});
assert.deepEqual(NAV_COMMAND_TARGETS['nav.status'], {kind: 'drawer', id: 'status'});
assert.deepEqual(NAV_COMMAND_TARGETS['nav.settings'], {kind: 'drawer', id: 'settings'});
assert.deepEqual(NAV_COMMAND_TARGETS['nav.memory'], {kind: 'workbench', section: 'memory'});
assert.deepEqual(NAV_COMMAND_TARGETS['nav.diary'], {kind: 'workbench', section: 'diary'});
assert.deepEqual(
  NAV_COMMAND_TARGETS['nav.governance'],
  {kind: 'drawer', id: 'settings', section: SETTINGS_SECURITY_SECTION},
);
assert.deepEqual(NAV_COMMAND_TARGETS['nav.logs'], {kind: 'drawer', id: 'status'});

// ---- 工作台两张卡片入口 ----
assert.deepEqual(
  WORKBENCH_CARDS.map((c) => c.section),
  ['memory', 'diary'],
  '工作台恰好两张卡片入口：记忆 / 日记',
);
assert.ok(WORKBENCH_CARDS.every((c) => c.title && c.sub), '卡片带标题与副题（入口可辨认）');

console.log('shell nav (sidebar consolidation): all assertions passed');
