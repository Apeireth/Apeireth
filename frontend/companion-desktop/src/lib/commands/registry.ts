// Ctrl+K 命令面板 —— 注册表纯逻辑（00-PHILOSOPHY §6 原则 3「一个入口」）。
//
// 纪律：
// - 注册表驱动：命令是数据（id/标题/别名/分组），执行闭包由 App 接线时注入；
//   本模块不 import 任何视图/运行时，保持 Node 单测可跑。
// - 0 装：能力不可用的命令显示但置灰（disabledReason），不藏也不假装可点。
// - 最近优先：localStorage 诚实持久化（RECENT_STORAGE_KEY），封顶 8 条；
//   置灰命令被执行时不得入账（调用方责任，测试覆盖纯函数部分）。

export interface CommandItem {
  id: string;
  /** 主标题（中文，可直搜）。 */
  title: string;
  /** 别名（拼音/英文缩写，小写比较）。 */
  aliases: readonly string[];
  /** 分组小字（导航 / 主题 / 动作…）。 */
  group: string;
  /** 右侧提示小字（快捷键、当前值等）。 */
  hint?: string;
  /** 0 装：不可用原因；存在即置灰不可执行。 */
  disabledReason?: string;
}

/** 标题/别名小写子串匹配；空串返回全量（保持传入顺序）。 */
export function filterCommands(
  items: readonly CommandItem[],
  query: string,
): CommandItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return [...items];
  return items.filter(
    (item) =>
      item.title.toLowerCase().includes(q) ||
      item.aliases.some((alias) => alias.toLowerCase().includes(q)),
  );
}

/**
 * 最近优先排序：recentIds 数组序即新旧（0 = 最近一次）；
 * 未入榜的保持注册序殿后。recentIds 里的未知 id 静默忽略（诚实容忍陈旧数据）。
 * 稳定排序，不改动传入数组。
 */
export function orderCommands(
  items: readonly CommandItem[],
  recentIds: readonly string[],
): CommandItem[] {
  const rank = new Map<string, number>();
  recentIds.forEach((id, index) => {
    if (!rank.has(id)) rank.set(id, index);
  });
  return [...items].sort((a, b) => {
    const ra = rank.get(a.id);
    const rb = rank.get(b.id);
    if (ra === undefined && rb === undefined) return 0;
    if (ra === undefined) return 1;
    if (rb === undefined) return -1;
    return ra - rb;
  });
}

/** 记录一次使用：置顶去重，封顶 cap 条。 */
export function pushRecentId(
  recent: readonly string[],
  id: string,
  cap = 8,
): string[] {
  return [id, ...recent.filter((existing) => existing !== id)].slice(0, cap);
}

export const RECENT_STORAGE_KEY = 'apeireth.command.recent.v1';

interface StorageLike {
  getItem(key: string): string | null;
}

interface StorageWriter {
  setItem(key: string, value: string): void;
}

function defaultStorage(): StorageLike & StorageWriter {
  return typeof localStorage !== 'undefined'
    ? localStorage
    : {getItem: () => null, setItem: () => {}};
}

/** 读取最近榜单：JSON 损坏/形状不符一律回落空榜（不猜、不修数据）。 */
export function loadRecentIds(storage: StorageLike = defaultStorage()): string[] {
  try {
    const raw = storage.getItem(RECENT_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((id): id is string => typeof id === 'string');
  } catch {
    return [];
  }
}

/** 持久化最近榜单（诚实写盘；失败静默——榜单丢了不碍事，命令本身不丢）。 */
export function saveRecentIds(
  ids: readonly string[],
  storage: StorageWriter = defaultStorage(),
): void {
  try {
    storage.setItem(RECENT_STORAGE_KEY, JSON.stringify(ids));
  } catch {
    // 存储满/隐私模式：最近优先退化为注册序，可接受。
  }
}
