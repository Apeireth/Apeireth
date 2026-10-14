// 泊位磁贴纯逻辑（folder-tiles-store.ts，真实模块）——磁贴账本与列举转磁贴证明。
//
// 契约要点：
//   1. 分组 CRUD + 展示序（dir→file→link，名称大小写不敏感）；
//   2. 校验 0 装：名字空/超限、target 空/超限、id 冲突/为空、非法 kind 一律抛；
//   3. buildTilesFromListing：id 可复算 `f:<name>`、目录优先、空名抛错不吞；
//   4. 快照副本防篡改。
import assert from 'node:assert/strict';
import {
  TileError,
  orderTiles,
  createTileGroup,
  addTile,
  removeTile,
  listTiles,
  buildTilesFromListing,
  parseTileSnapshot,
  serializeTileSnapshot,
  TilesStore,
  TILE_NAME_MAX,
  TILE_TARGET_MAX,
} from '../src/lib/starport/apps/folder-tiles-store.ts';

console.log('--- Starting Starport Folder Tiles Check ---');

const tile = (id, name, kind, target) => ({id, name, kind, target: target ?? `T:${name}`});

// 1. 分组 CRUD 与快照副本
{
  let groups = [createTileGroup('g1', '工作区')];
  groups = addTile(groups, 'g1', tile('t1', 'src', 'dir'));
  groups = addTile(groups, 'g1', tile('t2', 'readme', 'file'));
  assert.equal(groups[0].tiles.length, 2);

  const snapshot = listTiles(groups, 'g1');
  snapshot.length = 0;
  assert.equal(listTiles(groups, 'g1').length, 2, '快照是副本，防外部篡改');

  groups = removeTile(groups, 'g1', 't1');
  assert.deepEqual(listTiles(groups, 'g1').map((item) => item.id), ['t2']);
  console.log('  [1] group CRUD OK');
}

// 2. 校验 0 装
{
  const groups = [createTileGroup('g1', '组')];
  assert.throws(() => createTileGroup('  ', '组'), (error) => error instanceof TileError && error.code === 'invalid-argument');
  assert.throws(() => createTileGroup('g', ' '), (error) => error instanceof TileError && error.code === 'invalid-name');
  assert.throws(() => createTileGroup('g', 'x'.repeat(TILE_NAME_MAX + 1)), (error) => error instanceof TileError && error.code === 'invalid-name');
  assert.throws(() => addTile(groups, 'g1', tile('t', ' ', 'dir')), (error) => error instanceof TileError && error.code === 'invalid-name');
  assert.throws(() => addTile(groups, 'g1', {id: 't', name: 'n', kind: 'dir', target: ''}), (error) => error instanceof TileError && error.code === 'invalid-target');
  assert.throws(() => addTile(groups, 'g1', {id: 't', name: 'n', kind: 'dir', target: 'x'.repeat(TILE_TARGET_MAX + 1)}), (error) => error instanceof TileError && error.code === 'invalid-target');
  assert.throws(() => addTile(groups, 'g1', tile('', 'n', 'dir')), (error) => error instanceof TileError && error.code === 'invalid-argument');
  assert.throws(() => addTile(groups, 'g1', tile('t', 'n', 'other')), (error) => error instanceof TileError && error.code === 'invalid-kind');

  const one = addTile(groups, 'g1', tile('t1', 'a', 'dir'));
  assert.throws(() => addTile(one, 'g1', tile('t1', 'b', 'file')), (error) => error instanceof TileError && error.code === 'duplicate-id');
  assert.throws(() => removeTile(one, 'g1', 'ghost'), (error) => error instanceof TileError && error.code === 'missing');
  assert.throws(() => addTile(one, 'ghost', tile('t', 'n', 'dir')), (error) => error instanceof TileError && error.code === 'missing');
  console.log('  [2] zero-fabrication validation OK');
}

// 3. 展示序：dir → file → link，名称序
{
  const sorted = orderTiles([
    tile('3', 'zeta', 'link'),
    tile('1', 'beta', 'file'),
    tile('2', 'Alpha', 'dir'),
    tile('4', 'alpha', 'dir'),
    tile('5', 'a.txt', 'file'),
  ]);
  assert.deepEqual(
    sorted.map((item) => item.name),
    ['Alpha', 'alpha', 'a.txt', 'beta', 'zeta'],
    'dir 优先 → file → link，名称大小写不敏感',
  );
  console.log('  [3] display ordering OK');
}

