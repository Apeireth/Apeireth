// 记忆卷宗纯逻辑单测 — 过滤器计数 / 缺省态 / 乐观锁账本 / 409 判别 / graph 关联
// 直接 import 真实实现 ../src/lib/memory-ledger.ts（Node ≥23.6 类型擦除，DOM-free）。
// 契约依据: gateway-api-contract §5（诚实边界）/ 01-DESIGN-SYSTEM §5.6② / gap-plan §4.4-7。
import assert from 'node:assert/strict';

import {
  MEMORY_FILTERS,
  filterEpisodes,
  memoryFilterCounts,
  episodeCategoryLabel,
  episodeImportanceText,
  expectedRevOf,
  recordRevision,
  classifyMemoryMutationError,
  episodeGraphLinks,
  graphNodeLabel,
  ghostNumber,
  formatEpisodeTime,
} from '../src/lib/memory-ledger.ts';

const EP = (over) => ({
  id: 'ep-x',
  timestamp: 1726992000000,
  role: 'assistant',
  content: '……',
  sessionId: 's-1',
  ...over,
});

// ---- 过滤器：只认真实字段 ----
const list = [
  EP({id: 'a', role: 'assistant'}),
  EP({id: 'b', role: 'user'}),
  EP({id: 'c', role: 'assistant', protected: true}),
  EP({id: 'd', role: 'user', protected: false}),
];
assert.equal(filterEpisodes(list, 'all').length, 4);
assert.deepEqual(filterEpisodes(list, 'assistant').map((e) => e.id), ['a', 'c']);
assert.deepEqual(filterEpisodes(list, 'user').map((e) => e.id), ['b', 'd']);
assert.deepEqual(filterEpisodes(list, 'protected').map((e) => e.id), ['c'],
  'protected 缺省/false 都不算已保护');
assert.deepEqual(memoryFilterCounts(list), {all: 4, assistant: 2, user: 2, protected: 1});
assert.deepEqual(memoryFilterCounts([]), {all: 0, assistant: 0, user: 0, protected: 0});
assert.deepEqual(MEMORY_FILTERS.map((f) => f.id), ['all', 'assistant', 'user', 'protected'],
  '过滤器集合锁定——工作/近期/长期/画像等假分类不得回归');

// ---- 缺省态：category/importance 缺省不编 ----
assert.equal(episodeCategoryLabel(EP({})), '未分类', 'category 缺省 → 真实缺省态');
assert.equal(episodeCategoryLabel(EP({category: null})), '未分类');
assert.equal(episodeCategoryLabel(EP({category: '  '})), '未分类', '空白串同样按缺省处理');
assert.equal(episodeCategoryLabel(EP({category: '工作记忆'})), '工作记忆', '有真值就显示真值');
assert.equal(episodeImportanceText(EP({})), null, 'importance 缺省 → null（UI 不显示该行）');
assert.equal(episodeImportanceText(EP({importance: null})), null);
assert.equal(episodeImportanceText(EP({importance: 0.7})), '0.70');
assert.equal(episodeImportanceText(EP({importance: NaN})), null, 'NaN 按缺省处理');

// ---- 乐观锁修订账本 ----
assert.equal(expectedRevOf({}, 'a'), 0, '未操作过 = 初始 rev 0');
assert.equal(expectedRevOf({a: 3}, 'a'), 3);
const l1 = recordRevision({}, 'a', 1);
assert.equal(expectedRevOf(l1, 'a'), 1);
const l2 = recordRevision(l1, 'a', 2);
assert.equal(expectedRevOf(l2, 'a'), 2);
assert.equal(expectedRevOf(l1, 'a'), 1, '不可变更新——旧账本不被改写');
assert.equal(expectedRevOf(l2, 'b'), 0, '其他条目不受影响');

// ---- mutation 判别：409 = 冲突态，其余 error 原样 ----
assert.deepEqual(classifyMemoryMutationError({error: 'conflict', status: 409}), {kind: 'conflict'});
assert.deepEqual(classifyMemoryMutationError({error: 'HTTP 500', status: 500}),
  {kind: 'error', message: 'HTTP 500'});
assert.deepEqual(classifyMemoryMutationError({error: 'network down'}),
  {kind: 'error', message: 'network down'}, '无 status（网络层失败）按一般错误');

// ---- graph 关联 ----
const nodes = [
  {id: 's-1', label: '夜间长谈', kind: 'session'},
  {id: 'ep-1', label: '他喜欢纸质书', kind: 'episode'},
  {id: 'ep-2', label: '', kind: 'episode'},
];
const edges = [
  {from: 's-1', to: 'ep-1', weight: 1.0},
  {from: 's-1', to: 'ep-2', weight: 0.5, label: '包含'},
  {from: 's-2', to: 'ep-9', weight: 1.0},
];
assert.deepEqual(episodeGraphLinks('ep-1', nodes, edges), {
  node: nodes[1],
  edges: [edges[0]],
});
assert.deepEqual(episodeGraphLinks('ep-2', nodes, edges).edges, [edges[1]]);
assert.deepEqual(episodeGraphLinks('ep-x', nodes, edges), {node: null, edges: []},
  '图谱里没有该条目 = 无关联（诚实空态）');
assert.equal(graphNodeLabel('s-1', nodes), '夜间长谈');
assert.equal(graphNodeLabel('ep-2', nodes), 'ep-2', 'label 空 → 回落 id');
assert.equal(graphNodeLabel('11111111-1111-4111-8111-111111111111', nodes), '11111111…',
  '未知长 id → 截断，不编 label');

// ---- 幽灵编号 ----
assert.equal(ghostNumber(0), '01');
assert.equal(ghostNumber(8), '09');
assert.equal(ghostNumber(41), '42');

// ---- 时间：毫秒口径 + 秒级容错 ----
assert.equal(formatEpisodeTime(1726992000000), formatEpisodeTime(1726992000),
  '秒/毫秒双口径收敛到同一时刻');
assert.match(formatEpisodeTime(1726992000000), /9\/22/);

console.log('memory-ledger: all assertions passed');
