// 对话流滚动策略纯逻辑（打开滚底 / 上翻不拽回）
//
// 病灶（内测实测）：点开会话停在最上面（最老消息）——应自动滚到底部
// （最新消息 + 输入框 = 继续工作的位置）。新开/切换会话都要落底；
// 但用户主动上翻看历史后，流式追加绝不把人强行拽回底部。
//
// 判定常量与既有 handleScroll 行为保持一致（80px 贴底 / 150px 亮浮钮），
// 只是把口径从组件里提出来变成可单测的纯函数（tests/scroll-policy.mjs）。

/** 距底小于该值 = 贴底（流式跟随的前提）。 */
export const NEAR_BOTTOM_PX = 80;

/** 距底大于该值 = 亮「回到底部」浮钮。 */
export const SHOW_JUMP_PX = 150;

/** 滚动指令：bottom=落到底部；follow=跟随新内容；hold=原地不动。 */
export type ScrollDirective = 'bottom' | 'follow' | 'hold';

/** 打开会话的来源：新建 / 切换（两者都必须落底）。 */
export type OpenKind = 'new' | 'switch';

/** 距底距离 → 是否贴底。 */
export function isNearBottom(distanceToBottom: number): boolean {
  return distanceToBottom < NEAR_BOTTOM_PX;
}

/** 距底距离 → 是否亮「回到底部」浮钮。 */
export function showJumpButton(distanceToBottom: number): boolean {
  return distanceToBottom > SHOW_JUMP_PX;
}

/**
 * 打开/切换会话（含新建空会话）：一律滚到底部。
 * 上一屏停在哪不重要——新会话的第一眼应该是"最新 + 能继续写"。
 */
export function scrollOnOpen(_kind: OpenKind): ScrollDirective {
  return 'bottom';
}

/**
 * 流式追加 / 新卡片浮起时的滚动指令：
 * 用户仍贴底 → 跟随；已上翻（哪怕只翻过 80px）→ 原地不动（不拽回）。
 */
export function scrollOnAppend(nearBottom: boolean): ScrollDirective {
  return nearBottom ? 'follow' : 'hold';
}
