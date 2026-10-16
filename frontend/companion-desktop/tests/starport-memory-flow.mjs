// 深流记忆面纯逻辑（memory-flow-store.ts，真实模块）——账本/压缩/召回证明。
//
// 契约要点：
//   1. 账本 CRUD + kind/文本过滤 + 时间倒序；
//   2. 压缩视图：自然日时间窗折叠、kind 计数按稳定序、超预算丢最旧且
//      dropped 如实计数（0 装：不吞不平滑）；
//   3. 召回：词项命中计数降序（大小写不敏感、多词 OR）→ 时间降序 → 原序（稳定）；
//   4. 派发文本含 kind 前缀（账本一等字段的真实序列化）；
//   5. 0 装：空文本/非法 kind/非法时间/重复 id/非法预算一律抛错。
import assert from 'node:assert/strict';
import {
  appendObservation,
  removeObservation,
  listObservations,
  compressObservations,
  rankRecall,
  buildMemoryContent,
  COMPRESS_WINDOW_MS,
  MemoryFlowError,
} from '../src/lib/starport/apps/memory-flow-store.ts';

console.log('--- Starting Starport Memory Flow Check ---');

function obs(id, at, kind, text, source = '') {
  return {id, at, kind, text, source};
}

function codeOf(fn) {
  try {
    fn();
    return null;
  } catch (error) {
    return error instanceof MemoryFlowError ? error.code : 'not-flow-error';
  }
}

// ---------------------------------------------------------------------------
// 1. 账本 CRUD + 过滤 + 时间倒序
// ---------------------------------------------------------------------------
{
  let items = [];
  items = appendObservation(items, obs('o1', 1000, 'decision', '决定走事务写回'));
  items = appendObservation(items, obs('o2', 3000, 'fix', '修了区间越界', 'tool.filesystem'));
  items = appendObservation(items, obs('o3', 2000, 'decision', '决定压缩按日折叠'));

  const all = listObservations(items);
  assert.deepEqual(all.map((o) => o.id), ['o2', 'o3', 'o1'], '时间倒序');

  const decisions = listObservations(items, {kind: 'decision'});
  assert.deepEqual(decisions.map((o) => o.id), ['o3', 'o1'], 'kind 过滤');

  const byText = listObservations(items, {text: '写回'});
  assert.deepEqual(byText.map((o) => o.id), ['o1'], '文本过滤（大小写不敏感子串）');

  items = removeObservation(items, 'o2');
  assert.equal(items.length, 2);
  assert.ok(!items.some((o) => o.id === 'o2'));
  console.log('  [1] ledger CRUD + filter OK');
}

// ---------------------------------------------------------------------------
// 2. 压缩视图：kind 计数、要点拼接、预算裁剪与丢弃计数
// ---------------------------------------------------------------------------
{
  const day = COMPRESS_WINDOW_MS;
  const items = [
    obs('a1', day * 1 + 10, 'decision', '第一天决定'),
    obs('a2', day * 1 + 20, 'fix', '第一天修复'),
    obs('a3', day * 1 + 30, 'decision', '第二天前夜决定'),
    obs('b1', day * 2 + 10, 'note', '第二天笔记'),
  ];
  // a3 与 a1/a2 同日窗（day*1）。
  const wide = compressObservations(items, 10);
  assert.equal(wide.dropped, 0);
  assert.equal(wide.lines.length, 2, '两个自然日窗两行');
  assert.match(wide.lines[0], /^1970-01-02 \[decision×2 fix×1\] /, 'kind 标签与计数按稳定序');
  assert.match(wide.lines[0], /第一天决定；第一天修复；第二天前夜决定/, '文本按时间升序拼接');
  assert.match(wide.lines[1], /^1970-01-03 \[note×1\] /);

  // 预算 1：丢最旧窗（3 条），如实计数。
  const trimmed = compressObservations(items, 1);
  assert.equal(trimmed.lines.length, 1);
  assert.equal(trimmed.dropped, 3, '丢弃数=被丢弃窗口的观察条数');
  assert.match(trimmed.lines[0], /^1970-01-03 /, '保最新窗');

  // 空账本：零行零丢弃。
  assert.deepEqual(compressObservations([], 5), {lines: [], dropped: 0});
  console.log('  [2] compress window + budget OK');
}

