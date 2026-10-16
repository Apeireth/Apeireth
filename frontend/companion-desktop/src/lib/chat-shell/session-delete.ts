// 会话删除链（0 装：确认即真删 / 失败即回滚 + 错误帧）纯逻辑
//
// 病灶（内测实测）：确认删除后列表不消失，重启后又"复活"。根因是删除只动了
// 本地账（localStorage），而列表是「本地 ⋈ 后端账本」归并——后端账本行还在，
// 归并立刻把它补回来。真删必须打到后端删除端点（DELETE /v1/sessions/{id}），
// 并且在失败时把乐观移除整个回滚（列表 + 持久 + 陈账排除），同时亮错误帧。
//
// 链上三段（缺一即"像没删"）：
//   ① 前端乐观移除 —— 确认后立即从列表消失（不等网络）；
//   ② 后端删除端点 —— 真删后端账本行，重启后不复活；
//   ③ 列表刷新 —— 成功后对齐一次后端账本（reloadKey 节拍由调用方 bump）。
//
// 本文件刻意零运行时 DOM 依赖（纯函数 + 注入端口），Node 直接 import 可测
// （tests/session-delete.mjs），与 session-list.ts 同一约定。

import type {Conversation} from '../types';

/** 删除链的注入端口（App 接真实现，测试接假件）。 */
export interface DeleteSessionPorts {
  /** 当前本地会话列表（乐观移除的快照来源）。 */
  list(): Conversation[];
  /** 写回本地会话列表：乐观移除与失败回滚都走它。 */
  setList(next: Conversation[]): void;
  /** 持久化本地列表；抛错 = 持久失败（同样按失败回滚处理）。 */
  persist(): void;
  /**
   * 后端真删（DELETE /v1/sessions/{id}）。
   * 约定：后端回「会话不存在」（404 session_not_found）也算成功——本机草稿
   * 从未进过他的账本，没有可删的后端记录；其余任何失败必须抛错。
   */
  deleteRemote(id: string): Promise<void>;
  /** 失败回调：错误对象原样交出，调用方据此亮错误帧（不吞、不粉饰）。 */
  onError(caught: unknown): void;
}

export interface DeleteSessionOutcome {
  /** 删除完成（本地 + 后端都已真删）。 */
  ok: boolean;
  /** 是否执行了失败回滚（回滚证据：列表已还原）。 */
  rolledBack: boolean;
  /** 乐观移除是否已持久成功。 */
  persisted: boolean;
}

/**
 * 真删一个会话：
 *   乐观移除（立即消失）→ 持久 → 后端真删 → 任一步失败即整体回滚 + 亮帧。
 * 回滚 = 列表还原到删除前快照（持久化尽力而为），并撤销一切乐观标记由
 * 调用方按 onError 之后的 rolledBack 证据自行收尾。
 */
export async function deleteSession(
  id: string,
  ports: DeleteSessionPorts,
): Promise<DeleteSessionOutcome> {
  const snapshot = ports.list();
  const next = snapshot.filter((item) => item.id !== id);

  // ① 乐观移除：确认后立即从列表消失（同步生效，不等网络）。
  ports.setList(next);
  try {
    ports.persist();
  } catch (caught) {
    ports.setList(snapshot);
    ports.onError(caught);
    return {ok: false, rolledBack: true, persisted: false};
  }

  // ② 后端真删：重启不复活的根因面。失败 = 整个删除不算数。
  try {
    await ports.deleteRemote(id);
  } catch (caught) {
    ports.setList(snapshot);
    try {
      ports.persist();
    } catch {
      // 回滚持久化尽力而为：错误帧已按 caught 亮出，不再叠加第二笔噪音。
    }
    ports.onError(caught);
    return {ok: false, rolledBack: true, persisted: false};
  }

  return {ok: true, rolledBack: false, persisted: true};
}
