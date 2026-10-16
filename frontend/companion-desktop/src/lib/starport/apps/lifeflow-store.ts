// 信风 · 生活信息流纯逻辑（J 想法 MVP 切片，零依赖纯模块）
//
// 契约（J-tradewind.md）：
//   1. 切片边界如实 —— MVP 是「信息源登记 + 条目流管理」：多源订阅的统一
//      条目流抽象；多源抓取适配待深望抓取面扩批；
//   2. 注意力纪律 —— 条目带已读态与配额计数（刷流时长闸的计数面在此，
//      闸值本身归配额真接口 A）；
//   3. 0 装 —— 源/条目 id 冲突抛错、URL 非 HTTP(S) 抛错、标记不存在抛错。
//
// 测试：tests/starport-lifeflow.mjs（真实模块直测）。

export interface FeedSource {
  id: string;
  name: string;
  /** 信息源地址（只收 HTTP/HTTPS，与深望准入同一协议闸）。 */
  url: string;
}

export interface FeedItem {
  id: string;
  sourceId: string;
  title: string;
  summary: string | null;
  at: number;
  read: boolean;
}

function checkHttpUrl(value: string): string {
  const raw = typeof value === 'string' ? value.trim() : '';
  let parsed: URL;
  try {
    parsed = new URL(raw);
  } catch {
    throw new Error('信息源地址非法');
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error('信息源只收 HTTP(S)');
  }
  return parsed.href;
}

export function addSource(
  sources: readonly FeedSource[],
  source: {id: string; name: string; url: string},
): FeedSource[] {
  if (sources.some((s) => s.id === source.id)) throw new Error(`信息源 id 冲突：${source.id}`);
  const name = typeof source.name === 'string' ? source.name.trim() : '';
  if (!name) throw new Error('信息源名称不能为空');
  return [...sources, {id: source.id, name, url: checkHttpUrl(source.url)}];
}

export function addItems(items: readonly FeedItem[], incoming: readonly FeedItem[]): FeedItem[] {
  const seen = new Set(items.map((item) => item.id));
  for (const item of incoming) {
    if (seen.has(item.id)) throw new Error(`条目 id 冲突：${item.id}`);
    seen.add(item.id);
    if (!item.title.trim()) throw new Error('条目标题不能为空');
  }
  return [...items, ...incoming.map((item) => ({...item, read: false}))];
}

export function markRead(items: readonly FeedItem[], id: string, read: boolean): FeedItem[] {
  if (!items.some((item) => item.id === id)) throw new Error(`条目不存在：${id}`);
  return items.map((item) => (item.id === id ? {...item, read} : item));
}

/** 未读计数（配额计数面：刷流闸值归想法 A）。 */
export function countUnread(items: readonly FeedItem[]): number {
  return items.filter((item) => !item.read).length;
}

/** 条目流视图：最新在前，可只看未读。 */
export function listItems(items: readonly FeedItem[], options: {unreadOnly?: boolean} = {}): FeedItem[] {
  const filtered = options.unreadOnly ? items.filter((item) => !item.read) : [...items];
  return filtered.sort((a, b) => b.at - a.at);
}