// 4. buildTilesFromListing
{
  const tiles = buildTilesFromListing([
    {name: 'b.txt', is_dir: false},
    {name: 'Docs', is_dir: true},
    {name: 'a.txt', is_dir: false},
    {name: 'apps', is_dir: true},
  ]);
  assert.deepEqual(tiles.map((item) => item.name), ['apps', 'Docs', 'a.txt', 'b.txt'], '目录优先 + 名称序');
  assert.equal(tiles[0].id, 'f:apps', 'id 可复算');
  assert.equal(tiles[0].kind, 'dir');
  assert.equal(tiles[2].kind, 'file');
  assert.equal(tiles[2].target, 'a.txt', 'target 记相对名');

  assert.throws(() => buildTilesFromListing([{name: ' ', is_dir: false}]), (error) => error instanceof TileError, '空名条目不吞');
  assert.throws(() => buildTilesFromListing('nope'), (error) => error instanceof TileError && error.code === 'invalid-argument');
  console.log('  [4] listing→tiles mapping OK');
}

// 5. 持久化：快照往返闭环 + 稳定键序可复算
{
  let groups = [createTileGroup('g1', '工作区'), createTileGroup('g2', '生活')];
  groups = addTile(groups, 'g1', tile('t1', 'src', 'dir'));
  groups = addTile(groups, 'g1', tile('t2', 'note', 'link', 'https://a.test/'));
  groups = addTile(groups, 'g2', tile('t3', 'readme', 'file'));

  const text = serializeTileSnapshot(groups);
  const restored = parseTileSnapshot(JSON.parse(text));
  assert.deepEqual(restored, groups, 'serialize→parse 恒等（数组原序保真）');
  assert.equal(serializeTileSnapshot(restored), text, '稳定键序可复算（同账本恒同字符串）');

  assert.deepEqual(parseTileSnapshot(null), [], 'null = 全新起步');
  assert.deepEqual(parseTileSnapshot(undefined), [], 'undefined = 全新起步');
  console.log('  [5] snapshot round-trip OK');
}

// 6. 持久化：坏载荷拒绝矩阵（一律 corrupt-persistence，不静默修）
{
  for (const [raw, why] of [
    ['nope', '非数组载荷'],
    [42, '非对象载荷'],
    [{}, '非数组对象载荷'],
    [['x'], '分组条目非对象'],
    [{name: 'g', tiles: []}, '缺分组 id'],
    [{id: 7, name: 'g', tiles: []}, '分组 id 类型错'],
    [{id: ' ', name: 'g', tiles: []}, '分组 id 空'],
    [{id: 'g', name: ' ', tiles: []}, '分组名非法'],
    [{id: 'g', name: 'n', tiles: {}}, '磁贴非数组'],
    [{id: 'g', name: 'n', tiles: [['x']]}, '磁贴条目非对象'],
    [{id: 'g', name: 'n', tiles: [{id: 't', name: 'n', kind: 'dir'}]}, '磁贴缺 target'],
    [{id: 'g', name: 'n', tiles: [{id: 't', name: 'n', target: 'x', kind: 'other'}]}, '非法 kind'],
    [
      [{id: 'g', name: 'a', tiles: []}, {id: 'g', name: 'b', tiles: []}],
      '分组 id 重复',
    ],
    [
      [{id: 'g', name: 'n', tiles: [{id: 't', name: 'x', target: 'x', kind: 'dir'}, {id: 't', name: 'y', target: 'y', kind: 'file'}]}],
      '磁贴 id 重复',
    ],
  ]) {
    assert.throws(
      () => parseTileSnapshot(raw),
      (error) => error instanceof TileError && error.code === 'corrupt-persistence',
      `坏载荷必须拒绝：${why}`,
    );
  }
  console.log('  [6] corrupt-persistence matrix OK');
}

// 7. 持久化：适配器注入往返 + 自动 save + save 失败宁拒不写
{
  const adapter = {
    data: null,
    saves: 0,
    load() {
      return this.data;
    },
    save(groups) {
      this.saves += 1;
      this.data = groups;
    },
  };
  const store = new TilesStore({adapter});
  assert.deepEqual(store.getGroups(), [], '空载荷全新起步');
  store.addGroup('manual', '手动磁贴');
  store.addTile('manual', tile('t1', 'docs', 'dir'));
  assert.equal(adapter.saves, 2, '每次变更自动 save');
  assert.equal(adapter.data.length, 1, 'save 真落存快照');

  // 跨会话往返：新实例吃同一载荷。
  const reopened = new TilesStore({adapter});
  assert.deepEqual(reopened.getGroups(), store.getGroups(), '跨会话载荷往返恒等');
  reopened.removeTile('manual', 't1');
  assert.equal(adapter.saves, 3, '删除同样落存');

  // save 失败：如实抛且账本不推进（宁拒不写）。
  const failing = {
    load: () => null,
    save() {
      throw new Error('disk full');
    },
  };
  const strict = new TilesStore({adapter: failing});
  assert.throws(() => strict.addGroup('manual', '手动磁贴'), /disk full/, 'save 失败如实抛（不吞）');
  assert.deepEqual(strict.getGroups(), [], 'save 失败账本不推进（宁拒不写）');
  console.log('  [7] adapter round-trip OK');
}

console.log('--- Starport Folder Tiles Check PASSED ---');