// ---------------------------------------------------------------------------
// 3. 召回排序：命中计数、大小写不敏感、多词 OR、稳定 tie、空查询
// ---------------------------------------------------------------------------
{
  const items = [
    obs('r1', 100, 'note', '事务与区间'),
    obs('r2', 200, 'note', '事务事务重复词'),
    obs('r3', 300, 'note', '无关内容'),
    obs('r4', 150, 'note', '区间与哈希'),
  ];
  const ranked = rankRecall(items, '事务 区间');
  // r1: 2 词命中；r2: 1（事务计 1）；r4: 1（区间）；r3: 0。
  assert.deepEqual(ranked.map((o) => o.id), ['r1', 'r2', 'r4', 'r3'], '命中数降序');
  // r2(200) 在 r4(150) 前：同命中数按时间降序。

  const cased = [obs('e1', 10, 'note', 'Range CHECK failed'), obs('e2', 20, 'note', 'other')];
  assert.equal(rankRecall(cased, 'range')[0].id, 'e1', '大小写不敏感（命中者排前）');

  // 重复词不加成（可观察判定：去重后两词各中一条=tie 按时间降序；
  // 若重复计分则『事务』侧 2:1 领先，顺序反转）。
  const dup = [obs('d1', 1, 'note', '事务'), obs('d2', 2, 'note', '其他')];
  assert.deepEqual(
    rankRecall(dup, '事务 事务 其他').map((o) => o.id),
    ['d2', 'd1'],
    '重复词不加成（同命中 tie 按时间降序）',
  );

  // 稳定 tie：同命中同时间保持原序。
  const twins = [obs('t1', 500, 'note', '同文'), obs('t2', 500, 'note', '同文')];
  assert.deepEqual(rankRecall(twins, '同文').map((o) => o.id), ['t1', 't2'], '稳定排序');

  assert.deepEqual(rankRecall(items, '  ').map((o) => o.id), ['r3', 'r2', 'r4', 'r1'], '空查询=时间倒序');
  console.log('  [3] recall ranking OK');
}

// ---------------------------------------------------------------------------
// 4. 派发文本格式（契约 §5 content 串）
// ---------------------------------------------------------------------------
{
  assert.equal(
    buildMemoryContent(obs('m1', 1, 'decision', '走事务写回', 'tool.filesystem')),
    '[深流/decision] 走事务写回（来源 tool.filesystem）',
  );
  assert.equal(
    buildMemoryContent(obs('m2', 1, 'note', '无来源观察')),
    '[深流/note] 无来源观察',
  );
  console.log('  [4] dispatch content OK');
}

// ---------------------------------------------------------------------------
// 5. 0 装边界
// ---------------------------------------------------------------------------
{
  const items = [obs('x1', 1, 'note', '已有')];
  assert.equal(codeOf(() => appendObservation(items, obs('x2', 1, 'note', '   '))), 'invalid-text', '纯空白文本拒绝');
  assert.equal(codeOf(() => appendObservation(items, obs('x2', 1, 'note', ''))), 'invalid-text');
  assert.equal(codeOf(() => appendObservation(items, obs('x2', 1, 'mood', 'x'))), 'invalid-kind');
  assert.equal(codeOf(() => appendObservation(items, obs('x2', Number.NaN, 'note', 'x'))), 'invalid-time');
  assert.equal(codeOf(() => appendObservation(items, obs('x2', 'soon', 'note', 'x'))), 'invalid-time');
  assert.equal(codeOf(() => appendObservation(items, obs('', 1, 'note', 'x'))), 'invalid-id');
  assert.equal(codeOf(() => appendObservation(items, obs('x1', 1, 'note', 'x'))), 'duplicate-id');
  assert.equal(codeOf(() => appendObservation(items, {...obs('x2', 1, 'note', 'x'), source: 7})), 'invalid-source');
  assert.equal(codeOf(() => removeObservation(items, 'ghost')), 'missing');
  assert.equal(codeOf(() => compressObservations(items, 0)), 'invalid-budget');
  assert.equal(codeOf(() => compressObservations(items, 1.5)), 'invalid-budget');
  assert.equal(codeOf(() => listObservations(items, {kind: 'mood'})), 'invalid-kind');
  assert.equal(codeOf(() => buildMemoryContent(obs('z', 1, 'bad', 'x'))), 'invalid-kind');
  console.log('  [5] zero-fabrication boundaries OK');
}

console.log('--- Starport Memory Flow Check PASSED ---');
