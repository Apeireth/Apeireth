// 会话清理双档链（0 装：真删不复活 / 失败如实报 / 不谎报清空）纯逻辑
//
// 依据: docs/01-architecture/session-cleanup-options-spec.md（修复预案 + 语义拆分）。
// 病灶（同 session-delete.ts）：清理只动本地账 → 后端账本行经归并补回来（"复活"）；
// 且原"清空本地会话数据"一名只有半符（只清正文、不惊动后端账本）。
//
// 双档语义（红线：本模块绝不触碰记忆库——记忆遗忘是另一个动作，走 CoordinatedForget）：
//   ① recent 清除近期对话记录（保留长期记忆）——近 {RECENT_WINDOW_DAYS_DEFAULT} 天窗口内的
//      会话真删（本地正文 + 后端账本行），窗口外不动；
//   ② all   清除全部会话数据（保留长期记忆）——全部会话真删 + 调用日志全清（C2 裁决位）。
//
// 链上纪律（复用 session-delete.ts 四段）：
//   乐观移除 → 持久 → 后端逐条真删 → 列表刷新；任一步失败如实报，不粉饰。
//   - 本地持久化失败 = 整体回滚、不惊动后端（fullRollback）；
//   - 后端部分失败 = 已成的保持删除、失败项还原保留 + 亮帧列失败 id（不谎报"已清空"）；
//   - 后端账本拉不到 = **取消清理**（没有账本就无法保证"全部"，宁可不动手）。
//   - 404/400（后端本就没有这条）算成功——语义在 runtime.deleteBackendSession 保证。
//
// 本文件刻意零运行时 DOM 依赖（纯函数 + 注入端口），Node 直接 import 可测
// （tests/session-cleanup.mjs），与 session-delete.ts 同一约定。

import type {Conversation} from '../types';
import type {BackendLedgerSession} from './session-list';

/** 清理档位：recent=近窗口对话记录 / all=全部会话数据。 */
export type PurgeScope = 'recent' | 'all';

/** ① 档默认近期窗口（天）。C1 裁决位：7 天；改窗口必须同步确认弹窗文案。 */
export const RECENT_WINDOW_DAYS_DEFAULT = 7;

/** 清理链的注入端点（App 接真实现，测试接假件）。 */
export interface PurgeSessionsPorts {
  /** 当前本地会话列表（乐观移除的快照来源）。 */
  list(): Conversation[];
  /** 写回本地会话列表：乐观移除与失败还原都走它。 */
  setList(next: Conversation[]): void;
  /** 持久化本地列表；抛错 = 持久失败（按失败回滚处理）。 */
  persist(): void;
  /**
   * 后端账本快照（GET /v1/panel/sessions 映射形状）。
   * 由调用方先拉取再注入；拉取失败/不可用必须传 **null**——此时清理整体取消
   * （不谎报"已清空"），不是"跳过后端只清本地"。
   */
  backend(): BackendLedgerSession[] | null;
  /** 后端真删（DELETE /v1/sessions/{id}）；404/400 语义由 runtime 层保证算成功。 */
  deleteRemote(id: string): Promise<void>;
  /**
   * 调用日志清理（C2 裁决位）：'all' = 全清；集合 = 只清这些会话归属的条目。
   * 无 conversationId 归属的条目在集合模式下保留（0 装：无法归属就不冒充归属）。
   */
  clearCallLogs(target: 'all' | ReadonlySet<string>): void;
  /** 失败回调：错误原样交出 + 失败涉及的会话 id（成功部分不进失败列表）。 */
  onError(caught: unknown, failedIds: string[]): void;
}

export interface PurgeOutcome {
  /** 全部真删成功（含"没有需要清除的会话"的空转成功）。 */
  ok: boolean;
  /** 真删成的会话 id。 */
  deletedIds: string[];
  /** 失败保留的会话 id（本地+后端都还原该项，等待重试）。 */
  failedIds: string[];
  /** 本地持久化失败 → 已整体回滚且未惊动后端。 */
  fullRollback: boolean;
}

/** 近期窗口判定：lastActiveAt（epoch ms）落在 [now - 窗口, now] 内。
 *  时间戳为 0/非法（旧数据无从判定）= **不在窗口内**（保守不动手）。 */
export function withinRecentWindow(
  lastActiveAt: number,
  nowMs: number = Date.now(),
  windowDays: number = RECENT_WINDOW_DAYS_DEFAULT,
): boolean {
  if (!Number.isFinite(lastActiveAt) || lastActiveAt <= 0) return false;
  const cutoff = nowMs - windowDays * 86_400_000;
  return lastActiveAt >= cutoff && lastActiveAt <= nowMs;
}

/** 本地会话最后活跃时刻（与归并口径一致：updatedAt ?? createdAt）。 */
function localLastActive(conv: Conversation): number {
  return conv.updatedAt ?? conv.createdAt ?? 0;
}

