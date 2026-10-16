// 泊位 · 收纳面纯逻辑（I 批 3 切片，零依赖纯模块）
//
// 契约（I-starport-builtins.md「泊位」）：
//   1. 切片边界如实 —— MVP 是「收纳分区/快捷位管理」：分区（组）+ 快捷位
//      （名称 + 目标描述），真实文件夹映射磁贴待原生文件面接线（在册扩展位）；
//   2. 0 装 —— 名称空/超限抛错、id 冲突抛错、跨组移动组不存在抛错；
//   3. 持久化经注入适配器（load/save），本模块零存储依赖。
//
// 测试：tests/starport-organize.mjs（真实模块直测）。

export interface Shortcut {
  id: string;
  name: string;
  /** 目标描述（路径/网址/说明——真实文件映射待原生面，如实存文字目标）。 */
  target: string;
}

export interface Group {
  id: string;
  name: string;
  shortcuts: Shortcut[];
}

const NAME_LIMIT = 120;
const TARGET_LIMIT = 2000;

function checkName(name: string): string {
  const value = typeof name === 'string' ? name.trim() : '';
  if (!value) throw new Error('名称不能为空');
  if (value.length > NAME_LIMIT) throw new Error(`名称超限（>${NAME_LIMIT}）`);
  return value;
}

function checkTarget(target: string): string {
  const value = typeof target === 'string' ? target.trim() : '';
  if (!value) throw new Error('快捷位目标不能为空');
  if (value.length > TARGET_LIMIT) throw new Error(`目标超限（>${TARGET_LIMIT}）`);
  return value;
}

export function createGroup(id: string, name: string): Group {
  return {id, name: checkName(name), shortcuts: []};
}

export function addShortcut(
  groups: readonly Group[],
  groupId: string,
  shortcut: {id: string; name: string; target: string},
): Group[] {
  const group = groups.find((g) => g.id === groupId);
  if (!group) throw new Error(`分区不存在：${groupId}`);
  if (group.shortcuts.some((s) => s.id === shortcut.id)) {
    throw new Error(`快捷位 id 冲突：${shortcut.id}`);
  }
  const next: Shortcut = {
    id: shortcut.id,
    name: checkName(shortcut.name),
    target: checkTarget(shortcut.target),
  };
  return groups.map((g) =>
    g.id === groupId ? {...g, shortcuts: [...g.shortcuts, next]} : g,
  );
}

export function removeShortcut(groups: readonly Group[], groupId: string, shortcutId: string): Group[] {
  const group = groups.find((g) => g.id === groupId);
  if (!group) throw new Error(`分区不存在：${groupId}`);
  if (!group.shortcuts.some((s) => s.id === shortcutId)) {
    throw new Error(`快捷位不存在：${shortcutId}`);
  }
  return groups.map((g) =>
    g.id === groupId ? {...g, shortcuts: g.shortcuts.filter((s) => s.id !== shortcutId)} : g,
  );
}

/** 跨组移动快捷位（两组都必须存在，快捷位必须在源组）。 */
export function moveShortcut(
  groups: readonly Group[],
  shortcutId: string,
  fromGroupId: string,
  toGroupId: string,
): Group[] {
  const from = groups.find((g) => g.id === fromGroupId);
  const to = groups.find((g) => g.id === toGroupId);
  if (!from) throw new Error(`源分区不存在：${fromGroupId}`);
  if (!to) throw new Error(`目标分区不存在：${toGroupId}`);
  const shortcut = from.shortcuts.find((s) => s.id === shortcutId);
  if (!shortcut) throw new Error(`快捷位不在源分区：${shortcutId}`);
  return groups.map((g) => {
    if (g.id === fromGroupId) return {...g, shortcuts: g.shortcuts.filter((s) => s.id !== shortcutId)};
    if (g.id === toGroupId) return {...g, shortcuts: [...g.shortcuts, shortcut]};
    return g;
  });
}
