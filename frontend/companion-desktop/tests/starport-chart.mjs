// 星仪图表面纯逻辑（chart-store.ts，真实模块）——数据驱动几何证明。
//
// 契约要点：
//   1. 同数据同参恒同几何（纯函数）；数值→像素映射可手算核对；
//   2. min==max 退化居中（不除零）；单点折线合法；
//   3. 0 装：非有限值/长度不齐/非法画布一律抛错。
import assert from 'node:assert/strict';
import {
  computeLineGeometry,
  computeBarGeometry,
} from '../src/lib/starport/apps/chart-store.ts';

console.log('--- Starting Starport Chart Check ---');

const frame = {width: 100, height: 100, padding: 10};

// ---------------------------------------------------------------------------
// 1. 折线几何：数值→像素可手算核对
// ---------------------------------------------------------------------------
{
  const data = {
    labels: ['a', 'b', 'c'],
    series: [{name: 's1', points: [0, 5, 10]}],
  };
  const geo = computeLineGeometry(data, frame);
  assert.equal(geo.min, 0);
  assert.equal(geo.max, 10);
  // x: 10 / 50 / 90；y: 值 0→90（底），5→50（中），10→10（顶）。
  assert.match(geo.paths[0].d, /^M10\.00,90\.00 L50\.00,50\.00 L90\.00,10\.00$/);
  assert.equal(geo.ticks.length, 3);
  assert.equal(geo.ticks[1].x, 50);
  console.log('  [1] line geometry OK');
}

// ---------------------------------------------------------------------------
// 2. min==max 与单点：退化居中不除零
// ---------------------------------------------------------------------------
{
  const flat = computeLineGeometry(
    {labels: ['a', 'b'], series: [{name: 's', points: [7, 7]}]},
    frame,
  );
  assert.match(flat.paths[0].d, /50\.00/);
  const single = computeLineGeometry(
    {labels: ['only'], series: [{name: 's', points: [42]}]},
    frame,
  );
  assert.equal(single.ticks[0].x, 50, '单点居中');
  assert.match(single.paths[0].d, /^M50\.00,/);
  console.log('  [2] degenerate cases OK');
}

// ---------------------------------------------------------------------------
// 3. 柱状几何：槽位均分、高度可核对
// ---------------------------------------------------------------------------
{
  const data = {
    labels: ['a', 'b'],
    series: [
      {name: 's1', points: [0, 10]},
      {name: 's2', points: [10, 0]},
    ],
  };
  const bars = computeBarGeometry(data, frame).bars;
  assert.equal(bars.length, 4, '2 标签 × 2 序列');
  // 基线值 0 在 y=90，值 10 在 y=10：s1 第二柱高度 80（全值域）。
  const tall = bars.find((b) => b.name === 's1' && b.index === 1);
  assert.ok(tall);
  assert.equal(tall.height, 80);
  assert.equal(tall.y, 10);
  assert.ok(tall.x > 50, '第二槽位在右半区');
  console.log('  [3] bar geometry OK');
}

// ---------------------------------------------------------------------------
// 4. 0 装：非法输入抛错
// ---------------------------------------------------------------------------
{
  const bad = [
    [{labels: ['a'], series: [{name: 's', points: [Number.NaN]}]}, frame, '非有限值'],
    [{labels: ['a', 'b'], series: [{name: 's', points: [1]}]}, frame, '长度不齐'],
    [{labels: ['a'], series: []}, frame, '空序列'],
    [{labels: ['a'], series: [{name: 's', points: [1]}]}, {width: 0, height: 100, padding: 10}, '零宽画布'],
    [
      {labels: ['a'], series: [{name: 's', points: [1]}]},
      {width: 100, height: 100, padding: 60},
      'padding 越界',
    ],
  ];
  for (const [data, f, why] of bad) {
    assert.throws(() => computeLineGeometry(data, f), undefined, `必须抛错：${why}`);
    assert.throws(() => computeBarGeometry(data, f), undefined, `必须抛错：${why}`);
  }
  console.log('  [4] zero-fabrication boundaries OK');
}

console.log('--- Starport Chart Check PASSED ---');