/** 后端账本行最后活跃时刻（与归并口径一致：last_active_at ?? started_at）。 */
function backendLastActive(row: BackendLedgerSession): number {
  return row.last_active_at || row.started_at || 0;
}

/**
 * 双档真删清理：
 *   乐观移除 → 持久 → 后端逐条真删 → 调用日志清理；失败按链上纪律如实收场。
 * 清理成功的会话 id 由调用方登记（会话已清除登记表 → MemoryView 悬空引用占位）。
 */
export async function purgeSessions(
  scope: PurgeScope,
  ports: PurgeSessionsPorts,
  opts?: {nowMs?: number; windowDays?: number},
): Promise<PurgeOutcome> {
  const nowMs = opts?.nowMs ?? Date.now();
  const windowDays = opts?.windowDays ?? RECENT_WINDOW_DAYS_DEFAULT;
  const snapshot = ports.list();
  const backend = ports.backend();

  // 后端账本拿不到 = 取消清理：无法保证真删到位，宁可不动手也不谎报。
  if (backend === null) {
    const err = new Error('无法读取后端会话账本——为避免"清了又复活/谎报已清空"，本次清理已取消');
    ports.onError(err, []);
    return {ok: false, deletedIds: [], failedIds: [], fullRollback: false};
  }

  // 目标集合：本地 ∪ 后端账本（两账都清，列表才不会被陈账行补回来）。
  const targets = new Set<string>();
  const inScope = (lastActive: number): boolean =>
    scope === 'all' ? true : withinRecentWindow(lastActive, nowMs, windowDays);
  for (const conv of snapshot) {
    if (inScope(localLastActive(conv))) targets.add(conv.id);
  }
  for (const row of backend) {
    if (inScope(backendLastActive(row))) targets.add(row.id);
  }
  if (targets.size === 0) {
    return {ok: true, deletedIds: [], failedIds: [], fullRollback: false};
  }

  // ① 乐观移除 + ② 持久：持久失败 = 整体回滚，且不惊动后端。
  const next = snapshot.filter((item) => !targets.has(item.id));
  ports.setList(next);
  try {
    ports.persist();
  } catch (caught) {
    ports.setList(snapshot);
    try {
      ports.persist();
    } catch {
      // 回滚持久化尽力而为：错误帧已按 caught 亮出，不叠加第二笔噪音。
    }
    ports.onError(caught, []);
    return {ok: false, deletedIds: [], failedIds: [], fullRollback: true};
  }

  // ③ 后端逐条真删：成功保持删除，失败项还原保留（"删除不算数"）。
  const deleted: string[] = [];
  const failed: string[] = [];
  for (const id of targets) {
    try {
      await ports.deleteRemote(id);
      deleted.push(id);
    } catch {
      failed.push(id);
    }
  }

  if (failed.length > 0) {
    const deletedSet = new Set(deleted);
    ports.setList(snapshot.filter((item) => !deletedSet.has(item.id)));
    try {
      ports.persist();
    } catch {
      // 还原持久化尽力而为：失败清单已亮帧。
    }
    if (deleted.length > 0) ports.clearCallLogs(new Set(deleted));
    const err = new Error(
      `会话清理部分失败：${deleted.length} 个已删除，${failed.length} 个未能删除（已保留原样），失败会话 id：${failed.join(', ')}`,
    );
    ports.onError(err, failed);
    return {ok: false, deletedIds: deleted, failedIds: failed, fullRollback: false};
  }

  // ④ 调用日志清理（C2）：all 档全清；recent 档只清已删会话归属的条目。
  ports.clearCallLogs(scope === 'all' ? 'all' : new Set(deleted));
  return {ok: true, deletedIds: deleted, failedIds: [], fullRollback: false};
}

// ---------------------------------------------------------------------------
// 会话已清除登记表（MemoryView 悬空引用占位用）
//
// 只登记**本运行期真删成**的会话 id：MemoryView 对已登记 id 显示"会话已清除"
// 占位（记忆行完好、可继续过滤），对未登记 id 不做任何判定——0 装：重启后
// 登记表为空，占位消失但记忆照常显示，绝不冒充"已清除"。
// ---------------------------------------------------------------------------

const clearedSessions = new Set<string>();

/** 登记真删成的会话 id（清理链/单会话删除链的调用方在成功后调用）。 */
export function markSessionsCleared(ids: Iterable<string>): void {
  for (const id of ids) clearedSessions.add(id);
}

/** 该会话是否在本运行期被真删过。 */
export function isSessionCleared(id: string): boolean {
  return clearedSessions.has(id);
}

/** 清空登记表（测试用）。 */
export function resetClearedSessions(): void {
  clearedSessions.clear();
}
