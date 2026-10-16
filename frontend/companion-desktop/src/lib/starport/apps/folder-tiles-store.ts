// 泊位磁贴 · 纯逻辑（批次推进，零依赖纯模块）
//
// 契约（I-starport-builtins.md「泊位」）：
//   1. 磁贴模型 —— Tile {id, name, target, kind: 'dir'|'file'|'link'}；分组
//      账本 CRUD + 展示序（目录优先 → 文件 → 链接，名称序）；
//   2. 校验 0 装 —— 名字空/超限抛错、id 冲突抛错、target 空/超限抛错，
//      非法 kind 抛错（不猜不修）；
//   3. 列举转磁贴 —— buildTilesFromListing 把只读列举条目转磁贴（目录优先）；
//      列举条目名在目录内天然唯一，id 即 `f:<name>`（可复算），target 记相对
//      名（装配层拼基路径）；
//   4. 快照副本 —— 账本读出副本防外部篡改；
//   5. 映射持久化（批次推进）—— parseTileSnapshot 严格解析（字段缺失/类型
//      不符/重复 id/非法 kind 一律 corrupt-persistence，不静默修）；null/
//      undefined = 全新起步返回空表；serializeTileSnapshot 稳定键序可复算
//      （排序即承诺）；TilesStore 注入 {load, save} 适配器，每次变更自动
//      save，save 失败如实抛（不吞）。
//
// 测试：tests/starport-folder-tiles.mjs（真实模块直测）。

export type TileKind = 'dir' | 'file' | 'link';

export interface Tile {
  id: string;
  name: string;
  target: string;
  kind: TileKind;
}

export interface TileGroup {
  id: string;
  name: string;
  tiles: Tile[];
}

export interface ListingEntry {
  name: string;
  is_dir: boolean;
}

/** 持久化适配器（load 返回原始载荷；save 收快照副本）。 */
export interface TilesPersistenceAdapter {
  load(): unknown;
  save(groups: TileGroup[]): void;
}

export const TILE_NAME_MAX = 120;
export const TILE_TARGET_MAX = 2000;

export class TileError extends Error {
  readonly code: 'invalid-name' | 'invalid-target' | 'invalid-kind' | 'invalid-argument' | 'duplicate-id' | 'missing' | 'corrupt-persistence';

  constructor(code: TileError['code'], message: string) {
    super(message);
    this.name = 'TileError';
    this.code = code;
  }
}

const KIND_RANK: Record<TileKind, number> = {dir: 0, file: 1, link: 2};

function checkName(name: string): string {
  const value = typeof name === 'string' ? name.trim() : '';
  if (!value) throw new TileError('invalid-name', '名称不能为空');
  if (value.length > TILE_NAME_MAX) throw new TileError('invalid-name', `名称超限（>${TILE_NAME_MAX}）`);
  return value;
}

function checkTarget(target: string): string {
  const value = typeof target === 'string' ? target.trim() : '';
  if (!value) throw new TileError('invalid-target', '磁贴目标不能为空');
  if (value.length > TILE_TARGET_MAX) throw new TileError('invalid-target', `目标超限（>${TILE_TARGET_MAX}）`);
  return value;
}

function checkTile(input: {id: string; name: string; target: string; kind: TileKind}): Tile {
  const id = typeof input.id === 'string' ? input.id.trim() : '';
  if (!id) throw new TileError('invalid-argument', '磁贴 id 不能为空');
  if (!(input.kind in KIND_RANK)) throw new TileError('invalid-kind', `非法磁贴形态：${String(input.kind)}`);
  return {id, name: checkName(input.name), target: checkTarget(input.target), kind: input.kind};
}

