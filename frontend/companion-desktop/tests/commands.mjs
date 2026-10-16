// Ctrl+K 命令面板注册表纯逻辑单测 — 筛选 / 别名 / 最近优先 / 持久化容错
// 直接 import 真实实现 ../src/lib/commands/registry.ts（Node ≥23.6 类型擦除，
// 模块 DOM-free，与 governance/presence 测试同一约定）。
// 契约依据: 00-PHILOSOPHY §6 原则 3（一个入口）/ 原则 5（0 装）；gap-plan §4.3 P0。
import assert from 'node:assert/strict';

import {
  filterCommands,
  orderCommands,
  pushRecentId,
  loadRecentIds,
  saveRecentIds,
  RECENT_STORAGE_KEY,
} from '../src/lib/commands/registry.ts';

const REG = [
  {id: 'nav.chat', title: '打开对话', aliases: ['duihua', 'chat'], group: '导航'},
  {id: 'nav.governance', title: '打开治理卷宗', aliases: ['zhili', 'governance', 'gov'], group: '导航'},
  {id: 'nav.settings', title: '打开设置', aliases: ['shezhi', 'settings'], group: '导航'},
  {id: 'theme.night', title: '主题：夜航', aliases: ['night', 'zhuti'], group: '主题'},
  {id: 'act.reconnect', title: '健康检查重连', aliases: ['chonglian', 'reconnect'], group: '动作'},
  {
    id: 'act.approve',
    title: '批准当前待签文书',
    aliases: ['pizhun', 'approve'],
    group: '动作',
    disabledReason: '当前没有等待签字的文书',
  },
];

// ---- filterCommands ----
assert.equal(filterCommands(REG, '').length, 6, '空 query 返回全量');
assert.equal(filterCommands(REG, '   ').length, 6, '纯空白等同空 query');
assert.deepEqual(
  filterCommands(REG, '治理').map((c) => c.id),
  ['nav.governance'],
  '中文标题直搜',
);
assert.deepEqual(
  filterCommands(REG, 'zhil').map((c) => c.id),
  ['nav.governance'],
  '拼音别名前缀片段可搜',
);
assert.deepEqual(
  filterCommands(REG, 'GOV').map((c) => c.id),
  ['nav.governance'],
  '英文别名大小写不敏感',
);
assert.deepEqual(
  filterCommands(REG, 'theme').map((c) => c.id),
  [],
  '英文词不撞中文标题（zhuti 才是主题别名）',
);
assert.deepEqual(
  filterCommands(REG, 'zhuti').map((c) => c.id),
  ['theme.night'],
  '主题别名命中',
);
assert.equal(filterCommands(REG, '不存在的词').length, 0, '无匹配返回空');
assert.equal(
  filterCommands(REG, 'approve')[0]?.disabledReason,
  '当前没有等待签字的文书',
  '置灰命令仍可被搜到（0 装：显示但不藏）',
);

// ---- orderCommands ----
const ordered = orderCommands(REG, ['theme.night', 'nav.settings']);
assert.deepEqual(
  ordered.map((c) => c.id),
  ['theme.night', 'nav.settings', 'nav.chat', 'nav.governance', 'act.reconnect', 'act.approve'],
  '最近优先（榜单序）+ 未入榜保持注册序',
);
assert.deepEqual(
  orderCommands(REG, []).map((c) => c.id),
  REG.map((c) => c.id),
  '空榜单 = 注册序',
);
assert.deepEqual(
  orderCommands(REG, ['ghost.id', 'act.reconnect']).map((c) => c.id)[0],
  'act.reconnect',
  '榜单里的陈旧未知 id 静默忽略',
);
assert.deepEqual(
  orderCommands(REG, ['nav.chat', 'nav.chat', 'nav.governance']).map((c) => c.id).slice(0, 2),
  ['nav.chat', 'nav.governance'],
  '榜单重复 id 按首次位次',
);

// ---- pushRecentId ----
assert.deepEqual(pushRecentId([], 'a'), ['a'], '空榜入第一条');
assert.deepEqual(pushRecentId(['a', 'b'], 'b'), ['b', 'a'], '复用置顶去重');
assert.deepEqual(
  pushRecentId(['1', '2', '3'], '4', 3),
  ['4', '1', '2'],
  '封顶截断最旧',
);

// ---- loadRecentIds / saveRecentIds ----
function fakeStorage(initial) {
  const map = new Map(Object.entries(initial ?? {}));
  return {
    getItem: (k) => (map.has(k) ? map.get(k) : null),
    setItem: (k, v) => map.set(k, String(v)),
    map,
  };
}

assert.deepEqual(loadRecentIds(fakeStorage()), [], '无记录回落空榜');
assert.deepEqual(
  loadRecentIds(fakeStorage({[RECENT_STORAGE_KEY]: 'not-json{'})),
  [],
  'JSON 损坏回落空榜',
);
assert.deepEqual(
  loadRecentIds(fakeStorage({[RECENT_STORAGE_KEY]: '{"x":1}'})),
  [],
  '非数组形状回落空榜',
);
assert.deepEqual(
  loadRecentIds(fakeStorage({[RECENT_STORAGE_KEY]: '["a", 42, "b", null]'})),
  ['a', 'b'],
  '混型数组只留字符串',
);

const roundTrip = fakeStorage();
saveRecentIds(['x', 'y'], roundTrip);
assert.deepEqual(loadRecentIds(roundTrip), ['x', 'y'], '写读往返一致');
assert.equal(roundTrip.map.get(RECENT_STORAGE_KEY), '["x","y"]', '落盘即所读（诚实持久化）');

// 无存储环境（Node 默认无 localStorage）：不抛错、回落空榜
assert.deepEqual(loadRecentIds(), [], '无 localStorage 回落空榜不抛错');
saveRecentIds(['z']); // 静默无操作即成功

console.log('commands registry: all assertions passed');
