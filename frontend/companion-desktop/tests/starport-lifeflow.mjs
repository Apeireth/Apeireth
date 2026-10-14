// 信风信息流纯逻辑（lifeflow-store.ts，真实模块）——源登记与条目流证明。
//
// 契约要点：
//   1. 信息源只收 HTTP(S)（与深望同一协议闸）；id 冲突抛错；
//   2. 条目流去重、置读/未读、未读计数、最新在前；
//   3. 注意力纪律：未读计数是刷流闸的计数面（闸值归想法 A）。
import assert from 'node:assert/strict';
import {
  addSource,
  addItems,
  markRead,
  countUnread,
  listItems,
} from '../src/lib/starport/apps/lifeflow-store.ts';

console.log('--- Starting Starport Lifeflow Check ---');

// 1. 源登记 + 条目流
{
  let sources = addSource([], {id: 'src1', name: '视频源', url: 'https://video.example/feed'});
  sources = addSource(sources, {id: 'src2', name: '资讯源', url: 'http://news.example/rss'});
  assert.equal(sources.length, 2);

  let items = addItems([], [
    {id: 'i1', sourceId: 'src1', title: '第一条', summary: null, at: 100, read: true},
    {id: 'i2', sourceId: 'src2', title: '第二条', summary: '摘', at: 200, read: true},
  ]);
  assert.equal(countUnread(items), 2, '入库强制未读（read 归零）');

  items = markRead(items, 'i2', true);
  assert.equal(countUnread(items), 1);
  const view = listItems(items);
  assert.equal(view[0].id, 'i2', '最新在前');
  assert.equal(listItems(items, {unreadOnly: true}).length, 1);
  console.log('  [1] source + item stream OK');
}

// 2. 0 装边界
{
  assert.throws(() => addSource([], {id: 'x', name: '', url: 'https://a.test/'}), /名称不能为空/);
  assert.throws(() => addSource([], {id: 'x', name: 'a', url: 'ftp://a.test/'}), /只收 HTTP/);
  assert.throws(() => addSource([], {id: 'x', name: 'a', url: 'not url'}), /地址非法/);
  const src = addSource([], {id: 'x', name: 'a', url: 'https://a.test/'});
  assert.throws(() => addSource(src, {id: 'x', name: 'b', url: 'https://b.test/'}), /id 冲突/);

  const items = addItems([], [
    {id: 'i1', sourceId: 'x', title: 't', summary: null, at: 1, read: true},
  ]);
  assert.throws(() => addItems(items, [
    {id: 'i1', sourceId: 'x', title: 't2', summary: null, at: 2, read: true},
  ]), /id 冲突/);
  assert.throws(() => addItems(items, [
    {id: 'i2', sourceId: 'x', title: ' ', summary: null, at: 2, read: true},
  ]), /标题不能为空/);
  assert.throws(() => markRead(items, 'ghost', true), /条目不存在/);
  console.log('  [2] zero-fabrication boundaries OK');
}

console.log('--- Starport Lifeflow Check PASSED ---');
