// 侧栏收纳（IA）— 导航目标解析纯逻辑
//
// 内测实测反馈：左栏常驻九件太杂。收敛为四件——对话 / 工具 / 状态 / 设置；
// 其余页面换家（不是删除功能）：
//   历史  → 撤除入口（会话列表 + 搜索即历史；列表栏常驻即是历史卷宗）；
//   记忆  → 工作台（右上「工作台」按钮 → 记忆卷宗卡片，面板组件原样复用）；
//   日记  → 工作台（同上，他的日记卡片）；
//   治理  → 设置 ›「安全与治理」区（治理面板组件原样嵌入）；
//   日志  → 并入「状态」（活动与调用日志，面板组件原样复用）。
//
// 命令面板 nav.* 与旧深链 ?drawer=<id> 都由本表统一改道，避免三处口径漂移。
// 纯函数零 DOM 依赖，Node 直接 import 可测（tests/shell-nav.mjs）。

/** 侧栏常驻四件（顺序即渲染顺序）。 */
export type RailItem = 'chat' | 'tools' | 'status' | 'settings';

export const RAIL_ITEMS: readonly RailItem[] = ['chat', 'tools', 'status', 'settings'];

/** 工作台分区：回合视图（默认）/ 记忆卷宗 / 他的日记。 */
export type WorkbenchSection = 'turn' | 'memory' | 'diary';

/** 设置分区里承接旧治理页的那一个。 */
export const SETTINGS_SECURITY_SECTION = 'security';

/** 壳层导航目标：去哪一块、带哪个分区。 */
export type ShellTarget =
  | {kind: 'chat'}
  | {kind: 'drawer'; id: 'tools' | 'status' | 'settings'; section?: typeof SETTINGS_SECURITY_SECTION}
  | {kind: 'workbench'; section: WorkbenchSection};

/**
 * 旧深链 ?drawer=<id> → 新位置（被移页面的重定向表）。
 * 认不出的值返回 null（调用方落默认视图，不猜）。
 */
export function resolveLegacyDrawer(query: string | null): ShellTarget | null {
  switch (query) {
    case 'history':
      // 历史入口撤除：会话列表即历史，回到列表主页。
      return {kind: 'chat'};
    case 'memory':
      return {kind: 'workbench', section: 'memory'};
    case 'diary':
      return {kind: 'workbench', section: 'diary'};
    case 'governance':
      return {kind: 'drawer', id: 'settings', section: SETTINGS_SECURITY_SECTION};
    case 'logs':
      return {kind: 'drawer', id: 'status'};
    case 'tools':
      return {kind: 'drawer', id: 'tools'};
    case 'status':
      return {kind: 'drawer', id: 'status'};
    case 'settings':
      return {kind: 'drawer', id: 'settings'};
    default:
      return null;
  }
}

/**
 * 命令面板 nav.* → 新位置（与深链同一张表的镜像）。
 * nav.history 已随历史入口撤除；旧别名（lishi/history）不复存在。
 */
export const NAV_COMMAND_TARGETS: Readonly<Record<string, ShellTarget>> = {
  'nav.chat': {kind: 'chat'},
  'nav.tools': {kind: 'drawer', id: 'tools'},
  'nav.status': {kind: 'drawer', id: 'status'},
  'nav.settings': {kind: 'drawer', id: 'settings'},
  'nav.memory': {kind: 'workbench', section: 'memory'},
  'nav.diary': {kind: 'workbench', section: 'diary'},
  'nav.governance': {kind: 'drawer', id: 'settings', section: SETTINGS_SECURITY_SECTION},
  'nav.logs': {kind: 'drawer', id: 'status'},
};

/** 工作台两张卡片入口（记忆 / 日记）。 */
export const WORKBENCH_CARDS: ReadonlyArray<{section: WorkbenchSection; title: string; sub: string}> = [
  {section: 'memory', title: '记忆卷宗', sub: '情节记忆 · 检索 · 图谱 · 保护治理'},
  {section: 'diary', title: '他的日记', sub: '纸面档案调 · 空态契约页'},
];
