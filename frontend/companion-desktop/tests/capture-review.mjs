// 随手记卡（V1-b/d 上屏：真实模块 src/lib/capture-review.ts + features/quick 账本），
// 并对 views/Workbench.svelte 的随手记卡做源码级镜像校验。
// 契约（任务口径）：
//   1. 列表：最近 50 条账本（新→旧），与快捷窗回看同一本账（同源只读）。
//   2. 过滤：空白 = 全量；正文子串 / 序号（#3 或 3）命中即保留；不改排序。
//   3. 空态诚实：账本空不冒充有记录（零假旋钮）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  CAPTURE_REVIEW_LIMIT,
  captureReviewRows,
  filterCaptureRows,
} from '../src/lib/capture-review.ts';
import {
  CAPTURE_LEDGER_MAX,
  CAPTURE_RECENT_VIEW,
  CaptureLedger,
} from '../src/features/quick/quick-capture.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const workbenchSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'Workbench.svelte'),
  'utf8',
);

console.log('--- capture review card (V1-b/d) ---');

function check(name, fn) {
  fn();
  console.log(`  ok - ${name}`);
}

/** 内存账本桩（localStorage 不可用时 quick-capture 自身回落内存；这里显式给桩）。 */
function memoryLedger() {
  let raw = null;
  return new CaptureLedger({
    read: () => raw,
    write: (next) => {
      raw = next;
    },
  });
}

function seed(ledger, count, base = 1728000000000) {
  for (let i = 1; i <= count; i += 1) {
    const out = ledger.begin(`#条目 ${i}`, base + i * 1000);
    assert.ok(out.ok, `种子条目 ${i} 成功`);
    ledger.commit(out.record);
  }
  return ledger;
}

// ---------------------------------------------------------------------------
// 1. 列表：最近 50 条账本（新→旧）+ 与快捷窗同源口径
// ---------------------------------------------------------------------------
check('列表: 最近 50 条账本新→旧（上限 = 账本上限）', () => {
  assert.equal(CAPTURE_REVIEW_LIMIT, 50, '任务口径「最近 50 条账本」');
  assert.equal(CAPTURE_REVIEW_LIMIT, CAPTURE_LEDGER_MAX, '与账本上限同源（50 = 账本上限）');
  const ledger = seed(memoryLedger(), 60);
  const rows = captureReviewRows(ledger.records(), CAPTURE_REVIEW_LIMIT);
  assert.equal(rows.length, 50, '超上限只取最近 50');
  assert.equal(rows[0].text, '条目 60', '最新在前');
  assert.equal(rows[49].text, '条目 11', '第 50 条 = 序号 11（60 条里的最近 50）');
  assert.ok(rows[0].time.length === 5, '时间列 HH:mm（与快捷窗回看同格式）');
  assert.ok(rows.every((r, i) => i === 0 || rows[i - 1].at >= r.at), '单调新→旧');
});

check('列表: 与快捷窗回看同源（同一账本两种取量）', () => {
  const ledger = seed(memoryLedger(), 10);
  const quick = ledger.recent(CAPTURE_RECENT_VIEW);
  const workbench = captureReviewRows(ledger.records(), CAPTURE_REVIEW_LIMIT);
  assert.equal(quick.length, 5, '快捷窗回看 5 条（既有口径）');
  assert.equal(workbench.length, 10, '工作台回看 50 条上限内全量');
  assert.deepEqual(
    quick.map((r) => r.seq),
    workbench.slice(0, 5).map((r) => r.seq),
    '同一本账：工作台前 5 条与快捷窗回看同序同源',
  );
});

// ---------------------------------------------------------------------------
// 2. 过滤：空白全量 / 正文子串 / 序号命中 / 不改排序
// ---------------------------------------------------------------------------
check('过滤: 正文子串大小写不敏感；空白 = 全量', () => {
  const rows = captureReviewRows(
    [
      {seq: 1, text: '明早买牛奶', at: 1000, source: 'capture', protected: false},
      {seq: 2, text: 'MILK and eggs', at: 2000, source: 'capture', protected: false},
      {seq: 3, text: '给花浇水', at: 3000, source: 'capture', protected: false},
    ],
    CAPTURE_REVIEW_LIMIT,
  );
  assert.equal(filterCaptureRows(rows, '   ').length, 3, '空白查询 = 全量');
  assert.equal(filterCaptureRows(rows, 'milk').length, 1, '大小写不敏感命中 MILK');
  assert.equal(filterCaptureRows(rows, '牛奶').length, 1, '中文子串命中');
  assert.deepEqual(filterCaptureRows(rows, '买').map((r) => r.seq), [1]);
  assert.equal(filterCaptureRows(rows, '不存在的词').length, 0, '无命中 = 空列表（不回落全量）');
});

check('过滤: 序号命中（#3 / 3 同效）+ 过滤不改排序', () => {
  const rows = captureReviewRows(
    [
      {seq: 3, text: '三号', at: 3000, source: 'capture', protected: false},
      {seq: 1, text: '一号', at: 1000, source: 'capture', protected: false},
      {seq: 2, text: '二号', at: 2000, source: 'capture', protected: false},
    ],
    CAPTURE_REVIEW_LIMIT,
  );
  assert.deepEqual(rows.map((r) => r.seq), [3, 2, 1], '投影已按新→旧');
  assert.deepEqual(filterCaptureRows(rows, '#2').map((r) => r.seq), [2], '#2 命中序号 2');
  assert.deepEqual(filterCaptureRows(rows, '2').map((r) => r.seq), [2], '裸序号同效');
  assert.deepEqual(filterCaptureRows(rows, '二').map((r) => r.seq), [2], '正文命中');
  const mixed = filterCaptureRows(rows, '号');
  assert.deepEqual(mixed.map((r) => r.seq), [3, 2, 1], '过滤保持新→旧排序');
});

check('空态: 账本空不冒充有记录（零假旋钮）', () => {
  const rows = captureReviewRows([], CAPTURE_REVIEW_LIMIT);
  assert.deepEqual(rows, [], '空账本 = 空列表');
  assert.deepEqual(filterCaptureRows([], 'x'), [], '空列表过滤 = 空');
});

// ---------------------------------------------------------------------------
// 3. 源码镜像：卡片区随手记卡接线
// ---------------------------------------------------------------------------
check('源码镜像: 随手记卡挂卡片区 / 同源账本 / 搜索过滤接线', () => {
  assert.ok(workbenchSrc.includes('<StickyNote size={13} /> 随手记'), '随手记卡上屏');
  assert.ok(workbenchSrc.includes('captureReviewRows('), '列表走纯函数');
  assert.ok(workbenchSrc.includes('filterCaptureRows('), '过滤走纯函数');
  assert.ok(workbenchSrc.includes('new CaptureLedger()'), '与快捷窗回看同一本账（同源只读）');
  assert.ok(workbenchSrc.includes('CAPTURE_REVIEW_LIMIT'), '最近 50 条账本口径上屏');
  assert.ok(workbenchSrc.includes('bind:value={query}'), '搜索框接线');
  assert.ok(workbenchSrc.includes('记忆流是真源'), '账本定位注记在案（账本=回看缓存）');
});

console.log('capture-review: all checks passed');