/** 确定性码点比较（排序即承诺：不随区域漂移，与原生侧同口径）。 */
function byCodepoint(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** 展示序纯函数：dir → file → link，名称大小写不敏感升序 → 原名稳定序。 */
export function orderTiles(tiles: readonly Tile[]): Tile[] {
  return [...tiles].sort(
    (a, b) =>
      KIND_RANK[a.kind] - KIND_RANK[b.kind] ||
      byCodepoint(a.name.toLowerCase(), b.name.toLowerCase()) ||
      byCodepoint(a.name, b.name),
  );
}

export function createTileGroup(id: string, name: string): TileGroup {
  const groupId = typeof id === 'string' ? id.trim() : '';
  if (!groupId) throw new TileError('invalid-argument', '分组 id 不能为空');
  return {id: groupId, name: checkName(name), tiles: []};
}

/** 组内增磁贴（id 组内唯一，冲突抛错）。返回新分组数组。 */
export function addTile(groups: readonly TileGroup[], groupId: string, input: {id: string; name: string; target: string; kind: TileKind}): TileGroup[] {
  const group = groups.find((item) => item.id === groupId);
  if (!group) throw new TileError('missing', `分组不存在：${groupId}`);
  const tile = checkTile(input);
  if (group.tiles.some((item) => item.id === tile.id)) {
    throw new TileError('duplicate-id', `磁贴 id 冲突：${tile.id}`);
  }
  return groups.map((item) =>
    item.id === groupId ? {...item, tiles: orderTiles([...item.tiles, tile])} : item,
  );
}

export function removeTile(groups: readonly TileGroup[], groupId: string, tileId: string): TileGroup[] {
  const group = groups.find((item) => item.id === groupId);
  if (!group) throw new TileError('missing', `分组不存在：${groupId}`);
  if (!group.tiles.some((item) => item.id === tileId)) {
    throw new TileError('missing', `磁贴不存在：${tileId}`);
  }
  return groups.map((item) =>
    item.id === groupId ? {...item, tiles: item.tiles.filter((tile) => tile.id !== tileId)} : item,
  );
}

/** 读出快照（副本防篡改，按展示序）。 */
export function listTiles(groups: readonly TileGroup[], groupId: string): Tile[] {
  const group = groups.find((item) => item.id === groupId);
  if (!group) throw new TileError('missing', `分组不存在：${groupId}`);
  return orderTiles(group.tiles);
}

/**
 * 只读列举条目 → 磁贴（目录优先，经 orderTiles 统一展示序）。
 * id 可复算：`f:<name>`（目录内条目名天然唯一）；target 记相对名，基路径由
 * 装配层拼接。空名条目抛错（列举面不该产出空名，0 装不吞）。
 */
export function buildTilesFromListing(entries: readonly ListingEntry[]): Tile[] {
  if (!Array.isArray(entries)) throw new TileError('invalid-argument', '条目必须是数组');
  const tiles = entries.map((entry) => {
    const name = checkName(entry.name);
    return {
      id: `f:${name}`,
      name,
      target: name,
      kind: entry.is_dir ? ('dir' as const) : ('file' as const),
    };
  });
  return orderTiles(tiles);
}

// ---------------------------------------------------------------------------
// 映射持久化（批次推进）：严格解析 / 稳定键序序列化 / 适配器注入账本
// ---------------------------------------------------------------------------

function corrupt(message: string): TileError {
  return new TileError('corrupt-persistence', message);
}

/**
 * 严格解析持久化载荷（0 装：字段缺失/类型不符/重复 id/非法 kind 一律
 * corrupt-persistence 抛出，不修补、不留半条）。null/undefined = 全新起步，
 * 返回空表。解析保持数组原序（往返对称的另一半在 serialize）。
 */
export function parseTileSnapshot(raw: unknown): TileGroup[] {
  if (raw === null || raw === undefined) return [];
  if (!Array.isArray(raw)) throw corrupt('持久化载荷必须是分组数组');
  const seenGroups = new Set<string>();
  const groups: TileGroup[] = [];
  for (const groupItem of raw) {
    if (!groupItem || typeof groupItem !== 'object' || Array.isArray(groupItem)) {
      throw corrupt('分组条目必须是对象');
    }
    const g = groupItem as Record<string, unknown>;
    const {id, name, tiles} = g;
    if (typeof id !== 'string' || !id.trim()) throw corrupt('分组 id 缺失或非字符串');
    if (seenGroups.has(id)) throw corrupt(`分组 id 冲突：${id}`);
    seenGroups.add(id);
    let safeName: string;
    try {
      safeName = checkName(name as string);
    } catch (caught) {
      throw corrupt(`分组名非法：${caught instanceof Error ? caught.message : String(caught)}`);
    }
    if (!Array.isArray(tiles)) throw corrupt(`分组 ${id} 的磁贴必须是数组`);
    const seenTiles = new Set<string>();
    const safeTiles: Tile[] = [];
    for (const tileItem of tiles) {
      if (!tileItem || typeof tileItem !== 'object' || Array.isArray(tileItem)) {
        throw corrupt(`分组 ${id} 的磁贴条目必须是对象`);
      }
      const t = tileItem as Record<string, unknown>;
      if (
        typeof t.id !== 'string' || !t.id.trim() ||
        typeof t.name !== 'string' ||
        typeof t.target !== 'string' ||
        typeof t.kind !== 'string'
      ) {
        throw corrupt(`分组 ${id} 的磁贴字段缺失或类型不符`);
      }
      if (seenTiles.has(t.id)) throw corrupt(`磁贴 id 冲突：${t.id}`);
      seenTiles.add(t.id);
      let safeTile: Tile;
      try {
        safeTile = checkTile({
          id: t.id,
          name: t.name,
          target: t.target,
          kind: t.kind as TileKind,
        });
      } catch (caught) {
        throw corrupt(`磁贴 ${t.id} 非法：${caught instanceof Error ? caught.message : String(caught)}`);
      }
      safeTiles.push(safeTile);
    }
    groups.push({id, name: safeName, tiles: safeTiles});
  }
  return groups;
}

/**
 * 稳定键序序列化（排序即承诺：同账本恒同字符串，键序固定
 * id→name→target→kind / id→name→tiles，数组原序保真）。
 */
export function serializeTileSnapshot(groups: readonly TileGroup[]): string {
  const canonical = groups.map((group) => ({
    id: group.id,
    name: group.name,
    tiles: group.tiles.map((tile) => ({
      id: tile.id,
      name: tile.name,
      target: tile.target,
      kind: tile.kind,
    })),
  }));
  return JSON.stringify(canonical);
}

function cloneGroups(groups: readonly TileGroup[]): TileGroup[] {
  return groups.map((group) => ({
    id: group.id,
    name: group.name,
    tiles: group.tiles.map((tile) => ({...tile})),
  }));
}

/**
 * 磁贴账本（适配器注入持久化）。每次变更自动 save；save 失败如实抛。
 * 构造期载荷经 parseTileSnapshot 严格解析（坏数据抛 corrupt-persistence，
 * 调用方如实报错帧，不静默清库）。
 */
export class TilesStore {
  private groups: TileGroup[];
  private readonly adapter: TilesPersistenceAdapter | null;

  constructor(options?: {adapter?: TilesPersistenceAdapter | null}) {
    this.adapter = options?.adapter ?? null;
    this.groups = parseTileSnapshot(this.adapter ? this.adapter.load() : null);
  }

  private persist(next: TileGroup[]): void {
    // 事务语义（宁拒不写）：save 失败如实抛且账本不推进，不产生态与存储分歧。
    if (this.adapter) this.adapter.save(cloneGroups(next));
    this.groups = next;
  }

  /** 快照副本（防外部篡改）。 */
  getGroups(): TileGroup[] {
    return cloneGroups(this.groups);
  }

  addGroup(id: string, name: string): TileGroup[] {
    if (this.groups.some((group) => group.id === id)) {
      throw new TileError('duplicate-id', `分组 id 冲突：${id}`);
    }
    this.persist([...this.groups, createTileGroup(id, name)]);
    return this.getGroups();
  }

  addTile(groupId: string, input: {id: string; name: string; target: string; kind: TileKind}): TileGroup[] {
    this.persist(addTile(this.groups, groupId, input));
    return this.getGroups();
  }

  removeTile(groupId: string, tileId: string): TileGroup[] {
    this.persist(removeTile(this.groups, groupId, tileId));
    return this.getGroups();
  }

  listTiles(groupId: string): Tile[] {
    return listTiles(this.groups, groupId);
  }
}
