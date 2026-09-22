// 记忆卷宗主从化 纯逻辑（Archive 调）——过滤器计数 / 乐观锁修订账本 /
// 缺省态 / graph 关联 / 幽灵编号。DOM-free、零运行时依赖，Node 单测直 import
// （tests/memory-ledger.mjs）。
//
// 诚实边界（契约 §5 rc）：后端记忆 schema 不存 category/importance——两字段
// 缺省（可省略）。本模块只产出真实缺省态（「未分类」/ 不显示），禁编分类。
// 过滤器只能建立在真实字段上：role / protected / status / session_id。

import type {MemoryEpisodeItem} from './types';

// ---- 过滤器 chips（前代产品 范式移植：chips 带计数；§4.4 组件 7）----

/** 可选过滤器：全部 / 他说的 / 主人说的 / 已保护——全部对应真实字段。 */
export type MemoryFilter = 'all' | 'assistant' | 'user' | 'protected';

export const MEMORY_FILTERS: ReadonlyArray<{id: MemoryFilter; label: string}> = [
  {id: 'all', label: '全部'},
  {id: 'assistant', label: '他说的'},
  {id: 'user', label: '主人说的'},
  {id: 'protected', label: '已保护'},
];

export function filterEpisodes(list: MemoryEpisodeItem[], filter: MemoryFilter): MemoryEpisodeItem[] {
  switch (filter) {
    case 'all':
      return list;
    case 'assistant':
      return list.filter((e) => e.role === 'assistant');
    case 'user':
      return list.filter((e) => e.role === 'user');
    case 'protected':
      return list.filter((e) => e.protected === true);
  }
}

/** 各桶计数 = 当前已拉取窗口内的真实条数（窗口口径，UI title 注明）。 */
export function memoryFilterCounts(list: MemoryEpisodeItem[]): Record<MemoryFilter, number> {
  return {
    all: list.length,
    assistant: list.filter((e) => e.role === 'assistant').length,
    user: list.filter((e) => e.role === 'user').length,
    protected: list.filter((e) => e.protected === true).length,
  };
}

// ---- 缺省态（0 装：不编分类）----

/** category 缺省 →「未分类」真实缺省态（契约 §5：schema 不存该字段）。 */
export function episodeCategoryLabel(ep: Pick<MemoryEpisodeItem, 'category'>): string {
  const c = typeof ep.category === 'string' ? ep.category.trim() : '';
  return c || '未分类';
}

/** importance 缺省 → null（UI 不显示该行，而不是编一个 0.5）。 */
export function episodeImportanceText(ep: Pick<MemoryEpisodeItem, 'importance'>): string | null {
  return typeof ep.importance === 'number' && Number.isFinite(ep.importance)
    ? ep.importance.toFixed(2)
    : null;
}

// ---- 乐观锁修订账本（契约 §5：forget/protect/unprotect 带 expected_rev，
//      响应带回新 revision；409 = 别处改过，呈现真实冲突态，不静默重试）----

/** 客户端按 episode id 缓存已知最新 revision；未操作过 = 0（治理账本初始 rev）。 */
export type RevisionLedger = Record<string, number>;

export function expectedRevOf(ledger: RevisionLedger, id: string): number {
  return ledger[id] ?? 0;
}

export function recordRevision(ledger: RevisionLedger, id: string, revision: number): RevisionLedger {
  return {...ledger, [id]: revision};
}

// ---- mutation 结果判别 ----

/** runtime wrapper 的错误分支形状（status 由 postJson 带回；409 = 修订冲突）。 */
export interface MemoryMutationErrorShape {
  error: string;
  status?: number;
}

export type MemoryMutationOutcome =
  | {kind: 'conflict'}
  | {kind: 'error'; message: string};

export function classifyMemoryMutationError(err: MemoryMutationErrorShape): MemoryMutationOutcome {
  if (err.status === 409) return {kind: 'conflict'};
  return {kind: 'error', message: err.error};
}

// ---- graph 关联（契约 §5：nodes{id,label,kind=session|episode} /
//      edges{from,to,weight,label?}，v1 语义 = session→episode 包含边）----

export interface MemoryGraphNode {
  id: string;
  label: string;
  kind: string;
}

export interface MemoryGraphEdge {
  from: string;
  to: string;
  weight?: number;
  label?: string | null;
}

/** 某条 episode 在图谱里的节点与相连边（包含/被包含两个方向都算关联）。 */
export function episodeGraphLinks(
  episodeId: string,
  nodes: MemoryGraphNode[],
  edges: MemoryGraphEdge[],
): {node: MemoryGraphNode | null; edges: MemoryGraphEdge[]} {
  return {
    node: nodes.find((n) => n.id === episodeId) ?? null,
    edges: edges.filter((e) => e.from === episodeId || e.to === episodeId),
  };
}

/** 节点显示名：找不到节点时诚实回落为截断 id（不编 label）。 */
export function graphNodeLabel(id: string, nodes: MemoryGraphNode[]): string {
  const n = nodes.find((item) => item.id === id);
  if (n && n.label.trim()) return n.label;
  return id.length > 12 ? `${id.slice(0, 8)}…` : id;
}

// ---- 幽灵编号（Archive 调：巨型浅灰 01/02… 背景数字，§5.6②）----

export function ghostNumber(index: number): string {
  return String(index + 1).padStart(2, '0');
}

// ---- 时间（契约 §5：timestamp 已从 epoch 秒转换为契约的 epoch 毫秒；
//      容错旧数据仍是秒的口径）----

export function formatEpisodeTime(ts: number): string {
  const ms = ts > 1e11 ? ts : ts * 1000;
  return new Date(ms).toLocaleString('zh-CN', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}
